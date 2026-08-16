use crate::{services::ServiceControlAction, AppSettings};

#[derive(Debug, Clone)]
pub(crate) enum MonitoringCommand {
    SetSettings(Box<AppSettings>),
    SetPaused(bool),
    SetHidden(bool),
    // force refresh while paused; wired to UI later
    #[allow(dead_code)]
    RefreshNow,
    // graceful worker exit; test exercises it
    #[allow(dead_code)]
    Shutdown,
}

#[derive(Debug, Clone)]
pub(crate) enum ActionCommand {
    KillProcess(u32),
    KillProcessTree(u32),
    SuspendProcess(u32),
    ResumeProcess(u32),
    SetPriority { pid: u32, priority: String },
    SetAffinity { pid: u32, mask: usize },
    CleanRam,
    ControlService { name: String, action: ServiceControlAction },
    SetPowerPlan(String),
    // ponytail: startup item mutations (DisableStartup/EnableStartup/RemoveStartup)
    // deferred — UI still calls startup::* directly; add variants here when
    // transactional registry moves land.
}
