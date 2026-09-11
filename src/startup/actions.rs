use super::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
const APPROVED_RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const APPROVED_RUN32: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32";
const APPROVED_FOLDER: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder";
const MAX_BACKUP_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRegistryValue {
    value_type: String,
    bytes: Vec<u8>,
}

// Content is embedded: a record can never redirect restore to an arbitrary XML/file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum StartupQuarantinePayload {
    Registry {
        value: RawRegistryValue,
        approved_value: Option<RawRegistryValue>,
    },
    StartupFolder {
        original_path: String,
        bytes: Vec<u8>,
    },
    ScheduledTask {
        xml: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupQuarantineRecord {
    version: u32,
    pub id: String,
    pub created_at: String,
    pub item_name: String,
    pub locator: StartupLocator,
    payload: StartupQuarantinePayload,
}

/// Only prepare_startup_restore can construct this capability. Neither audit text
/// nor deserialized UI commands can supply privileged restore authority.
#[derive(Clone, Debug)]
pub struct ReviewedStartupRestore {
    record: Arc<StartupQuarantineRecord>,
    encoded: Arc<[u8]>,
}

impl ReviewedStartupRestore {
    pub fn id(&self) -> &str {
        &self.record.id
    }
    pub fn item_name(&self) -> &str {
        &self.record.item_name
    }
    pub fn summary(&self) -> String {
        use sha2::{Digest, Sha256};
        let destination = match &self.record.locator {
            StartupLocator::Registry {
                hive,
                value_path,
                value_name,
                ..
            } => format!("{hive:?}\\{value_path}\\{value_name}"),
            StartupLocator::StartupFolder { .. } => match &self.record.payload {
                StartupQuarantinePayload::StartupFolder { original_path, .. } => original_path.clone(),
                _ => unreachable!(),
            },
            StartupLocator::ScheduledTask { task_path, task_name } => format!("{task_path}{task_name}"),
        };
        format!(
            "Restore {} to {}. Protected backup SHA-256: {:x}. Existing destinations are never replaced.",
            self.item_name(),
            destination,
            Sha256::digest(&self.encoded)
        )
    }
}

pub(crate) fn valid_quarantine_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

pub(crate) fn new_quarantine_id() -> String {
    let nanos = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros() * 1_000);
    format!("{}-{nanos}", std::process::id())
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.ends_with([' ', '.'])
        && !name.chars().any(|ch| ch.is_control() || "\\/:*?\"<>|".contains(ch))
}

fn validate_locator_scope(locator: &StartupLocator) -> Result<(), String> {
    match locator {
        StartupLocator::Registry {
            hive,
            value_path,
            enabled_value_path,
            approved_path,
            value_name,
        } => {
            let wow = *hive == StartupRegistryHive::LocalMachine && enabled_value_path.eq_ignore_ascii_case(RUN32);
            let expected = if wow { RUN32 } else { RUN };
            if !enabled_value_path.eq_ignore_ascii_case(expected)
                || !(value_path.eq_ignore_ascii_case(expected)
                    || (!wow && value_path.eq_ignore_ascii_case(&format!("{RUN}_Disabled"))))
                || !approved_path.eq_ignore_ascii_case(if wow { APPROVED_RUN32 } else { APPROVED_RUN })
                || !valid_name(value_name)
            {
                return Err("Registry startup locator is outside the allowed Run/StartupApproved scope".into());
            }
        }
        StartupLocator::StartupFolder {
            enabled_path,
            disabled_path,
            approved_hive,
            approved_path,
            approved_name,
        } => {
            if !valid_name(approved_name) || !approved_path.eq_ignore_ascii_case(APPROVED_FOLDER) {
                return Err("Invalid startup folder approval scope".into());
            }
            #[cfg(target_os = "windows")]
            {
                let root = crate::app_paths::startup_folder(*approved_hive == StartupRegistryHive::LocalMachine)?;
                validate_folder_paths(Path::new(enabled_path), Path::new(disabled_path), &root, approved_name)?;
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = (enabled_path, disabled_path, approved_hive);
                return Err("Startup actions require Windows".into());
            }
        }
        StartupLocator::ScheduledTask { task_path, task_name } => {
            if !valid_name(task_name)
                || !task_path.starts_with('\\')
                || !task_path.ends_with('\\')
                || (task_path != "\\" && !task_path[1..task_path.len() - 1].split('\\').all(valid_name))
                || task_path.to_ascii_lowercase().starts_with("\\microsoft\\windows\\")
            {
                return Err("Scheduled task locator is outside the allowed non-system task scope".into());
            }
        }
    }
    Ok(())
}

fn validate_folder_paths(enabled: &Path, disabled: &Path, root: &Path, name: &str) -> Result<(), String> {
    if !valid_name(name) || enabled != root.join(name) || disabled != root.join("_disabled").join(name) {
        return Err("Startup destination escaped its exact known-folder scope".into());
    }
    Ok(())
}

fn validate_record(record: &StartupQuarantineRecord, id: &str) -> Result<(), String> {
    if record.version != 1 || record.id != id || !valid_quarantine_id(id) {
        return Err("Startup backup identity/version is invalid; legacy backups require manual revalidation".into());
    }
    validate_locator_scope(&record.locator)?;
    match (&record.locator, &record.payload) {
        (StartupLocator::Registry { .. }, StartupQuarantinePayload::Registry { value, approved_value }) => {
            if !matches!(value.value_type.as_str(), "REG_SZ" | "REG_EXPAND_SZ")
                || value.bytes.len() % 2 != 0
                || approved_value
                    .as_ref()
                    .is_some_and(|value| value.value_type != "REG_BINARY")
            {
                return Err("Invalid startup registry payload type".into());
            }
        }
        (
            StartupLocator::StartupFolder {
                enabled_path,
                disabled_path,
                ..
            },
            StartupQuarantinePayload::StartupFolder { original_path, .. },
        ) if original_path == enabled_path || original_path == disabled_path => {}
        (StartupLocator::ScheduledTask { .. }, StartupQuarantinePayload::ScheduledTask { xml })
            if !xml.trim().is_empty() => {}
        _ => return Err("Startup backup payload does not match its allowed destination".into()),
    }
    Ok(())
}

fn verify_review(review: &ReviewedStartupRestore, current: &[u8]) -> Result<(), String> {
    if current != review.encoded.as_ref() {
        return Err("Protected backup changed after review; cancel and review the backup again".into());
    }
    validate_record(&review.record, review.id())
}

#[derive(Debug, PartialEq, Eq)]
enum FolderEnable {
    ApprovalOnly,
    Move,
}
fn folder_enable_state(enabled: bool, disabled: bool) -> Result<FolderEnable, String> {
    match (enabled, disabled) {
        (true, false) => Ok(FolderEnable::ApprovalOnly),
        (false, true) => Ok(FolderEnable::Move),
        (true, true) => Err("Both enabled and disabled startup files exist; refusing a destination collision".into()),
        (false, false) => Err("Neither exact startup file exists; refresh the inventory".into()),
    }
}

#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Write};
    use std::os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    };
    use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
    use windows::Win32::Security::Authorization::*;
    use windows::Win32::Security::*;
    use windows::Win32::Storage::FileSystem::*;

    // FILE_SHARE_READ only: the same opened object is verified and mutated.
    const READ: u32 = 0x80000000;
    const WRITE: u32 = 0x40000000;
    const DELETE_ACCESS: u32 = 0x10000;
    const READ_CONTROL_ACCESS: u32 = 0x20000;
    const REPARSE: u32 = 0x400;
    const OPEN_REPARSE: u32 = 0x00200000;
    const BACKUP_SEMANTICS: u32 = 0x02000000;

    fn handle(file: &File) -> HANDLE {
        HANDLE(file.as_raw_handle())
    }
    struct Descriptor(PSECURITY_DESCRIPTOR);
    impl Drop for Descriptor {
        fn drop(&mut self) {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.0.0)));
            }
        }
    }

    pub(super) fn ancestors(path: &Path) -> Result<Vec<File>, String> {
        if !path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
        {
            return Err("An absolute non-traversing startup path is required".into());
        }
        let mut locks = Vec::new();
        let mut parents: Vec<_> = path.ancestors().skip(1).collect();
        parents.reverse();
        for parent in parents {
            let file = OpenOptions::new()
                .access_mode(READ_CONTROL_ACCESS | 0x80)
                .share_mode(3)
                .custom_flags(BACKUP_SEMANTICS | OPEN_REPARSE)
                .open(parent)
                .map_err(|error| format!("Could not lock startup ancestor: {error}"))?;
            let metadata = file.metadata().map_err(|error| error.to_string())?;
            if !metadata.is_dir() || metadata.file_attributes() & REPARSE != 0 {
                return Err("Startup path has a reparse-point or non-directory ancestor".into());
            }
            locks.push(file);
        }
        Ok(locks)
    }

    fn check_file(file: &File) -> Result<(), String> {
        let metadata = file.metadata().map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata.file_attributes() & REPARSE != 0 {
            return Err("Startup source is not an ordinary non-reparse file".into());
        }
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        unsafe { GetFileInformationByHandle(handle(file), &mut info) }.map_err(|error| error.to_string())?;
        if info.nNumberOfLinks != 1 {
            return Err("Hard-linked startup content is not eligible for restore or mutation".into());
        }
        Ok(())
    }

    pub(super) fn open_source(path: &Path) -> Result<File, String> {
        let file = OpenOptions::new()
            .access_mode(READ | DELETE_ACCESS | READ_CONTROL_ACCESS)
            .share_mode(1)
            .custom_flags(OPEN_REPARSE)
            .open(path)
            .map_err(|error| format!("Could not lock startup source: {error}"))?;
        check_file(&file)?;
        Ok(file)
    }

    pub(super) fn read_bytes(file: &mut File) -> Result<Vec<u8>, String> {
        if file.metadata().map_err(|error| error.to_string())?.len() > MAX_BACKUP_BYTES {
            return Err("Startup backup exceeds the 32 MiB safety limit".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_BACKUP_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() as u64 > MAX_BACKUP_BYTES {
            return Err("Startup backup exceeds the safety limit".into());
        }
        Ok(bytes)
    }

    pub(super) fn delete_owned(file: &File) -> Result<(), String> {
        let info = FILE_DISPOSITION_INFO { DeleteFile: true };
        unsafe {
            SetFileInformationByHandle(
                handle(file),
                FileDispositionInfo,
                (&info as *const FILE_DISPOSITION_INFO).cast(),
                std::mem::size_of_val(&info) as u32,
            )
        }
        .map_err(|error| format!("Could not delete the exact opened startup object: {error}"))
    }

    pub(super) fn create_content(path: &Path, bytes: &[u8]) -> Result<File, String> {
        let mut file = OpenOptions::new()
            .access_mode(WRITE | DELETE_ACCESS | READ_CONTROL_ACCESS)
            .share_mode(1)
            .custom_flags(OPEN_REPARSE)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| {
                format!("Could not create startup destination (existing entries are never replaced): {error}")
            })?;
        if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
            let cleanup = delete_owned(&file);
            return Err(format!(
                "Could not persist startup content: {error}; owned-file cleanup: {cleanup:?}"
            ));
        }
        Ok(file)
    }

    fn restricted_acl(file: &File, directory: bool) -> Result<(), String> {
        unsafe {
            let mut owner = PSID::default();
            let mut acl = std::ptr::null_mut();
            let mut descriptor = PSECURITY_DESCRIPTOR::default();
            GetSecurityInfo(
                handle(file),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                Some(&mut owner),
                None,
                Some(&mut acl),
                None,
                Some(&mut descriptor),
            )
            .ok()
            .map_err(|error| error.to_string())?;
            let _descriptor = Descriptor(descriptor);
            let trusted_sid = |sid| {
                IsWellKnownSid(sid, WinBuiltinAdministratorsSid).as_bool()
                    || IsWellKnownSid(sid, WinLocalSystemSid).as_bool()
            };
            if owner.0.is_null() || !trusted_sid(owner) || acl.is_null() || (*acl).AceCount == 0 {
                return Err("Backup owner/ACL is untrusted; existing writable backups are never promoted".into());
            }
            let mut control = 0;
            let mut revision = 0;
            GetSecurityDescriptorControl(descriptor, &mut control, &mut revision).map_err(|error| error.to_string())?;
            if directory && control & SE_DACL_PROTECTED.0 == 0 {
                return Err("Quarantine directory must have a protected non-inheriting ACL".into());
            }
            for index in 0..(*acl).AceCount {
                let mut ace = std::ptr::null_mut();
                GetAce(acl, u32::from(index), &mut ace).map_err(|error| error.to_string())?;
                let ace = &*(ace as *const ACCESS_ALLOWED_ACE);
                // Only ordinary allow ACEs for SYSTEM/Administrators. No creator-owner,
                // user, conditional, object-specific, or inherited broad grants.
                if ace.Header.AceType != 0 || !trusted_sid(PSID((&ace.SidStart as *const u32).cast_mut().cast())) {
                    return Err("Backup ACL allows an untrusted identity; refusing elevated restore".into());
                }
            }
        }
        Ok(())
    }

    pub(super) struct Store {
        pub root: PathBuf,
        _ancestors: Vec<File>,
        _directory: File,
    }
    impl Store {
        pub fn open(create: bool) -> Result<Self, String> {
            if !crate::privilege::is_app_elevated() {
                return Err(
                    "Startup quarantine and restore require administrator approval for the protected backup store"
                        .into(),
                );
            }
            Self::at(crate::app_paths::protected_startup_quarantine_dir()?, create)
        }
        fn at(root: PathBuf, create: bool) -> Result<Self, String> {
            let parents = ancestors(&root)?;
            if create && !root.try_exists().map_err(|error| error.to_string())? {
                let name: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
                unsafe {
                    let mut descriptor = PSECURITY_DESCRIPTOR::default();
                    ConvertStringSecurityDescriptorToSecurityDescriptorW(
                        windows::core::w!("O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"),
                        1,
                        &mut descriptor,
                        None,
                    )
                    .map_err(|error| error.to_string())?;
                    let _descriptor = Descriptor(descriptor);
                    let attributes = SECURITY_ATTRIBUTES {
                        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                        lpSecurityDescriptor: descriptor.0,
                        bInheritHandle: false.into(),
                    };
                    CreateDirectoryW(windows::core::PCWSTR(name.as_ptr()), Some(&attributes))
                        .map_err(|error| format!("Could not create protected quarantine directory: {error}"))?;
                }
            }
            let directory = OpenOptions::new().access_mode(READ_CONTROL_ACCESS | 0x80).share_mode(3)
                .custom_flags(BACKUP_SEMANTICS | OPEN_REPARSE).open(&root)
                .map_err(|error| format!("Protected startup backup is unavailable: {error}. Legacy per-user backups remain untouched; export them for manual revalidation, not elevated Undo."))?;
            let metadata = directory.metadata().map_err(|error| error.to_string())?;
            if !metadata.is_dir() || metadata.file_attributes() & REPARSE != 0 {
                return Err("Quarantine root is a reparse point or not a directory".into());
            }
            restricted_acl(&directory, true)?;
            Ok(Self {
                root,
                _ancestors: parents,
                _directory: directory,
            })
        }
        pub fn record_path(&self, id: &str) -> Result<PathBuf, String> {
            if !valid_quarantine_id(id) {
                return Err("Invalid startup quarantine identifier".into());
            }
            Ok(self.root.join(format!("{id}.json")))
        }
        pub fn read(&self, id: &str) -> Result<(File, Vec<u8>), String> {
            let mut file = open_source(&self.record_path(id)?)?;
            restricted_acl(&file, false)?;
            let bytes = read_bytes(&mut file)?;
            Ok((file, bytes))
        }
        pub fn save(&self, record: &StartupQuarantineRecord) -> Result<File, String> {
            let bytes = serde_json::to_vec(record).map_err(|error| error.to_string())?;
            if bytes.len() as u64 > MAX_BACKUP_BYTES {
                return Err("Encoded startup backup exceeds the safety limit".into());
            }
            let file = create_content(&self.record_path(&record.id)?, &bytes)?;
            if let Err(error) = restricted_acl(&file, false) {
                let _ = delete_owned(&file);
                return Err(error);
            }
            Ok(file)
        }
    }

    pub(super) fn move_file(source: &Path, destination: &Path) -> Result<(), String> {
        let _source_parents = ancestors(source)?;
        let _destination_parents = ancestors(destination)?;
        let mut source_file = open_source(source)?;
        let mut destination_file = OpenOptions::new()
            .access_mode(WRITE | DELETE_ACCESS)
            .share_mode(1)
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|error| format!("Could not create startup destination without replacement: {error}"))?;
        let copied = std::io::copy(&mut source_file, &mut destination_file)
            .and_then(|_| destination_file.sync_all())
            .map_err(|error| error.to_string());
        let result = copied.and_then(|_| delete_owned(&source_file));
        if let Err(error) = result {
            let rollback = delete_owned(&destination_file);
            return Err(format!(
                "Startup move failed: {error}; owned-destination rollback: {rollback:?}"
            ));
        }
        Ok(())
    }

    pub(super) fn ensure_disabled_parent(path: &Path) -> Result<(), String> {
        let parent = path.parent().ok_or("Invalid disabled path")?;
        let _locks = ancestors(parent)?;
        match std::fs::create_dir(parent) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                // Opening all ancestors of the file validates and locks this directory.
                ancestors(path).map(|_| ())
            }
            Err(error) => Err(format!("Could not create disabled startup folder: {error}")),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn existing_destination_is_never_overwritten_or_removed() {
            let root = std::env::temp_dir().join(format!("sysmon-startup-collision-{}", new_quarantine_id()));
            std::fs::create_dir(&root).unwrap();
            let source = root.join("source.lnk");
            let destination = root.join("destination.lnk");
            std::fs::write(&source, b"old backup").unwrap();
            std::fs::write(&destination, b"replacement").unwrap();
            assert!(move_file(&source, &destination).is_err());
            assert_eq!(std::fs::read(&source).unwrap(), b"old backup");
            assert_eq!(std::fs::read(&destination).unwrap(), b"replacement");
            std::fs::remove_dir_all(root).unwrap();
        }
        #[test]
        fn rollback_deletes_only_the_opened_owned_object() {
            let root = std::env::temp_dir().join(format!("sysmon-startup-rollback-{}", new_quarantine_id()));
            std::fs::create_dir(&root).unwrap();
            let source = root.join("source.lnk");
            let destination = root.join("destination.lnk");
            std::fs::write(&source, b"untouched source").unwrap();
            let owned = create_content(&destination, b"partial transition").unwrap();
            delete_owned(&owned).unwrap();
            drop(owned);
            assert!(!destination.exists());
            assert_eq!(std::fs::read(&source).unwrap(), b"untouched source");
            std::fs::remove_dir_all(root).unwrap();
        }
        #[test]
        #[ignore = "Requires elevated Windows token; only touches a disposable directory"]
        fn protected_store_round_trip_and_untrusted_acl_refusal() {
            let parent = std::env::temp_dir().join(format!("sysmon-startup-acl-{}", new_quarantine_id()));
            std::fs::create_dir(&parent).unwrap();
            let root = parent.join("protected");
            let store = Store::at(root, true).unwrap();
            let record = super::super::tests::registry_record();
            let saved = store.save(&record).unwrap();
            drop(saved);
            let (file, bytes) = store.read(&record.id).unwrap();
            assert_eq!(
                serde_json::from_slice::<StartupQuarantineRecord>(&bytes).unwrap().id,
                record.id
            );
            delete_owned(&file).unwrap();
            drop(file);
            drop(store);
            let untrusted = parent.join("untrusted");
            std::fs::create_dir(&untrusted).unwrap();
            assert!(Store::at(untrusted, true).is_err());
            std::fs::remove_dir_all(parent).unwrap();
        }
    }
}

#[cfg(target_os = "windows")]
pub fn prepare_startup_restore(id: &str) -> Result<ReviewedStartupRestore, String> {
    let store = native::Store::open(false)?;
    let (_file, bytes) = store.read(id)?;
    let record: StartupQuarantineRecord =
        serde_json::from_slice(&bytes).map_err(|error| format!("Invalid protected startup record: {error}"))?;
    validate_record(&record, id)?;
    Ok(ReviewedStartupRestore {
        record: Arc::new(record),
        encoded: bytes.into(),
    })
}

#[cfg(not(target_os = "windows"))]
pub fn prepare_startup_restore(_id: &str) -> Result<ReviewedStartupRestore, String> {
    Err("Startup actions are only supported on Windows".into())
}

#[cfg(target_os = "windows")]
fn registry_root(hive: &StartupRegistryHive) -> winreg::RegKey {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    winreg::RegKey::predef(match hive {
        StartupRegistryHive::CurrentUser => HKEY_CURRENT_USER,
        StartupRegistryHive::LocalMachine => HKEY_LOCAL_MACHINE,
    })
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
    let vtype = match value.value_type.as_str() {
        "REG_SZ" => REG_SZ,
        "REG_EXPAND_SZ" => REG_EXPAND_SZ,
        "REG_BINARY" => REG_BINARY,
        _ => return Err("Unsupported startup registry value type".into()),
    };
    Ok(winreg::RegValue {
        bytes: value.bytes.clone().into(),
        vtype,
    })
}

#[cfg(target_os = "windows")]
fn optional_registry_value(key: &winreg::RegKey, name: &str) -> Result<Option<RawRegistryValue>, String> {
    match key.get_raw_value(name) {
        Ok(value) => Ok(Some(raw_registry_value(&value))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Could not inspect startup destination: {error}")),
    }
}

#[cfg(target_os = "windows")]
fn read_approval(hive: &StartupRegistryHive, path: &str, name: &str) -> Result<Option<RawRegistryValue>, String> {
    match registry_root(hive).open_subkey_with_flags(path, winreg::enums::KEY_READ) {
        Ok(key) => optional_registry_value(&key, name),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Could not inspect startup approval: {error}")),
    }
}

#[cfg(target_os = "windows")]
fn powershell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(target_os = "windows")]
fn ps_run_checked(script: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let script = format!(
        "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); {script}"
    );
    let output = std::process::Command::new("powershell.exe")
        .creation_flags(0x08000000)
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|error| format!("Could not launch PowerShell: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "Startup task operation failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(target_os = "windows")]
fn set_startup_approved(hive: &StartupRegistryHive, path: &str, name: &str, enabled: bool) -> Result<(), String> {
    let mut bytes = [0u8; 12];
    bytes[0] = if enabled { 2 } else { 3 };
    let (key, _) = registry_root(hive)
        .create_subkey(path)
        .map_err(|error| error.to_string())?;
    let value = winreg::RegValue {
        bytes: bytes.to_vec().into(),
        vtype: winreg::enums::REG_BINARY,
    };
    key.set_raw_value(name, &value)
        .map_err(|error| format!("Could not update startup approval: {error}"))?;
    if key.get_raw_value(name).map_err(|error| error.to_string())? != value {
        return Err("Startup approval changed during the operation".into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn disable_startup(locator: &StartupLocator) -> Result<(), String> {
    validate_locator_scope(locator)?;
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
            // Windows' native approval-only form avoids moving user files on disable.
            // Legacy moved-file entries remain supported by enable_startup.
            if folder_enable_state(
                Path::new(enabled_path)
                    .try_exists()
                    .map_err(|error| error.to_string())?,
                Path::new(disabled_path)
                    .try_exists()
                    .map_err(|error| error.to_string())?,
            )? == FolderEnable::Move
            {
                return Err("Startup file is already in the disabled folder; refresh its state".into());
            }
            let _locks = native::ancestors(Path::new(enabled_path))?;
            let _file = native::open_source(Path::new(enabled_path))?;
            set_startup_approved(approved_hive, approved_path, approved_name, false)
        }
        StartupLocator::ScheduledTask { task_path, task_name } => ps_run_checked(&format!(
            "Disable-ScheduledTask -TaskPath {} -TaskName {} | Out-Null",
            powershell_literal(task_path),
            powershell_literal(task_name)
        ))
        .map(|_| ()),
    }
}

#[cfg(target_os = "windows")]
pub fn enable_startup(locator: &StartupLocator) -> Result<(), String> {
    validate_locator_scope(locator)?;
    match locator {
        StartupLocator::Registry {
            hive,
            value_path,
            enabled_value_path,
            approved_path,
            value_name,
        } => {
            if value_path == enabled_value_path {
                return set_startup_approved(hive, approved_path, value_name, true);
            }
            use winreg::enums::{KEY_READ, KEY_WRITE};
            let root = registry_root(hive);
            let source = root
                .open_subkey_with_flags(value_path, KEY_READ | KEY_WRITE)
                .map_err(|error| error.to_string())?;
            let value = source.get_raw_value(value_name).map_err(|error| error.to_string())?;
            let (destination, _) = root
                .create_subkey(enabled_value_path)
                .map_err(|error| error.to_string())?;
            refuse_registry_collision(optional_registry_value(&destination, value_name)?.as_ref())?;
            destination
                .set_raw_value(value_name, &value)
                .map_err(|error| error.to_string())?;
            // Keep the disabled source until approval succeeds. Rollback only a value
            // still byte-for-byte equal to this operation's newly created value.
            if let Err(error) = set_startup_approved(hive, approved_path, value_name, true) {
                let rollback = remove_registry_owned(&destination, value_name, &raw_registry_value(&value));
                return Err(format!(
                    "{error}; enabled-value rollback: {rollback:?}; disabled original retained"
                ));
            }
            if source.get_raw_value(value_name).map_err(|error| error.to_string())? != value {
                return Err("Startup enabled, but disabled copy changed externally and was retained".into());
            }
            source
                .delete_value(value_name)
                .map_err(|error| format!("Startup enabled, but disabled copy could not be removed: {error}"))
        }
        StartupLocator::StartupFolder {
            enabled_path,
            disabled_path,
            approved_hive,
            approved_path,
            approved_name,
        } => {
            let enabled = Path::new(enabled_path);
            let disabled = Path::new(disabled_path);
            match folder_enable_state(
                enabled.try_exists().map_err(|error| error.to_string())?,
                disabled.try_exists().map_err(|error| error.to_string())?,
            )? {
                FolderEnable::ApprovalOnly => {
                    let _locks = native::ancestors(enabled)?;
                    let _file = native::open_source(enabled)?;
                    set_startup_approved(approved_hive, approved_path, approved_name, true)
                }
                FolderEnable::Move => {
                    // Set approval first: if denied, the file remains in _disabled.
                    // A failed move is reported, never followed by deleting an unrelated file.
                    set_startup_approved(approved_hive, approved_path, approved_name, true)?;
                    native::move_file(disabled, enabled)
                        .map_err(|error| format!("Approval enabled but startup file remains disabled: {error}"))
                }
            }
        }
        StartupLocator::ScheduledTask { task_path, task_name } => ps_run_checked(&format!(
            "Enable-ScheduledTask -TaskPath {} -TaskName {} | Out-Null",
            powershell_literal(task_path),
            powershell_literal(task_name)
        ))
        .map(|_| ()),
    }
}

fn refuse_registry_collision(existing: Option<&RawRegistryValue>) -> Result<(), String> {
    if existing.is_some() {
        Err("Startup registry destination already exists; refusing to replace it".into())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn remove_registry_owned(key: &winreg::RegKey, name: &str, owned: &RawRegistryValue) -> Result<(), String> {
    if optional_registry_value(key, name)?.as_ref() != Some(owned) {
        return Err("Registry rollback refused because the value changed externally".into());
    }
    key.delete_value(name).map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn task_export_script(task_path: &str, task_name: &str) -> String {
    format!(
        "$task=Get-ScheduledTask -TaskPath {} -TaskName {}; if (@($task).Count -ne 1) {{ throw 'Task identity is ambiguous' }}; if ([string]$task.Principal.LogonType -in @('Password','InteractiveTokenOrPassword')) {{ throw 'This task requires credentials not present in XML. Quarantine refused before deletion; export and restore with Task Scheduler and credentials.' }}; if (-not ($task.Triggers | Where-Object {{ $_.CimClass.CimClassName -eq 'MSFT_TaskLogonTrigger' }})) {{ throw 'Only logon startup tasks may be quarantined' }}; Export-ScheduledTask -TaskPath {} -TaskName {}",
        powershell_literal(task_path),
        powershell_literal(task_name),
        powershell_literal(task_path),
        powershell_literal(task_name)
    )
}

#[cfg(target_os = "windows")]
pub fn quarantine_startup(item_name: &str, locator: &StartupLocator) -> Result<String, String> {
    validate_locator_scope(locator)?;
    let store = native::Store::open(true)?;
    let id = new_quarantine_id();
    let mut source_file = None;
    let mut source_locks = Vec::new();
    let payload = match locator {
        StartupLocator::Registry {
            hive,
            value_path,
            approved_path,
            value_name,
            ..
        } => {
            let key = registry_root(hive)
                .open_subkey_with_flags(value_path, winreg::enums::KEY_READ)
                .map_err(|error| error.to_string())?;
            let value = key.get_raw_value(value_name).map_err(|error| error.to_string())?;
            StartupQuarantinePayload::Registry {
                value: raw_registry_value(&value),
                approved_value: read_approval(hive, approved_path, value_name)?,
            }
        }
        StartupLocator::StartupFolder {
            enabled_path,
            disabled_path,
            ..
        } => {
            let enabled = Path::new(enabled_path);
            let disabled = Path::new(disabled_path);
            let state = folder_enable_state(
                enabled.try_exists().map_err(|error| error.to_string())?,
                disabled.try_exists().map_err(|error| error.to_string())?,
            )?;
            let source = if state == FolderEnable::ApprovalOnly {
                enabled
            } else {
                disabled
            };
            source_locks = native::ancestors(source)?;
            let mut file = native::open_source(source)?;
            let bytes = native::read_bytes(&mut file)?;
            source_file = Some(file);
            StartupQuarantinePayload::StartupFolder {
                original_path: source.to_string_lossy().into_owned(),
                bytes,
            }
        }
        StartupLocator::ScheduledTask { task_path, task_name } => StartupQuarantinePayload::ScheduledTask {
            xml: ps_run_checked(&task_export_script(task_path, task_name))?,
        },
    };
    let record = StartupQuarantineRecord {
        version: 1,
        id: id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        item_name: item_name.into(),
        locator: locator.clone(),
        payload,
    };
    validate_record(&record, &id)?;
    let backup = store.save(&record)?;
    let mutation = match (&record.locator, &record.payload) {
        (
            StartupLocator::Registry {
                hive,
                value_path,
                value_name,
                ..
            },
            StartupQuarantinePayload::Registry { value, .. },
        ) => {
            let key = registry_root(hive)
                .open_subkey_with_flags(value_path, winreg::enums::KEY_READ | winreg::enums::KEY_WRITE)
                .map_err(|error| error.to_string())?;
            // Approval state is not removed: quarantine has one registry mutation,
            // and restore refuses an intervening approval change rather than overwriting it.
            remove_registry_owned(&key, value_name, value)
        }
        (StartupLocator::StartupFolder { .. }, StartupQuarantinePayload::StartupFolder { .. }) => {
            native::delete_owned(source_file.as_ref().ok_or("Missing locked startup source")?)
        }
        (StartupLocator::ScheduledTask { task_path, task_name }, StartupQuarantinePayload::ScheduledTask { .. }) => {
            let path = store.record_path(&id)?;
            // Re-export and compare immediately before delete; password-backed tasks
            // fail again here if their principal changed while the backup was saved.
            ps_run_checked(&format!("$saved=(Get-Content -LiteralPath {} -Encoding UTF8 -Raw | ConvertFrom-Json).payload.ScheduledTask.xml; $current=& {{ {} }}; if ($current.Trim() -cne $saved.Trim()) {{ throw 'Task changed after backup; quarantine refused' }}; Unregister-ScheduledTask -TaskPath {} -TaskName {} -Confirm:$false",
                powershell_literal(&path.to_string_lossy()), task_export_script(task_path, task_name), powershell_literal(task_path), powershell_literal(task_name))).map(|_| ())
        }
        _ => Err("Startup quarantine payload mismatch".into()),
    };
    drop(source_file);
    drop(source_locks);
    if let Err(error) = mutation {
        // Never fabricate a restore after a failed mutation. Keep the protected
        // evidence because a task provider can report failure after changing state.
        return Err(format!(
            "{error}. Protected backup {id} retained for verified recovery; no automatic rollback was attempted."
        ));
    }
    drop(backup);
    Ok(id)
}

#[cfg(target_os = "windows")]
pub fn restore_startup(review: &ReviewedStartupRestore) -> Result<StartupQuarantineRecord, String> {
    let store = native::Store::open(false)?;
    let (record_file, bytes) = store.read(review.id())?;
    verify_review(review, &bytes)?;
    let record = &review.record;
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
            if read_approval(hive, approved_path, value_name)? != *approved_value {
                return Err("Startup approval changed since quarantine; refusing stale restore".into());
            }
            let root = registry_root(hive);
            let key = root
                .open_subkey_with_flags(value_path, winreg::enums::KEY_READ | winreg::enums::KEY_WRITE)
                .map_err(|error| {
                    format!("Original startup key is unavailable; refusing to create a replacement key: {error}")
                })?;
            refuse_registry_collision(optional_registry_value(&key, value_name)?.as_ref())?;
            key.set_raw_value(value_name, &restore_raw_registry_value(value)?)
                .map_err(|error| error.to_string())?;
            if optional_registry_value(&key, value_name)?.as_ref() != Some(value) {
                return Err(
                    "Startup value changed during restore; backup retained and external value left untouched".into(),
                );
            }
        }
        (
            StartupLocator::StartupFolder { disabled_path, .. },
            StartupQuarantinePayload::StartupFolder { original_path, bytes },
        ) => {
            let destination = Path::new(original_path);
            if original_path == disabled_path {
                native::ensure_disabled_parent(destination)?;
            }
            let _locks = native::ancestors(destination)?;
            let _file = native::create_content(destination, bytes)?;
        }
        (StartupLocator::ScheduledTask { task_path, task_name }, StartupQuarantinePayload::ScheduledTask { .. }) => {
            let path = store.record_path(review.id())?;
            // Register-ScheduledTask without -Force is create-only. XML remains in
            // the same ACL-checked, write/delete-locked record throughout execution.
            ps_run_checked(&format!(
                "$xml=(Get-Content -LiteralPath {} -Encoding UTF8 -Raw | ConvertFrom-Json).payload.ScheduledTask.xml; $settings=[System.Xml.XmlReaderSettings]::new(); $settings.DtdProcessing=[System.Xml.DtdProcessing]::Prohibit; $settings.XmlResolver=$null; $reader=[System.Xml.XmlReader]::Create([System.IO.StringReader]::new($xml),$settings); $doc=[System.Xml.XmlDocument]::new(); $doc.XmlResolver=$null; try {{ $doc.Load($reader) }} finally {{ $reader.Dispose() }}; if ($doc.SelectSingleNode(\"//*[local-name()='LogonType' and (text()='Password' or text()='InteractiveTokenOrPassword')]\")) {{ throw 'Task restore requires credentials; no task was registered' }}; if (-not $doc.SelectSingleNode(\"//*[local-name()='Triggers']/*[local-name()='LogonTrigger']\")) {{ throw 'Backup is not a logon startup task' }}; $service=New-Object -ComObject 'Schedule.Service'; $service.Connect(); $folder=$service.GetFolder({}); if (@($folder.GetTasks(1) | Where-Object {{ $_.Name -ieq {} }}).Count -ne 0) {{ throw 'Task destination already exists; refusing replacement' }}; Register-ScheduledTask -TaskPath {} -TaskName {} -Xml $xml -ErrorAction Stop | Out-Null",
                powershell_literal(&path.to_string_lossy()),
                powershell_literal(task_path),
                powershell_literal(task_name),
                powershell_literal(task_path),
                powershell_literal(task_name)
            ))?;
        }
        _ => return Err("Startup restore payload mismatch".into()),
    }
    native::delete_owned(&record_file)
        .map_err(|error| format!("Startup restored, but protected backup retirement failed: {error}"))?;
    Ok(record.as_ref().clone())
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
pub fn restore_startup(_review: &ReviewedStartupRestore) -> Result<StartupQuarantineRecord, String> {
    Err("Startup actions are only supported on Windows".into())
}

#[cfg(target_os = "windows")]
pub fn open_file_location(path: &str) {
    let _ = std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", path.trim().trim_matches('"')))
        .spawn();
}
#[cfg(target_os = "windows")]
pub fn search_online(name: &str) {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    use std::os::windows::process::CommandExt;
    let query = format!(
        "https://www.google.com/search?q=what+is+{}",
        utf8_percent_encode(name, NON_ALPHANUMERIC)
    );
    let _ = std::process::Command::new("cmd")
        .creation_flags(0x08000000)
        .args(["/c", "start", "", &query])
        .spawn();
}
#[cfg(not(target_os = "windows"))]
pub fn open_file_location(_path: &str) {}
#[cfg(not(target_os = "windows"))]
pub fn search_online(_name: &str) {}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn registry_record() -> StartupQuarantineRecord {
        StartupQuarantineRecord {
            version: 1,
            id: "fixture-1".into(),
            created_at: "fixture".into(),
            item_name: "Fixture".into(),
            locator: StartupLocator::Registry {
                hive: StartupRegistryHive::CurrentUser,
                value_path: RUN.into(),
                enabled_value_path: RUN.into(),
                approved_path: APPROVED_RUN.into(),
                value_name: "Fixture".into(),
            },
            payload: StartupQuarantinePayload::Registry {
                value: RawRegistryValue {
                    value_type: "REG_SZ".into(),
                    bytes: vec![65, 0, 0, 0],
                },
                approved_value: None,
            },
        }
    }
    #[test]
    fn metadata_changes_invalidate_confirmed_restore() {
        let record = registry_record();
        let encoded = serde_json::to_vec(&record).unwrap();
        let review = ReviewedStartupRestore {
            record: Arc::new(record.clone()),
            encoded: encoded.clone().into(),
        };
        verify_review(&review, &encoded).unwrap();
        let mut tampered = record;
        tampered.item_name = "replacement".into();
        assert!(verify_review(&review, &serde_json::to_vec(&tampered).unwrap()).is_err());
        tampered.locator = StartupLocator::Registry {
            hive: StartupRegistryHive::LocalMachine,
            value_path: r"Software\Sensitive".into(),
            enabled_value_path: RUN.into(),
            approved_path: APPROVED_RUN.into(),
            value_name: "Fixture".into(),
        };
        assert!(validate_record(&tampered, "fixture-1").is_err());
    }
    #[test]
    fn destinations_cannot_escape_startup_root() {
        let root = Path::new("fixture-startup");
        assert!(
            validate_folder_paths(
                &root.join("Good.lnk"),
                &root.join("_disabled/Good.lnk"),
                root,
                "Good.lnk"
            )
            .is_ok()
        );
        assert!(
            validate_folder_paths(
                &root.join("../outside.lnk"),
                &root.join("_disabled/outside.lnk"),
                root,
                "outside.lnk"
            )
            .is_err()
        );
        assert!(
            validate_folder_paths(
                &root.join("Good.lnk"),
                &root.join("elsewhere/Good.lnk"),
                root,
                "Good.lnk"
            )
            .is_err()
        );
    }
    #[test]
    fn both_disabled_representations_and_collisions_are_distinct() {
        assert_eq!(folder_enable_state(true, false).unwrap(), FolderEnable::ApprovalOnly);
        assert_eq!(folder_enable_state(false, true).unwrap(), FolderEnable::Move);
        assert!(folder_enable_state(true, true).is_err());
        assert!(folder_enable_state(false, false).is_err());
    }
    #[test]
    fn registry_collision_rejects_even_identical_existing_bytes() {
        let existing = RawRegistryValue {
            value_type: "REG_SZ".into(),
            bytes: vec![65, 0],
        };
        assert!(refuse_registry_collision(Some(&existing)).is_err());
        assert_eq!(existing.bytes, vec![65, 0]);
        assert!(refuse_registry_collision(None).is_ok());
    }
    #[test]
    fn system_and_traversing_task_destinations_are_rejected() {
        for task_path in [
            r"\Microsoft\Windows\Maintenance\",
            r"\Vendor\..\",
            r"\Vendor\\",
            "relative",
        ] {
            assert!(
                validate_locator_scope(&StartupLocator::ScheduledTask {
                    task_path: task_path.into(),
                    task_name: "Fixture".into()
                })
                .is_err()
            );
        }
    }
}
