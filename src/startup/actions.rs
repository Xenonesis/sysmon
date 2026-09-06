use super::*;

use std::path::{Path, PathBuf};

// ─── Actions ─────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RawRegistryValue {
    value_type: String,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum StartupQuarantinePayload {
    Registry {
        value: RawRegistryValue,
        approved_value: Option<RawRegistryValue>,
    },
    StartupFolder {
        quarantined_path: String,
        original_path: String,
    },
    ScheduledTask {
        xml_path: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartupQuarantineRecord {
    pub id: String,
    pub created_at: String,
    pub item_name: String,
    pub locator: StartupLocator,
    payload: StartupQuarantinePayload,
}

fn quarantine_root() -> Result<PathBuf, String> {
    crate::app_paths::startup_quarantine_dir().ok_or_else(|| "Application data directory is unavailable".to_string())
}

pub(crate) fn valid_quarantine_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn quarantine_record_path(id: &str) -> Result<PathBuf, String> {
    if !valid_quarantine_id(id) {
        return Err("Invalid startup quarantine identifier".into());
    }
    Ok(quarantine_root()?.join(format!("{id}.json")))
}

pub fn quarantine_exists(id: &str) -> bool {
    quarantine_record_path(id).is_ok_and(|path| path.is_file())
}

fn save_quarantine_record(record: &StartupQuarantineRecord) -> Result<(), String> {
    let path = quarantine_record_path(&record.id)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Invalid startup quarantine path".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| format!("Could not create quarantine directory: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    let encoded =
        serde_json::to_vec_pretty(record).map_err(|error| format!("Could not serialize quarantine record: {error}"))?;
    std::fs::write(&temporary, encoded).map_err(|error| format!("Could not write quarantine record: {error}"))?;
    std::fs::rename(&temporary, &path).map_err(|error| format!("Could not finalize quarantine record: {error}"))
}

fn load_quarantine_record(id: &str) -> Result<StartupQuarantineRecord, String> {
    let path = quarantine_record_path(id)?;
    let bytes = std::fs::read(&path).map_err(|error| format!("Could not read quarantine record: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("Invalid quarantine record: {error}"))
}

pub(crate) fn new_quarantine_id() -> String {
    let nanos = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros() * 1_000);
    format!("{}-{nanos}", std::process::id())
}

fn move_file_transactional(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Err(format!("Destination already exists at {}", destination.display()));
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("Could not create destination folder: {error}"))?;
    }
    match std::fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            let copied = std::fs::copy(source, destination).map_err(|copy_error| {
                format!("Could not move file ({rename_error}); copy fallback failed: {copy_error}")
            })?;
            let expected = match std::fs::metadata(source) {
                Ok(metadata) => metadata.len(),
                Err(error) => {
                    let _ = std::fs::remove_file(destination);
                    return Err(format!("Could not verify source file: {error}"));
                }
            };
            if copied != expected {
                let _ = std::fs::remove_file(destination);
                return Err(format!("Copy fallback wrote {copied} of {expected} bytes"));
            }
            if let Err(error) = std::fs::File::open(destination).and_then(|file| file.sync_all()) {
                let _ = std::fs::remove_file(destination);
                return Err(format!("Could not flush copied startup file: {error}"));
            }
            if let Err(error) = std::fs::remove_file(source) {
                let _ = std::fs::remove_file(destination);
                return Err(format!(
                    "Copied startup file but could not remove the original: {error}"
                ));
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
fn registry_root(hive: &StartupRegistryHive) -> winreg::RegKey {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    match hive {
        StartupRegistryHive::CurrentUser => winreg::RegKey::predef(HKEY_CURRENT_USER),
        StartupRegistryHive::LocalMachine => winreg::RegKey::predef(HKEY_LOCAL_MACHINE),
    }
}

#[cfg(target_os = "windows")]
fn raw_registry_value(value: &winreg::RegValue<'_>) -> RawRegistryValue {
    RawRegistryValue {
        value_type: format!("{:?}", value.vtype),
        bytes: value.bytes.to_vec(),
    }
}

#[cfg(target_os = "windows")]
fn restore_raw_registry_value(value: &RawRegistryValue) -> Result<winreg::RegValue<'static>, String> {
    use winreg::enums::*;
    let value_type = match value.value_type.as_str() {
        "REG_NONE" => REG_NONE,
        "REG_SZ" => REG_SZ,
        "REG_EXPAND_SZ" => REG_EXPAND_SZ,
        "REG_BINARY" => REG_BINARY,
        "REG_DWORD" => REG_DWORD,
        "REG_DWORD_BIG_ENDIAN" => REG_DWORD_BIG_ENDIAN,
        "REG_LINK" => REG_LINK,
        "REG_MULTI_SZ" => REG_MULTI_SZ,
        "REG_RESOURCE_LIST" => REG_RESOURCE_LIST,
        "REG_FULL_RESOURCE_DESCRIPTOR" => REG_FULL_RESOURCE_DESCRIPTOR,
        "REG_RESOURCE_REQUIREMENTS_LIST" => REG_RESOURCE_REQUIREMENTS_LIST,
        "REG_QWORD" => REG_QWORD,
        other => return Err(format!("Unsupported registry value type {other}")),
    };
    Ok(winreg::RegValue {
        bytes: value.bytes.clone().into(),
        vtype: value_type,
    })
}

#[cfg(target_os = "windows")]
fn powershell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(target_os = "windows")]
fn ps_run_checked(script: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("powershell")
        .creation_flags(0x08000000)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|error| format!("Could not launch PowerShell: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if detail.is_empty() {
            format!("PowerShell exited with {}", output.status)
        } else {
            detail
        })
    }
}

#[cfg(target_os = "windows")]
fn set_startup_approved(hive: &StartupRegistryHive, path: &str, name: &str, enabled: bool) -> Result<(), String> {
    let bytes: [u8; 12] = if enabled {
        [0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    } else {
        [0x03, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    };
    let root = registry_root(hive);
    let (key, _) = root
        .create_subkey(path)
        .map_err(|error| format!("Could not open StartupApproved registry key: {error}"))?;
    let value = winreg::RegValue {
        bytes: (&bytes[..]).into(),
        vtype: winreg::enums::REG_BINARY,
    };
    key.set_raw_value(name, &value)
        .map_err(|error| format!("Could not update StartupApproved state: {error}"))?;
    let stored = key
        .get_raw_value(name)
        .map_err(|error| format!("Could not verify StartupApproved state: {error}"))?;
    let expected = if enabled { 0x02 } else { 0x03 };
    if stored.bytes.first().copied() != Some(expected) {
        return Err("StartupApproved state did not match the requested value".into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn disable_startup(locator: &StartupLocator) -> Result<(), String> {
    match locator {
        StartupLocator::Registry {
            hive,
            approved_path,
            value_name,
            ..
        } => set_startup_approved(hive, approved_path, value_name, false),
        StartupLocator::StartupFolder {
            enabled_path,
            disabled_path,
            approved_hive,
            approved_path,
            approved_name,
        } => {
            let source = Path::new(enabled_path);
            let destination = Path::new(disabled_path);
            if !source.is_file() {
                return Err(format!("Startup file was not found at {}", source.display()));
            }
            if destination.exists() {
                return Err(format!(
                    "Disabled startup destination already exists at {}",
                    destination.display()
                ));
            }
            move_file_transactional(source, destination)
                .map_err(|error| format!("Could not disable startup file: {error}"))?;
            if let Err(error) = set_startup_approved(approved_hive, approved_path, approved_name, false) {
                let _ = move_file_transactional(destination, source);
                return Err(error);
            }
            Ok(())
        }
        StartupLocator::ScheduledTask { task_path, task_name } => {
            let script = format!(
                "$ErrorActionPreference='Stop'; Disable-ScheduledTask -TaskPath {} -TaskName {} | Out-Null",
                powershell_literal(task_path),
                powershell_literal(task_name)
            );
            ps_run_checked(&script).map(|_| ())
        }
    }
}

#[cfg(target_os = "windows")]
pub fn enable_startup(locator: &StartupLocator) -> Result<(), String> {
    match locator {
        StartupLocator::Registry {
            hive,
            value_path,
            enabled_value_path,
            approved_path,
            value_name,
        } => {
            let mut moved_value = None;
            if value_path != enabled_value_path {
                use winreg::enums::{KEY_READ, KEY_WRITE};
                let root = registry_root(hive);
                let source = root
                    .open_subkey_with_flags(value_path, KEY_READ)
                    .map_err(|error| format!("Could not open disabled startup value: {error}"))?;
                let value = source
                    .get_raw_value(value_name)
                    .map_err(|error| format!("Could not read disabled startup value: {error}"))?;
                let (destination, _) = root
                    .create_subkey(enabled_value_path)
                    .map_err(|error| format!("Could not open enabled startup key: {error}"))?;
                destination
                    .set_raw_value(value_name, &value)
                    .map_err(|error| format!("Could not restore startup value: {error}"))?;
                let source = root
                    .open_subkey_with_flags(value_path, KEY_WRITE)
                    .map_err(|error| format!("Could not reopen disabled startup key: {error}"))?;
                if let Err(error) = source.delete_value(value_name) {
                    let _ = destination.delete_value(value_name);
                    return Err(format!("Could not remove disabled startup copy: {error}"));
                }
                moved_value = Some(value);
            }
            if let Err(error) = set_startup_approved(hive, approved_path, value_name, true) {
                if let Some(value) = moved_value {
                    let root = registry_root(hive);
                    if let Ok((source, _)) = root.create_subkey(value_path) {
                        let _ = source.set_raw_value(value_name, &value);
                    }
                    if let Ok(destination) = root.open_subkey_with_flags(enabled_value_path, winreg::enums::KEY_WRITE) {
                        let _ = destination.delete_value(value_name);
                    }
                }
                return Err(error);
            }
            Ok(())
        }
        StartupLocator::StartupFolder {
            enabled_path,
            disabled_path,
            approved_hive,
            approved_path,
            approved_name,
        } => {
            let source = Path::new(disabled_path);
            let destination = Path::new(enabled_path);
            if !source.is_file() {
                return Err(format!("Disabled startup file was not found at {}", source.display()));
            }
            if destination.exists() {
                return Err(format!(
                    "Startup destination already exists at {}",
                    destination.display()
                ));
            }
            move_file_transactional(source, destination)
                .map_err(|error| format!("Could not re-enable startup file: {error}"))?;
            if let Err(error) = set_startup_approved(approved_hive, approved_path, approved_name, true) {
                let _ = move_file_transactional(destination, source);
                return Err(error);
            }
            Ok(())
        }
        StartupLocator::ScheduledTask { task_path, task_name } => {
            let script = format!(
                "$ErrorActionPreference='Stop'; Enable-ScheduledTask -TaskPath {} -TaskName {} | Out-Null",
                powershell_literal(task_path),
                powershell_literal(task_name)
            );
            ps_run_checked(&script).map(|_| ())
        }
    }
}

#[cfg(target_os = "windows")]
pub fn quarantine_startup(item_name: &str, locator: &StartupLocator) -> Result<String, String> {
    use winreg::enums::{KEY_READ, KEY_WRITE};

    let id = new_quarantine_id();
    let payload = match locator {
        StartupLocator::Registry {
            hive,
            value_path,
            approved_path,
            value_name,
            ..
        } => {
            let root = registry_root(hive);
            let value_key = root
                .open_subkey_with_flags(value_path, KEY_READ)
                .map_err(|error| format!("Could not open startup registry key: {error}"))?;
            let value = value_key
                .get_raw_value(value_name)
                .map_err(|error| format!("Could not read startup registry value: {error}"))?;
            let approved_value = root
                .open_subkey_with_flags(approved_path, KEY_READ)
                .ok()
                .and_then(|key| key.get_raw_value(value_name).ok())
                .map(|value| raw_registry_value(&value));
            StartupQuarantinePayload::Registry {
                value: raw_registry_value(&value),
                approved_value,
            }
        }
        StartupLocator::StartupFolder {
            enabled_path,
            disabled_path,
            ..
        } => {
            let source = if Path::new(enabled_path).is_file() {
                Path::new(enabled_path)
            } else if Path::new(disabled_path).is_file() {
                Path::new(disabled_path)
            } else {
                return Err("The exact startup file no longer exists".into());
            };
            let file_name = source
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| "Startup file name is invalid".to_string())?;
            let destination = quarantine_root()?.join("files").join(format!("{id}-{file_name}"));
            StartupQuarantinePayload::StartupFolder {
                quarantined_path: destination.to_string_lossy().into_owned(),
                original_path: source.to_string_lossy().into_owned(),
            }
        }
        StartupLocator::ScheduledTask { task_path, task_name } => {
            let xml_path = quarantine_root()?.join("tasks").join(format!("{id}.xml"));
            if let Some(parent) = xml_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("Could not create task quarantine directory: {error}"))?;
            }
            let script = format!(
                "$ErrorActionPreference='Stop'; Export-ScheduledTask -TaskPath {} -TaskName {} | Set-Content -LiteralPath {} -Encoding Unicode",
                powershell_literal(task_path),
                powershell_literal(task_name),
                powershell_literal(&xml_path.to_string_lossy())
            );
            ps_run_checked(&script)?;
            if std::fs::metadata(&xml_path).map_or(true, |metadata| metadata.len() == 0) {
                return Err("Scheduled task export produced no backup XML".into());
            }
            StartupQuarantinePayload::ScheduledTask {
                xml_path: xml_path.to_string_lossy().into_owned(),
            }
        }
    };

    let record = StartupQuarantineRecord {
        id: id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        item_name: item_name.to_string(),
        locator: locator.clone(),
        payload,
    };
    if let Err(error) = save_quarantine_record(&record) {
        if let StartupQuarantinePayload::ScheduledTask { xml_path } = &record.payload {
            let _ = std::fs::remove_file(xml_path);
        }
        return Err(error);
    }

    let mutation = match (&record.locator, &record.payload) {
        (
            StartupLocator::Registry {
                hive,
                value_path,
                approved_path,
                value_name,
                ..
            },
            _,
        ) => {
            let root = registry_root(hive);
            let value_key = root
                .open_subkey_with_flags(value_path, KEY_READ | KEY_WRITE)
                .map_err(|error| format!("Could not open startup registry key for quarantine: {error}"))?;
            value_key
                .delete_value(value_name)
                .map_err(|error| format!("Could not quarantine startup registry value: {error}"))?;
            if let Ok(approved_key) = root.open_subkey_with_flags(approved_path, KEY_WRITE) {
                let _ = approved_key.delete_value(value_name);
            }
            if value_key.get_raw_value(value_name).is_ok() {
                Err("Startup registry value still exists after quarantine".into())
            } else {
                Ok(())
            }
        }
        (
            StartupLocator::StartupFolder {
                enabled_path,
                disabled_path,
                ..
            },
            StartupQuarantinePayload::StartupFolder { quarantined_path, .. },
        ) => {
            let source = if Path::new(enabled_path).is_file() {
                Path::new(enabled_path)
            } else {
                Path::new(disabled_path)
            };
            let destination = Path::new(quarantined_path);
            move_file_transactional(source, destination)
                .map_err(|error| format!("Could not move startup file into quarantine: {error}"))
        }
        (StartupLocator::ScheduledTask { task_path, task_name }, StartupQuarantinePayload::ScheduledTask { .. }) => {
            let script = format!(
                "$ErrorActionPreference='Stop'; Unregister-ScheduledTask -TaskPath {} -TaskName {} -Confirm:$false",
                powershell_literal(task_path),
                powershell_literal(task_name)
            );
            ps_run_checked(&script).map(|_| ())
        }
        _ => Err("Quarantine record does not match the startup locator".into()),
    };

    if let Err(error) = mutation {
        match &record.payload {
            StartupQuarantinePayload::Registry { .. } => {
                let _ = restore_startup(&id);
            }
            StartupQuarantinePayload::StartupFolder { quarantined_path, .. } => {
                if Path::new(quarantined_path).exists() {
                    let _ = restore_startup(&id);
                } else {
                    let _ = std::fs::remove_file(quarantine_record_path(&id)?);
                }
            }
            StartupQuarantinePayload::ScheduledTask { xml_path } => {
                let _ = std::fs::remove_file(xml_path);
                let _ = std::fs::remove_file(quarantine_record_path(&id)?);
            }
        }
        return Err(error);
    }
    Ok(id)
}

#[cfg(target_os = "windows")]
pub fn restore_startup(id: &str) -> Result<StartupQuarantineRecord, String> {
    use winreg::enums::KEY_WRITE;

    let record = load_quarantine_record(id)?;
    match (&record.locator, &record.payload) {
        (
            StartupLocator::Registry {
                hive,
                value_path,
                approved_path,
                value_name,
                ..
            },
            StartupQuarantinePayload::Registry { value, approved_value },
        ) => {
            let root = registry_root(hive);
            let (value_key, _) = root
                .create_subkey(value_path)
                .map_err(|error| format!("Could not open startup registry key for restore: {error}"))?;
            value_key
                .set_raw_value(value_name, &restore_raw_registry_value(value)?)
                .map_err(|error| format!("Could not restore startup registry value: {error}"))?;
            if let Some(approved) = approved_value {
                let (approved_key, _) = root
                    .create_subkey(approved_path)
                    .map_err(|error| format!("Could not open StartupApproved key for restore: {error}"))?;
                approved_key
                    .set_raw_value(value_name, &restore_raw_registry_value(approved)?)
                    .map_err(|error| format!("Could not restore StartupApproved value: {error}"))?;
            } else if let Ok(approved_key) = root.open_subkey_with_flags(approved_path, KEY_WRITE) {
                let _ = approved_key.delete_value(value_name);
            }
            if value_key.get_raw_value(value_name).is_err() {
                return Err("Restored startup registry value could not be verified".into());
            }
        }
        (
            StartupLocator::StartupFolder { .. },
            StartupQuarantinePayload::StartupFolder {
                quarantined_path,
                original_path,
            },
        ) => {
            let destination = Path::new(original_path);
            if destination.exists() {
                return Err(format!(
                    "Cannot restore because {} already exists",
                    destination.display()
                ));
            }
            move_file_transactional(Path::new(quarantined_path), destination)
                .map_err(|error| format!("Could not restore startup file: {error}"))?;
        }
        (
            StartupLocator::ScheduledTask { task_path, task_name },
            StartupQuarantinePayload::ScheduledTask { xml_path },
        ) => {
            let script = format!(
                "$ErrorActionPreference='Stop'; $xml = Get-Content -LiteralPath {} -Raw; Register-ScheduledTask -TaskPath {} -TaskName {} -Xml $xml | Out-Null",
                powershell_literal(xml_path),
                powershell_literal(task_path),
                powershell_literal(task_name)
            );
            ps_run_checked(&script)?;
        }
        _ => return Err("Quarantine record does not match the startup locator".into()),
    }

    std::fs::remove_file(quarantine_record_path(id)?).map_err(|error| {
        format!("Startup item was restored, but its quarantine record could not be removed: {error}")
    })?;
    if let StartupQuarantinePayload::ScheduledTask { xml_path } = &record.payload {
        let _ = std::fs::remove_file(xml_path);
    }
    Ok(record)
}

#[cfg(not(target_os = "windows"))]
pub fn disable_startup(_locator: &StartupLocator) -> Result<(), String> {
    Err("Startup actions are only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn enable_startup(_locator: &StartupLocator) -> Result<(), String> {
    Err("Startup actions are only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn quarantine_startup(_item_name: &str, _locator: &StartupLocator) -> Result<String, String> {
    Err("Startup actions are only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn restore_startup(_id: &str) -> Result<StartupQuarantineRecord, String> {
    Err("Startup actions are only supported on Windows".into())
}

#[cfg(any())]
#[allow(dead_code)]
pub fn remove_startup_item(name: &str, source: &str) -> bool {
    use winreg::RegKey;
    use winreg::enums::*;

    if source.contains("HKCU") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu
            .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
            .and_then(|k| k.delete_value(name));
        let _ = hkcu
            .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run_Disabled", KEY_WRITE)
            .and_then(|k| k.delete_value(name));
        let _ = hkcu
            .open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
                KEY_WRITE,
            )
            .and_then(|k| k.delete_value(name));
        true
    } else if source.contains("HKLM") {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let _ = hklm
            .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
            .and_then(|k| k.delete_value(name));
        let _ = hklm
            .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run_Disabled", KEY_WRITE)
            .and_then(|k| k.delete_value(name));
        let _ = hklm
            .open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
                KEY_WRITE,
            )
            .and_then(|k| k.delete_value(name));
        let _ = hklm
            .open_subkey_with_flags(r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
            .and_then(|k| k.delete_value(name));
        true
    } else if source.contains("Startup Folder") {
        let mut deleted = false;
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut p = std::path::PathBuf::from(appdata);
            p.push(r"Microsoft\Windows\Start Menu\Programs\Startup");
            if let Ok(entries) = std::fs::read_dir(&p) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    let stem = ep.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if stem.eq_ignore_ascii_case(name) {
                        deleted = std::fs::remove_file(ep).is_ok() || deleted;
                    }
                }
            }
            let mut dis_p = p.clone();
            dis_p.push("_disabled");
            if let Ok(entries) = std::fs::read_dir(&dis_p) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    let stem = ep.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if stem.eq_ignore_ascii_case(name) {
                        deleted = std::fs::remove_file(ep).is_ok() || deleted;
                    }
                }
            }
        }
        deleted
    } else if source.contains("Task Scheduler") {
        let safe_name = name.replace('\'', "''");
        let script = format!(
            "Unregister-ScheduledTask -TaskName '{}' -Confirm:$false -EA SilentlyContinue; if ($?) {{ 'SUCCESS' }}",
            safe_name
        );
        if let Some(out) = ps_run(&script) {
            out.contains("SUCCESS")
        } else {
            false
        }
    } else {
        false
    }
}

/// Disable a startup item using native Windows StartupApproved binary flags (reversible).
#[cfg(any())]
#[allow(dead_code)]
pub fn disable_startup_item(name: &str, source: &str, _command: &str) -> bool {
    use winreg::RegKey;
    use winreg::enums::*;

    let disabled_bytes: [u8; 12] = [0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    if source.contains("HKCU") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
        if let Ok((app_key, _)) = hkcu.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&disabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            return app_key.set_raw_value(name, &reg_val).is_ok();
        }
        false
    } else if source.contains("HKLM") {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let path = if source.contains("32-bit") {
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32"
        } else {
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
        };
        if let Ok((app_key, _)) = hklm.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&disabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            return app_key.set_raw_value(name, &reg_val).is_ok();
        }
        false
    } else if source.contains("Startup Folder") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder";
        if let Ok((app_key, _)) = hkcu.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&disabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            let _ = app_key.set_raw_value(name, &reg_val);
        }

        // Also move file to _disabled if present in User Startup
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut src_path = std::path::PathBuf::from(appdata);
            src_path.push(r"Microsoft\Windows\Start Menu\Programs\Startup");
            let mut dst_path = src_path.clone();
            dst_path.push("_disabled");
            let _ = std::fs::create_dir_all(&dst_path);

            if let Ok(entries) = std::fs::read_dir(&src_path) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    let stem = ep.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if stem.eq_ignore_ascii_case(name) {
                        if let Some(fname) = ep.file_name() {
                            let mut dest = dst_path.clone();
                            dest.push(fname);
                            return std::fs::rename(&ep, &dest).is_ok();
                        }
                    }
                }
            }
        }
        true
    } else if source.contains("Task Scheduler") {
        let safe_name = name.replace('\'', "''");
        let script = format!(
            "Disable-ScheduledTask -TaskName '{}' -EA SilentlyContinue; if ($?) {{ 'SUCCESS' }}",
            safe_name
        );
        if let Some(out) = ps_run(&script) {
            out.contains("SUCCESS")
        } else {
            false
        }
    } else {
        false
    }
}

/// Re-enable a previously disabled item. Returns true on success.
#[cfg(any())]
#[allow(dead_code)]
pub fn reenable_startup_item(name: &str, source: &str) -> bool {
    use winreg::RegKey;
    use winreg::enums::*;

    let enabled_bytes: [u8; 12] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    if source.contains("HKCU") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
        if let Ok((app_key, _)) = hkcu.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&enabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            let _ = app_key.set_raw_value(name, &reg_val);
        }

        // Also restore from Run_Disabled if it was stored there
        let disabled_path = r"Software\Microsoft\Windows\CurrentVersion\Run_Disabled";
        let run_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
        if let Ok(dis_key) = hkcu.open_subkey_with_flags(disabled_path, KEY_READ) {
            if let Ok(val) = dis_key.get_raw_value(name) {
                if let Ok(run_key) = hkcu.open_subkey_with_flags(run_path, KEY_WRITE) {
                    let _ = run_key.set_raw_value(name, &val);
                    if let Ok(dis_key_w) = hkcu.open_subkey_with_flags(disabled_path, KEY_WRITE) {
                        let _ = dis_key_w.delete_value(name);
                    }
                }
            }
        }
        true
    } else if source.contains("HKLM") {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let path = if source.contains("32-bit") {
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32"
        } else {
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
        };
        if let Ok((app_key, _)) = hklm.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&enabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            let _ = app_key.set_raw_value(name, &reg_val);
        }
        true
    } else if source.contains("Startup Folder") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder";
        if let Ok((app_key, _)) = hkcu.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&enabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            let _ = app_key.set_raw_value(name, &reg_val);
        }

        // Also restore from _disabled folder back to Startup
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut startup_path = std::path::PathBuf::from(appdata);
            startup_path.push(r"Microsoft\Windows\Start Menu\Programs\Startup");
            let mut disabled_path = startup_path.clone();
            disabled_path.push("_disabled");

            if let Ok(entries) = std::fs::read_dir(&disabled_path) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    let stem = ep.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if stem.eq_ignore_ascii_case(name) {
                        if let Some(fname) = ep.file_name() {
                            let mut dest = startup_path.clone();
                            dest.push(fname);
                            return std::fs::rename(&ep, &dest).is_ok();
                        }
                    }
                }
            }
        }
        true
    } else if source.contains("Task Scheduler") {
        let safe_name = name.replace('\'', "''");
        let script = format!(
            "Enable-ScheduledTask -TaskName '{}' -EA SilentlyContinue; if ($?) {{ 'SUCCESS' }}",
            safe_name
        );
        if let Some(out) = ps_run(&script) {
            out.contains("SUCCESS")
        } else {
            false
        }
    } else {
        false
    }
}

#[cfg(target_os = "windows")]
pub fn open_file_location(path: &str) {
    let clean_path = path.trim().trim_matches('"');
    let _ = std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", clean_path))
        .spawn();
}

#[cfg(target_os = "windows")]
pub fn search_online(name: &str) {
    use std::os::windows::process::CommandExt;
    let query = format!("https://www.google.com/search?q=what+is+{}", urlenccode(name));
    let _ = std::process::Command::new("cmd")
        .creation_flags(0x08000000)
        .args(["/c", "start", "", &query])
        .spawn();
}

fn urlenccode(s: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

#[cfg(any())]
pub fn remove_startup_item(_name: &str, _source: &str) -> bool {
    false
}
#[cfg(any())]
pub fn disable_startup_item(_name: &str, _source: &str, _command: &str) -> bool {
    false
}
#[cfg(any())]
pub fn reenable_startup_item(_name: &str, _source: &str) -> bool {
    false
}
#[cfg(not(target_os = "windows"))]
pub fn open_file_location(_path: &str) {}
#[cfg(not(target_os = "windows"))]
pub fn search_online(_name: &str) {}
