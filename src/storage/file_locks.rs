//! Bounded file/folder Restart Manager coverage and native volume-handle users.
//! Neither provider certifies that deletion or device ejection is safe.
use crate::processes::ProcessIdentity;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockingProcess {
    pub identity: Option<ProcessIdentity>,
    pub pid: u32,
    pub name: String,
    pub app_type: String,
    pub is_service: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InspectionKind {
    File,
    Directory,
    Volume,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileLockResult {
    pub path: String,
    pub kind: InspectionKind,
    pub processes: Vec<LockingProcess>,
    pub error: Option<String>,
    pub files_scanned: usize,
    pub entries_skipped: usize,
    pub partial: bool,
    pub cancelled: bool,
    pub coverage: Vec<String>,
}
impl FileLockResult {
    fn new(path: &str) -> Self {
        Self {
            path: path.into(),
            kind: InspectionKind::Unknown,
            processes: Vec::new(),
            error: None,
            files_scanned: 0,
            entries_skipped: 0,
            partial: false,
            cancelled: false,
            coverage: vec![
                "No result guarantees safe deletion or device ejection; users may change immediately after inspection."
                    .into(),
            ],
        }
    }
    fn gap(&mut self, message: String) {
        self.partial = true;
        self.entries_skipped += 1;
        if self.coverage.len() < 100 {
            self.coverage.push(message);
        }
    }
}

pub fn find_locking_processes(path: &str, cancel: &AtomicBool) -> FileLockResult {
    let mut result = FileLockResult::new(path);
    if cancel.load(Ordering::Relaxed) {
        result.cancelled = true;
        result.partial = true;
        return result;
    }
    #[cfg(not(windows))]
    {
        result.error = Some("Lock inspection is only supported on Windows".into());
        result.partial = true;
        result
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        let target = Path::new(path);
        let metadata = match std::fs::symlink_metadata(target) {
            Ok(metadata) => metadata,
            Err(error) => {
                result.error = Some(error.to_string());
                result.partial = true;
                return result;
            }
        };
        if metadata.file_attributes() & 0x400 != 0 {
            result.error = Some("Reparse targets are not inspected".into());
            result.partial = true;
            return result;
        }
        result.kind = if metadata.is_dir() {
            if target.parent().is_none() {
                InspectionKind::Volume
            } else {
                InspectionKind::Directory
            }
        } else {
            InspectionKind::File
        };
        if result.kind == InspectionKind::Volume {
            match volume_users(target) {
                Ok(processes) => result.processes = processes,
                Err(error) => result.gap(format!("Native volume-handle user query unavailable: {error}")),
            }
            result.gap("Volume file enumeration is bounded, and device/driver/ejection veto coverage is unavailable; this is not a safe-eject check.".into());
        }
        let started = Instant::now();
        let mut pending = vec![target.to_path_buf()];
        let mut visited = 0;
        let mut files = Vec::new();
        while let Some(next) = pending.pop() {
            if cancel.load(Ordering::Relaxed) || visited >= 4096 || started.elapsed() >= Duration::from_secs(15) {
                result.cancelled = cancel.load(Ordering::Relaxed);
                result.gap(
                    "Inspection cancelled or 4096-entry/15-second budget reached; remaining contents unscanned.".into(),
                );
                break;
            }
            visited += 1;
            // Hold every ancestor against replacement while enumerating this path.
            let _guards = match pin_inspection_path(&next) {
                Ok(guards) => guards,
                Err(error) => {
                    result.gap(format!("{}: {error}", next.display()));
                    continue;
                }
            };
            let metadata = match std::fs::symlink_metadata(&next) {
                Ok(value) => value,
                Err(error) => {
                    result.gap(format!("{}: {error}", next.display()));
                    continue;
                }
            };
            if metadata.is_dir() {
                match std::fs::read_dir(&next) {
                    Ok(entries) => {
                        for entry in entries {
                            if pending.len() + visited >= 4096
                                || cancel.load(Ordering::Relaxed)
                                || started.elapsed() >= Duration::from_secs(15)
                            {
                                result.gap(format!("{}: enumeration budget reached or cancelled", next.display()));
                                break;
                            }
                            match entry {
                                Ok(entry) => pending.push(entry.path()),
                                Err(error) => result.gap(format!("{}: {error}", next.display())),
                            }
                        }
                    }
                    Err(error) => result.gap(format!("{}: {error}", next.display())),
                }
            } else if metadata.is_file() {
                files.push(next);
            }
        }
        // Small independent batches avoid a failed resource hiding every other file.
        for batch in files.chunks(32) {
            if cancel.load(Ordering::Relaxed) || started.elapsed() >= Duration::from_secs(15) {
                result.cancelled = cancel.load(Ordering::Relaxed);
                result.gap("Restart Manager querying stopped; remaining files unscanned.".into());
                break;
            }
            match restart_manager_users(batch, cancel) {
                Ok(processes) => {
                    result.files_scanned += batch.len();
                    result.processes.extend(processes);
                }
                Err(error) => {
                    result.entries_skipped += batch.len().saturating_sub(1);
                    result.gap(error);
                }
            }
        }
        result
            .processes
            .sort_by_key(|p| (p.pid, p.identity.map(|i| i.creation_time)));
        result
            .processes
            .dedup_by_key(|p| (p.pid, p.identity.map(|i| i.creation_time)));
        result.coverage.push(format!("Restart Manager queried {} ordinary files. Open directory handles, kernel drivers and unregistered users may not be represented. Native calls cannot be interrupted mid-call.", result.files_scanned));
        result.cancelled |= cancel.load(Ordering::Relaxed);
        result.partial |= result.cancelled;
        result
    }
}

#[cfg(windows)]
fn pin_inspection_path(path: &Path) -> std::io::Result<Vec<std::fs::File>> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    if !path.is_absolute() || path.components().any(|p| matches!(p, std::path::Component::ParentDir)) {
        return Err(std::io::Error::other(
            "An absolute path without parent traversal is required",
        ));
    }
    let mut held = Vec::new();
    // The target file may be exclusively open (the very condition being inspected).
    // Pin only directories; RM observes the named file at query time, not a deletion authority.
    let parts: Vec<_> = path.ancestors().filter(|p| p.has_root()).collect();
    for part in parts.into_iter().rev() {
        let metadata = std::fs::symlink_metadata(part)?;
        if part == path && metadata.is_file() {
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(std::io::Error::other("Reparse file excluded"));
            }
            break;
        }
        let file = std::fs::OpenOptions::new()
            .access_mode(0x80)
            .share_mode(1)
            .custom_flags(0x02000000 | 0x00200000)
            .open(part)?;
        if file.metadata()?.file_attributes() & 0x400 != 0 {
            return Err(std::io::Error::other("Reparse directory/ancestor excluded"));
        }
        held.push(file);
    }
    Ok(held)
}

#[cfg(windows)]
fn restart_manager_users(paths: &[PathBuf], cancel: &AtomicBool) -> Result<Vec<LockingProcess>, String> {
    use std::os::windows::ffi::OsStrExt;
    let mut guards = Vec::new();
    for path in paths {
        guards.extend(pin_inspection_path(path).map_err(|e| format!("{}: {e}", path.display()))?);
    }
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct UniqueProcess {
        pid: u32,
        started: [u32; 2],
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ProcessInfo {
        process: UniqueProcess,
        app: [u16; 256],
        service: [u16; 64],
        kind: u32,
        status: u32,
        session: u32,
        restartable: i32,
    }
    #[link(name = "rstrtmgr")]
    unsafe extern "system" {
        fn RmStartSession(session: *mut u32, flags: u32, key: *mut u16) -> u32;
        fn RmRegisterResources(
            session: u32,
            files: u32,
            names: *const *const u16,
            apps: u32,
            processes: *const UniqueProcess,
            services: u32,
            service_names: *const *const u16,
        ) -> u32;
        fn RmGetList(
            session: u32,
            needed: *mut u32,
            count: *mut u32,
            processes: *mut ProcessInfo,
            reasons: *mut u32,
        ) -> u32;
        fn RmEndSession(session: u32) -> u32;
    }
    struct Session(u32);
    impl Drop for Session {
        fn drop(&mut self) {
            unsafe {
                RmEndSession(self.0);
            }
        }
    }
    let mut handle = 0;
    let mut key = [0; 33];
    let status = unsafe { RmStartSession(&mut handle, 0, key.as_mut_ptr()) };
    if status != 0 {
        return Err(format!("RmStartSession: {status}"));
    }
    let session = Session(handle);
    let names: Vec<Vec<u16>> = paths
        .iter()
        .map(|p| p.as_os_str().encode_wide().chain(Some(0)).collect())
        .collect();
    let pointers: Vec<_> = names.iter().map(|s| s.as_ptr()).collect();
    let status = unsafe {
        RmRegisterResources(
            session.0,
            pointers.len() as u32,
            pointers.as_ptr(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
        )
    };
    if status != 0 {
        return Err(format!("RmRegisterResources: {status}"));
    }
    let mut needed = 0;
    let mut reasons = 0;
    let mut infos: Vec<ProcessInfo> = Vec::new();
    for _ in 0..4 {
        if cancel.load(Ordering::Relaxed) {
            return Err("Inspection cancelled".into());
        }
        let mut count = infos.len() as u32;
        let status = unsafe {
            RmGetList(
                session.0,
                &mut needed,
                &mut count,
                if infos.is_empty() {
                    std::ptr::null_mut()
                } else {
                    infos.as_mut_ptr()
                },
                &mut reasons,
            )
        };
        if status == 0 {
            infos.truncate(count as usize);
            return Ok(infos
                .iter()
                .map(|info| {
                    let decode =
                        |s: &[u16]| String::from_utf16_lossy(&s[..s.iter().position(|c| *c == 0).unwrap_or(s.len())]);
                    let mut name = decode(&info.app);
                    if name.is_empty() {
                        name = decode(&info.service);
                    }
                    if name.is_empty() {
                        name = format!("PID {}", info.process.pid);
                    }
                    let creation_time = u64::from(info.process.started[0]) | (u64::from(info.process.started[1]) << 32);
                    LockingProcess {
                        pid: info.process.pid,
                        identity: (creation_time != 0).then_some(ProcessIdentity {
                            pid: info.process.pid,
                            creation_time,
                        }),
                        name,
                        app_type: match info.kind {
                            3 => "Windows Service",
                            1000 => "Critical System Service",
                            4 => "Windows Explorer",
                            _ => "Application",
                        }
                        .into(),
                        is_service: matches!(info.kind, 3 | 1000),
                    }
                })
                .collect());
        }
        if status != 234 || needed > 16_384 {
            return Err(format!("RmGetList: {status}; requested rows {needed}"));
        }
        infos.resize_with(needed as usize, || unsafe { std::mem::zeroed() });
    }
    Err("Restart Manager users changed repeatedly; coverage incomplete".into())
}

#[cfg(windows)]
fn volume_users(path: &Path) -> Result<Vec<LockingProcess>, String> {
    use std::ffi::c_void;
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    let name = path.to_string_lossy();
    if name.len() != 3 || !name.as_bytes()[0].is_ascii_alphabetic() || &name[1..] != ":\\" {
        return Err("Only drive-letter volume roots support native volume queries".into());
    }
    let file = std::fs::OpenOptions::new()
        .access_mode(0x80)
        .share_mode(7)
        .open(format!("\\\\.\\{}", &name[..2]))
        .map_err(|e| e.to_string())?;
    #[repr(C)]
    struct IoStatus {
        status: usize,
        information: usize,
    }
    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQueryInformationFile(
            file: *mut c_void,
            status: *mut IoStatus,
            data: *mut c_void,
            length: u32,
            class: u32,
        ) -> i32;
    }
    // FILE_PROCESS_IDS_USING_FILE_INFORMATION: count followed by pointer-sized PIDs.
    let mut data = vec![0usize; 4097];
    let mut io = IoStatus {
        status: 0,
        information: 0,
    };
    let status = unsafe {
        NtQueryInformationFile(
            file.as_raw_handle(),
            &mut io,
            data.as_mut_ptr().cast(),
            (data.len() * std::mem::size_of::<usize>()) as u32,
            47,
        )
    };
    if status < 0 {
        return Err(format!(
            "FileProcessIdsUsingFileInformation NTSTATUS 0x{:08X}",
            status as u32
        ));
    }
    let count = data[0];
    if count > 4096 || io.information < (count + 1) * std::mem::size_of::<usize>() {
        return Err("Native volume process list exceeded capacity or was truncated".into());
    }
    Ok(data[1..1 + count]
        .iter()
        .filter_map(|pid| u32::try_from(*pid).ok())
        .map(|pid| LockingProcess {
            // This provider returns PIDs only, so cannot bind a later mutation to the
            // original owner. RM rows supply creation FILETIME separately.
            pid,
            identity: None,
            name: format!("PID {pid}"),
            app_type: "Native volume handle user (identity unavailable; not an ejection veto)".into(),
            is_service: false,
        })
        .collect())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;
    use std::sync::atomic::AtomicU64;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "sysmon-locks-{}-{stamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn parent_folder_reports_owned_open_file_with_creation_identity() {
        let fixture = Fixture::new();
        let nested = fixture.0.join("nested");
        std::fs::create_dir(&nested).unwrap();
        let file = std::fs::File::create(nested.join("held")).unwrap();
        let expected = crate::processes::process_identity(std::process::id()).unwrap();
        let result = find_locking_processes(fixture.0.to_str().unwrap(), &AtomicBool::new(false));
        assert_eq!(result.kind, InspectionKind::Directory);
        assert_eq!(result.files_scanned, 1, "{result:?}");
        assert!(
            result.processes.iter().any(|p| p.identity == Some(expected)),
            "{result:?}"
        );
        drop(file);
    }
    #[test]
    fn denied_subtree_is_partial_and_does_not_hide_accessible_file() {
        let fixture = Fixture::new();
        let inaccessible = fixture.0.join("busy-directory");
        std::fs::create_dir(&inaccessible).unwrap();
        std::fs::write(inaccessible.join("unscanned"), b"fixture").unwrap();
        let held_directory = std::fs::OpenOptions::new()
            .access_mode(0x40000000)
            .share_mode(0)
            .custom_flags(0x02000000)
            .open(&inaccessible)
            .unwrap();
        let held_file = std::fs::File::create(fixture.0.join("visible")).unwrap();
        let result = find_locking_processes(fixture.0.to_str().unwrap(), &AtomicBool::new(false));
        assert!(result.partial, "{result:?}");
        assert!(result.entries_skipped > 0);
        assert_eq!(result.files_scanned, 1);
        assert!(
            result.processes.iter().any(|p| p.pid == std::process::id()),
            "{result:?}"
        );
        drop((held_file, held_directory));
    }
    #[test]
    fn cancelled_inspection_does_not_enumerate_fixture() {
        let fixture = Fixture::new();
        std::fs::write(fixture.0.join("unscanned"), b"fixture").unwrap();
        let result = find_locking_processes(fixture.0.to_str().unwrap(), &AtomicBool::new(true));
        assert!(result.cancelled && result.partial);
        assert_eq!(result.files_scanned, 0);
        assert!(result.processes.is_empty());
    }
}
