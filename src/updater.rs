use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const UPDATE_CHECK_URL: &str = "https://api.github.com/repos/Xenonesis/sysmon/releases/latest";

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
        match self.fetch_latest_release() {
            Ok(release) => {
                let latest_version = release.tag_name.trim_start_matches('v');
                let current_version = CURRENT_VERSION;

                self.update_info.latest_version = latest_version.to_string();
                self.update_info.update_available =
                    self.is_newer_version(current_version, latest_version);

                // Find the installer asset
                for asset in release.assets {
                    if asset.name.ends_with(".zip") || asset.name.ends_with(".exe") {
                        self.update_info.download_url = asset.browser_download_url;
                        break;
                    }
                }

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

    pub fn download_and_install_update(&self, download_url: &str) -> Result<(), String> {
        let temp_dir = std::env::temp_dir();
        let installer_path = temp_dir.join("system-monitor-update.zip");

        // Download the update using ureq
        let response = ureq::get(download_url)
            .set("User-Agent", "SystemMonitor/1.0")
            .call()
            .map_err(|e| format!("Failed to download update: {}", e))?;

        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| format!("Failed to read update file: {}", e))?;

        fs::write(&installer_path, &bytes)
            .map_err(|e| format!("Failed to write update file: {}", e))?;

        // Extract ZIP
        let extract_dir = temp_dir.join("system-monitor-update");
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir).ok();
        }
        fs::create_dir_all(&extract_dir)
            .map_err(|e| format!("Failed to create extract dir: {}", e))?;

        // Use zip crate or PowerShell as fallback for extraction
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            use std::os::windows::process::CommandExt;

            let output = Command::new("powershell")
                .creation_flags(0x08000000)
                .arg("-Command")
                .arg(format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    installer_path.display(),
                    extract_dir.display()
                ))
                .output()
                .map_err(|e| format!("Failed to extract update: {}", e))?;

            if !output.status.success() {
                return Err("Failed to extract update archive".to_string());
            }
        }

        // Run the installer
        let installer_script = extract_dir.join("installer.ps1");
        if installer_script.exists() {
            #[cfg(target_os = "windows")]
            {
                use std::process::Command;
                use std::os::windows::process::CommandExt;

                Command::new("powershell")
                    .creation_flags(0x08000000)
                    .arg("-ExecutionPolicy")
                    .arg("Bypass")
                    .arg("-File")
                    .arg(&installer_script)
                    .arg("-Silent")
                    .spawn()
                    .map_err(|e| format!("Failed to run installer: {}", e))?;

                std::process::exit(0);
            }
        } else {
            Err("Installer script not found in update package".to_string())
        }
    }

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
