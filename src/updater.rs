use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const UPDATE_CHECK_URL: &str = "https://api.github.com/repos/Xenonesis/sysmon/releases/latest";
const MAX_INSTALLER_BYTES: u64 = 100 * 1024 * 1024;

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
                self.update_info.update_available =
                    self.is_newer_version(current_version, latest_version);

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
                self.update_info.update_available = self.update_info.update_available
                    && !self.update_info.download_url.is_empty();

                Ok(self.update_info.clone())
            }
            Err(e) => Err(format!("Failed to check for updates: {}", e)),
        }
    }

    fn fetch_latest_release(&self) -> Result<GitHubRelease, String> {
        let response = ureq::get(UPDATE_CHECK_URL)
            .set("Accept", "application/vnd.github.v3+json")
            .set("User-Agent", "SystemMonitor/1.0")
            .call()
            .map_err(|e| format!("Failed to fetch release info: {}", e))?;

        let mut body = String::new();
        response
            .into_reader()
            .read_to_string(&mut body)
            .map_err(|e| format!("Failed to read response body: {}", e))?;

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
        let unique = format!("system-monitor-setup-{}-{}.exe", std::process::id(), chrono::Utc::now().timestamp_millis());
        let installer_path = std::env::temp_dir().join(unique);

        // Download the update using ureq
        let response = ureq::get(download_url)
            .set("User-Agent", "SystemMonitor/1.0")
            .call()
            .map_err(|e| format!("Failed to download update: {}", e))?;

        let mut bytes = Vec::new();
        response.into_reader().take(MAX_INSTALLER_BYTES + 1).read_to_end(&mut bytes)
            .map_err(|e| format!("Failed to read update file: {e}"))?;
        if bytes.len() as u64 > MAX_INSTALLER_BYTES { return Err("Update installer exceeds size limit".into()); };

        fs::write(&installer_path, &bytes)
            .map_err(|e| format!("Failed to write update file: {}", e))?;

        // Silent install — replaces the exe, shortcuts, and uninstall entry in
        // one pass. Only installer assets are offered; the process exits so the
        // installer can replace files freely.
        {
            #[cfg(target_os = "windows")]
            {
                use std::process::Command;
                use std::os::windows::process::CommandExt;
                verify_authenticode(&installer_path)?;
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

#[cfg(target_os = "windows")]
fn verify_authenticode(path: &std::path::Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let escaped = path.to_string_lossy().replace('"', "\"");
    let script = format!("if ((Get-AuthenticodeSignature -LiteralPath \"{escaped}\").Status -eq 'Valid') {{ exit 0 }} else {{ exit 1 }}");
    let status = std::process::Command::new("powershell")
        .creation_flags(0x08000000)
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status().map_err(|e| format!("Failed to verify installer signature: {e}"))?;
    if status.success() { Ok(()) } else { Err("Installer Authenticode signature is invalid".into()) }
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
        assert!(validate_asset_url("https://github.com/Xenonesis/sysmon/releases/download/v2.6.0/SystemMonitor-2.6.0-setup.exe").is_ok());
    }

    #[test]
    fn rejects_untrusted_asset_urls() {
        for url in [
            "http://github.com/Xenonesis/sysmon/releases/download/v2/a.exe",
            "https://evil.example/a.exe",
            "https://github.com/other/sysmon/releases/download/v2/a.exe",
            "https://github.com/Xenonesis/sysmon/releases/download/v2/a.zip",
        ] { assert!(validate_asset_url(url).is_err(), "accepted {url}"); }
    }
}
