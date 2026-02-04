use std::fs;
use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const UPDATE_CHECK_URL: &str = "https://api.github.com/repos/Xenonesis/sysmon/releases/latest";
const DOWNLOAD_URL_BASE: &str = "https://github.com/Xenonesis/sysmon/releases/latest/download/";

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
        // Try to fetch latest release info from GitHub
        match self.fetch_latest_release() {
            Ok(release) => {
                let latest_version = release.tag_name.trim_start_matches('v');
                let current_version = CURRENT_VERSION;

                self.update_info.latest_version = latest_version.to_string();
                self.update_info.update_available = self.is_newer_version(current_version, latest_version);

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
        // Use a simple HTTP request to fetch release info
        // For now, we'll use std library features
        // In production, you might want to use reqwest or ureq
        
        // This is a simplified version - in production, you'd want proper HTTP handling
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("powershell")
                .arg("-Command")
                .arg(format!(
                    "Invoke-RestMethod -Uri '{}' -Headers @{{Accept='application/vnd.github.v3+json'}}",
                    UPDATE_CHECK_URL
                ))
                .output()
                .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

            if output.status.success() {
                let json_str = String::from_utf8_lossy(&output.stdout);
                serde_json::from_str(&json_str)
                    .map_err(|e| format!("Failed to parse GitHub response: {}", e))
            } else {
                Err("Failed to fetch release info".to_string())
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("Update checking not implemented for this platform".to_string())
        }
    }

    fn is_newer_version(&self, current: &str, latest: &str) -> bool {
        // Simple version comparison
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
        #[cfg(target_os = "windows")]
        {
            // Get temp directory
            let temp_dir = std::env::temp_dir();
            let installer_path = temp_dir.join("system-monitor-update.zip");

            // Download the update
            let output = Command::new("powershell")
                .arg("-Command")
                .arg(format!(
                    "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                    download_url,
                    installer_path.display()
                ))
                .output()
                .map_err(|e| format!("Failed to download update: {}", e))?;

            if !output.status.success() {
                return Err("Failed to download update file".to_string());
            }

            // Extract and run installer
            let extract_dir = temp_dir.join("system-monitor-update");
            if extract_dir.exists() {
                fs::remove_dir_all(&extract_dir).ok();
            }
            fs::create_dir_all(&extract_dir).map_err(|e| format!("Failed to create extract dir: {}", e))?;

            // Extract ZIP
            let output = Command::new("powershell")
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

            // Run the installer
            let installer_script = extract_dir.join("installer.ps1");
            if installer_script.exists() {
                Command::new("powershell")
                    .arg("-ExecutionPolicy")
                    .arg("Bypass")
                    .arg("-File")
                    .arg(installer_script)
                    .arg("-Silent")
                    .spawn()
                    .map_err(|e| format!("Failed to run installer: {}", e))?;

                // Exit current application to allow update
                std::process::exit(0);
            } else {
                return Err("Installer script not found in update package".to_string());
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("Update installation not implemented for this platform".to_string())
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
