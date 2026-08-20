use crate::{services::ServiceControlAction, startup::StartupLocator, AppSettings};

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
    DisableStartup { item_name: String, locator: StartupLocator },
    EnableStartup { item_name: String, locator: StartupLocator },
    QuarantineStartup { item_name: String, locator: StartupLocator },
    RestoreStartup { item_name: String, quarantine_id: String },
}
