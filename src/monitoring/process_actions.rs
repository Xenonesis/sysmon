use crate::app::models::{SystemMonitor, is_excluded};

use tracing::{info, warn};

use sysinfo::Pid;

impl SystemMonitor {
    pub(crate) fn kill_process(&mut self, pid: u32) -> bool {
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        if let Some(process) = self.sys.process(Pid::from_u32(pid)) {
            let result = process.kill();
            if result {
                info!(pid = pid, "Process killed successfully");
            } else {
                warn!(pid = pid, "Failed to kill process");
            }
            result
        } else {
            warn!(pid = pid, "Process not found for kill");
            false
        }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn suspend_process(&mut self, pid: u32) -> bool {
        use ntapi::ntpsapi::NtSuspendProcess;
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};

        unsafe {
            if let Ok(h) = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid) {
                if !h.is_invalid() {
                    let result = NtSuspendProcess(h.0 as *mut _);
                    let _ = CloseHandle(h);
                    result == 0
                } else {
                    false
                }
            } else {
                false
            }
        }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn resume_process(&mut self, pid: u32) -> bool {
        use ntapi::ntpsapi::NtResumeProcess;
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};

        unsafe {
            if let Ok(h) = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid) {
                if !h.is_invalid() {
                    let result = NtResumeProcess(h.0 as *mut _);
                    let _ = CloseHandle(h);
                    result == 0
                } else {
                    false
                }
            } else {
                false
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn suspend_process(&mut self, _pid: u32) -> bool {
        false
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn resume_process(&mut self, _pid: u32) -> bool {
        false
    }

    #[cfg(target_os = "windows")]
    pub fn clean_ram(&mut self, exclusions: &[String], smart_only: bool) -> u64 {
        use windows::Win32::Foundation::{CloseHandle, E_ACCESSDENIED};
        use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA};

        info!(
            excluded = exclusions.len(),
            "RAM clean operation initiated (native API)"
        );
        let mem_before = self.sys.used_memory();
        let mut trimmed = 0u32;
        let mut access_denied = 0u32;
        let mut errored = 0u32;

        let mut foreground_pid = 0;
        if smart_only {
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
                let hwnd = GetForegroundWindow();
                if !hwnd.0.is_null() {
                    GetWindowThreadProcessId(hwnd, Some(&mut foreground_pid));
                }
            }
        }

        unsafe {
            for (pid, process) in self.sys.processes() {
                if is_excluded(&process.name().to_string_lossy(), exclusions) {
                    continue;
                }
                let pid_u32 = pid.as_u32();
                if smart_only && pid_u32 == foreground_pid {
                    continue;
                }
                match OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, false, pid_u32) {
                    Ok(h) if !h.is_invalid() => {
                        match EmptyWorkingSet(h) {
                            Ok(()) => trimmed += 1,
                            Err(e) if e.code() == E_ACCESSDENIED => access_denied += 1,
                            Err(_) => errored += 1,
                        }
                        let _ = CloseHandle(h);
                    }
                    Err(e) if e.code() == E_ACCESSDENIED => access_denied += 1,
                    _ => errored += 1,
                }
            }
        }

        self.sys.refresh_memory();
        let mem_after = self.sys.used_memory();
        let freed = mem_before.saturating_sub(mem_after);
        info!(
            freed_mb = freed / 1024 / 1024,
            trimmed = trimmed,
            access_denied = access_denied,
            errored = errored,
            "RAM clean complete"
        );
        freed
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn clean_ram(&mut self, _exclusions: &[String], _smart_only: bool) -> u64 {
        0
    }

    // Startup item collection and actions are now in startup.rs module

    #[cfg(target_os = "windows")]
    pub(crate) fn set_process_priority(pid: u32, priority: &str) -> bool {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_CREATION_FLAGS, SetPriorityClass};

        let priority_class: PROCESS_CREATION_FLAGS = match priority {
            "Realtime" => windows::Win32::System::Threading::REALTIME_PRIORITY_CLASS,
            "High" => windows::Win32::System::Threading::HIGH_PRIORITY_CLASS,
            "AboveNormal" => windows::Win32::System::Threading::ABOVE_NORMAL_PRIORITY_CLASS,
            "Normal" => windows::Win32::System::Threading::NORMAL_PRIORITY_CLASS,
            "BelowNormal" => windows::Win32::System::Threading::BELOW_NORMAL_PRIORITY_CLASS,
            "Idle" => windows::Win32::System::Threading::IDLE_PRIORITY_CLASS,
            _ => return false,
        };

        unsafe {
            if let Ok(h) = OpenProcess(windows::Win32::System::Threading::PROCESS_SET_INFORMATION, false, pid) {
                if !h.is_invalid() {
                    let result = SetPriorityClass(h, priority_class);
                    let _ = CloseHandle(h);
                    result.is_ok()
                } else {
                    false
                }
            } else {
                false
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn set_process_priority(_pid: u32, _priority: &str) -> bool {
        false
    }
}
