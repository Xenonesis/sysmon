use crate::app::models::*;
use crate::monitoring;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::time::Instant;
use sysinfo::{Disks, Networks, System};

#[cfg(target_os = "windows")]
use nvml_wrapper::Nvml;

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        #[cfg(target_os = "windows")]
        let nvml = Nvml::init().ok();

        #[cfg(target_os = "windows")]
        let wmi_thermal = wmi::WMIConnection::with_namespace_path("ROOT\\WMI").ok();

        SystemMonitor {
            sys,
            disks,
            networks,
            #[cfg(target_os = "windows")]
            nvml,
            #[cfg(target_os = "windows")]
            wmi_thermal,
            last_network_update: Instant::now(),
            last_disk_update: Instant::now(),
            previous_network_totals: std::collections::HashMap::new(),
            previous_disk_totals: std::collections::HashMap::new(),
        }
    }
}

pub(crate) struct SystemMonitorApp {
    pub(crate) data: Arc<RwLock<SystemData>>,
    pub(crate) quit_requested: bool,
    pub(crate) settings_save_error: Option<String>,
    pub(crate) settings_integration_error: Option<String>,
    pub(crate) last_monitoring_paused: bool,
    pub(crate) update_check_result_share: Arc<Mutex<Option<Result<crate::updater::UpdateInfo, String>>>>,
    pub(crate) update_check_pending: bool,
    pub(crate) update_check_status: Option<String>,
    pub(crate) app_channels: crate::app::AppChannels,
    pub(crate) latest_snapshot: Option<monitoring::SystemSnapshot>,
    pub(crate) action_pending: bool,
    pub(crate) action_status: Option<String>,
    pub(crate) pending_action_plan: Option<crate::app::actions::ActionPlan>,
    pub(crate) action_history: Vec<crate::app::actions::ActionHistoryEntry>,
    pub(crate) show_action_history: bool,
    pub(crate) session_recorder: crate::persistence::session::SessionRecorder,
    pub(crate) session_status: Option<String>,
    pub(crate) timeline: crate::timeline::TimelineHandle,
    pub(crate) timeline_ui: crate::timeline::TimelineUiState,
    pub(crate) telemetry_commands: std::sync::mpsc::SyncSender<crate::telemetry::HubCommand>,
    pub(crate) settings: AppSettings,
    pub(crate) shared_settings: Arc<Mutex<AppSettings>>,
    pub(crate) selected_tab: Tab,
    pub(crate) show_settings: bool,
    pub(crate) show_export: bool,
    pub(crate) show_alerts: bool,
    pub(crate) show_process_manager: bool,
    pub(crate) selected_process_pid: Option<crate::processes::ProcessIdentity>,
    pub(crate) details_pid: Option<crate::processes::ProcessIdentity>,
    pub(crate) kill_tree_pid: Option<crate::processes::ProcessIdentity>,
    pub(crate) service_page: crate::app::page_state::ServicePageState,
    pub(crate) storage_page: crate::app::page_state::StoragePageState,
    pub(crate) crash_reports: crate::diagnostics::minidump::CrashScanState,
    pub(crate) window_picker_active: bool,
    pub(crate) process_search: String,
    pub(crate) process_sort_column: crate::processes::ProcessSortColumn,
    pub(crate) process_sort_ascending: bool,
    pub(crate) show_export_csv: bool,
    pub(crate) updater: crate::updater::Updater,
    pub(crate) update_info_share: Arc<Mutex<Option<crate::updater::UpdateInfo>>>,
    pub(crate) show_update_notification: bool,
    pub(crate) update_check_time: Option<Instant>,
    /// `true` while the installer is being downloaded/verified in the background.
    pub(crate) update_downloading: bool,
    /// Last error from a failed install attempt; shown in the banner.
    pub(crate) update_error: Option<String>,
    /// Background thread writes `Some(Ok(()))` or `Some(Err(msg))` here when done.
    pub(crate) update_result_share: Arc<Mutex<Option<Result<crate::updater::InstallOutcome, String>>>>,
    pub(crate) ram_cleaner_state: RamCleanerState,
    pub(crate) startup_items: Vec<crate::startup::StartupItem>,
    pub(crate) startup_items_loaded: bool,
    pub(crate) startup_items_loading: bool,
    pub(crate) startup_items_share: Arc<Mutex<Option<Vec<crate::startup::StartupItem>>>>,
    pub(crate) startup_search: String,
    pub(crate) startup_sort: crate::startup::StartupSortColumn,
    pub(crate) startup_sort_ascending: bool,
    pub(crate) startup_filter_impact: Option<crate::startup::ImpactTier>,
    pub(crate) startup_filter_signed: Option<bool>,
    pub(crate) startup_filter_broken: bool,
    pub(crate) startup_show_confirm: Option<String>,
    pub(crate) boot_diagnostics: Option<crate::startup::BootDiagnostics>,
    pub(crate) boot_diagnostics_loaded: bool,
    pub(crate) boot_diagnostics_share: Arc<Mutex<Option<crate::startup::BootDiagnostics>>>,
    pub(crate) show_shortcuts: bool,
    pub(crate) suspend_process_pid: Option<crate::processes::ProcessIdentity>,
    pub(crate) resume_process_pid: Option<crate::processes::ProcessIdentity>,
    pub(crate) suspended_pids: std::collections::HashSet<crate::processes::ProcessIdentity>,
    pub(crate) priority_change: Option<(crate::processes::ProcessIdentity, String)>,
    pub(crate) process_tree_view: bool,
    pub(crate) affinity_change: Option<(crate::processes::ProcessIdentity, crate::processes::AffinityPreset)>,
    pub(crate) network_socket_search: String,
    #[allow(dead_code)]
    #[cfg(target_os = "windows")]
    pub(crate) tray_icon: Option<tray_icon::TrayIcon>,
    #[cfg(target_os = "windows")]
    pub(crate) tray_menu_show_id: Option<tray_icon::menu::MenuId>,
    #[cfg(target_os = "windows")]
    pub(crate) tray_menu_quit_id: Option<tray_icon::menu::MenuId>,
    #[cfg(target_os = "windows")]
    pub(crate) tray_menu_clean_id: Option<tray_icon::menu::MenuId>,
    #[cfg(target_os = "windows")]
    pub(crate) tray_menu_procman_id: Option<tray_icon::menu::MenuId>,
    #[cfg(target_os = "windows")]
    pub(crate) tray_menu_pause_id: Option<tray_icon::menu::MenuId>,
    #[cfg(target_os = "windows")]
    pub(crate) tray_menu_pause_item: Option<tray_icon::menu::CheckMenuItem>,
    #[cfg(target_os = "windows")]
    // kept alive for tray ownership; never read directly
    #[allow(dead_code)]
    pub(crate) tray_menu_handle: Option<tray_icon::menu::Menu>,
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub(crate) tray_menu_power_item: Option<tray_icon::menu::Submenu>,
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub(crate) tray_menu_power_items:
        std::collections::HashMap<tray_icon::menu::MenuId, tray_icon::menu::CheckMenuItem>,
    #[cfg(target_os = "windows")]
    pub(crate) tray_menu_power_guids: std::collections::HashMap<tray_icon::menu::MenuId, String>,
    #[cfg(target_os = "windows")]
    pub(crate) _hotkey_manager: Option<global_hotkey::GlobalHotKeyManager>,
    #[cfg(target_os = "windows")]
    pub(crate) clean_ram_hotkey: Option<global_hotkey::hotkey::HotKey>,
    pub(crate) is_hidden: bool,
    pub(crate) widget_open: bool,
    /// Whether we have already applied the start_minimized setting on the first frame.
    pub(crate) start_minimized_applied: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tab {
    Overview,
    Performance,
    Processes,
    CpuCores,
    Storage,
    Network,
    SystemInfo,
    Alerts,
    RamCleaner,
    StartupManager,
    Services,
    Diagnostics,
    Timeline,
    About,
}

impl Drop for SystemMonitorApp {
    fn drop(&mut self) {
        let _ = self
            .app_channels
            .monitoring_sender
            .send(crate::app::commands::MonitoringCommand::Shutdown);
        let _ = self.telemetry_commands.try_send(crate::telemetry::HubCommand::Shutdown);
        self.timeline.shutdown();
    }
}

pub(crate) use crate::monitoring::snapshot_convert::{load_icon, load_tray_icon, snapshot_from_data};
