//! Windows Restart Manager file and drive lock detection.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockingProcess {
    pub pid: u32,
    pub name: String,
    pub app_type: String,
    pub is_service: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileLockResult {
    pub path: String,
    pub processes: Vec<LockingProcess>,
    pub error: Option<String>,
}

#[cfg(not(target_os = "windows"))]
pub fn find_locking_processes(path: &str) -> FileLockResult {
    FileLockResult {
        path: path.to_string(),
        processes: Vec::new(),
        error: Some("File lock inspection is only supported on Windows".into()),
    }
}

#[cfg(target_os = "windows")]
fn decode_utf16_null_terminated(slice: &[u16]) -> String {
    let len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf16_lossy(&slice[..len])
}

#[cfg(target_os = "windows")]
fn map_rm_app_type(app_type: u32) -> (&'static str, bool) {
    match app_type {
        0 => ("Unknown Application", false),
        1 => ("Desktop App", false),
        2 => ("Background Window", false),
        3 => ("Windows Service", true),
        4 => ("Windows Explorer", false),
        5 => ("Console App", false),
        1000 => ("Critical System Service", true),
        _ => ("Application", false),
    }
}

#[cfg(target_os = "windows")]
pub fn find_locking_processes(path: &str) -> FileLockResult {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::ERROR_MORE_DATA;

    const CCH_RM_SESSION_KEY: usize = 32;
    const CCH_RM_MAX_APP_NAME: usize = 255;
    const CCH_RM_MAX_SVC_NAME: usize = 63;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RM_UNIQUE_PROCESS {
        dw_process_id: u32,
        process_start_time: windows_sys::Win32::Foundation::FILETIME,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct RM_PROCESS_INFO {
        process: RM_UNIQUE_PROCESS,
        str_app_name: [u16; CCH_RM_MAX_APP_NAME + 1],
        str_service_short_name: [u16; CCH_RM_MAX_SVC_NAME + 1],
        application_type: u32,
        app_status: u32,
        tss_session_id: u32,
        b_restartable: i32,
    }

    impl Default for RM_PROCESS_INFO {
        fn default() -> Self {
            unsafe { std::mem::zeroed() }
        }
    }

    #[link(name = "rstrtmgr")]
    unsafe extern "system" {
        fn RmStartSession(pSessionHandle: *mut u32, dwSessionFlags: u32, strSessionKey: *mut u16) -> u32;
        fn RmRegisterResources(
            dwSessionHandle: u32,
            nFiles: u32,
            rgsFilenames: *const *const u16,
            nApplications: u32,
            rgApplications: *const RM_UNIQUE_PROCESS,
            nServices: u32,
            rgsServiceNames: *const *const u16,
        ) -> u32;
        fn RmGetList(
            dwSessionHandle: u32,
            pnProcInfoNeeded: *mut u32,
            pnProcInfo: *mut u32,
            rgAffectedApps: *mut RM_PROCESS_INFO,
            lpdwRebootReasons: *mut u32,
        ) -> u32;
        fn RmEndSession(dwSessionHandle: u32) -> u32;
    }

    struct RmSession(u32);

    impl Drop for RmSession {
        fn drop(&mut self) {
            unsafe {
                RmEndSession(self.0);
            }
        }
    }

    let mut session_handle: u32 = 0;
    let mut session_key = [0u16; CCH_RM_SESSION_KEY + 1];

    let start_res = unsafe { RmStartSession(&mut session_handle, 0, session_key.as_mut_ptr()) };
    if start_res != 0 {
        return FileLockResult {
            path: path.to_string(),
            processes: Vec::new(),
            error: Some(format!("RmStartSession failed with error code {start_res}")),
        };
    }

    let session = RmSession(session_handle);

    let wide_path: Vec<u16> = OsStr::new(path).encode_wide().chain(std::iter::once(0)).collect();
    let file_ptrs = [wide_path.as_ptr()];

    let reg_res = unsafe {
        RmRegisterResources(
            session.0,
            1,
            file_ptrs.as_ptr(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
        )
    };

    if reg_res != 0 {
        return FileLockResult {
            path: path.to_string(),
            processes: Vec::new(),
            error: Some(format!("RmRegisterResources failed with error code {reg_res}")),
        };
    }

    let mut n_proc_needed: u32 = 0;
    let mut n_proc: u32 = 0;
    let mut reboot_reasons: u32 = 0;

    let get_res = unsafe {
        RmGetList(
            session.0,
            &mut n_proc_needed,
            &mut n_proc,
            std::ptr::null_mut(),
            &mut reboot_reasons,
        )
    };

    let mut processes = Vec::new();
    let mut error = None;

    if get_res == ERROR_MORE_DATA || (get_res == 0 && n_proc_needed > 0) {
        let count = n_proc_needed as usize;
        let mut proc_info: Vec<RM_PROCESS_INFO> = vec![RM_PROCESS_INFO::default(); count];
        n_proc = n_proc_needed;

        let get_list_res = unsafe {
            RmGetList(
                session.0,
                &mut n_proc_needed,
                &mut n_proc,
                proc_info.as_mut_ptr(),
                &mut reboot_reasons,
            )
        };

        if get_list_res == 0 {
            for info in proc_info.iter().take(n_proc as usize) {
                let mut name = decode_utf16_null_terminated(&info.str_app_name);
                if name.is_empty() {
                    name = decode_utf16_null_terminated(&info.str_service_short_name);
                }
                if name.is_empty() {
                    name = format!("PID {}", info.process.dw_process_id);
                }

                let (app_type_str, is_service) = map_rm_app_type(info.application_type);
                processes.push(LockingProcess {
                    pid: info.process.dw_process_id,
                    name,
                    app_type: app_type_str.to_string(),
                    is_service,
                });
            }
        } else {
            error = Some(format!(
                "RmGetList failed to retrieve process list with error code {get_list_res}"
            ));
        }
    } else if get_res != 0 {
        error = Some(format!("RmGetList failed with error code {get_res}"));
    }

    FileLockResult {
        path: path.to_string(),
        processes,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_file_lock_result_empty_for_unlocked_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("sysmon_lock_test_empty.txt");
        {
            let mut f = File::create(&test_file).expect("create test file");
            writeln!(f, "test data").expect("write test file");
        }

        let result = find_locking_processes(test_file.to_str().unwrap());
        assert_eq!(result.path, test_file.to_str().unwrap());
        // An unlocked closed file should have no locking processes
        assert!(result.processes.is_empty());
        assert!(result.error.is_none());

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_file_lock_detects_open_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("sysmon_lock_test_locked.txt");
        let f = File::create(&test_file).expect("create test file");

        let result = find_locking_processes(test_file.to_str().unwrap());
        assert_eq!(result.path, test_file.to_str().unwrap());
        assert!(result.error.is_none());
        let current_pid = std::process::id();
        let has_current_proc = result.processes.iter().any(|p| p.pid == current_pid);
        assert!(
            has_current_proc,
            "Expected current process (PID {}) to be detected locking {:?}, found: {:?}",
            current_pid, test_file, result.processes
        );

        drop(f);
        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_file_lock_result_serialization() {
        let result = FileLockResult {
            path: "C:\\test\\file.txt".to_string(),
            processes: vec![LockingProcess {
                pid: 1234,
                name: "test_process.exe".to_string(),
                app_type: "Desktop App".to_string(),
                is_service: false,
            }],
            error: None,
        };

        let json = serde_json::to_string(&result).expect("serialize FileLockResult");
        let deserialized: FileLockResult = serde_json::from_str(&json).expect("deserialize FileLockResult");
        assert_eq!(result, deserialized);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_decode_utf16_null_terminated() {
        let buffer = [b'H' as u16, b'i' as u16, 0, b'X' as u16];
        assert_eq!(decode_utf16_null_terminated(&buffer), "Hi");

        let buffer_no_null = [b'A' as u16, b'B' as u16];
        assert_eq!(decode_utf16_null_terminated(&buffer_no_null), "AB");

        let buffer_empty: [u16; 0] = [];
        assert_eq!(decode_utf16_null_terminated(&buffer_empty), "");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_map_rm_app_type_enum() {
        assert_eq!(map_rm_app_type(0), ("Unknown Application", false));
        assert_eq!(map_rm_app_type(1), ("Desktop App", false));
        assert_eq!(map_rm_app_type(2), ("Background Window", false));
        assert_eq!(map_rm_app_type(3), ("Windows Service", true));
        assert_eq!(map_rm_app_type(4), ("Windows Explorer", false));
        assert_eq!(map_rm_app_type(5), ("Console App", false));
        assert_eq!(map_rm_app_type(1000), ("Critical System Service", true));
        assert_eq!(map_rm_app_type(9999), ("Application", false));
    }
}
