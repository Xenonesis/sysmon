use crate::{services::ServiceControlAction, AppSettings};

#[derive(Debug, Clone)]
pub(crate) enum MonitoringCommand {
    SetSettings(AppSettings),
    SetPaused(bool),
    SetHidden(bool),
    RefreshNow,
    Shutdown,
}

#[derive(Debug, Clone)]
pub(crate) enum ActionCommand {
    KillProcess(u32),
    KillProcessTree(u32),
    SuspendProcess(u32),
    ResumeProcess(u32),
    SetPriority { pid: u32, priority: String },
    CleanRam,
    ControlService { name: String, action: ServiceControlAction },
    SetPowerPlan(String),
    DisableStartup { identity: String },
    EnableStartup { identity: String },
    RemoveStartup { identity: String },
}
