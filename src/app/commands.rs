use crate::{
    AppSettings,
    processes::{AffinityPreset, ProcessIdentity},
    services::{ServiceControlAction, ServiceUndo},
    startup::{ReviewedStartupRestore, StartupLocator},
    storage::reclaimer::ReviewedCleanup,
};

#[derive(Debug, Clone)]
pub(crate) enum MonitoringCommand {
    SetSettings(Box<AppSettings>),
    SetPaused(bool),
    SetHidden(bool),
    SetConsumerDemand {
        recording: bool,
        process_manager: bool,
    },
    // force refresh while paused; wired to UI later
    #[allow(dead_code)]
    RefreshNow,
    // graceful worker exit; test exercises it
    #[allow(dead_code)]
    Shutdown,
}

#[derive(Debug, Clone)]
pub(crate) enum ActionCommand {
    KillProcess(ProcessIdentity),
    KillProcessTree(ProcessIdentity),
    SuspendProcess(ProcessIdentity),
    ResumeProcess(ProcessIdentity),
    SetPriority {
        identity: ProcessIdentity,
        priority: String,
    },
    SetAffinity {
        identity: ProcessIdentity,
        preset: AffinityPreset,
    },
    CleanRam,
    AutoCleanRam {
        exclusions: Vec<String>,
        smart_only: bool,
        budget_bytes: Option<u64>,
        target_percent: f32,
        idle_only: bool,
    },
    ControlService {
        name: String,
        action: ServiceControlAction,
    },
    UndoService {
        name: String,
        undo: ServiceUndo,
    },
    SetPowerPlan(String),
    DisableStartup {
        item_name: String,
        locator: StartupLocator,
    },
    EnableStartup {
        item_name: String,
        locator: StartupLocator,
    },
    QuarantineStartup {
        item_name: String,
        locator: StartupLocator,
    },
    RestoreStartup {
        review: ReviewedStartupRestore,
    },
    ReclaimStorageCaches(ReviewedCleanup),
}

impl ActionCommand {
    #[allow(dead_code)]
    pub fn requires_elevation(&self) -> bool {
        match self {
            Self::ReclaimStorageCaches(review) => review.category_ids().iter().any(|id| id == "windows_update"),
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn summary(&self) -> String {
        match self {
            Self::ReclaimStorageCaches(review) => format!(
                "Reclaim {} reviewed files ({} bytes): {}",
                review.file_count(),
                review.size_bytes(),
                review.category_ids().join(", ")
            ),
            _ => format!("{:?}", self),
        }
    }
}

/// Side effects requested by presentation code and executed by the app shell.
///
/// Keeping these outside page rendering makes headless UI tests deterministic
/// and prevents page modules from launching processes or calling Windows APIs.
#[derive(Debug, Clone)]
pub(crate) enum UiIntent {
    CheckUpdates,
    OpenServicesConsole,
    RelaunchAsAdmin,
    ControlService { name: String, action: ServiceControlAction },
}
