//! Centralized application storage locations.
//!
//! Production uses the platform directories resolved by `directories`. Tests
//! can scope all local data to a disposable directory without mutating global
//! environment variables, which keeps parallel test runs hermetic.

use std::path::PathBuf;

#[cfg(test)]
std::thread_local! {
    static TEST_DATA_LOCAL_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn data_local_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(path) = TEST_DATA_LOCAL_DIR.with(|slot| slot.borrow().clone()) {
        return Some(path);
    }

    directories::ProjectDirs::from("com", "Xenonesis", "SystemMonitor").map(|dirs| dirs.data_local_dir().to_path_buf())
}

pub(crate) fn config_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(path) = TEST_DATA_LOCAL_DIR.with(|slot| slot.borrow().clone()) {
        return Some(path.join("config"));
    }

    directories::ProjectDirs::from("com", "Xenonesis", "SystemMonitor").map(|dirs| dirs.config_dir().to_path_buf())
}

pub(crate) fn sessions_dir() -> Option<PathBuf> {
    data_local_dir().map(|path| path.join("sessions"))
}

/// New backups never share the legacy per-user trust domain.
#[cfg(target_os = "windows")]
pub(crate) fn protected_startup_quarantine_dir() -> Result<PathBuf, String> {
    known_folder(windows::Win32::UI::Shell::CSIDL_COMMON_APPDATA).map(|root| root.join("SysMon-Protected-Startup-v1"))
}

#[cfg(target_os = "windows")]
pub(crate) fn startup_folder(common: bool) -> Result<PathBuf, String> {
    use windows::Win32::UI::Shell::{CSIDL_COMMON_STARTUP, CSIDL_STARTUP};
    known_folder(if common { CSIDL_COMMON_STARTUP } else { CSIDL_STARTUP })
}

#[cfg(target_os = "windows")]
fn known_folder(id: u32) -> Result<PathBuf, String> {
    use std::os::windows::ffi::OsStringExt;
    let mut path = [0u16; 260];
    unsafe { windows::Win32::UI::Shell::SHGetFolderPathW(None, id as i32, None, 0, &mut path) }
        .map_err(|error| format!("Could not resolve Windows known folder: {error}"))?;
    let end = path.iter().position(|ch| *ch == 0).unwrap_or(path.len());
    Ok(std::ffi::OsString::from_wide(&path[..end]).into())
}

pub(crate) fn timeline_db_path() -> Option<PathBuf> {
    data_local_dir().map(|path| path.join("history").join("timeline.sqlite3"))
}

pub(crate) fn action_log_path() -> Option<PathBuf> {
    data_local_dir().map(|path| path.join("action-audit.jsonl"))
}

#[cfg(test)]
pub(crate) fn with_test_data_local_dir<T>(path: PathBuf, operation: impl FnOnce() -> T) -> T {
    struct Restore(Option<PathBuf>);

    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_DATA_LOCAL_DIR.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }

    let previous = TEST_DATA_LOCAL_DIR.with(|slot| slot.replace(Some(path)));
    let _restore = Restore(previous);
    operation()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_override_is_scoped_and_derives_child_paths() {
        let root = std::env::temp_dir().join("sysmon-path-test");
        with_test_data_local_dir(root.clone(), || {
            assert_eq!(data_local_dir(), Some(root.clone()));
            assert_eq!(sessions_dir(), Some(root.join("sessions")));
        });
    }
}
