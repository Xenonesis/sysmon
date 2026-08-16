use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::time::Instant;
use sysinfo::{Disks, Networks, System};

#[cfg(target_os = "windows")]
use nvml_wrapper::Nvml;
#[cfg(target_os = "windows")]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Win32Battery {
    pub(crate) design_capacity: Option<u32>,
    pub(crate) full_charge_capacity: Option<u32>,
    pub(crate) battery_status: Option<u16>,
}

#[cfg(target_os = "windows")]
fn battery_status_label(status: u16) -> Option<&'static str> {
    match status {
        1 => Some("Discharging"),
        2 => Some("AC Power"),
        3 => Some("Fully Charged"),
        4 => Some("Low"),
        5 => Some("Critical"),
        6 => Some("Charging"),
        7 => Some("Charging and High"),
        8 => Some("Charging and Low"),
        9 => Some("Charging and Critical"),
        10 => Some("Undefined"),
        11 => Some("Partially Charged"),
        _ => Some("Unknown"),
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn get_battery_info(wmi_con: &wmi::WMIConnection) -> Option<BatteryInfo> {
    let results: Result<Vec<Win32Battery>, _> =
        wmi_con.raw_query("SELECT DesignCapacity, FullChargeCapacity, BatteryStatus, DischargeRate FROM Win32_Battery");
    if let Ok(mut bats) = results {
        if let Some(bat) = bats.pop() {
            let discharge_state = bat.battery_status.and_then(battery_status_label).map(|s| s.to_string());
            return Some(BatteryInfo {
                design_capacity: bat.design_capacity.unwrap_or(0),
                full_charge_capacity: bat.full_charge_capacity.unwrap_or(0),
                status: bat.battery_status.unwrap_or(0),
                discharge_state,
                present: true,
            });
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub(crate) fn play_alert_sound() {
    use std::os::windows::process::CommandExt;
    std::thread::spawn(|| {
        let _ = std::process::Command::new("powershell")
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .arg("-c")
            .arg("[System.Media.SystemSounds]::Exclamation.Play()")
            .output();
    });
}

#[cfg(target_os = "windows")]
pub(crate) fn play_success_sound() {
    use std::os::windows::process::CommandExt;
    std::thread::spawn(|| {
        let _ = std::process::Command::new("powershell")
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .arg("-c")
            .arg("[System.Media.SystemSounds]::Asterisk.Play()")
            .output();
    });
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn play_alert_sound() {}

#[cfg(not(target_os = "windows"))]
pub(crate) fn play_success_sound() {}

// Data structures
#[derive(Clone)]
pub(crate) struct CpuCoreInfo {
    pub(crate) core_id: usize,
    pub(crate) usage: f32,
    #[allow(dead_code)]
    pub(crate) name: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct GpuInfo {
    pub(crate) name: String,
    pub(crate) utilization: f32,
    pub(crate) memory_used: Option<u64>,
    pub(crate) memory_total: Option<u64>,
    pub(crate) temperature: Option<u32>,
    pub(crate) clock_mhz: Option<u32>,
    pub(crate) power_watts: Option<f32>,
    pub(crate) fan_percent: Option<u32>,
}

pub(crate) fn cpu_cores_from_telemetry(snapshot: &crate::telemetry::TelemetrySnapshot) -> Vec<CpuCoreInfo> {
    let count = snapshot.metrics.get("cpu.core_count").copied().unwrap_or_default() as usize;
    (0..count)
        .filter_map(|core_id| {
            snapshot
                .metrics
                .get(&format!("cpu.core.{core_id}.usage"))
                .map(|usage| CpuCoreInfo {
                    core_id,
                    usage: *usage as f32,
                    name: format!("Core {core_id}"),
                })
        })
        .collect()
}

pub(crate) fn gpus_from_telemetry(snapshot: &crate::telemetry::TelemetrySnapshot) -> Vec<GpuInfo> {
    let count = snapshot.metrics.get("gpu.device_count").copied().unwrap_or_default() as usize;
    let mut gpus: Vec<_> = (0..count)
        .map(|index| {
            let prefix = format!("gpu.{index}");
            let metric = |name: &str| snapshot.metrics.get(&format!("{prefix}.{name}")).copied();
            GpuInfo {
                name: snapshot
                    .labels
                    .get(&format!("{prefix}.name"))
                    .cloned()
                    .unwrap_or_else(|| format!("GPU {index}")),
                utilization: metric("utilization").unwrap_or_default() as f32,
                memory_used: metric("vram_used").map(|value| value as u64),
                memory_total: metric("vram_total").map(|value| value as u64),
                temperature: metric("temperature").map(|value| value as u32),
                clock_mhz: metric("clock_graphics").map(|value| value as u32),
                power_watts: metric("power_draw_mw").map(|value| value as f32 / 1_000.0),
                fan_percent: metric("fan_speed").map(|value| value as u32),
            }
        })
        .collect();

    let generic_count = snapshot.metrics.get("gpu.generic_count").copied().unwrap_or_default() as usize;
    let generic_utilization = snapshot
        .metrics
        .get("gpu.generic.utilization")
        .copied()
        .unwrap_or_default() as f32;
    let generic_memory_used = snapshot.metrics.get("gpu.generic.vram_used").map(|value| *value as u64);
    for index in 0..generic_count {
        let prefix = format!("gpu.generic.{index}");
        let name = snapshot
            .labels
            .get(&format!("{prefix}.name"))
            .cloned()
            .unwrap_or_else(|| format!("GPU {index}"));
        if gpus.iter().any(|gpu| gpu.name.eq_ignore_ascii_case(&name)) {
            continue;
        }
        gpus.push(GpuInfo {
            name,
            utilization: snapshot
                .metrics
                .get(&format!("{prefix}.utilization"))
                .copied()
                .map(|value| value as f32)
                .unwrap_or(generic_utilization),
            memory_used: generic_memory_used,
            memory_total: snapshot
                .metrics
                .get(&format!("{prefix}.vram_total"))
                .map(|value| *value as u64),
            temperature: None,
            clock_mhz: None,
            power_watts: None,
            fan_percent: None,
        });
    }
    gpus
}

#[derive(Clone, Serialize)]
pub(crate) struct DiskInfo {
    pub(crate) name: String,
    pub(crate) mount_point: String,
    pub(crate) total_space: u64,
    pub(crate) available_space: u64,
    pub(crate) usage_percentage: f32,
    pub(crate) file_system: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct NetworkInfo {
    pub(crate) interface: String,
    pub(crate) received: u64,
    pub(crate) transmitted: u64,
    pub(crate) received_rate: f64,
    pub(crate) transmitted_rate: f64,
}

#[derive(Clone)]
pub(crate) struct AlertInfo {
    pub(crate) timestamp: String,
    pub(crate) alert_type: AlertType,
    pub(crate) message: String,
    pub(crate) value: f32,
}

impl AlertInfo {
    pub fn key(&self) -> String {
        match self.alert_type {
            AlertType::GpuTempHigh => format!(
                "gpu:{}",
                self.message
                    .rsplit_once('(')
                    .map(|(_, v)| v.trim_end_matches(')'))
                    .unwrap_or("unknown")
            ),
            AlertType::DiskSpaceLow => format!(
                "disk:{}",
                self.message.split(" is almost full").next().unwrap_or(&self.message)
            ),
            AlertType::CpuHigh => "cpu".into(),
            AlertType::MemoryHigh => "memory".into(),
            AlertType::StartupHighImpact => "startup".into(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum AlertType {
    CpuHigh,
    MemoryHigh,
    GpuTempHigh,
    DiskSpaceLow,
    #[allow(dead_code)]
    StartupHighImpact,
}

// Swap / Page File info
#[derive(Clone, Serialize)]
pub(crate) struct SwapInfo {
    pub(crate) total: u64,
    pub(crate) used: u64,
    pub(crate) percentage: f32,
}

// Battery info

// StartupItem is now in startup.rs module

// RAM Cleaner state
#[derive(Clone)]
pub(crate) struct RamCleanerState {
    pub(crate) last_cleaned: Option<Instant>,
    pub(crate) last_cleaned_display: String,
    pub(crate) bytes_freed: u64,
    pub(crate) auto_clean_enabled: bool,
    pub(crate) auto_clean_threshold: f32, // percentage threshold for auto-clean
    pub(crate) auto_clean_interval: u64,  // seconds between auto-cleans
    pub(crate) auto_clean_target: f32,    // stop cleaning once usage drops below this
    pub(crate) auto_clean_exclusions: Vec<String>, // process names never touched
    pub(crate) auto_clean_idle_only: bool, // clean only after idle period
    pub(crate) auto_clean_smart_only: bool, // only clean inactive/background apps
    pub(crate) auto_clean_notify: bool,   // show freed-MB notification per auto-clean
    pub(crate) auto_clean_max_mb: u64,    // max MB freed per pass (0 = unlimited)
    pub(crate) is_cleaning: bool,
    pub(crate) clean_count: u32,
}

#[derive(Clone, Serialize)]
pub(crate) struct SystemInfo {
    pub(crate) os_name: String,
    pub(crate) os_version: String,
    pub(crate) kernel_version: String,
    pub(crate) hostname: String,
    pub(crate) uptime: u64,
    pub(crate) cpu_count: usize,
    pub(crate) cpu_brand: String,
    pub(crate) motherboard: Option<String>,
    pub(crate) bios_version: Option<String>,
    pub(crate) gpu_driver: Option<String>,
    pub(crate) os_build: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
    System,
}

fn deserialize_app_theme<'de, D>(deserializer: D) -> Result<AppTheme, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct AppThemeVisitor;

    impl<'de> serde::de::Visitor<'de> for AppThemeVisitor {
        type Value = AppTheme;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a boolean or theme string ('Dark', 'Light', 'System')")
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(if v { AppTheme::Dark } else { AppTheme::Light })
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match v.to_ascii_lowercase().as_str() {
                "dark" => Ok(AppTheme::Dark),
                "light" => Ok(AppTheme::Light),
                "system" => Ok(AppTheme::System),
                _ => Err(serde::de::Error::unknown_variant(v, &["Dark", "Light", "System"])),
            }
        }
    }

    deserializer.deserialize_any(AppThemeVisitor)
}

// Settings structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct AppSettings {
    pub(crate) refresh_interval: u64,
    pub(crate) show_graphs: bool,
    pub(crate) show_gpu: bool,
    pub(crate) show_processes: bool,
    pub(crate) show_notifications: bool,
    pub(crate) notification_cpu_threshold: f32,
    pub(crate) notification_memory_threshold: f32,
    pub(crate) notification_temp_threshold: u32,
    #[serde(default, alias = "theme_dark", deserialize_with = "deserialize_app_theme")]
    pub(crate) theme: AppTheme,
    pub(crate) show_per_core_cpu: bool,
    pub(crate) process_count: usize,
    pub(crate) auto_clear_alerts: bool,
    pub(crate) auto_start: bool,
    pub(crate) start_minimized: bool,
    #[serde(default = "default_show_cpu_cores")]
    pub(crate) show_cpu_cores: bool,
    #[serde(default = "default_show_widget")]
    pub(crate) show_widget: bool,
    pub(crate) minimize_to_tray: bool,
    #[serde(default = "default_auto_ram_clean")]
    pub(crate) auto_ram_clean: bool,
    #[serde(default = "default_ram_clean_threshold")]
    pub(crate) ram_clean_threshold: f32,
    #[serde(default = "default_enable_sounds")]
    pub(crate) enable_sounds: bool,
    #[serde(default)]
    pub(crate) startup_optimization_history: Vec<crate::startup::StartupOptimizationEntry>,
    #[serde(default)]
    pub(crate) last_boot_diagnostics: Option<crate::startup::BootDiagnostics>,
    #[serde(default = "default_auto_clean_interval")]
    pub(crate) auto_clean_interval: u64,
    #[serde(default = "default_auto_clean_target")]
    pub(crate) auto_clean_target: f32,
    #[serde(default)]
    pub(crate) auto_clean_exclusions: Vec<String>,
    #[serde(default)]
    pub(crate) auto_clean_idle_only: bool,
    #[serde(default)]
    pub(crate) auto_clean_smart_only: bool,
    #[serde(default = "default_auto_clean_notify")]
    pub(crate) auto_clean_notify: bool,
    #[serde(default)]
    pub(crate) auto_clean_max_mb: u64,
    #[serde(default = "default_notification_disk_threshold")]
    pub(crate) notification_disk_threshold: f32,
    #[serde(default)]
    pub(crate) sidebar_collapsed: bool,
}

fn default_notification_disk_threshold() -> f32 {
    90.0
}

fn default_auto_clean_interval() -> u64 {
    300
}

fn default_auto_clean_target() -> f32 {
    70.0
}

fn default_auto_clean_notify() -> bool {
    true
}

fn default_enable_sounds() -> bool {
    true
}

fn default_show_cpu_cores() -> bool {
    true
}
fn default_show_widget() -> bool {
    false
}
fn default_auto_ram_clean() -> bool {
    false
}
fn default_ram_clean_threshold() -> f32 {
    85.0
}

// RAM cleaner pure logic
pub(crate) fn is_excluded(name: &str, exclusions: &[String]) -> bool {
    let name = name.to_lowercase();
    exclusions.iter().any(|ex| name == ex.to_lowercase())
}

pub(crate) fn should_stop_cleaning(usage_pct: f64, target: f64, freed: u64, budget_left: u64) -> bool {
    usage_pct <= target || freed == 0 || freed >= budget_left
}

pub(crate) struct SystemMonitor {
    pub(crate) sys: System,
    pub(crate) disks: Disks,
    pub(crate) networks: Networks,
    #[cfg(target_os = "windows")]
    pub(crate) nvml: Option<Nvml>,
    #[cfg(target_os = "windows")]
    pub(crate) wmi_com: Option<std::rc::Rc<wmi::COMLibrary>>,
    #[cfg(target_os = "windows")]
    pub(crate) wmi_thermal: Option<wmi::WMIConnection>,
    #[cfg(target_os = "windows")]
    pub(crate) wmi_gpu_engine_class: Option<String>,
    #[cfg(target_os = "windows")]
    pub(crate) wmi_gpu_memory_class: Option<String>,
    pub(crate) last_network_update: Instant,
    pub(crate) last_disk_update: Instant,
    pub(crate) previous_network_totals: std::collections::HashMap<String, (u64, u64)>,
    pub(crate) previous_disk_totals: (u64, u64),
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            refresh_interval: 2,
            show_graphs: true,
            show_gpu: true,
            show_processes: true,
            show_notifications: false,
            notification_cpu_threshold: 90.0,
            notification_memory_threshold: 90.0,
            notification_temp_threshold: 85,
            theme: AppTheme::Dark,
            show_per_core_cpu: false,
            process_count: 15,
            auto_clear_alerts: false,
            auto_start: false,
            start_minimized: false,
            minimize_to_tray: false,
            auto_ram_clean: false,
            ram_clean_threshold: 85.0,
            enable_sounds: true,
            startup_optimization_history: Vec::new(),
            last_boot_diagnostics: None,
            auto_clean_interval: 300,
            auto_clean_target: 70.0,
            auto_clean_exclusions: Vec::new(),
            auto_clean_idle_only: false,
            auto_clean_smart_only: false,
            auto_clean_notify: true,
            auto_clean_max_mb: 0,
            show_cpu_cores: true,
            show_widget: false,
            notification_disk_threshold: 90.0,
            sidebar_collapsed: false,
        }
    }
}

impl AppSettings {
    #[cfg(target_os = "windows")]
    pub fn set_auto_start(&self, enable: bool) -> Result<(), Box<dyn std::error::Error>> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
        let (key, _) = hkcu.create_subkey(path)?;

        if enable {
            let exe_path = std::env::current_exe()?;
            key.set_value("SystemMonitor", &format!("\"{}\"", exe_path.to_string_lossy()))?;
        } else {
            key.delete_value("SystemMonitor").ok();
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn set_auto_start(&self, _enable: bool) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl AppSettings {
    pub fn load() -> Self {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "Xenonesis", "SystemMonitor") {
            let config_path = config_dir.config_dir().join("settings.json");
            if let Ok(settings) = crate::persistence::settings::load(&config_path) {
                return settings;
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "Xenonesis", "SystemMonitor") {
            let config_path = config_dir.config_dir();
            fs::create_dir_all(config_path)?;
            let config_file = config_path.join("settings.json");
            crate::persistence::settings::save(&config_file, self)?;
        }
        Ok(())
    }
}

// Historical data point
#[derive(Clone, Copy, Serialize)]
pub(crate) struct DataPoint {
    pub(crate) time: f64,
    pub(crate) value: f64,
}

// Shared state between threads
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct BatteryInfo {
    pub(crate) design_capacity: u32,
    pub(crate) full_charge_capacity: u32,
    pub(crate) status: u16,
    pub(crate) discharge_state: Option<String>,
    pub(crate) present: bool,
}
#[derive(Clone)]

pub(crate) struct SystemData {
    pub(crate) memory_total: u64,
    pub(crate) memory_used: u64,
    pub(crate) memory_percentage: f32,
    pub(crate) cpu_usage: f32,
    pub(crate) cpu_cores: Vec<CpuCoreInfo>,
    pub(crate) gpu_info: Vec<GpuInfo>,
    pub(crate) top_processes: Vec<crate::processes::ProcessInfo>,
    pub(crate) monitoring_paused: bool,
    pub(crate) selected_process_pid: Option<u32>,
    pub(crate) selected_process_details: Option<(u32, crate::processes::ProcessDetails)>,
    pub(crate) disk_info: Vec<DiskInfo>,
    pub(crate) network_info: Vec<NetworkInfo>,
    pub(crate) system_info: SystemInfo,
    pub(crate) cpu_temperature: Option<f32>,
    pub(crate) last_update: String,
    pub(crate) cpu_history: crate::monitoring::history::BoundedHistory<DataPoint>,
    pub(crate) memory_history: crate::monitoring::history::BoundedHistory<DataPoint>,
    pub(crate) gpu_history: crate::monitoring::history::BoundedHistory<DataPoint>,
    pub(crate) cpu_temp_history: crate::monitoring::history::BoundedHistory<DataPoint>,
    pub(crate) network_download_history: VecDeque<DataPoint>,
    pub(crate) network_upload_history: VecDeque<DataPoint>,
    pub(crate) alerts: Vec<AlertInfo>,
    pub(crate) start_time: Instant,
    pub(crate) swap_info: SwapInfo,
    pub(crate) battery_info: Option<BatteryInfo>,
    pub(crate) network_sample_count: u32,
    pub(crate) high_impact_startup_count: usize,
    pub(crate) ram_clean_freed_bytes: u64,
    pub(crate) ram_clean_is_cleaning: bool,
    pub(crate) disk_read_rate: f64,
    pub(crate) disk_write_rate: f64,
    pub(crate) disk_read_history: VecDeque<DataPoint>,
    pub(crate) disk_write_history: VecDeque<DataPoint>,
    pub(crate) is_hidden: bool,
    pub(crate) selected_tab: crate::Tab,
    pub(crate) services: Vec<crate::services::ServiceInfo>,
    pub(crate) last_activity: Instant,
    pub(crate) telemetry_history_stats: std::collections::HashMap<String, crate::telemetry::HistoryStats>,
    pub(crate) provider_status: std::collections::HashMap<String, bool>,
}

impl Default for SystemData {
    fn default() -> Self {
        Self {
            memory_total: 0,
            memory_used: 0,
            memory_percentage: 0.0,
            cpu_usage: 0.0,
            cpu_cores: Vec::new(),
            gpu_info: Vec::new(),
            top_processes: Vec::new(),
            monitoring_paused: false,
            selected_process_pid: None,
            selected_process_details: None,
            disk_info: Vec::new(),
            network_info: Vec::new(),
            system_info: SystemInfo {
                os_name: String::new(),
                os_version: String::new(),
                kernel_version: String::new(),
                hostname: String::new(),
                uptime: 0,
                cpu_count: 0,
                cpu_brand: String::new(),
                motherboard: None,
                bios_version: None,
                gpu_driver: None,
                os_build: None,
            },
            cpu_temperature: None,
            last_update: String::new(),
            cpu_history: crate::monitoring::history::BoundedHistory::new(60),
            memory_history: crate::monitoring::history::BoundedHistory::new(60),
            gpu_history: crate::monitoring::history::BoundedHistory::new(60),
            cpu_temp_history: crate::monitoring::history::BoundedHistory::new(60),
            network_download_history: VecDeque::new(),
            network_upload_history: VecDeque::new(),
            alerts: Vec::new(),
            start_time: Instant::now(),
            swap_info: SwapInfo {
                total: 0,
                used: 0,
                percentage: 0.0,
            },
            battery_info: None,
            network_sample_count: 0,
            high_impact_startup_count: 0,
            ram_clean_freed_bytes: 0,
            ram_clean_is_cleaning: false,
            disk_read_rate: 0.0,
            disk_write_rate: 0.0,
            disk_read_history: VecDeque::new(),
            disk_write_history: VecDeque::new(),
            is_hidden: false,
            selected_tab: crate::Tab::Overview,
            last_activity: Instant::now(),
            services: Vec::new(),
            telemetry_history_stats: std::collections::HashMap::new(),
            provider_status: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_theme_defaults_to_dark() {
        assert_eq!(AppTheme::default(), AppTheme::Dark);
    }

    #[test]
    fn app_settings_deserializes_legacy_theme_dark_boolean() {
        let json_true = r#"{"refresh_interval":2,"show_graphs":true,"show_gpu":true,"show_processes":true,"show_notifications":false,"notification_cpu_threshold":90.0,"notification_memory_threshold":90.0,"notification_temp_threshold":85,"theme_dark":true,"show_per_core_cpu":false,"process_count":15,"auto_clear_alerts":false,"auto_start":false,"start_minimized":false,"minimize_to_tray":false}"#;
        let settings_dark: AppSettings = serde_json::from_str(json_true).unwrap();
        assert_eq!(settings_dark.theme, AppTheme::Dark);

        let json_false = r#"{"refresh_interval":2,"show_graphs":true,"show_gpu":true,"show_processes":true,"show_notifications":false,"notification_cpu_threshold":90.0,"notification_memory_threshold":90.0,"notification_temp_threshold":85,"theme_dark":false,"show_per_core_cpu":false,"process_count":15,"auto_clear_alerts":false,"auto_start":false,"start_minimized":false,"minimize_to_tray":false}"#;
        let settings_light: AppSettings = serde_json::from_str(json_false).unwrap();
        assert_eq!(settings_light.theme, AppTheme::Light);
    }

    #[test]
    fn app_settings_deserializes_new_theme_enum() {
        let json_dark = r#"{"refresh_interval":2,"show_graphs":true,"show_gpu":true,"show_processes":true,"show_notifications":false,"notification_cpu_threshold":90.0,"notification_memory_threshold":90.0,"notification_temp_threshold":85,"theme":"Dark","show_per_core_cpu":false,"process_count":15,"auto_clear_alerts":false,"auto_start":false,"start_minimized":false,"minimize_to_tray":false}"#;
        let settings: AppSettings = serde_json::from_str(json_dark).unwrap();
        assert_eq!(settings.theme, AppTheme::Dark);

        let json_light = r#"{"refresh_interval":2,"show_graphs":true,"show_gpu":true,"show_processes":true,"show_notifications":false,"notification_cpu_threshold":90.0,"notification_memory_threshold":90.0,"notification_temp_threshold":85,"theme":"Light","show_per_core_cpu":false,"process_count":15,"auto_clear_alerts":false,"auto_start":false,"start_minimized":false,"minimize_to_tray":false}"#;
        let settings: AppSettings = serde_json::from_str(json_light).unwrap();
        assert_eq!(settings.theme, AppTheme::Light);

        let json_system = r#"{"refresh_interval":2,"show_graphs":true,"show_gpu":true,"show_processes":true,"show_notifications":false,"notification_cpu_threshold":90.0,"notification_memory_threshold":90.0,"notification_temp_threshold":85,"theme":"System","show_per_core_cpu":false,"process_count":15,"auto_clear_alerts":false,"auto_start":false,"start_minimized":false,"minimize_to_tray":false}"#;
        let settings: AppSettings = serde_json::from_str(json_system).unwrap();
        assert_eq!(settings.theme, AppTheme::System);
    }

    #[test]
    fn app_settings_round_trip_serialization() {
        let original = AppSettings {
            theme: AppTheme::System,
            ..Default::default()
        };
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.theme, AppTheme::System);
    }
}
