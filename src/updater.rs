use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const UPDATE_CHECK_URL: &str = "https://api.github.com/repos/Xenonesis/sysmon/releases/latest";
const MAX_INSTALLER_BYTES: u64 = 100 * 1024 * 1024;
const MAX_RELEASE_METADATA_BYTES: u64 = 1024 * 1024;
const MAX_CHECKSUM_BYTES: u64 = 4 * 1024;

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(60))
        .timeout_write(std::time::Duration::from_secs(30))
        .build()
}

pub(crate) fn validate_asset_url(url: &str) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    if !lower.starts_with("https://github.com/xenonesis/sysmon/releases/download/") || !lower.ends_with(".exe") {
        return Err("Unexpected update asset URL".into());
    }
    Ok(())
}

fn validate_checksum_url(url: &str) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    if !lower.starts_with("https://github.com/xenonesis/sysmon/releases/download/") || !lower.ends_with(".exe.sha256") {
        return Err("Unexpected checksum asset URL".into());
    }
    Ok(())
}

// Must match the AppId in installer.iss.
const INSTALLER_APP_ID: &str = "{3F2A9C41-8E7D-4B6A-9C21-5D8E4F1A7B62}";

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub download_url: String,
    pub checksum_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

impl Default for UpdateInfo {
    fn default() -> Self {
        Self {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: CURRENT_VERSION.to_string(),
            update_available: false,
            download_url: String::new(),
            checksum_url: String::new(),
        }
    }
}

pub struct Updater {
    update_info: UpdateInfo,
}

impl Clone for Updater {
    fn clone(&self) -> Self {
        Self {
            update_info: self.update_info.clone(),
        }
    }
}

impl Updater {
    pub fn new() -> Self {
        Self {
            update_info: UpdateInfo::default(),
        }
    }

    pub fn check_for_updates(&mut self) -> Result<UpdateInfo, String> {
        // Updates apply to installed apps only — portable builds are no longer
        // published, so a portable exe would have nothing safe to download.
        if !self.is_installed() {
            return Ok(UpdateInfo::default());
        }

        match self.fetch_latest_release() {
            Ok(release) => {
                let latest_version = release.tag_name.trim_start_matches('v');
                let current_version = CURRENT_VERSION;

                self.update_info.latest_version = latest_version.to_string();
                self.update_info.update_available = self.is_newer_version(current_version, latest_version);

                // Clear any URLs from a previous check in this session.
                self.update_info.download_url.clear();
                self.update_info.checksum_url.clear();

                // Only the installer asset (SystemMonitor-<ver>-setup.exe) is
                // offered, and only together with the SHA-256 checksum file
                // published next to it. Without a published checksum the
                // download cannot be verified, so no update is offered.
                for asset in release.assets {
                    let name = asset.name.to_lowercase();
                    if name.contains("setup") && name.ends_with(".exe.sha256") {
                        self.update_info.checksum_url = asset.browser_download_url;
                    } else if name.contains("setup") && name.ends_with(".exe") {
                        self.update_info.download_url = asset.browser_download_url;
                    }
                }
                self.update_info.update_available = self.update_info.update_available
                    && !self.update_info.download_url.is_empty()
                    && !self.update_info.checksum_url.is_empty();

                Ok(self.update_info.clone())
            }
            Err(e) => Err(format!("Failed to check for updates: {}", e)),
        }
    }

    fn fetch_latest_release(&self) -> Result<GitHubRelease, String> {
        let response = http_agent()
            .get(UPDATE_CHECK_URL)
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "SystemMonitor/1.0")
            .call()
            .map_err(|e| format!("Failed to fetch release info: {}", e))?;

        let mut body = String::new();
        response
            .into_reader()
            .take(MAX_RELEASE_METADATA_BYTES + 1)
            .read_to_string(&mut body)
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        if body.len() as u64 > MAX_RELEASE_METADATA_BYTES {
            return Err("Release metadata exceeds size limit".into());
        }

        serde_json::from_str(&body).map_err(|e| format!("Failed to parse GitHub response: {}", e))
    }

    fn is_newer_version(&self, current: &str, latest: &str) -> bool {
        let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
        let latest_parts: Vec<u32> = latest.split('.').filter_map(|s| s.parse().ok()).collect();

        for i in 0..3 {
            let curr = current_parts.get(i).unwrap_or(&0);
            let lat = latest_parts.get(i).unwrap_or(&0);

            match lat.cmp(curr) {
                std::cmp::Ordering::Greater => return true,
                std::cmp::Ordering::Less => return false,
                std::cmp::Ordering::Equal => {}
            }
        }

        false
    }

    fn is_installed(&self) -> bool {
        if let Ok(exe) = std::env::current_exe() {
            let path = exe.to_string_lossy().to_lowercase();
            if path.contains("\\program files\\") || path.contains("\\program files (x86)\\") {
                return true;
            }
        }
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            let key = format!(
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{}_is1",
                INSTALLER_APP_ID
            );
            if hklm.open_subkey(&key).is_ok() {
                return true;
            }
        }
        false
    }

    fn fetch_expected_checksum(&self, checksum_url: &str) -> Result<String, String> {
        let response = http_agent()
            .get(checksum_url)
            .set("User-Agent", "SystemMonitor/1.0")
            .call()
            .map_err(|e| format!("Failed to download checksum: {}", e))?;

        let mut body = String::new();
        response
            .into_reader()
            .take(MAX_CHECKSUM_BYTES + 1)
            .read_to_string(&mut body)
            .map_err(|e| format!("Failed to read checksum file: {e}"))?;
        if body.len() as u64 > MAX_CHECKSUM_BYTES {
            return Err("Checksum file exceeds size limit".into());
        }

        parse_checksum_file(&body).ok_or_else(|| "Checksum file contains no SHA-256 hash".to_string())
    }

    pub fn download_and_install_update(&self, download_url: &str, checksum_url: &str) -> Result<(), String> {
        validate_asset_url(download_url)?;
        validate_checksum_url(checksum_url)?;
        let expected_sha256 = self.fetch_expected_checksum(checksum_url)?;

        // Download the update using ureq
        let response = http_agent()
            .get(download_url)
            .set("User-Agent", "SystemMonitor/1.0")
            .call()
            .map_err(|e| format!("Failed to download update: {}", e))?;
        if response
            .header("Content-Length")
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|size| size > MAX_INSTALLER_BYTES)
        {
            return Err("Update installer exceeds size limit".into());
        }

        let mut bytes = Vec::new();
        response
            .into_reader()
            .take(MAX_INSTALLER_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("Failed to read update file: {e}"))?;
        if bytes.len() as u64 > MAX_INSTALLER_BYTES {
            return Err("Update installer exceeds size limit".into());
        }

        // Verify the downloaded installer against the SHA-256 checksum
        // published with the release before writing or executing it.
        verify_sha256(&bytes, &expected_sha256)?;

        let unique = format!(
            "system-monitor-setup-{}-{}.exe",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        );
        let installer_path = std::env::temp_dir().join(unique);
        fs::write(&installer_path, &bytes).map_err(|e| format!("Failed to write update file: {}", e))?;

        // Silent install — replaces the exe, shortcuts, and uninstall entry in
        // one pass. Only installer assets are offered; the process exits so the
        // installer can replace files freely.
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;
            Command::new(&installer_path)
                .creation_flags(0x08000000)
                .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
                .spawn()
                .map_err(|e| {
                    let _ = fs::remove_file(&installer_path);
                    format!("Failed to spawn installer: {}", e)
                })?;
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = fs::remove_file(&installer_path);
            Err("Installer updates are only supported on Windows".to_string())
        }
    }

    #[allow(dead_code)]
    pub fn get_update_info(&self) -> &UpdateInfo {
        &self.update_info
    }
}

impl Clone for UpdateInfo {
    fn clone(&self) -> Self {
        Self {
            current_version: self.current_version.clone(),
            latest_version: self.latest_version.clone(),
            update_available: self.update_available,
            download_url: self.download_url.clone(),
            checksum_url: self.checksum_url.clone(),
        }
    }
}

/// Parses a `sha256sum`-style file (`<hash> *<name>` or `<hash>  <name>`) and
/// returns the first 64-hex-digit hash, lowercased.
fn parse_checksum_file(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(hash) = line.split_whitespace().next() {
            if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(hash.to_ascii_lowercase());
            }
        }
    }
    None
}

fn hex_digest(digest: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(digest.len() * 2);
    for &b in digest {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn verify_sha256(bytes: &[u8], expected_hex: &str) -> Result<(), String> {
    let digest = Sha256::digest(bytes);
    let actual = hex_digest(&digest);
    if actual.eq_ignore_ascii_case(expected_hex.trim()) {
        Ok(())
    } else {
        Err(format!(
            "Installer SHA-256 mismatch: expected {expected_hex}, got {actual}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_expected_release_asset() {
        assert!(validate_asset_url(
            "https://github.com/Xenonesis/sysmon/releases/download/v2.6.0/SystemMonitor-2.6.0-setup.exe"
        )
        .is_ok());
    }

    #[test]
    fn rejects_untrusted_asset_urls() {
        for url in [
            "http://github.com/Xenonesis/sysmon/releases/download/v2/a.exe",
            "https://evil.example/a.exe",
            "https://github.com/other/sysmon/releases/download/v2/a.exe",
            "https://github.com/Xenonesis/sysmon/releases/download/v2/a.zip",
        ] {
            assert!(validate_asset_url(url).is_err(), "accepted {url}");
        }
    }

    #[test]
    fn accepts_expected_checksum_asset() {
        assert!(validate_checksum_url(
            "https://github.com/Xenonesis/sysmon/releases/download/v3.7.2/SystemMonitor-3.7.2-setup.exe.sha256"
        )
        .is_ok());
    }

    #[test]
    fn rejects_untrusted_checksum_urls() {
        for url in [
            "http://github.com/Xenonesis/sysmon/releases/download/v3/a.exe.sha256",
            "https://evil.example/a.exe.sha256",
            "https://github.com/other/sysmon/releases/download/v3/a.exe.sha256",
            // The installer itself is not a valid checksum asset.
            "https://github.com/Xenonesis/sysmon/releases/download/v3/a.exe",
        ] {
            assert!(validate_checksum_url(url).is_err(), "accepted {url}");
        }
    }

    #[test]
    fn parses_sha256_checksum_file() {
        let hash = "a1".repeat(32);
        // sha256sum binary-mode format, as published by the release workflow.
        assert_eq!(
            parse_checksum_file(&format!("{hash} *SystemMonitor-3.7.2-setup.exe")),
            Some(hash.clone())
        );
        // GNU coreutils text-mode (two-space) format.
        assert_eq!(
            parse_checksum_file(&format!("{hash}  SystemMonitor-3.7.2-setup.exe\n")),
            Some(hash.clone())
        );
        // Uppercase hashes are normalized.
        assert_eq!(
            parse_checksum_file(&format!("{} *setup.exe", hash.to_uppercase())),
            Some(hash)
        );
        // Empty, missing or malformed content yields no hash.
        assert_eq!(parse_checksum_file(""), None);
        assert_eq!(parse_checksum_file("nothash *setup.exe"), None);
        assert_eq!(parse_checksum_file(&format!("{} *setup.exe", "z".repeat(64))), None);
    }

    #[test]
    fn verifies_installer_checksum() {
        let bytes = b"sysmon installer payload";
        let digest = Sha256::digest(bytes);
        let hex = hex_digest(&digest);
        assert!(verify_sha256(bytes, &hex).is_ok());
        // Case-insensitive comparison against the published hash.
        assert!(verify_sha256(bytes, &hex.to_uppercase()).is_ok());
        // Any tampering changes the digest and must be rejected.
        assert!(verify_sha256(b"tampered installer payload", &hex).is_err());
    }
}
