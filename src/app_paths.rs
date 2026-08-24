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

pub(crate) fn startup_quarantine_dir() -> Option<PathBuf> {
    data_local_dir().map(|path| path.join("startup-quarantine"))
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
