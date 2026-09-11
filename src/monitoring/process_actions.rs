use crate::app::models::{SystemMonitor, is_excluded};
use crate::processes::{ProcessIdentity, open_process_for_action, process_identity};
use windows_sys::Win32::System::Threading::*;

#[derive(Debug, Default, Clone)]
pub struct RamCleanOutcome {
    pub attempted: u32,
    pub trimmed: u32,
    pub denied: u32,
    pub failed: u32,
    /// Sum of observed per-process working-set decreases; not physical RAM recovered.
    pub working_set_reduction: u64,
    pub stopped: bool,
}

impl std::fmt::Display for RamCleanOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Working-set trim: {} attempted, {} trimmed, {} denied, {} failed; observed working-set decrease {} bytes (not guaranteed physical RAM recovered)",
            self.attempted, self.trimmed, self.denied, self.failed, self.working_set_reduction
        )
    }
}

/// Windows last-input time is session input, independent of repaint activity.
pub fn user_idle_duration() -> Result<std::time::Duration, String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    let mut input = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if unsafe { GetLastInputInfo(&mut input) } == 0 {
        return Err(format!("GetLastInputInfo: {}", std::io::Error::last_os_error()));
    }
    let now = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount() };
    Ok(std::time::Duration::from_millis(now.wrapping_sub(input.dwTime) as u64))
}

pub(crate) fn should_stop_trim(
    observed: u64,
    budget: Option<u64>,
    usage: f32,
    target: Option<f32>,
    idle_allowed: bool,
) -> bool {
    !idle_allowed || budget.is_some_and(|limit| observed >= limit) || target.is_some_and(|limit| usage <= limit)
}

impl SystemMonitor {
    pub(crate) fn kill_process(&mut self, identity: ProcessIdentity) -> Result<(), String> {
        let handle = open_process_for_action(identity, PROCESS_TERMINATE)?;
        if unsafe { TerminateProcess(handle.0, 1) } == 0 {
            return Err(format!(
                "TerminateProcess({identity}): {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    pub(crate) fn suspend_process(&mut self, identity: ProcessIdentity) -> Result<(), String> {
        let handle = open_process_for_action(identity, PROCESS_SUSPEND_RESUME)?;
        let status = unsafe { ntapi::ntpsapi::NtSuspendProcess(handle.0.cast()) };
        if status < 0 {
            return Err(format!(
                "NtSuspendProcess({identity}): NTSTATUS {:#010x}",
                status as u32
            ));
        }
        Ok(())
    }

    pub(crate) fn resume_process(&mut self, identity: ProcessIdentity) -> Result<(), String> {
        let handle = open_process_for_action(identity, PROCESS_SUSPEND_RESUME)?;
        let status = unsafe { ntapi::ntpsapi::NtResumeProcess(handle.0.cast()) };
        if status < 0 {
            return Err(format!("NtResumeProcess({identity}): NTSTATUS {:#010x}", status as u32));
        }
        Ok(())
    }

    /// Budget is best-effort: a single EmptyWorkingSet may overshoot it by one process.
    pub fn clean_ram(
        &mut self,
        exclusions: &[String],
        smart_only: bool,
        budget_bytes: Option<u64>,
        target_percent: Option<f32>,
        idle_only: bool,
    ) -> Result<RamCleanOutcome, String> {
        use windows_sys::Win32::System::ProcessStatus::{
            EmptyWorkingSet, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
        // Refresh before inventory and baseline, including processes launched after worker startup.
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        self.sys.refresh_memory();
        let mut outcome = RamCleanOutcome::default();
        let targets: Vec<_> = self
            .sys
            .processes()
            .iter()
            .filter_map(|(pid, process)| {
                if pid.as_u32() == std::process::id()
                    || pid.as_u32() <= 4
                    || is_excluded(&process.name().to_string_lossy(), exclusions)
                {
                    return None;
                }
                Some(process_identity(pid.as_u32()))
            })
            .collect();
        for target in targets {
            self.sys.refresh_memory();
            let usage = self.sys.used_memory() as f32 / self.sys.total_memory().max(1) as f32 * 100.0;
            let idle_allowed = !idle_only || user_idle_duration()?.as_secs() >= 120;
            if should_stop_trim(
                outcome.working_set_reduction,
                budget_bytes,
                usage,
                target_percent,
                idle_allowed,
            ) {
                outcome.stopped = true;
                break;
            }
            let identity = match target {
                Ok(identity) => identity,
                Err(error) => {
                    outcome.attempted += 1;
                    if error.contains("os error 5") {
                        outcome.denied += 1;
                    } else {
                        outcome.failed += 1;
                    }
                    continue;
                }
            };
            if smart_only {
                let mut foreground_pid = 0;
                unsafe {
                    GetWindowThreadProcessId(GetForegroundWindow(), &mut foreground_pid);
                }
                if identity.pid == foreground_pid {
                    continue;
                }
            }
            outcome.attempted += 1;
            let handle = match open_process_for_action(identity, PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA) {
                Ok(handle) => handle,
                Err(error) => {
                    if error.contains("os error 5") {
                        outcome.denied += 1;
                    } else {
                        outcome.failed += 1;
                    }
                    continue;
                }
            };
            let mut before = PROCESS_MEMORY_COUNTERS {
                cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                ..Default::default()
            };
            if unsafe {
                GetProcessMemoryInfo(
                    handle.0,
                    &mut before,
                    std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                )
            } == 0
            {
                outcome.failed += 1;
                continue;
            }
            if unsafe { EmptyWorkingSet(handle.0) } == 0 {
                if std::io::Error::last_os_error().raw_os_error() == Some(5) {
                    outcome.denied += 1;
                } else {
                    outcome.failed += 1;
                }
                continue;
            }
            outcome.trimmed += 1;
            let mut after = PROCESS_MEMORY_COUNTERS {
                cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                ..Default::default()
            };
            if unsafe {
                GetProcessMemoryInfo(
                    handle.0,
                    &mut after,
                    std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                )
            } != 0
            {
                outcome.working_set_reduction = outcome
                    .working_set_reduction
                    .saturating_add(before.WorkingSetSize.saturating_sub(after.WorkingSetSize) as u64);
            } else {
                outcome.failed += 1;
            }
        }
        if outcome.trimmed == 0 && outcome.attempted > 0 {
            return Err(outcome.to_string());
        }
        Ok(outcome)
    }

    pub(crate) fn set_process_priority(identity: ProcessIdentity, priority: &str) -> Result<(), String> {
        let class = match priority {
            "High" => HIGH_PRIORITY_CLASS,
            "AboveNormal" => ABOVE_NORMAL_PRIORITY_CLASS,
            "Normal" => NORMAL_PRIORITY_CLASS,
            "BelowNormal" => BELOW_NORMAL_PRIORITY_CLASS,
            "Idle" => IDLE_PRIORITY_CLASS,
            _ => return Err(format!("Unsupported process priority: {priority}")),
        };
        let handle = open_process_for_action(identity, PROCESS_SET_INFORMATION)?;
        if unsafe { SetPriorityClass(handle.0, class) } == 0 {
            return Err(format!(
                "SetPriorityClass({identity}): {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_process_policy_stops_after_single_process_overshoot() {
        let mut operations = 0;
        let mut measured = 0;
        for change in [4096, 4096] {
            if should_stop_trim(measured, Some(1024), 90.0, Some(50.0), true) {
                break;
            }
            operations += 1;
            measured += change;
        }
        assert_eq!(operations, 1);
        assert_eq!(measured, 4096);
        assert!(should_stop_trim(0, None, 40.0, Some(50.0), true));
        assert!(should_stop_trim(0, None, 90.0, None, false));
    }
}
