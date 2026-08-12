use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const UPDATE_CHECK_URL: &str = "https://api.github.com/repos/Xenonesis/sysmon/releases/latest";
const MAX_INSTALLER_BYTES: u64 = 100 * 1024 * 1024;
const MAX_RELEASE_METADATA_BYTES: u64 = 1024 * 1024;
const SIGNER_THUMBPRINT: &str = match option_env!("SYSMON_SIGNER_THUMBPRINT") {
    Some(value) => value,
    None => "",
};

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

// Must match the AppId in installer.iss.
const INSTALLER_APP_ID: &str = "{3F2A9C41-8E7D-4B6A-9C21-5D8E4F1A7B62}";

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub download_url: String,
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

                // Clear any URL from a previous check in this session.
                self.update_info.download_url.clear();

                // Only the installer asset (SystemMonitor-<ver>-setup.exe) is
                // offered; portable builds are no longer published.
                for asset in release.assets {
                    let name = asset.name.to_lowercase();
                    if name.contains("setup") && name.ends_with(".exe") {
                        self.update_info.download_url = asset.browser_download_url;
                        break;
                    }
                }
                self.update_info.update_available =
                    self.update_info.update_available && !self.update_info.download_url.is_empty();

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

            if lat > curr {
                return true;
            } else if lat < curr {
                return false;
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

    pub fn download_and_install_update(&self, download_url: &str) -> Result<(), String> {
        validate_asset_url(download_url)?;
        let unique = format!(
            "system-monitor-setup-{}-{}.exe",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        );
        let installer_path = std::env::temp_dir().join(unique);

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
        };

        fs::write(&installer_path, &bytes).map_err(|e| format!("Failed to write update file: {}", e))?;

        // Silent install — replaces the exe, shortcuts, and uninstall entry in
        // one pass. Only installer assets are offered; the process exits so the
        // installer can replace files freely.
        {
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                use std::process::Command;
                if let Err(error) = verify_authenticode(&installer_path) {
                    let _ = fs::remove_file(&installer_path);
                    return Err(error);
                }
                Command::new(&installer_path)
                    .creation_flags(0x08000000)
                    .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
                    .spawn()
                    .map_err(|e| format!("Failed to spawn installer: {}", e))?;
                Ok(())
            }
            #[cfg(not(target_os = "windows"))]
            {
                return Err("Installer updates are only supported on Windows".to_string());
            }
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
        }
    }
}

fn sig_acceptable_with_expected(status: &str, thumbprint: Option<&str>, expected: &str) -> bool {
    if expected.trim().is_empty() || status != "Valid" {
        return false;
    }
    thumbprint.is_some_and(|actual| actual.replace(' ', "").eq_ignore_ascii_case(&expected.replace(' ', "")))
}

fn sig_acceptable(status: &str, thumbprint: Option<&str>) -> bool {
    sig_acceptable_with_expected(status, thumbprint, SIGNER_THUMBPRINT)
}

#[cfg(target_os = "windows")]
fn verify_authenticode(path: &std::path::Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let escaped = path.to_string_lossy().replace('\'', "''");
    // PowerShell only reports status + signer thumbprint; the acceptance
    // decision lives in Rust (sig_acceptable) so it is testable and is the
    // single source of truth.
    let script = format!(
        "$sig = Get-AuthenticodeSignature -LiteralPath '{escaped}'; \
         $tp = if ($sig.SignerCertificate) {{ $sig.SignerCertificate.Thumbprint }} else {{ '' }}; \
         Write-Output ('STATUS=' + $sig.Status); Write-Output ('THUMB=' + $tp)"
    );
    let out = std::process::Command::new("powershell")
        .creation_flags(0x08000000)
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| format!("Failed to verify installer signature: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "PowerShell signature verification failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut status: Option<String> = None;
    let mut thumb: Option<String> = None;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("STATUS=") {
            status = Some(v.trim().to_string());
        }
        if let Some(v) = line.strip_prefix("THUMB=") {
            let t = v.trim();
            thumb = if t.is_empty() { None } else { Some(t.to_string()) };
        }
    }
    let status = status.ok_or("PowerShell signature check returned no status")?;
    if sig_acceptable(status.as_str(), thumb.as_deref()) {
        Ok(())
    } else {
        Err(format!("Installer Authenticode signature is invalid (status {status})"))
    }
}

#[cfg(not(target_os = "windows"))]
fn verify_authenticode(_path: &std::path::Path) -> Result<(), String> {
    Err("Installer verification is only supported on Windows".into())
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
    fn signature_accepts_only_expected_publisher() {
        let expected = "AABBCC";
        assert!(sig_acceptable_with_expected("Valid", Some("aa bb cc"), expected));
        assert!(!sig_acceptable_with_expected("NotTrusted", Some("AABBCC"), expected));
        assert!(!sig_acceptable_with_expected("Valid", Some("other"), expected));
        assert!(!sig_acceptable_with_expected("Valid", None, expected));
        assert!(!sig_acceptable_with_expected("Valid", Some(expected), ""));
    }

    #[test]
    fn signature_rejects_tamper_wrong_signer_and_unsigned() {
        // tamper/unsigned -> wrong status, never pinned-check bypass
        let expected = "AABBCC";
        assert!(!sig_acceptable_with_expected("HashMismatch", Some(expected), expected));
        assert!(!sig_acceptable_with_expected("NotSigned", Some(expected), expected));
        assert!(!sig_acceptable_with_expected("NoSignature", Some(expected), expected));
        // Trust failures are rejected even if a certificate is present.
        assert!(!sig_acceptable_with_expected("UnknownError", Some(expected), expected));
        assert!(!sig_acceptable_with_expected(
            "UnknownError",
            Some("DEADBEEF"),
            expected
        ));
        assert!(!sig_acceptable_with_expected("UnknownError", None, expected));
    }
}
