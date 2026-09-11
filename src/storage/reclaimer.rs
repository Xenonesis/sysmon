//! Reviewed, conservative cleanup. Paths are never used as deletion authority.
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MIN_AGE: Duration = Duration::from_secs(7 * 24 * 3600);
const MAX_ENTRIES: usize = 50_000;
const MAX_DURATION: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct ReclaimCategory {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub paths: Vec<PathBuf>,
    pub size_bytes: u64,
    pub file_count: usize,
    pub skipped: usize,
    pub failed: usize,
    pub issues: Vec<CleanupIssue>,
    pub complete: bool,
    files: Vec<ReviewedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupIssue {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct ReviewedCleanup {
    files: Vec<ReviewedFile>,
    category_ids: Vec<String>,
}
impl ReviewedCleanup {
    pub fn from_categories(categories: &[ReclaimCategory], selected: &HashSet<String>) -> Self {
        let mut result = Self::default();
        let mut seen = HashSet::new();
        for category in categories.iter().filter(|c| selected.contains(c.id)) {
            if !category.files.is_empty() {
                result.category_ids.push(category.id.to_string());
            }
            for file in &category.files {
                if seen.insert(file.snapshot.identity) {
                    result.files.push(file.clone());
                }
            }
        }
        result
    }
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
    pub fn size_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.snapshot.size).sum()
    }
    pub fn category_ids(&self) -> &[String] {
        &self.category_ids
    }
}

#[derive(Debug, Clone, Default)]
pub struct CleanupResult {
    pub removed: usize,
    pub removed_bytes: u64,
    pub skipped: usize,
    pub failed: usize,
    pub cancelled: bool,
    pub issues: Vec<CleanupIssue>,
}
impl CleanupResult {
    pub fn summary(&self) -> String {
        format!(
            "{}Removed {} files ({} bytes); skipped {}; failed {}.",
            if self.cancelled { "Cancelled. " } else { "" },
            self.removed,
            self.removed_bytes,
            self.skipped,
            self.failed
        )
    }
}

#[derive(Debug, Clone)]
struct ReviewedFile {
    category: &'static str,
    path: PathBuf,
    ancestors: Vec<FileIdentity>,
    snapshot: Snapshot,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileIdentity {
    volume: u32,
    index: u64,
    created: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Snapshot {
    identity: FileIdentity,
    size: u64,
    modified: u64,
    changed: i64,
    attributes: u32,
    links: u32,
}

pub fn resolve_category_paths(id: &str) -> Option<Vec<PathBuf>> {
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let windows = std::env::var_os("SystemRoot").map(PathBuf::from);
    Some(match id {
        "shader_cache" => local
            .map(|p| vec![p.join("D3DSCache"), p.join("NVIDIA/DXCache"), p.join("AMD/DxCache")])
            .unwrap_or_default(),
        "crash_dumps" => windows
            .map(|p| p.join("Minidump"))
            .into_iter()
            .chain(local.map(|p| p.join("CrashDumps")))
            .collect(),
        "user_temp" => vec![std::env::temp_dir()],
        // No authoritative Windows Update lifecycle eligibility provider is available.
        "windows_update" => Vec::new(),
        _ => return None,
    })
}

fn filetime_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_add(11_644_473_600)
        .saturating_mul(10_000_000)
}
fn eligibility(category: &str, path: &Path, value: Snapshot, now: u64) -> Result<(), &'static str> {
    if value.attributes & (0x1 | 0x4 | 0x10 | 0x400) != 0 {
        return Err("Read-only, system, directory or reparse entry preserved");
    }
    if value.links != 1 {
        return Err("Hard-linked file preserved");
    }
    let cutoff = now.saturating_sub(MIN_AGE.as_secs() * 10_000_000);
    if value.modified > cutoff || value.identity.created > cutoff {
        return Err("File is newer than seven days");
    }
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match category {
        "crash_dumps" if extension == "dmp" => Ok(()),
        "shader_cache" => Ok(()),
        "user_temp"
            if !matches!(
                extension.as_str(),
                "exe" | "msi" | "msp" | "msix" | "cab" | "lock" | "lck"
            ) =>
        {
            Ok(())
        }
        _ => Err("File type or lifecycle is not eligible for cleanup"),
    }
}
fn issue(issues: &mut Vec<CleanupIssue>, path: &Path, reason: impl ToString) {
    // Counts remain exact even when the bounded diagnostic sample fills up.
    if issues.len() < 100 {
        issues.push(CleanupIssue {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        });
    }
}

pub fn scan_reclaimable_caches(cancel: &AtomicBool) -> Vec<ReclaimCategory> {
    [
        ("shader_cache", "DirectX & GPU Shader Caches", "Reviewed ordinary shader files older than seven days; caches may need rebuilding."),
        ("crash_dumps", "Windows & Application Crash Dumps", "Reviewed .dmp files older than seven days. Deletion permanently removes diagnostic evidence."),
        ("user_temp", "User Temporary Files (%TEMP%)", "Reviewed ordinary files older than seven days; installers, locks, read-only and linked files are excluded. Age does not prove an application no longer needs a file."),
        ("windows_update", "Windows Update Download Cache", "Unavailable: authoritative package lifecycle eligibility is not supported. Use Windows Storage settings."),
    ].into_iter().map(|(id, label, description)| {
        scan_category(id, label, description, resolve_category_paths(id).unwrap_or_default(), cancel)
    }).collect()
}

fn scan_category(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    paths: Vec<PathBuf>,
    cancel: &AtomicBool,
) -> ReclaimCategory {
    let mut result = ReclaimCategory {
        id,
        label,
        description,
        paths,
        size_bytes: 0,
        file_count: 0,
        skipped: 0,
        failed: 0,
        issues: Vec::new(),
        complete: true,
        files: Vec::new(),
    };
    let started = Instant::now();
    let mut visited = 0;
    let now = filetime_now();
    let mut pending = result.paths.clone();
    while let Some(path) = pending.pop() {
        if cancel.load(Ordering::Relaxed) || visited >= MAX_ENTRIES || started.elapsed() >= MAX_DURATION {
            result.complete = false;
            issue(
                &mut result.issues,
                &path,
                "Scan cancelled or entry/time budget reached; unscanned contents excluded",
            );
            break;
        }
        visited += 1;
        let anchored = match AnchoredPath::open(&path, false) {
            Ok(value) => value,
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    result.failed += 1;
                    issue(&mut result.issues, &path, error);
                }
                continue;
            }
        };
        if anchored.snapshot.attributes & 0x10 != 0 {
            match std::fs::read_dir(&path) {
                Ok(entries) => {
                    for entry in entries {
                        if pending.len() + visited >= MAX_ENTRIES
                            || cancel.load(Ordering::Relaxed)
                            || started.elapsed() >= MAX_DURATION
                        {
                            result.complete = false;
                            issue(
                                &mut result.issues,
                                &path,
                                "Directory enumeration budget reached or cancelled",
                            );
                            break;
                        }
                        match entry {
                            Ok(entry) => pending.push(entry.path()),
                            Err(error) => {
                                result.failed += 1;
                                issue(&mut result.issues, &path, error);
                            }
                        }
                    }
                }
                Err(error) => {
                    result.failed += 1;
                    issue(&mut result.issues, &path, error);
                }
            }
        } else {
            match eligibility(id, &path, anchored.snapshot, now) {
                Ok(()) => {
                    result.size_bytes = result.size_bytes.saturating_add(anchored.snapshot.size);
                    result.file_count += 1;
                    result.files.push(ReviewedFile {
                        category: id,
                        path,
                        ancestors: anchored.identities(),
                        snapshot: anchored.snapshot,
                    });
                }
                Err(reason) => {
                    result.skipped += 1;
                    issue(&mut result.issues, &path, reason);
                }
            }
        }
    }
    result
}

pub fn clean_reviewed(reviewed: &ReviewedCleanup, cancel: &AtomicBool) -> CleanupResult {
    let mut result = CleanupResult::default();
    for (index, file) in reviewed.files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            result.cancelled = true;
            result.skipped += reviewed.files.len() - index;
            break;
        }
        let anchored = match AnchoredPath::open(&file.path, true) {
            Ok(value) => value,
            Err(error) => {
                result.failed += 1;
                issue(&mut result.issues, &file.path, error);
                continue;
            }
        };
        if anchored.identities() != file.ancestors || anchored.snapshot != file.snapshot {
            result.skipped += 1;
            issue(
                &mut result.issues,
                &file.path,
                "File or ancestor identity/metadata changed after review",
            );
            continue;
        }
        if let Err(reason) = eligibility(file.category, &file.path, anchored.snapshot, filetime_now()) {
            result.skipped += 1;
            issue(&mut result.issues, &file.path, reason);
            continue;
        }
        if cancel.load(Ordering::Relaxed) {
            result.cancelled = true;
            result.skipped += reviewed.files.len() - index;
            break;
        }
        match anchored.delete() {
            Ok(()) => {
                result.removed += 1;
                result.removed_bytes = result.removed_bytes.saturating_add(file.snapshot.size);
            }
            Err(error) => {
                result.failed += 1;
                issue(&mut result.issues, &file.path, error);
            }
        }
    }
    if cancel.load(Ordering::Relaxed) {
        result.cancelled = true;
    }
    result
}

/// Every ancestor is held without FILE_SHARE_WRITE/DELETE before the next path
/// component is opened. This prevents rename, replacement and reparse mutation.
/// Final deletion uses the exact validated handle, never remove_file(path).
struct AnchoredPath {
    handles: Vec<std::fs::File>,
    snapshots: Vec<Snapshot>,
    snapshot: Snapshot,
}
impl AnchoredPath {
    fn identities(&self) -> Vec<FileIdentity> {
        self.snapshots[..self.snapshots.len() - 1]
            .iter()
            .map(|s| s.identity)
            .collect()
    }
    #[cfg(windows)]
    fn open(path: &Path, delete: bool) -> std::io::Result<Self> {
        use std::os::windows::fs::OpenOptionsExt;
        use std::path::{Component, Prefix};
        let mut components = path.components();
        match components.next() {
            Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) => (),
            _ => return Err(std::io::Error::other("Only absolute local-drive paths are supported")),
        }
        if !matches!(components.next(), Some(Component::RootDir))
            || !components.clone().all(|c| matches!(c, Component::Normal(_)))
        {
            return Err(std::io::Error::other("Ambiguous or relative path rejected"));
        }
        let chain: Vec<_> = path.ancestors().filter(|p| p.has_root()).collect();
        let mut handles = Vec::with_capacity(chain.len());
        let mut snapshots = Vec::with_capacity(chain.len());
        for part in chain.into_iter().rev() {
            let final_part = part == path;
            let mut options = std::fs::OpenOptions::new();
            // FILE_READ_ATTRIBUTES, SYNCHRONIZE; DELETE only on approved target.
            options
                .access_mode(0x80 | 0x100000 | if final_part && delete { 0x10000 } else { 0 })
                .share_mode(if final_part && delete { 0 } else { 1 })
                .custom_flags(0x02000000 | 0x00200000);
            let handle = options.open(part)?;
            let snapshot = native::snapshot(&handle)?;
            if snapshot.attributes & 0x400 != 0 || (!final_part && snapshot.attributes & 0x10 == 0) {
                return Err(std::io::Error::other(
                    "Reparse point or non-directory ancestor rejected",
                ));
            }
            snapshots.push(snapshot);
            handles.push(handle);
        }
        let snapshot = *snapshots.last().ok_or_else(|| std::io::Error::other("Empty path"))?;
        Ok(Self {
            handles,
            snapshots,
            snapshot,
        })
    }
    #[cfg(not(windows))]
    fn open(_: &Path, _: bool) -> std::io::Result<Self> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Handle-anchored cleanup is only supported on Windows",
        ))
    }
    #[cfg(windows)]
    fn delete(&self) -> std::io::Result<()> {
        native::delete(self.handles.last().expect("anchored target"))
    }
    #[cfg(not(windows))]
    fn delete(&self) -> std::io::Result<()> {
        Err(std::io::Error::other("Cleanup unsupported"))
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    #[repr(C)]
    #[derive(Default)]
    struct FileInfo {
        attributes: u32,
        created: [u32; 2],
        accessed: [u32; 2],
        modified: [u32; 2],
        volume: u32,
        size_high: u32,
        size_low: u32,
        links: u32,
        index_high: u32,
        index_low: u32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct BasicInfo {
        created: i64,
        accessed: i64,
        modified: i64,
        changed: i64,
        attributes: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFileInformationByHandle(handle: *mut c_void, info: *mut FileInfo) -> i32;
        fn GetFileInformationByHandleEx(handle: *mut c_void, class: i32, info: *mut c_void, size: u32) -> i32;
        fn SetFileInformationByHandle(handle: *mut c_void, class: i32, info: *const c_void, size: u32) -> i32;
    }
    fn pair(low: u32, high: u32) -> u64 {
        u64::from(low) | (u64::from(high) << 32)
    }
    pub(super) fn snapshot(file: &std::fs::File) -> std::io::Result<Snapshot> {
        let mut info = FileInfo::default();
        let mut basic = BasicInfo::default();
        // Both information calls refer to the same pinned object.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0
            || unsafe {
                GetFileInformationByHandleEx(
                    file.as_raw_handle(),
                    0,
                    (&mut basic as *mut BasicInfo).cast(),
                    std::mem::size_of::<BasicInfo>() as u32,
                )
            } == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Snapshot {
            identity: FileIdentity {
                volume: info.volume,
                index: pair(info.index_low, info.index_high),
                created: pair(info.created[0], info.created[1]),
            },
            size: pair(info.size_low, info.size_high),
            modified: pair(info.modified[0], info.modified[1]),
            changed: basic.changed,
            attributes: info.attributes,
            links: info.links,
        })
    }
    pub(super) fn delete(file: &std::fs::File) -> std::io::Result<()> {
        // FILE_DISPOSITION_INFO contains a Win32 BOOLEAN, not BOOL.
        let delete: u8 = 1;
        if unsafe { SetFileInformationByHandle(file.as_raw_handle(), 4, (&delete as *const u8).cast(), 1) } == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::ffi::c_void;
    use std::os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle};
    use std::sync::atomic::AtomicU64;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "sysmon-cleanup-{}-{}-{}",
                std::process::id(),
                filetime_now(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).expect("create isolated fixture");
            Self(path)
        }
        fn dir(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            std::fs::create_dir_all(&path).unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetFileTime(handle: *mut c_void, created: *const u64, accessed: *const u64, written: *const u64) -> i32;
        fn DeviceIoControl(
            handle: *mut c_void,
            code: u32,
            input: *const c_void,
            input_size: u32,
            output: *mut c_void,
            output_size: u32,
            returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
    }
    fn aged_file(path: &Path) {
        std::fs::write(path, b"eligible fixture").unwrap();
        let file = std::fs::OpenOptions::new().access_mode(0x100).open(path).unwrap();
        let time = filetime_now() - (MIN_AGE.as_secs() + 3600) * 10_000_000;
        assert_ne!(
            unsafe { SetFileTime(file.as_raw_handle(), &time, &time, &time) },
            0,
            "{}",
            std::io::Error::last_os_error()
        );
    }
    fn junction(link: &Path, target: &Path) {
        std::fs::create_dir(link).unwrap();
        let file = std::fs::OpenOptions::new()
            .access_mode(0x40000000)
            .share_mode(0)
            .custom_flags(0x02000000 | 0x00200000)
            .open(link)
            .unwrap();
        let target: Vec<u16> = std::ffi::OsString::from(format!("\\??\\{}", target.display()))
            .encode_wide()
            .collect();
        let name_bytes = target.len() * 2;
        let mut data = Vec::new();
        data.extend_from_slice(&0xA0000003u32.to_le_bytes());
        data.extend_from_slice(&((8 + name_bytes + 4) as u16).to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&(name_bytes as u16).to_le_bytes());
        data.extend_from_slice(&((name_bytes + 2) as u16).to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        for code in target {
            data.extend_from_slice(&code.to_le_bytes());
        }
        data.extend_from_slice(&[0; 4]);
        let mut returned = 0;
        assert_ne!(
            unsafe {
                DeviceIoControl(
                    file.as_raw_handle(),
                    0x900A4,
                    data.as_ptr().cast(),
                    data.len() as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            },
            0,
            "junction fixture requires NTFS: {}",
            std::io::Error::last_os_error()
        );
    }
    fn review(root: PathBuf) -> (ReclaimCategory, ReviewedCleanup) {
        let category = scan_category(
            "shader_cache",
            "fixture",
            "fixture",
            vec![root],
            &AtomicBool::new(false),
        );
        let reviewed =
            ReviewedCleanup::from_categories(std::slice::from_ref(&category), &HashSet::from(["shader_cache".into()]));
        (category, reviewed)
    }

    #[test]
    fn reviewed_cleanup_preserves_new_changed_recent_and_locked_files() {
        let fixture = Fixture::new();
        let root = fixture.dir("cache");
        for name in ["ordinary", "changed", "replaced", "locked"] {
            aged_file(&root.join(name));
        }
        std::fs::write(root.join("recent"), b"recent").unwrap();
        let (scan, reviewed) = review(root.clone());
        assert_eq!(scan.file_count, 4);
        assert_eq!(scan.skipped, 1);
        aged_file(&root.join("unapproved"));
        std::fs::write(root.join("changed"), b"different contents").unwrap();
        std::fs::rename(root.join("replaced"), root.join("old-replaced")).unwrap();
        aged_file(&root.join("replaced"));
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(root.join("locked"))
            .unwrap();
        let result = clean_reviewed(&reviewed, &AtomicBool::new(false));
        assert_eq!(result.removed, 1);
        assert_eq!(result.skipped, 2);
        assert_eq!(result.failed, 1);
        assert!(!root.join("ordinary").exists());
        for name in ["changed", "replaced", "locked", "recent", "unapproved"] {
            assert!(root.join(name).exists(), "{name} must survive");
        }
        drop(locked);
    }

    #[test]
    #[allow(clippy::permissions_set_readonly_false)] // fixture teardown restores writability on Windows
    fn cancelled_review_never_mutates_and_readonly_is_preserved() {
        let fixture = Fixture::new();
        let root = fixture.dir("cache");
        let path = root.join("keep");
        aged_file(&path);
        let (_, reviewed) = review(root.clone());
        let result = clean_reviewed(&reviewed, &AtomicBool::new(true));
        assert!(result.cancelled);
        assert_eq!(result.removed, 0);
        assert!(path.exists());
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions).unwrap();
        let result = clean_reviewed(&reviewed, &AtomicBool::new(false));
        assert_eq!(result.removed, 0);
        assert!(std::fs::metadata(&path).unwrap().permissions().readonly());
        // Fixture teardown only, never production cleanup.
        #[allow(clippy::permissions_set_readonly_false)]
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(false);
        std::fs::set_permissions(&path, permissions).unwrap();
    }

    #[test]
    fn junction_roots_children_and_ancestors_preserve_outside_sentinels() {
        let fixture = Fixture::new();
        let outside = fixture.dir("outside");
        aged_file(&outside.join("sentinel"));
        let root_link = fixture.0.join("root-link");
        junction(&root_link, &outside);
        let (scan, reviewed) = review(root_link);
        assert_eq!(scan.file_count, 0);
        assert!(scan.failed > 0);
        assert_eq!(clean_reviewed(&reviewed, &AtomicBool::new(false)).removed, 0);
        let root = fixture.dir("cache");
        aged_file(&root.join("ordinary"));
        junction(&root.join("child"), &outside);
        let (_, reviewed) = review(root);
        assert_eq!(clean_reviewed(&reviewed, &AtomicBool::new(false)).removed, 1);
        let inner = outside.join("inner");
        std::fs::create_dir(&inner).unwrap();
        aged_file(&inner.join("sentinel"));
        let ancestor = fixture.0.join("ancestor");
        junction(&ancestor, &outside);
        let (scan, reviewed) = review(ancestor.join("inner"));
        assert!(scan.failed > 0);
        assert_eq!(clean_reviewed(&reviewed, &AtomicBool::new(false)).removed, 0);
        assert!(outside.join("sentinel").exists());
        assert!(inner.join("sentinel").exists());
    }

    #[test]
    fn replacement_between_review_and_delete_is_rejected_and_open_ancestors_cannot_move() {
        let fixture = Fixture::new();
        let root = fixture.dir("cache");
        let child = root.join("child");
        std::fs::create_dir(&child).unwrap();
        aged_file(&child.join("approved"));
        let (_, reviewed) = review(root.clone());
        let pinned = AnchoredPath::open(&child.join("approved"), true).unwrap();
        assert!(std::fs::rename(&root, fixture.0.join("moved-root")).is_err());
        assert!(std::fs::rename(&child, root.join("moved-child")).is_err());
        assert!(std::fs::rename(child.join("approved"), child.join("moved-file")).is_err());
        drop(pinned);
        let outside = fixture.dir("outside");
        aged_file(&outside.join("approved"));
        std::fs::rename(&child, root.join("old-child")).unwrap();
        junction(&child, &outside);
        let result = clean_reviewed(&reviewed, &AtomicBool::new(false));
        assert_eq!(result.removed, 0);
        assert_eq!(result.failed, 1);
        assert!(outside.join("approved").exists());
        assert!(root.join("old-child/approved").exists());
    }
}
