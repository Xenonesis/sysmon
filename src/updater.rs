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
    let config = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(10)))
        .timeout_recv_response(Some(std::time::Duration::from_secs(60)))
        .timeout_send_body(Some(std::time::Duration::from_secs(30)))
        .build();
    ureq::Agent::new_with_config(config)
}

fn canonical_version(version: &str) -> Result<[u32; 3], String> {
    let mut components = version.split('.');
    let mut parsed = [0; 3];
    for slot in &mut parsed {
        let part = components.next().ok_or("Version must contain major.minor.patch")?;
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) || (part.len() > 1 && part.starts_with('0')) {
            return Err("Version is not canonical major.minor.patch".into());
        }
        *slot = part.parse().map_err(|_| "Version component exceeds supported range")?;
    }
    if components.next().is_some() {
        return Err("Version must contain exactly three components".into());
    }
    Ok(parsed)
}

pub(crate) fn validate_asset_url(url: &str) -> Result<(), String> {
    let suffix = url
        .strip_prefix("https://github.com/Xenonesis/sysmon/releases/download/")
        .ok_or("Unexpected update asset URL")?;
    let (tag, filename) = suffix.split_once('/').ok_or("Missing release tag or filename")?;
    let version = tag.strip_prefix('v').ok_or("Release tag must start with one v")?;
    canonical_version(version)?;
    if filename != format!("SystemMonitor-{version}-setup.exe") {
        return Err("Installer filename must match the canonical release tag".into());
    }
    Ok(())
}

fn validate_checksum_url(url: &str) -> Result<(), String> {
    validate_asset_url(url.strip_suffix(".sha256").ok_or("Unexpected checksum asset URL")?)
}

fn validate_release_pair(download_url: &str, checksum_url: &str) -> Result<String, String> {
    validate_asset_url(download_url)?;
    validate_checksum_url(checksum_url)?;
    if checksum_url != format!("{download_url}.sha256") {
        return Err("Installer and checksum must be matching assets from the same release".into());
    }
    Ok(download_url.rsplit('/').next().unwrap().to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallOutcome {
    /// A verified helper is ready. The UI must exit gracefully; installation has NOT completed.
    Launched,
    Completed {
        reboot_required: bool,
    },
    Canceled,
    Failed {
        code: u32,
    },
    Error(String),
}

fn installer_outcome(code: u32) -> InstallOutcome {
    match code {
        0 => InstallOutcome::Completed { reboot_required: false },
        3010 => InstallOutcome::Completed { reboot_required: true },
        2 | 5 => InstallOutcome::Canceled,
        code => InstallOutcome::Failed { code },
    }
}

fn matches_install_location(exe: &std::path::Path, location: &std::path::Path) -> bool {
    match (
        fs::canonicalize(exe),
        fs::canonicalize(location.join("system-monitor.exe")),
    ) {
        (Ok(actual), Ok(registered)) => actual == registered,
        _ => false,
    }
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
impl Default for Updater {
    fn default() -> Self {
        Self::new()
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
            return Err("Updates are only available for the executable at the registered installation location".into());
        }

        match self.fetch_latest_release() {
            Ok(release) => {
                let latest_version = release
                    .tag_name
                    .strip_prefix('v')
                    .ok_or("Release tag must start with one v")?;
                let latest = canonical_version(latest_version)?;
                let current = canonical_version(CURRENT_VERSION)?;

                self.update_info.latest_version = latest_version.to_string();
                self.update_info.update_available = latest > current;

                // Clear any URLs from a previous check in this session.
                self.update_info.download_url.clear();
                self.update_info.checksum_url.clear();

                // Only the installer asset (SystemMonitor-<ver>-setup.exe) is
                // offered, and only together with the SHA-256 checksum file
                // published next to it. Without a published checksum the
                // download cannot be verified, so no update is offered.
                let expected_installer = format!("SystemMonitor-{latest_version}-setup.exe");
                let expected_checksum = format!("{expected_installer}.sha256");
                for asset in release.assets {
                    let name = asset.name;
                    if name == expected_checksum {
                        self.update_info.checksum_url = asset.browser_download_url;
                    } else if name == expected_installer {
                        self.update_info.download_url = asset.browser_download_url;
                    }
                }
                if self.update_info.update_available {
                    validate_release_pair(&self.update_info.download_url, &self.update_info.checksum_url)
                        .map_err(|e| format!("New release is not installable: {e}"))?;
                }

                Ok(self.update_info.clone())
            }
            Err(e) => Err(format!("Failed to check for updates: {}", e)),
        }
    }

    fn fetch_latest_release(&self) -> Result<GitHubRelease, String> {
        let response = http_agent()
            .get(UPDATE_CHECK_URL)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "SystemMonitor/1.0")
            .call()
            .map_err(|e| format!("Failed to fetch release info: {}", e))?;

        let body = response
            .into_body()
            .into_with_config()
            .limit(MAX_RELEASE_METADATA_BYTES)
            .read_to_string()
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        serde_json::from_str(&body).map_err(|e| format!("Failed to parse GitHub response: {}", e))
    }

    pub(crate) fn is_installed(&self) -> bool {
        #[cfg(windows)]
        {
            let Ok(exe) = std::env::current_exe() else {
                return false;
            };
            registered_install_location().is_ok_and(|location| matches_install_location(&exe, &location))
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    fn fetch_expected_checksum(&self, checksum_url: &str, expected_filename: &str) -> Result<String, String> {
        let response = http_agent()
            .get(checksum_url)
            .header("User-Agent", "SystemMonitor/1.0")
            .call()
            .map_err(|e| format!("Failed to download checksum: {}", e))?;

        let body = response
            .into_body()
            .into_with_config()
            .limit(MAX_CHECKSUM_BYTES)
            .read_to_string()
            .map_err(|e| format!("Failed to read checksum file: {e}"))?;

        parse_checksum_file(&body, expected_filename)
            .ok_or_else(|| "Checksum file does not contain the expected installer hash and filename".to_string())
    }

    pub fn download_and_install_update(
        &self,
        download_url: &str,
        checksum_url: &str,
    ) -> Result<InstallOutcome, String> {
        if !self.is_installed() {
            return Err("This executable is not the registered installed copy".into());
        }
        if !crate::privilege::is_app_elevated() {
            return Err(
                "Restart System Monitor as administrator before installing an update; no installer was launched".into(),
            );
        }
        configured_signer_pin()?;
        let installer_filename = validate_release_pair(download_url, checksum_url)?;
        let version = installer_filename
            .strip_prefix("SystemMonitor-")
            .unwrap()
            .strip_suffix("-setup.exe")
            .unwrap();
        if canonical_version(version)? <= canonical_version(CURRENT_VERSION)? {
            return Err("Updater refuses a non-newer release".into());
        }
        let expected_sha256 = self.fetch_expected_checksum(checksum_url, &installer_filename)?;

        // Download the update using ureq
        let response = http_agent()
            .get(download_url)
            .header("User-Agent", "SystemMonitor/1.0")
            .call()
            .map_err(|e| format!("Failed to download update: {}", e))?;
        if response
            .headers()
            .get("Content-Length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|size| size > MAX_INSTALLER_BYTES)
        {
            return Err("Update installer exceeds size limit".into());
        }

        let mut bytes = Vec::new();
        response
            .into_body()
            .into_with_config()
            .limit(MAX_INSTALLER_BYTES)
            .reader()
            .read_to_end(&mut bytes)
            .map_err(|e| format!("Failed to read update file: {e}"))?;

        // GitHub's adjacent checksum supplies integrity, NOT independent authenticity.
        verify_sha256(&bytes, &expected_sha256)?;
        #[cfg(windows)]
        {
            stage_and_launch_helper(&bytes, &expected_sha256)
        }
        #[cfg(not(windows))]
        {
            Err("Installer updates are only supported on Windows".into())
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

/// Parses a `sha256sum`-style file and accepts only the exact installer name.
fn parse_checksum_file(text: &str, expected_filename: &str) -> Option<String> {
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        if let (Some(hash), Some(filename)) = (fields.next(), fields.next()) {
            let filename = filename.strip_prefix('*').unwrap_or(filename);
            if fields.next().is_none()
                && filename == expected_filename
                && hash.len() == 64
                && hash.chars().all(|c| c.is_ascii_hexdigit())
            {
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

// Public release configuration, never a secret and never supplied by downloaded
// metadata. This is SHA-256 over the DER leaf code-signing certificate.
fn configured_signer_pin() -> Result<&'static str, String> {
    let pin = option_env!("SYSMON_SIGNER_CERT_SHA256").unwrap_or("");
    if pin.len() != 64 || !pin.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(
            "Automatic installation is unavailable: this build has no configured publisher certificate SHA-256 pin"
                .into(),
        );
    }
    Ok(pin)
}

#[cfg(windows)]
fn registered_install_location() -> Result<std::path::PathBuf, String> {
    use winreg::{RegKey, enums::*};
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(
            format!(
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{}_is1",
                INSTALLER_APP_ID
            ),
            KEY_READ | KEY_WOW64_64KEY,
        )
        .map_err(|e| format!("Registered installation is unavailable: {e}"))?;
    let location: String = key.get_value("InstallLocation").map_err(|e| e.to_string())?;
    if location.is_empty() {
        return Err("Registered InstallLocation is empty".into());
    }
    fs::canonicalize(location).map_err(|e| e.to_string())
}

#[cfg(windows)]
pub(crate) use native_update::{run_startup_task, take_install_outcome, verify_authenticode_path};
#[cfg(not(windows))]
pub(crate) fn take_install_outcome() -> Option<InstallOutcome> {
    None
}
#[cfg(windows)]
use native_update::stage_and_launch_helper;

#[cfg(windows)]
mod native_update {
    use super::*;
    use crate::privilege::{self, ElevationOutcome, StartupMessage};
    use std::{
        cell::RefCell,
        fs::{File, OpenOptions},
        io::{Seek, Write},
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            fs::OpenOptionsExt,
            io::{AsRawHandle, FromRawHandle},
        },
        path::{Path, PathBuf},
    };
    use windows::{
        Win32::{
            Foundation::*,
            Security::{Authorization::*, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, WinTrust::*},
            Storage::FileSystem::*,
            System::SystemInformation::GetWindowsDirectoryW,
        },
        core::{PCWSTR, w},
    };

    thread_local! { static INSTALL_RESULT: RefCell<Option<(InstallOutcome, PathBuf)>> = const { RefCell::new(None) }; }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }
    fn locked_file(path: &Path) -> Result<File, String> {
        OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ.0)
            .open(path)
            .map_err(|e| format!("Could not lock {}: {e}", path.display()))
    }
    fn check_file_hash(file: &mut File, expected: &str) -> Result<(), String> {
        file.rewind().map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut block = [0u8; 65536];
        loop {
            let count = file.read(&mut block).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            hasher.update(&block[..count]);
        }
        if !hex_digest(&hasher.finalize()).eq_ignore_ascii_case(expected) {
            return Err("Staged installer content changed".into());
        }
        file.rewind().map_err(|e| e.to_string())
    }

    pub(crate) fn verify_authenticode_path(path: &Path) -> Result<(), String> {
        verify_authenticode(path, &locked_file(path)?)
    }

    fn verify_authenticode(path: &Path, file: &File) -> Result<(), String> {
        let expected = configured_signer_pin()?;
        let name = wide(path);
        let mut file_info = WINTRUST_FILE_INFO {
            cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: PCWSTR(name.as_ptr()),
            hFile: HANDLE(file.as_raw_handle()),
            ..Default::default()
        };
        let mut trust = WINTRUST_DATA {
            cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
            dwUnionChoice: WTD_CHOICE_FILE,
            Anonymous: WINTRUST_DATA_0 { pFile: &mut file_info },
            dwStateAction: WTD_STATEACTION_VERIFY,
            dwProvFlags: WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT | WTD_DISABLE_MD2_MD4,
            ..Default::default()
        };
        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        // The provider verifies the already-open file, including chain, code-signing
        // usage, timestamp and revocation. A valid but different publisher is rejected.
        let status = unsafe {
            WinVerifyTrust(
                HWND(-1isize as *mut _),
                &mut action,
                (&mut trust as *mut WINTRUST_DATA).cast(),
            )
        };
        let verified = (|| {
            if status != 0 {
                return Err(format!(
                    "Authenticode trust verification failed (0x{:08x})",
                    status as u32
                ));
            }
            unsafe {
                let provider = WTHelperProvDataFromStateData(trust.hWVTStateData);
                if provider.is_null() {
                    return Err("Authenticode provider supplied no signer".into());
                }
                let signer = WTHelperGetProvSignerFromChain(provider, 0, false, 0);
                if signer.is_null() || (*signer).csCertChain == 0 || (*signer).pasCertChain.is_null() {
                    return Err("Authenticode signer chain is missing".into());
                }
                let cert = (*(*signer).pasCertChain).pCert;
                if cert.is_null() || (*cert).pbCertEncoded.is_null() || (*cert).cbCertEncoded == 0 {
                    return Err("Authenticode signer certificate is missing".into());
                }
                let der = std::slice::from_raw_parts((*cert).pbCertEncoded, (*cert).cbCertEncoded as usize);
                if !hex_digest(&Sha256::digest(der)).eq_ignore_ascii_case(expected) {
                    return Err("Installer signer does not match the configured publisher certificate".into());
                }
            }
            Ok(())
        })();
        trust.dwStateAction = WTD_STATEACTION_CLOSE;
        unsafe {
            WinVerifyTrust(
                HWND(-1isize as *mut _),
                &mut action,
                (&mut trust as *mut WINTRUST_DATA).cast(),
            );
        }
        verified
    }

    fn staging_root() -> Result<PathBuf, String> {
        let mut buffer = vec![0u16; 32768];
        let length = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
        if length == 0 || length >= buffer.len() {
            return Err("Windows directory is unavailable".into());
        }
        Ok(PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length])).join("Temp"))
    }

    struct Stage {
        path: PathBuf,
        directory: Option<File>,
        keep: bool,
    }
    impl Stage {
        fn create() -> Result<Self, String> {
            let path = staging_root()?.join(format!("SysMonUpdate-{}", privilege::random_id()?));
            let name = wide(&path);
            let mut descriptor = PSECURITY_DESCRIPTOR::default();
            // Protected DACL and high integrity: medium-integrity same-user code
            // cannot rewrite installer/helper bytes or grant itself access.
            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    w!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)S:(ML;OICI;NW;;;HI)"),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    None,
                )
                .map_err(|e| e.to_string())?;
            }
            let attributes = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor.0,
                bInheritHandle: false.into(),
            };
            let created = unsafe { CreateDirectoryW(PCWSTR(name.as_ptr()), Some(&attributes)) };
            unsafe {
                let _ = LocalFree(Some(HLOCAL(descriptor.0)));
            }
            created.map_err(|e| format!("Could not create protected update staging: {e}"))?;
            let mut stage = Self {
                path,
                directory: None,
                keep: false,
            };
            let handle = unsafe {
                CreateFileW(
                    PCWSTR(name.as_ptr()),
                    FILE_READ_ATTRIBUTES.0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                    None,
                )
                .map_err(|e| e.to_string())?
            };
            stage.directory = Some(unsafe { File::from_raw_handle(handle.0) });
            Ok(stage)
        }
        fn write_locked(&self, name: &str, bytes: &[u8]) -> Result<File, String> {
            let path = self.path.join(name);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .share_mode(0)
                .open(&path)
                .map_err(|e| e.to_string())?;
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            drop(file);
            // This transition occurs inside our new admin-only directory. The
            // resulting read-only handle denies writes AND deletion until exit.
            locked_file(&path)
        }
    }
    impl Drop for Stage {
        fn drop(&mut self) {
            self.directory.take();
            if !self.keep {
                cleanup_stage(&self.path);
            }
        }
    }
    fn cleanup_stage(path: &Path) {
        // Never recurse or accept a caller-selected directory for cleanup.
        let Ok(root) = staging_root() else {
            return;
        };
        let Some(name) = path
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("SysMonUpdate-"))
        else {
            return;
        };
        if path.parent() != Some(root.as_path()) || name.len() != 32 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
            return;
        }
        for name in ["installer.exe", "helper.exe"] {
            let _ = fs::remove_file(path.join(name));
        }
        let _ = fs::remove_dir(path);
    }

    pub(super) fn stage_and_launch_helper(bytes: &[u8], expected: &str) -> Result<InstallOutcome, String> {
        let target =
            fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let mut source = locked_file(&target)?;
        verify_authenticode(&target, &source)?;
        let mut stage = Stage::create()?;
        let mut installer = stage.write_locked("installer.exe", bytes)?;
        check_file_hash(&mut installer, expected)?;
        verify_authenticode(&stage.path.join("installer.exe"), &installer)?;
        let mut helper_bytes = Vec::new();
        source.read_to_end(&mut helper_bytes).map_err(|e| e.to_string())?;
        let _helper = stage.write_locked("helper.exe", &helper_bytes)?;
        let result = privilege::launch_successor(
            &stage.path.join("helper.exe"),
            &StartupMessage::Install {
                installer: stage.path.join("installer.exe"),
                target,
                sha256: expected.to_string(),
            },
            false,
        )?;
        if result != ElevationOutcome::Ready {
            return Ok(InstallOutcome::Canceled);
        }
        // The authenticated helper now independently holds both installer and
        // directory locks. They overlap these guards: there is no unlocked gap.
        stage.keep = true;
        Ok(InstallOutcome::Launched)
    }

    pub(crate) fn run_startup_task() -> Result<bool, String> {
        match privilege::take_startup_message() {
            Some(StartupMessage::Install {
                installer,
                target,
                sha256,
            }) => {
                run_install_helper(&installer, &target, &sha256)?;
                Ok(true)
            }
            Some(StartupMessage::Resume(outcome, stage)) => {
                INSTALL_RESULT.with(|result| *result.borrow_mut() = Some((outcome, stage)));
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn run_install_helper(installer: &Path, target: &Path, expected: &str) -> Result<(), String> {
        let own = fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let stage_path = own.parent().ok_or("Update helper has no parent directory")?;
        if installer != stage_path.join("installer.exe") {
            return Err("Installer is outside the authenticated helper staging directory".into());
        }
        let installed = registered_install_location()?.join("system-monitor.exe");
        if fs::canonicalize(target).map_err(|e| e.to_string())?
            != fs::canonicalize(&installed).map_err(|e| e.to_string())?
        {
            return Err("Update target no longer matches the registered installation".into());
        }
        // The helper keeps the parent directory and exact verified file open
        // throughout installer execution; parent exits only after these succeed.
        let name = wide(stage_path);
        let directory = unsafe {
            CreateFileW(
                PCWSTR(name.as_ptr()),
                FILE_READ_ATTRIBUTES.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                None,
            )
            .map_err(|e| e.to_string())?
        };
        let directory = unsafe { File::from_raw_handle(directory.0) };
        let mut file = locked_file(installer)?;
        check_file_hash(&mut file, expected)?;
        verify_authenticode(installer, &file)?;
        privilege::complete_startup()?;
        let outcome = match std::process::Command::new(installer)
            .args([
                "/SILENT",
                "/SUPPRESSMSGBOXES",
                "/NORESTART",
                "/RESTARTEXITCODE=3010",
                "/NOCLOSEAPPLICATIONS",
                "/NORESTARTAPPLICATIONS",
            ])
            .arg(format!("/DIR={}", installed.parent().unwrap().display()))
            .status()
        {
            Ok(status) => installer_outcome(status.code().unwrap_or(-1) as u32),
            Err(error) => InstallOutcome::Error(format!("Installer launch failed: {error}")),
        };
        drop(file);
        // Even cancellation/failure reopens the retained installed application.
        // Refuse to execute replacement content that lacks the publisher identity.
        verify_authenticode_path(&installed)?;
        privilege::launch_successor(
            &installed,
            &StartupMessage::Resume(outcome, stage_path.to_path_buf()),
            false,
        )?;
        drop(directory);
        Ok(())
    }

    pub(crate) fn take_install_outcome() -> Option<InstallOutcome> {
        INSTALL_RESULT
            .with(|result| result.borrow_mut().take())
            .map(|(outcome, stage)| {
                // First GUI logic runs after complete_startup observed helper exit.
                cleanup_stage(&stage);
                outcome
            })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn locked_staged_bytes_deny_replacement_and_mutation() {
            let path = std::env::temp_dir().join(format!("sysmon-lock-{}.exe", privilege::random_id().unwrap()));
            fs::write(&path, b"fixture").unwrap();
            let mut held = locked_file(&path).unwrap();
            assert!(OpenOptions::new().write(true).open(&path).is_err());
            assert!(fs::remove_file(&path).is_err());
            check_file_hash(&mut held, &hex_digest(&Sha256::digest(b"fixture"))).unwrap();
            assert!(check_file_hash(&mut held, &hex_digest(&Sha256::digest(b"tampered"))).is_err());
            drop(held);
            fs::remove_file(path).unwrap();
        }
        #[test]
        fn exclusive_staging_write_never_overwrites_existing_content() {
            let path = std::env::temp_dir().join(format!("sysmon-exclusive-{}", privilege::random_id().unwrap()));
            fs::create_dir(&path).unwrap();
            let stage = Stage {
                path: path.clone(),
                directory: None,
                keep: true,
            };
            fs::write(path.join("installer.exe"), b"sentinel").unwrap();
            assert!(stage.write_locked("installer.exe", b"replacement").is_err());
            assert_eq!(fs::read(path.join("installer.exe")).unwrap(), b"sentinel");
            fs::remove_file(path.join("installer.exe")).unwrap();
            fs::remove_dir(path).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_versions_reject_lossy_comparison_inputs() {
        for version in [
            "1.2",
            "1.2.3.4",
            "01.2.3",
            "1.x.3",
            "v1.2.3",
            "1.2.3-beta",
            " 1.2.3",
            "1.2.3+build",
        ] {
            assert!(canonical_version(version).is_err(), "{version}");
        }
        assert!(canonical_version("3.10.0").unwrap() > canonical_version("3.9.99").unwrap());
    }

    #[test]
    fn release_path_must_bind_canonical_tag_and_filename() {
        for suffix in [
            "v3.8.0/SystemMonitor-3.9.0-setup.exe",
            "v3.8.0/../SystemMonitor-3.8.0-setup.exe",
            "vv3.8.0/SystemMonitor-3.8.0-setup.exe",
            "v3.8.0/SystemMonitor-3.8.0-setup.exe?x=.exe",
        ] {
            let url = format!("https://github.com/Xenonesis/sysmon/releases/download/{suffix}");
            assert!(validate_release_pair(&url, &format!("{url}.sha256")).is_err());
        }
    }

    #[test]
    fn completion_distinguishes_cancellation_failure_and_reboot() {
        assert_eq!(
            installer_outcome(0),
            InstallOutcome::Completed { reboot_required: false }
        );
        assert_eq!(
            installer_outcome(3010),
            InstallOutcome::Completed { reboot_required: true }
        );
        assert_eq!(installer_outcome(2), InstallOutcome::Canceled);
        assert_eq!(installer_outcome(5), InstallOutcome::Canceled);
        assert_eq!(installer_outcome(4), InstallOutcome::Failed { code: 4 });
    }

    #[test]
    fn registered_install_does_not_identify_a_portable_copy() {
        let root = std::env::temp_dir().join(format!("sysmon-path-test-{}", std::process::id()));
        fs::create_dir_all(root.join("installed")).unwrap();
        fs::create_dir_all(root.join("portable")).unwrap();
        fs::write(root.join("installed/system-monitor.exe"), b"installed").unwrap();
        fs::write(root.join("portable/system-monitor.exe"), b"portable").unwrap();
        assert!(matches_install_location(
            &root.join("installed/system-monitor.exe"),
            &root.join("installed")
        ));
        assert!(!matches_install_location(
            &root.join("portable/system-monitor.exe"),
            &root.join("installed")
        ));
        assert!(!matches_install_location(
            &root.join("missing/system-monitor.exe"),
            &root.join("installed")
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn accepts_expected_release_asset() {
        assert!(
            validate_asset_url(
                "https://github.com/Xenonesis/sysmon/releases/download/v2.6.0/SystemMonitor-2.6.0-setup.exe"
            )
            .is_ok()
        );
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
        assert!(
            validate_checksum_url(
                "https://github.com/Xenonesis/sysmon/releases/download/v3.7.6/SystemMonitor-3.7.6-setup.exe.sha256"
            )
            .is_ok()
        );
    }

    #[test]
    fn accepts_only_matching_release_pair() {
        let installer = "https://github.com/Xenonesis/sysmon/releases/download/v3.8.0/SystemMonitor-3.8.0-setup.exe";
        assert_eq!(
            validate_release_pair(installer, &format!("{installer}.sha256")).unwrap(),
            "SystemMonitor-3.8.0-setup.exe"
        );
        assert!(
            validate_release_pair(
                installer,
                "https://github.com/Xenonesis/sysmon/releases/download/v3.7.7/SystemMonitor-3.7.7-setup.exe.sha256"
            )
            .is_err()
        );
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
            parse_checksum_file(
                &format!("{hash} *SystemMonitor-3.7.6-setup.exe"),
                "SystemMonitor-3.7.6-setup.exe"
            ),
            Some(hash.clone())
        );
        // GNU coreutils text-mode (two-space) format.
        assert_eq!(
            parse_checksum_file(
                &format!("{hash}  SystemMonitor-3.7.6-setup.exe\n"),
                "SystemMonitor-3.7.6-setup.exe"
            ),
            Some(hash.clone())
        );
        // Uppercase hashes are normalized.
        assert_eq!(
            parse_checksum_file(&format!("{} *setup.exe", hash.to_uppercase()), "setup.exe"),
            Some(hash.clone())
        );
        // Empty, missing or malformed content yields no hash.
        assert_eq!(parse_checksum_file("", "setup.exe"), None);
        assert_eq!(parse_checksum_file("nothash *setup.exe", "setup.exe"), None);
        assert_eq!(
            parse_checksum_file(&format!("{} *setup.exe", "z".repeat(64)), "setup.exe"),
            None
        );
        assert_eq!(parse_checksum_file(&format!("{hash} *other.exe"), "setup.exe"), None);
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
