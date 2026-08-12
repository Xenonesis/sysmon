#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
pub(crate) mod app;
pub(crate) mod ui;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use chrono::Local;
mod diagnostics;
mod monitoring;
mod persistence;
mod power;
mod privilege;
mod processes;
pub mod providers;
mod services;
mod startup;
pub mod telemetry;
mod updater;
use eframe::egui;
use processes::{ProcessInfo, ProcessSortColumn};
use startup::{BootDiagnostics, ImpactTier, Recommendation, StartupItem, StartupOptimizationEntry, StartupSortColumn};

use rfd::FileDialog;
use tracing::{error, info, warn};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, Pid, System};

#[cfg(target_os = "windows")]
use nvml_wrapper::Nvml;
#[cfg(target_os = "windows")]
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, Submenu},
    TrayIconBuilder,
};
#[cfg(target_os = "windows")]
use wmi::COMLibrary;

#[cfg(target_os = "windows")]
fn play_alert_sound() {
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
fn play_success_sound() {
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
fn play_alert_sound() {}

#[cfg(not(target_os = "windows"))]
fn play_success_sound() {}

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

fn cpu_cores_from_telemetry(snapshot: &telemetry::TelemetrySnapshot) -> Vec<CpuCoreInfo> {
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

fn gpus_from_telemetry(snapshot: &telemetry::TelemetrySnapshot) -> Vec<GpuInfo> {
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
    fn key(&self) -> String {
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
enum AlertType {
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
    pub(crate) theme_dark: bool,
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
    pub(crate) startup_optimization_history: Vec<StartupOptimizationEntry>,
    #[serde(default)]
    pub(crate) last_boot_diagnostics: Option<BootDiagnostics>,
    #[serde(default = "default_auto_clean_interval")]
    pub(crate) auto_clean_interval: u64,
    #[serde(default = "default_auto_clean_target")]
    pub(crate) auto_clean_target: f32,
    #[serde(default)]
    pub(crate) auto_clean_exclusions: Vec<String>,
    #[serde(default)]
    pub(crate) auto_clean_idle_only: bool,
    #[serde(default = "default_auto_clean_notify")]
    pub(crate) auto_clean_notify: bool,
    #[serde(default)]
    pub(crate) auto_clean_max_mb: u64,
    #[serde(default = "default_notification_disk_threshold")]
    pub(crate) notification_disk_threshold: f32,
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
fn is_excluded(name: &str, exclusions: &[String]) -> bool {
    let name = name.to_lowercase();
    exclusions.iter().any(|ex| name == ex.to_lowercase())
}

fn should_stop_cleaning(usage_pct: f64, target: f64, freed: u64, budget_left: u64) -> bool {
    usage_pct <= target || freed == 0 || freed >= budget_left
}

struct SystemMonitor {
    sys: System,
    disks: Disks,
    networks: Networks,
    #[cfg(target_os = "windows")]
    nvml: Option<Nvml>,
    #[cfg(target_os = "windows")]
    wmi_com: Option<std::rc::Rc<wmi::COMLibrary>>,
    #[cfg(target_os = "windows")]
    wmi_thermal: Option<wmi::WMIConnection>,
    #[cfg(target_os = "windows")]
    wmi_gpu_engine_class: Option<String>,
    #[cfg(target_os = "windows")]
    wmi_gpu_memory_class: Option<String>,
    last_network_update: Instant,
    last_disk_update: Instant,
    previous_network_totals: std::collections::HashMap<String, (u64, u64)>,
    previous_disk_totals: (u64, u64),
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
            theme_dark: true,
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
            auto_clean_notify: true,
            auto_clean_max_mb: 0,
            show_cpu_cores: true,
            show_widget: false,
            notification_disk_threshold: 90.0,
        }
    }
}

impl AppSettings {
    #[cfg(target_os = "windows")]
    fn set_auto_start(&self, enable: bool) -> Result<(), Box<dyn std::error::Error>> {
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
    fn load() -> Self {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "Xenonesis", "SystemMonitor") {
            let config_path = config_dir.config_dir().join("settings.json");
            if let Ok(settings) = persistence::settings::load(&config_path) {
                return settings;
            }
        }
        Self::default()
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "Xenonesis", "SystemMonitor") {
            let config_path = config_dir.config_dir();
            fs::create_dir_all(config_path)?;
            let config_file = config_path.join("settings.json");
            persistence::settings::save(&config_file, self)?;
        }
        Ok(())
    }
}

impl SystemMonitor {
    fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        #[cfg(target_os = "windows")]
        let nvml = Nvml::init().ok();

        // Probe which WMI GPU performance counter class name is available on this system.
        // Windows versions differ: some use "GPUPerformanceMonitors", others use "GPUPerformanceCounters".
        #[cfg(target_os = "windows")]
        let (wmi_com, wmi_gpu_engine_class, wmi_gpu_memory_class) = {
            let com = COMLibrary::new().ok().map(std::rc::Rc::new);
            let mut engine_class = None;
            let mut memory_class = None;
            if let Some(com_lib) = com.as_ref() {
                if let Ok(wmi) = wmi::WMIConnection::new(com_lib.clone()) {
                    for prefix in &["GPUPerformanceCounters", "GPUPerformanceMonitors"] {
                        if engine_class.is_none() {
                            let q = format!(
                                "SELECT UtilizationPercentage FROM Win32_PerfFormattedData_{}_GPUEngine",
                                prefix
                            );
                            if wmi
                                .raw_query::<std::collections::HashMap<String, wmi::Variant>>(&q)
                                .is_ok()
                            {
                                engine_class = Some(format!("Win32_PerfFormattedData_{}_GPUEngine", prefix));
                            }
                        }
                        if memory_class.is_none() {
                            let q = format!(
                                "SELECT LocalUsage FROM Win32_PerfFormattedData_{}_GPULocalAdapterMemory",
                                prefix
                            );
                            if wmi
                                .raw_query::<std::collections::HashMap<String, wmi::Variant>>(&q)
                                .is_ok()
                            {
                                memory_class =
                                    Some(format!("Win32_PerfFormattedData_{}_GPULocalAdapterMemory", prefix));
                            }
                        }
                    }
                }
            }
            (com, engine_class, memory_class)
        };

        #[cfg(target_os = "windows")]
        let wmi_thermal = wmi_com
            .as_ref()
            .and_then(|com| wmi::WMIConnection::with_namespace_path("ROOT\\WMI", com.clone()).ok());

        SystemMonitor {
            sys,
            disks,
            networks,
            #[cfg(target_os = "windows")]
            nvml,
            #[cfg(target_os = "windows")]
            wmi_com,
            #[cfg(target_os = "windows")]
            wmi_thermal,
            #[cfg(target_os = "windows")]
            wmi_gpu_engine_class,
            #[cfg(target_os = "windows")]
            wmi_gpu_memory_class,
            last_network_update: Instant::now(),
            last_disk_update: Instant::now(),
            previous_network_totals: std::collections::HashMap::new(),
            previous_disk_totals: (0, 0),
        }
    }

    fn get_memory_info(&self) -> (u64, u64, f32) {
        let total = self.sys.total_memory();
        let used = self.sys.used_memory();
        let percentage = (used as f64 / total as f64) * 100.0;
        (total, used, percentage as f32)
    }

    fn get_cpu_usage(&mut self) -> f32 {
        self.sys.global_cpu_info().cpu_usage()
    }

    fn get_top_processes(&self, count: usize) -> Vec<ProcessInfo> {
        let cpu_count = self.sys.cpus().len().max(1) as f32;
        let mut processes: Vec<_> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, process)| {
                // Try to use the exe path's file name if `name()` is empty or not helpful
                let mut name_str = process.name().to_string();
                if name_str.is_empty() {
                    if let Some(exe_path) = process.exe() {
                        if let Some(file_name) = exe_path.file_name() {
                            name_str = file_name.to_string_lossy().into_owned();
                        }
                    }
                }

                ProcessInfo {
                    pid: pid.as_u32(),
                    name: name_str,
                    cpu_usage: process.cpu_usage() / cpu_count,
                    memory: process.memory(),
                    status: format!("{:?}", process.status()),
                    disk_read_bytes: process.disk_usage().read_bytes,
                    disk_written_bytes: process.disk_usage().written_bytes,
                }
            })
            .collect();

        processes.sort_by_key(|process| std::cmp::Reverse(process.memory));
        processes.truncate(count);
        processes
    }

    fn get_cpu_cores_info(&self) -> Vec<CpuCoreInfo> {
        self.sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(id, cpu)| CpuCoreInfo {
                core_id: id,
                usage: cpu.cpu_usage(),
                name: cpu.name().to_string(),
            })
            .collect()
    }

    fn kill_process(&mut self, pid: u32) -> bool {
        self.sys.refresh_processes();
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
    fn suspend_process(&mut self, pid: u32) -> bool {
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
    fn resume_process(&mut self, pid: u32) -> bool {
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
    fn suspend_process(&mut self, _pid: u32) -> bool {
        false
    }

    #[cfg(not(target_os = "windows"))]
    fn resume_process(&mut self, _pid: u32) -> bool {
        false
    }

    fn get_swap_info(&self) -> SwapInfo {
        let total = self.sys.total_swap();
        let used = self.sys.used_swap();
        let percentage = if total > 0 {
            (used as f64 / total as f64 * 100.0) as f32
        } else {
            0.0
        };
        SwapInfo {
            total,
            used,
            percentage,
        }
    }

    #[cfg(target_os = "windows")]
    fn clean_ram(&mut self, exclusions: &[String]) -> u64 {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION};

        info!(
            excluded = exclusions.len(),
            "RAM clean operation initiated (native API)"
        );
        let mem_before = self.sys.used_memory();
        let mut success_count = 0;
        let mut fail_count = 0;

        unsafe {
            for (pid, process) in self.sys.processes() {
                if is_excluded(process.name(), exclusions) {
                    continue;
                }
                let pid_u32 = pid.as_u32();
                if let Ok(h) = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid_u32) {
                    if !h.is_invalid() {
                        if EmptyWorkingSet(h).is_ok() {
                            success_count += 1;
                        } else {
                            fail_count += 1;
                        }
                        let _ = CloseHandle(h);
                    } else {
                        fail_count += 1;
                    }
                } else {
                    fail_count += 1;
                }
            }
        }

        self.sys.refresh_memory();
        let mem_after = self.sys.used_memory();
        let freed = mem_before.saturating_sub(mem_after);
        info!(
            freed_mb = freed / 1024 / 1024,
            success = success_count,
            failed = fail_count,
            "RAM clean complete"
        );
        freed
    }

    #[cfg(not(target_os = "windows"))]
    fn clean_ram(&mut self, _exclusions: &[String]) -> u64 {
        0
    }

    // Startup item collection and actions are now in startup.rs module

    #[cfg(target_os = "windows")]
    fn set_process_priority(pid: u32, priority: &str) -> bool {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, SetPriorityClass, PROCESS_CREATION_FLAGS};

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
    fn set_process_priority(_pid: u32, _priority: &str) -> bool {
        false
    }

    #[cfg(target_os = "windows")]
    fn get_gpu_info(&self, include_wmi: bool) -> Vec<GpuInfo> {
        let mut gpus = Vec::new();
        let mut nvml_names: Vec<String> = Vec::new();

        // Collect all NVML (NVIDIA) GPUs
        if let Some(ref nvml) = self.nvml {
            if let Ok(device_count) = nvml.device_count() {
                for i in 0..device_count {
                    if let Ok(device) = nvml.device_by_index(i) {
                        let name = device.name().unwrap_or_else(|_| "Unknown GPU".to_string());
                        let utilization = device.utilization_rates().map(|u| u.gpu).unwrap_or(0);
                        let memory = device.memory_info().ok();
                        let temperature = device
                            .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                            .ok();
                        let clock_mhz = device
                            .clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics)
                            .ok();
                        let power_watts = device.power_usage().ok().map(|mw| mw as f32 / 1000.0);
                        let fan_percent = device.fan_speed(0).ok();

                        nvml_names.push(name.clone());
                        gpus.push(GpuInfo {
                            name,
                            utilization: utilization as f32,
                            memory_used: memory.as_ref().map(|m| m.used),
                            memory_total: memory.as_ref().map(|m| m.total),
                            temperature,
                            clock_mhz,
                            power_watts,
                            fan_percent,
                        });
                    }
                }
            }
        }

        // Also collect WMI GPUs (AMD/Intel) — skip any already covered by NVML
        if include_wmi {
            if let Some(wmi_gpus) = self.get_gpu_info_wmi() {
                for wmi_gpu in wmi_gpus {
                    let dominated = nvml_names.iter().any(|n| {
                        n.to_lowercase().contains(&wmi_gpu.name.to_lowercase())
                            || wmi_gpu.name.to_lowercase().contains(&n.to_lowercase())
                    });
                    if !dominated {
                        gpus.push(wmi_gpu);
                    }
                }
            }
        }

        gpus
    }

    #[cfg(target_os = "windows")]
    fn get_battery_wmi(&self) -> Option<wmi::WMIConnection> {
        let com = self.wmi_com.as_ref()?;
        wmi::WMIConnection::new(com.clone()).ok()
    }

    #[cfg(target_os = "windows")]
    fn get_gpu_info_wmi(&self) -> Option<Vec<GpuInfo>> {
        let com = self.wmi_com.as_ref()?;
        let wmi = wmi::WMIConnection::new(com.clone()).ok()?;

        let results: Vec<std::collections::HashMap<String, wmi::Variant>> = wmi
            .raw_query("SELECT Name, DriverVersion, VideoProcessor, AdapterRAM FROM Win32_VideoController")
            .ok()?;

        if results.is_empty() {
            return None;
        }

        let mut gpus = Vec::new();

        for gpu_entry in &results {
            let name = gpu_entry
                .get("Name")
                .and_then(|v| match v {
                    wmi::Variant::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "Unknown GPU".to_string());

            if name.contains("Microsoft Basic Display Adapter") || name.contains("Standard VGA") {
                continue;
            }

            let adapter_ram = gpu_entry.get("AdapterRAM").and_then(|v| match v {
                wmi::Variant::UI4(n) => Some(*n as u64),
                wmi::Variant::UI8(n) => Some(*n),
                wmi::Variant::I4(n) => Some(*n as u64),
                _ => None,
            });

            let mut utilization = 0.0;
            if let Some(ref engine_class) = self.wmi_gpu_engine_class {
                let q = format!("SELECT Name, UtilizationPercentage FROM {}", engine_class);
                if let Ok(perf_results) = wmi.raw_query::<std::collections::HashMap<String, wmi::Variant>>(&q) {
                    let mut max_util = 0u64;
                    for engine in perf_results {
                        if let Some(val) = engine.get("UtilizationPercentage") {
                            let u = match val {
                                wmi::Variant::UI1(n) => *n as u64,
                                wmi::Variant::UI2(n) => *n as u64,
                                wmi::Variant::UI4(n) => *n as u64,
                                wmi::Variant::UI8(n) => *n,
                                wmi::Variant::I1(n) => *n as u64,
                                wmi::Variant::I2(n) => *n as u64,
                                wmi::Variant::I4(n) => *n as u64,
                                wmi::Variant::I8(n) => *n as u64,
                                wmi::Variant::String(s) => s.parse().unwrap_or(0),
                                _ => 0,
                            };
                            if u > max_util {
                                max_util = u;
                            }
                        }
                    }
                    utilization = (max_util as f32).min(100.0);
                }
            }

            let mut memory_used = None;
            if let Some(ref mem_class) = self.wmi_gpu_memory_class {
                let q = format!("SELECT LocalUsage FROM {}", mem_class);
                if let Ok(mem_results) = wmi.raw_query::<std::collections::HashMap<String, wmi::Variant>>(&q) {
                    let mut total_used = 0u64;
                    for instance in mem_results {
                        if let Some(val) = instance.get("LocalUsage") {
                            let u = match val {
                                wmi::Variant::UI1(n) => *n as u64,
                                wmi::Variant::UI2(n) => *n as u64,
                                wmi::Variant::UI4(n) => *n as u64,
                                wmi::Variant::UI8(n) => *n,
                                wmi::Variant::I1(n) => *n as u64,
                                wmi::Variant::I2(n) => *n as u64,
                                wmi::Variant::I4(n) => *n as u64,
                                wmi::Variant::I8(n) => *n as u64,
                                wmi::Variant::String(s) => s.parse().unwrap_or(0),
                                _ => 0,
                            };
                            total_used = total_used.saturating_add(u);
                        }
                    }
                    if total_used > 0 {
                        memory_used = Some(total_used);
                    }
                }
            }

            gpus.push(GpuInfo {
                name,
                utilization,
                memory_used,
                memory_total: adapter_ram,
                temperature: None,
                clock_mhz: None,
                power_watts: None,
                fan_percent: None,
            });
        }

        if gpus.is_empty() {
            None
        } else {
            Some(gpus)
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_gpu_info(&self, _include_wmi: bool) -> Vec<GpuInfo> {
        Vec::new()
    }

    #[cfg(target_os = "windows")]
    fn get_cpu_temperature_wmi(&self) -> Option<f32> {
        let wmi = self.wmi_thermal.as_ref()?;

        let results = wmi
            .raw_query::<std::collections::HashMap<String, wmi::Variant>>(
                "SELECT CurrentTemperature FROM MSAcpi_ThermalZoneTemperature",
            )
            .ok()?;

        if results.is_empty() {
            return None;
        }

        // Temperature is in tenths of degrees Kelvin
        if let Some(val) = results[0].get("CurrentTemperature") {
            let temp_k_tenths = match val {
                wmi::Variant::UI4(n) => *n as f32,
                wmi::Variant::I4(n) => *n as f32,
                wmi::Variant::UI8(n) => *n as f32,
                _ => return None,
            };

            // Convert to Celsius: (K / 10) - 273.15. A value of 0 means the thermal zone
            // has no data (not absolute zero), so treat it as unavailable.
            if temp_k_tenths == 0.0 {
                return None;
            }
            let temp_c = (temp_k_tenths / 10.0) - 273.15;
            return Some(temp_c.round());
        }

        None
    }

    #[cfg(not(target_os = "windows"))]
    fn get_cpu_temperature_wmi(&self) -> Option<f32> {
        None
    }

    #[cfg(target_os = "windows")]
    /// One-time WMI queries for motherboard/BIOS/GPU-driver/OS-build details.
    /// Any failure returns `None` for the failed fields; WMI init failure returns all `None`.
    fn get_wmi_system_details() -> (Option<String>, Option<String>, Option<String>, Option<String>) {
        use std::rc::Rc;
        use wmi::{COMLibrary, Variant, WMIConnection};
        let com = match COMLibrary::new() {
            Ok(c) => Rc::new(c),
            Err(_) => return (None, None, None, None),
        };
        let wmi = match WMIConnection::new(com) {
            Ok(w) => w,
            Err(_) => return (None, None, None, None),
        };
        let one = |query: &str, field: &str| -> Option<String> {
            let rows: Vec<std::collections::HashMap<String, Variant>> = wmi.raw_query(query).ok()?;
            rows.first().and_then(|row| row.get(field)).and_then(|v| match v {
                Variant::String(s) => Some(s.clone()),
                _ => None,
            })
        };
        let motherboard = one("SELECT Manufacturer, Product FROM Win32_BaseBoard", "Manufacturer").map(|m| {
            if m.trim().is_empty() {
                "N/A".to_string()
            } else {
                m
            }
        });
        let bios_version = one("SELECT SMBIOSBIOSVersion FROM Win32_BIOS", "SMBIOSBIOSVersion");
        let gpu_driver = one("SELECT DriverVersion FROM Win32_VideoController", "DriverVersion");
        let os_build = one("SELECT BuildNumber FROM Win32_OperatingSystem", "BuildNumber");
        (motherboard, bios_version, gpu_driver, os_build)
    }

    #[cfg(not(target_os = "windows"))]
    fn get_wmi_system_details() -> (Option<String>, Option<String>, Option<String>, Option<String>) {
        (None, None, None, None)
    }

    fn get_disk_info(&self) -> Vec<DiskInfo> {
        self.disks
            .iter()
            .map(|disk| {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total.saturating_sub(available);
                let usage_percentage = if total > 0 {
                    (used as f64 / total as f64 * 100.0) as f32
                } else {
                    0.0
                };

                DiskInfo {
                    name: disk.name().to_string_lossy().to_string(),
                    mount_point: disk.mount_point().to_string_lossy().to_string(),
                    total_space: total,
                    available_space: available,
                    usage_percentage,
                    file_system: disk.file_system().to_string_lossy().to_string(),
                }
            })
            .collect()
    }

    fn get_disk_io(&mut self, _refresh_interval: u64) -> (f64, f64) {
        let elapsed = self.last_disk_update.elapsed();
        let (total_read, total_written) =
            self.sys
                .processes()
                .values()
                .fold((0u64, 0u64), |(read, written), process| {
                    let usage = process.disk_usage();
                    (
                        read.saturating_add(usage.read_bytes),
                        written.saturating_add(usage.written_bytes),
                    )
                });
        let read_rate = monitoring::rates::counter_rate(Some(self.previous_disk_totals.0), total_read, elapsed);
        let write_rate = monitoring::rates::counter_rate(Some(self.previous_disk_totals.1), total_written, elapsed);
        self.previous_disk_totals = (total_read, total_written);
        self.last_disk_update = Instant::now();
        (
            read_rate.value_per_second / 1024.0 / 1024.0,
            write_rate.value_per_second / 1024.0 / 1024.0,
        )
    }

    fn check_alerts(&self, settings: &AppSettings, data: &SystemData) -> Vec<AlertInfo> {
        let mut alerts = Vec::new();
        let timestamp = Local::now().format("%H:%M:%S").to_string();

        // CPU alert
        if settings.show_notifications && data.cpu_usage > settings.notification_cpu_threshold {
            alerts.push(AlertInfo {
                timestamp: timestamp.clone(),
                alert_type: AlertType::CpuHigh,
                message: format!("CPU usage is high: {:.1}%", data.cpu_usage),
                value: data.cpu_usage,
            });
        }

        // Memory alert
        if settings.show_notifications && data.memory_percentage > settings.notification_memory_threshold {
            alerts.push(AlertInfo {
                timestamp: timestamp.clone(),
                alert_type: AlertType::MemoryHigh,
                message: format!("Memory usage is high: {:.1}%", data.memory_percentage),
                value: data.memory_percentage,
            });
        }

        // GPU temperature alert
        if settings.show_notifications {
            for gpu in &data.gpu_info {
                if let Some(temp) = gpu.temperature {
                    if temp > settings.notification_temp_threshold {
                        alerts.push(AlertInfo {
                            timestamp: timestamp.clone(),
                            alert_type: AlertType::GpuTempHigh,
                            message: format!("GPU temperature is high: {}°C ({})", temp, gpu.name),
                            value: temp as f32,
                        });
                    }
                }
            }
        }

        // Disk space alerts
        for disk in &data.disk_info {
            if disk.usage_percentage > 90.0 {
                alerts.push(AlertInfo {
                    timestamp: timestamp.clone(),
                    alert_type: AlertType::DiskSpaceLow,
                    message: format!("Disk {} is almost full: {:.1}%", disk.name, disk.usage_percentage),
                    value: disk.usage_percentage,
                });
            }
        }

        // Startup High Impact alert
        if data.high_impact_startup_count > 0 {
            alerts.push(AlertInfo {
                timestamp: timestamp.clone(),
                alert_type: AlertType::StartupHighImpact,
                message: format!(
                    "{} startup item(s) have High impact on boot time",
                    data.high_impact_startup_count
                ),
                value: data.high_impact_startup_count as f32,
            });
        }

        alerts
    }

    fn get_network_info(&mut self) -> Vec<NetworkInfo> {
        let elapsed = self.last_network_update.elapsed();
        let mut current_totals = std::collections::HashMap::new();
        let network_info = self
            .networks
            .iter()
            .map(|(interface, data)| {
                let current = (data.received(), data.transmitted());
                let previous = self.previous_network_totals.get(interface).copied();
                current_totals.insert(interface.clone(), current);
                let received_rate = monitoring::rates::counter_rate(previous.map(|p| p.0), current.0, elapsed);
                let transmitted_rate = monitoring::rates::counter_rate(previous.map(|p| p.1), current.1, elapsed);
                NetworkInfo {
                    interface: interface.clone(),
                    received: current.0,
                    transmitted: current.1,
                    received_rate: received_rate.value_per_second / 1024.0 / 1024.0,
                    transmitted_rate: transmitted_rate.value_per_second / 1024.0 / 1024.0,
                }
            })
            .collect();
        self.previous_network_totals = current_totals;
        self.last_network_update = Instant::now();
        network_info
    }

    fn get_system_info(&self) -> SystemInfo {
        let (motherboard, bios_version, gpu_driver, os_build) = Self::get_wmi_system_details();
        SystemInfo {
            os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            uptime: System::uptime(),
            cpu_count: self.sys.cpus().len(),
            cpu_brand: self
                .sys
                .cpus()
                .first()
                .map(|cpu| cpu.brand().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            motherboard,
            bios_version,
            gpu_driver,
            os_build,
        }
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
    pub(crate) top_processes: Vec<ProcessInfo>,
    pub(crate) monitoring_paused: bool,
    pub(crate) selected_process_pid: Option<u32>,
    pub(crate) selected_process_details: Option<(u32, processes::ProcessDetails)>,
    pub(crate) disk_info: Vec<DiskInfo>,
    pub(crate) network_info: Vec<NetworkInfo>,
    pub(crate) system_info: SystemInfo,
    pub(crate) cpu_temperature: Option<f32>,
    pub(crate) last_update: String,
    pub(crate) cpu_history: monitoring::history::BoundedHistory<DataPoint>,
    pub(crate) memory_history: monitoring::history::BoundedHistory<DataPoint>,
    pub(crate) gpu_history: monitoring::history::BoundedHistory<DataPoint>,
    pub(crate) cpu_temp_history: monitoring::history::BoundedHistory<DataPoint>,
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
    pub(crate) selected_tab: Tab,
    pub(crate) services: Vec<services::ServiceInfo>,
    pub(crate) last_activity: Instant,
    pub(crate) telemetry_history_stats: std::collections::HashMap<String, telemetry::HistoryStats>,
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
            cpu_history: monitoring::history::BoundedHistory::new(60),
            memory_history: monitoring::history::BoundedHistory::new(60),
            gpu_history: monitoring::history::BoundedHistory::new(60),
            cpu_temp_history: monitoring::history::BoundedHistory::new(60),
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
            selected_tab: Tab::Overview,
            last_activity: Instant::now(),
            services: Vec::new(),
            telemetry_history_stats: std::collections::HashMap::new(),
            provider_status: std::collections::HashMap::new(),
        }
    }
}

pub(crate) struct SystemMonitorApp {
    pub(crate) data: Arc<Mutex<SystemData>>,
    pub(crate) app_channels: app::AppChannels,
    pub(crate) latest_snapshot: Option<monitoring::SystemSnapshot>,
    pub(crate) action_pending: bool,
    pub(crate) action_status: Option<String>,
    pub(crate) pending_action_plan: Option<app::actions::ActionPlan>,
    pub(crate) action_history: Vec<app::actions::ActionHistoryEntry>,
    pub(crate) show_action_history: bool,
    pub(crate) session_recorder: persistence::session::SessionRecorder,
    pub(crate) session_status: Option<String>,
    telemetry_commands: std::sync::mpsc::SyncSender<telemetry::HubCommand>,
    pub(crate) settings: AppSettings,
    pub(crate) shared_settings: Arc<Mutex<AppSettings>>,
    pub(crate) selected_tab: Tab,
    pub(crate) show_settings: bool,
    pub(crate) show_export: bool,
    pub(crate) show_alerts: bool,
    pub(crate) show_process_manager: bool,
    pub(crate) selected_process_pid: Option<u32>,
    pub(crate) details_pid: Option<u32>,
    pub(crate) kill_tree_pid: Option<u32>,
    pub(crate) pending_service_action: Option<services::ServiceAction>,
    pub(crate) service_search: String,
    pub(crate) service_state_filter: Option<String>,
    pub(crate) process_search: String,
    pub(crate) process_sort_column: ProcessSortColumn,
    pub(crate) process_sort_ascending: bool,
    pub(crate) show_export_csv: bool,
    pub(crate) updater: updater::Updater,
    pub(crate) update_info_share: Arc<Mutex<Option<updater::UpdateInfo>>>,
    pub(crate) show_update_notification: bool,
    pub(crate) update_check_time: Option<Instant>,
    pub(crate) ram_cleaner_state: RamCleanerState,
    pub(crate) startup_items: Vec<StartupItem>,
    pub(crate) startup_items_loaded: bool,
    pub(crate) startup_items_loading: bool,
    pub(crate) startup_items_share: Arc<Mutex<Option<Vec<StartupItem>>>>,
    pub(crate) startup_search: String,
    pub(crate) startup_sort: StartupSortColumn,
    pub(crate) startup_sort_ascending: bool,
    pub(crate) startup_filter_impact: Option<ImpactTier>,
    pub(crate) startup_filter_signed: Option<bool>,
    pub(crate) startup_filter_broken: bool,
    pub(crate) startup_show_confirm: Option<usize>,
    pub(crate) boot_diagnostics: Option<BootDiagnostics>,
    pub(crate) boot_diagnostics_loaded: bool,
    pub(crate) boot_diagnostics_share: Arc<Mutex<Option<BootDiagnostics>>>,
    pub(crate) show_shortcuts: bool,
    pub(crate) suspend_process_pid: Option<u32>,
    pub(crate) resume_process_pid: Option<u32>,
    pub(crate) suspended_pids: std::collections::HashSet<u32>,
    pub(crate) priority_change: Option<(u32, String)>,
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
    pub(crate) is_hidden: bool,
    pub(crate) widget_open: bool,
    /// Whether we have already applied the start_minimized setting on the first frame.
    pub(crate) start_minimized_applied: bool,
}

#[derive(Clone, Copy, PartialEq)]
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
    About,
}

impl SystemMonitorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Install image loaders for showing the logo
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Load settings
        let settings = AppSettings::load();

        // Load Windows system fonts at runtime to support all standard symbols and checkmarks
        #[cfg(target_os = "windows")]
        {
            let mut fonts = egui::FontDefinitions::default();
            let mut proportional_loaded = false;
            let mut monospace_loaded = false;

            // Load Segoe UI for standard proportional text
            let font_paths = ["C:\\Windows\\Fonts\\segoeui.ttf", "C:\\Windows\\Fonts\\SegoeUI.ttf"];
            for path in &font_paths {
                if let Ok(font_bytes) = std::fs::read(path) {
                    fonts
                        .font_data
                        .insert("segoe_ui".to_owned(), egui::FontData::from_owned(font_bytes));
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .insert(0, "segoe_ui".to_owned());
                    proportional_loaded = true;
                    break;
                }
            }

            // Load Consolas for monospace text
            let mono_paths = ["C:\\Windows\\Fonts\\consola.ttf", "C:\\Windows\\Fonts\\Consola.ttf"];
            for path in &mono_paths {
                if let Ok(font_bytes) = std::fs::read(path) {
                    fonts
                        .font_data
                        .insert("consolas".to_owned(), egui::FontData::from_owned(font_bytes));
                    fonts
                        .families
                        .entry(egui::FontFamily::Monospace)
                        .or_default()
                        .insert(0, "consolas".to_owned());
                    monospace_loaded = true;
                    break;
                }
            }

            if proportional_loaded || monospace_loaded {
                cc.egui_ctx.set_fonts(fonts);
            }
        }

        // Configure fonts and style
        let mut style = (*cc.egui_ctx.style()).clone();

        // Premium spacing
        style.spacing.item_spacing = egui::vec2(16.0, 12.0);
        style.spacing.button_padding = egui::vec2(16.0, 10.0);
        style.spacing.interact_size = egui::vec2(32.0, 32.0); // Touch target minimums
        style.spacing.window_margin = egui::Margin::same(20.0);
        style.spacing.menu_margin = egui::Margin::same(12.0);

        // Typographic hierarchy (slightly larger for premium feel)
        use egui::{FontFamily, FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Heading, FontId::new(24.0, FontFamily::Proportional)),
            (
                TextStyle::Name("Subheading".into()),
                FontId::new(18.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
            (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        ]
        .into();

        // Apply theme — custom "Terminal Noir" / "Midnight Indigo" dark or standard light
        if settings.theme_dark {
            let mut visuals = egui::Visuals::dark();
            // Deep charcoal backgrounds
            visuals.panel_fill = ThemePalette::BG_DEEP;
            visuals.window_fill = ThemePalette::BG_SURFACE;
            visuals.extreme_bg_color = ThemePalette::BG_DEEPEST;

            // Accent for selections and interactions
            visuals.selection.bg_fill = ThemePalette::ACCENT_PRIMARY;
            visuals.selection.stroke = egui::Stroke::NONE;
            visuals.hyperlink_color = ThemePalette::ACCENT_PRIMARY;

            // Subtle borders & widgets
            visuals.widgets.noninteractive.bg_fill = ThemePalette::BG_CARD;
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_PRIMARY);

            // Inactive
            visuals.widgets.inactive.bg_fill = ThemePalette::WIDGET_INACTIVE;
            visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SECONDARY);

            // Hovered
            visuals.widgets.hovered.bg_fill = ThemePalette::WIDGET_HOVERED;
            visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER_LIGHT);
            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);

            // Active
            visuals.widgets.active.bg_fill = ThemePalette::ACCENT_ACTIVE;
            visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);

            // Rounding (Terminal Noir Minimal 4px)
            visuals.window_rounding = egui::Rounding::same(4.0);
            visuals.menu_rounding = egui::Rounding::same(4.0);
            visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
            visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
            visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
            visuals.widgets.active.rounding = egui::Rounding::same(4.0);

            // Window chrome and depth
            visuals.window_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
            visuals.window_shadow = egui::epaint::Shadow {
                offset: egui::vec2(0.0, 4.0),
                blur: 16.0,
                spread: 0.0,
                color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 40),
            };

            visuals.popup_shadow = egui::epaint::Shadow {
                offset: egui::vec2(0.0, 8.0),
                blur: 40.0,
                spread: 0.0,
                color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 20),
            };

            cc.egui_ctx.set_visuals(visuals);
        } else {
            let mut visuals = egui::Visuals::light();
            // Clean, Apple-like light theme backgrounds
            visuals.panel_fill = egui::Color32::from_rgb(245, 245, 247);
            visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
            visuals.extreme_bg_color = egui::Color32::from_rgb(235, 235, 240);

            // Accent overrides
            visuals.selection.bg_fill = ThemePalette::ACCENT_PRIMARY;
            visuals.selection.stroke = egui::Stroke::NONE;

            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(250, 250, 250);
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 220, 225));
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 45));

            visuals.window_rounding = egui::Rounding::same(8.0);
            visuals.menu_rounding = egui::Rounding::same(8.0);

            cc.egui_ctx.set_visuals(visuals);
        }

        cc.egui_ctx.set_style(style);

        let data = Arc::new(Mutex::new(SystemData::default()));
        let data_clone = Arc::clone(&data);
        let shared_settings = Arc::new(Mutex::new(settings.clone()));
        let shared_settings_clone = Arc::clone(&shared_settings);
        let mut app_channels = app::AppChannels::new();
        let monitoring_receiver = app_channels
            .monitoring_receiver
            .take()
            .expect("monitoring receiver missing");
        let action_receiver = app_channels.action_receiver.take().expect("action receiver missing");
        let action_events = app_channels.event_sender.clone();
        let monitoring_events = app_channels.event_sender.clone();

        thread::Builder::new()
            .name("actions".to_string())
            .spawn(move || run_action_worker(action_receiver, action_events))
            .expect("failed to spawn action worker");

        let (mut telemetry_hub, mut telemetry_reader, telemetry_commands) = telemetry::TelemetryHub::new();
        telemetry_hub.add_provider(Box::new(providers::sysinfo_provider::SysinfoProvider::new()));
        telemetry_hub.add_provider(Box::new(providers::nvml_provider::NvmlProvider::new()));
        telemetry_hub.add_provider(Box::new(providers::wmi_provider::WmiProvider::new()));
        telemetry_hub.add_provider(Box::new(providers::windows_gpu_provider::WindowsGpuProvider::new()));
        thread::Builder::new()
            .name("telemetry_hub".to_string())
            .spawn(move || telemetry_hub.run())
            .expect("failed to spawn telemetry hub");
        let telemetry_commands_for_monitor = telemetry_commands.clone();

        // Background thread for monitoring
        thread::Builder::new()
            .name("monitoring".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let mut monitor = SystemMonitor::new();

                // Get system info once (doesn't change)
                let system_info = monitor.get_system_info();
                let mut battery_check_counter: u32 = 0;
                let mut temperature_check_counter: u32 = 0;
                let mut service_check_counter: u32 = 0;
                let mut last_alert_time: std::collections::HashMap<AlertType, Instant> =
                    std::collections::HashMap::new();
                let mut last_hidden_tick = Instant::now();
                let mut last_selected_tab = data_clone.lock().selected_tab;
                let mut latest_telemetry = telemetry::TelemetrySnapshot::default();

                loop {
                    let mut force_refresh = false;
                    while let Ok(command) = monitoring_receiver.try_recv() {
                        match command {
                            app::commands::MonitoringCommand::SetSettings(new_settings) => {
                                *shared_settings_clone.lock() = *new_settings;
                            }
                            app::commands::MonitoringCommand::SetPaused(paused) => {
                                data_clone.lock().monitoring_paused = paused
                            }
                            app::commands::MonitoringCommand::SetHidden(hidden) => {
                                data_clone.lock().is_hidden = hidden;
                                let _ = telemetry_commands_for_monitor
                                    .try_send(telemetry::HubCommand::SetBackgroundMode(hidden));
                            }
                            app::commands::MonitoringCommand::RefreshNow => {
                                force_refresh = true;
                                let _ = telemetry_commands_for_monitor.try_send(telemetry::HubCommand::ForceRefresh);
                            }
                            app::commands::MonitoringCommand::Shutdown => {
                                let _ = telemetry_commands_for_monitor.try_send(telemetry::HubCommand::Shutdown);
                                return;
                            }
                        }
                    }
                    if let Some(snapshot) = telemetry_reader.latest_if_updated() {
                        latest_telemetry = snapshot;
                    }
                    if !force_refresh {
                        thread::sleep(Duration::from_millis(500));
                    }

                    // Read hidden status, current tab, and pause state
                    let (is_hidden, selected_tab, paused) = {
                        let data = data_clone.lock();
                        (data.is_hidden, data.selected_tab, data.monitoring_paused)
                    };

                    // Read current settings from shared state
                    let (refresh_interval, process_count, settings_snapshot) = {
                        let s = shared_settings_clone.lock();
                        (s.refresh_interval, s.process_count, s.clone())
                    };

                    let is_minimized_tick = is_hidden && last_hidden_tick.elapsed().as_secs() < 10;

                    if is_minimized_tick {
                        continue;
                    }

                    if paused {
                        continue;
                    }

                    if is_hidden {
                        last_hidden_tick = Instant::now();
                    }

                    // Rich process, disk and network details still use sysinfo's
                    // native structures; core CPU/RAM/GPU values come from the hub.
                    if !is_hidden {
                        monitor.sys.refresh_processes();
                        monitor.disks.refresh();
                        monitor.networks.refresh();
                    }

                    let fallback_memory = monitor.get_memory_info();
                    let total_mem = latest_telemetry
                        .metrics
                        .get("memory.total")
                        .copied()
                        .map(|value| value as u64)
                        .unwrap_or(fallback_memory.0);
                    let used_mem = latest_telemetry
                        .metrics
                        .get("memory.used")
                        .copied()
                        .map(|value| value as u64)
                        .unwrap_or(fallback_memory.1);
                    let mem_percentage = if total_mem == 0 {
                        0.0
                    } else {
                        used_mem as f32 / total_mem as f32 * 100.0
                    };
                    let cpu_usage = latest_telemetry
                        .metrics
                        .get("cpu.global_usage")
                        .copied()
                        .map(|value| value as f32)
                        .unwrap_or_else(|| monitor.get_cpu_usage());

                    // Optimized queries
                    let need_cpu_cores = !is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::CpuCores);
                    let need_cpu_temp = !is_hidden && selected_tab == Tab::Overview;
                    let need_gpu_wmi =
                        !is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::Performance);
                    let need_gpu_info =
                        need_gpu_wmi || settings_snapshot.show_notifications || settings_snapshot.show_graphs;
                    // Fetch processes for both Processes tab (all) and Overview tab (top N summary)
                    let need_processes =
                        !is_hidden && (selected_tab == Tab::Processes || selected_tab == Tab::Overview);
                    let need_disks = (!is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::Storage))
                        || settings_snapshot.show_notifications
                        || settings_snapshot.show_graphs;
                    let need_network = !is_hidden
                        && (selected_tab == Tab::Overview
                            || selected_tab == Tab::Network
                            || selected_tab == Tab::Performance);

                    let cpu_cores = if need_cpu_cores {
                        let cores = cpu_cores_from_telemetry(&latest_telemetry);
                        if cores.is_empty() {
                            monitor.get_cpu_cores_info()
                        } else {
                            cores
                        }
                    } else {
                        Vec::new()
                    };

                    let cpu_temperature = if need_cpu_temp && temperature_check_counter % 10 == 0 {
                        monitor.get_cpu_temperature_wmi()
                    } else {
                        None
                    };
                    temperature_check_counter = temperature_check_counter.wrapping_add(1);

                    let gpu_info = if need_gpu_info {
                        let hub_gpus = gpus_from_telemetry(&latest_telemetry);
                        if hub_gpus.is_empty() {
                            monitor.get_gpu_info(need_gpu_wmi)
                        } else {
                            hub_gpus
                        }
                    } else {
                        Vec::new()
                    };

                    let top_processes = if need_processes {
                        // On Processes tab, fetch ALL processes so search/sort works on the full list.
                        // On Overview tab, only fetch the top N by memory for the summary panel.
                        let fetch_count = if selected_tab == Tab::Processes {
                            usize::MAX
                        } else {
                            process_count
                        };
                        monitor.get_top_processes(fetch_count)
                    } else {
                        Vec::new()
                    };

                    let disk_info = if need_disks {
                        monitor.get_disk_info()
                    } else {
                        Vec::new()
                    };

                    let network_info = if need_network {
                        monitor.get_network_info()
                    } else {
                        Vec::new()
                    };

                    let swap_total = latest_telemetry
                        .metrics
                        .get("memory.total_swap")
                        .copied()
                        .map(|v| v as u64);
                    let swap_used = latest_telemetry
                        .metrics
                        .get("memory.used_swap")
                        .copied()
                        .map(|v| v as u64);
                    let swap_info = match (swap_total, swap_used) {
                        (Some(total), Some(used)) => SwapInfo {
                            total,
                            used,
                            percentage: if total == 0 {
                                0.0
                            } else {
                                used as f32 / total as f32 * 100.0
                            },
                        },
                        _ => monitor.get_swap_info(),
                    };

                    let (disk_read_rate, disk_write_rate) = if !is_hidden {
                        monitor.get_disk_io(refresh_interval)
                    } else {
                        (0.0, 0.0)
                    };

                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

                    // Get battery info every 15 ticks (~7.5s) — retain previous value if unavailable
                    if battery_check_counter % 15 == 0 {
                        let mut bi = None;
                        if let Some(wmi_con) = monitor.get_battery_wmi() {
                            bi = get_battery_info(&wmi_con);
                        }
                        if let Some(bi) = bi {
                            let mut data = data_clone.lock();
                            data.battery_info = Some(bi);
                        }
                    }
                    battery_check_counter = battery_check_counter.wrapping_add(1);

                    // Poll services every 60 ticks (~30s) — WMI queries are expensive
                    if !is_hidden && selected_tab == Tab::Services {
                        let services_list = if last_selected_tab != Tab::Services
                            || data_clone.lock().services.is_empty()
                            || service_check_counter % 60 == 0
                        {
                            if let Some(ref com) = monitor.wmi_com {
                                services::get_services_with_com(Some(com))
                            } else {
                                services::get_services()
                            }
                        } else {
                            Vec::new()
                        };
                        if !services_list.is_empty() {
                            let mut data = data_clone.lock();
                            data.services = services_list;
                        }
                    }
                    service_check_counter = service_check_counter.wrapping_add(1);
                    last_selected_tab = selected_tab;

                    // Calculate total network rates
                    let total_download_rate: f64 = network_info.iter().map(|n| n.received_rate).sum();
                    let total_upload_rate: f64 = network_info.iter().map(|n| n.transmitted_rate).sum();

                    {
                        let mut data = data_clone.lock();
                        let elapsed = data.start_time.elapsed().as_secs_f64();

                        // Update current values
                        data.memory_total = total_mem;
                        data.memory_used = used_mem;
                        data.memory_percentage = mem_percentage;
                        data.cpu_usage = cpu_usage;
                        if need_cpu_cores {
                            data.cpu_cores = cpu_cores;
                        }
                        if need_cpu_temp {
                            data.cpu_temperature = cpu_temperature;
                        }
                        if need_gpu_info {
                            data.gpu_info = gpu_info.clone();
                        }
                        if need_processes {
                            data.top_processes = top_processes;
                        }
                        if need_disks {
                            data.disk_info = disk_info;
                        }
                        if need_network {
                            data.network_info = network_info;
                        }
                        data.system_info = system_info.clone();
                        data.last_update = timestamp;
                        data.swap_info = swap_info;
                        if !is_hidden {
                            data.disk_read_rate = disk_read_rate;
                            data.disk_write_rate = disk_write_rate;
                        }
                        data.network_sample_count += 1;
                        data.telemetry_history_stats = latest_telemetry.history_stats.clone();
                        data.provider_status = latest_telemetry.provider_status.clone();

                        // Check for alerts
                        let mut new_alerts = monitor.check_alerts(&settings_snapshot, &data);
                        let active_keys: std::collections::HashSet<String> =
                            data.alerts.iter().map(AlertInfo::key).collect();
                        new_alerts.retain(|alert| !active_keys.contains(&alert.key()));

                        if !new_alerts.is_empty() && settings_snapshot.enable_sounds {
                            play_alert_sound();
                        }

                        // Send desktop notifications for new alerts with a 5-minute cooldown
                        if settings_snapshot.show_notifications {
                            for alert in &new_alerts {
                                let now = Instant::now();
                                let should_notify = last_alert_time
                                    .get(&alert.alert_type)
                                    .is_none_or(|&last| now.duration_since(last).as_secs() > 300);

                                if should_notify {
                                    let _ = notify_rust::Notification::new()
                                        .summary("System Monitor Alert")
                                        .body(&alert.message)
                                        .timeout(notify_rust::Timeout::Milliseconds(5000))
                                        .show();
                                    last_alert_time.insert(alert.alert_type.clone(), now);
                                }
                            }
                        }

                        data.alerts.extend(new_alerts);

                        // Auto-clear resolved alerts
                        if settings_snapshot.auto_clear_alerts {
                            let temp_gpu_info = data.gpu_info.clone();
                            let high_impact_count = data.high_impact_startup_count;
                            let disk_alert_active = data.disk_info.iter().any(|disk| disk.usage_percentage > 90.0);
                            data.alerts.retain(|alert| match alert.alert_type {
                                AlertType::CpuHigh => cpu_usage > settings_snapshot.notification_cpu_threshold,
                                AlertType::MemoryHigh => {
                                    mem_percentage > settings_snapshot.notification_memory_threshold
                                }
                                AlertType::GpuTempHigh => temp_gpu_info.iter().any(|gpu| {
                                    gpu.temperature.is_some_and(|temperature| {
                                        temperature > settings_snapshot.notification_temp_threshold
                                    })
                                }),
                                AlertType::DiskSpaceLow => disk_alert_active,
                                AlertType::StartupHighImpact => high_impact_count > 0,
                            });
                        }

                        // Keep only last 10 alerts
                        while data.alerts.len() > 10 {
                            data.alerts.remove(0);
                        }
                        // Update history (keep last 60 data points)
                        data.cpu_history.push(DataPoint {
                            time: elapsed,
                            value: cpu_usage as f64,
                        });
                        data.memory_history.push(DataPoint {
                            time: elapsed,
                            value: mem_percentage as f64,
                        });

                        if need_gpu_info {
                            let gpu_util = data.gpu_info.first().map(|gpu| gpu.utilization as f64);
                            if let Some(val) = gpu_util {
                                data.gpu_history.push(DataPoint {
                                    time: elapsed,
                                    value: val,
                                });
                            }
                        }

                        // Network history — skip first sample (inflated rates)
                        if need_network && data.network_sample_count > 1 {
                            data.network_download_history.push_back(DataPoint {
                                time: elapsed,
                                value: total_download_rate,
                            });
                            data.network_upload_history.push_back(DataPoint {
                                time: elapsed,
                                value: total_upload_rate,
                            });
                        }
                        if !is_hidden && data.network_sample_count > 1 {
                            data.disk_read_history.push_back(DataPoint {
                                time: elapsed,
                                value: disk_read_rate,
                            });
                            data.disk_write_history.push_back(DataPoint {
                                time: elapsed,
                                value: disk_write_rate,
                            });
                        }

                        // cpu_history capped at 60 by BoundedHistory; trim the rest
                        while data.network_download_history.len() > 60 {
                            data.network_download_history.pop_front();
                        }
                        while data.network_upload_history.len() > 60 {
                            data.network_upload_history.pop_front();
                        }
                        while data.disk_read_history.len() > 60 {
                            data.disk_read_history.pop_front();
                        }
                        while data.disk_write_history.len() > 60 {
                            data.disk_write_history.pop_front();
                        }
                    }

                    let snapshot = snapshot_from_data(&data_clone.lock());
                    let _ = monitoring_events.send(app::events::AppEvent::Snapshot(Box::new(snapshot)));

                    // Process details for the selected row (recompute only when selection changed)
                    let selected_pid = {
                        let d = data_clone.lock();
                        d.selected_process_pid
                    };
                    if let Some(pid) = selected_pid {
                        let cached = {
                            let d = data_clone.lock();
                            d.selected_process_details.as_ref().map(|(p, _)| *p)
                        };
                        if cached != Some(pid) {
                            if let Some(details) = processes::lookup_details(&monitor.sys, pid) {
                                let mut d = data_clone.lock();
                                d.selected_process_details = Some((pid, details));
                            }
                        }
                    }

                    if is_hidden {
                        // Minimized: sleep 10s
                        thread::sleep(Duration::from_millis(10000));
                    } else {
                        let sleep_ms = (refresh_interval * 1000).saturating_sub(500);
                        thread::sleep(Duration::from_millis(sleep_ms));
                    }
                }
            })
            .expect("failed to spawn monitoring thread");

        let mut tray_icon = None;
        let mut tray_menu_show_id = None;
        let mut tray_menu_quit_id = None;
        let mut tray_menu_clean_id = None;
        let mut tray_menu_procman_id = None;
        let mut tray_menu_pause_id = None;
        let mut tray_menu_pause_item = None;
        let mut tray_menu_handle = None;
        let mut tray_menu_power_item = None;
        let mut tray_menu_power_items: std::collections::HashMap<_, _> = Default::default();
        let mut tray_menu_power_guids: std::collections::HashMap<_, _> = Default::default();

        #[cfg(target_os = "windows")]
        if let Some(icon) = load_tray_icon() {
            let tray_menu = Menu::new();
            let show_i = MenuItem::new("Show Dashboard", true, None);
            let clean_i = MenuItem::new("Clean RAM Now", true, None);
            let procman_i = MenuItem::new("Open Process Manager", true, None);
            let pause_i = CheckMenuItem::new("Pause Monitoring", true, false, None);
            let quit_i = MenuItem::new("Quit System Monitor", true, None);

            tray_menu_show_id = Some(show_i.id().clone());
            tray_menu_clean_id = Some(clean_i.id().clone());
            tray_menu_procman_id = Some(procman_i.id().clone());
            tray_menu_pause_id = Some(pause_i.id().clone());
            tray_menu_quit_id = Some(quit_i.id().clone());
            let pause_item = pause_i.clone();
            let menu_handle = tray_menu.clone();

            let power_plans = power::get_power_plans();
            if !power_plans.is_empty() {
                let mut owned_power_items: Vec<CheckMenuItem> = Vec::new();
                for plan in &power_plans {
                    let item = CheckMenuItem::new(plan.name.clone(), true, plan.is_active, None);
                    tray_menu_power_guids.insert(item.id().clone(), plan.guid.clone());
                    owned_power_items.push(item);
                }
                let power_submenu = {
                    let refs: Vec<&dyn tray_icon::menu::IsMenuItem> = owned_power_items
                        .iter()
                        .map(|item| item as &dyn tray_icon::menu::IsMenuItem)
                        .collect();
                    Submenu::with_items("Power Plan", true, &refs).expect("failed to build power plan submenu")
                };
                tray_menu_power_item = Some(power_submenu);
                for item in owned_power_items {
                    tray_menu_power_items.insert(item.id().clone(), item);
                }
                let _ = tray_menu.append(tray_menu_power_item.as_ref().expect("power submenu built"));
            }

            let _ = tray_menu.append_items(&[&show_i, &clean_i, &procman_i, &pause_i, &quit_i]);

            if let Ok(tray) = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("System Monitor")
                .with_icon(icon)
                .build()
            {
                tray_icon = Some(tray);
            }
            tray_menu_pause_item = Some(pause_item);
            tray_menu_handle = Some(menu_handle);
        }

        let startup_items_share = Arc::new(Mutex::new(None));
        let boot_diagnostics_share = Arc::new(Mutex::new(None));

        let startup_share_clone = Arc::clone(&startup_items_share);
        let boot_share_clone = Arc::clone(&boot_diagnostics_share);
        let ctx_clone = cc.egui_ctx.clone();
        std::thread::Builder::new()
            .name("startup_loader".to_string())
            .spawn(move || {
                let items = crate::startup::get_startup_items();
                *startup_share_clone.lock() = Some(items);
                if let Some(diag) = crate::startup::get_boot_diagnostics() {
                    *boot_share_clone.lock() = Some(diag);
                }
                ctx_clone.request_repaint();
            })
            .ok();

        Self {
            app_channels,
            latest_snapshot: None,
            action_pending: false,
            action_status: None,
            pending_action_plan: None,
            action_history: persistence::action_log::load_recent(100)
                .into_iter()
                .map(|record| app::actions::ActionHistoryEntry { record, undo: None })
                .collect(),
            show_action_history: false,
            session_recorder: persistence::session::SessionRecorder::default(),
            session_status: None,
            telemetry_commands,
            data,
            settings: settings.clone(),
            shared_settings,
            selected_tab: Tab::Overview,
            show_settings: false,
            show_export: false,
            show_alerts: false,
            show_process_manager: false,
            selected_process_pid: None,
            details_pid: None,
            kill_tree_pid: None,
            process_search: String::new(),
            process_sort_column: ProcessSortColumn::Memory,
            process_sort_ascending: false,
            show_export_csv: false,
            updater: updater::Updater::new(),
            update_info_share: Arc::new(Mutex::new(None)),
            show_update_notification: true,
            update_check_time: None,
            ram_cleaner_state: RamCleanerState {
                last_cleaned: None,
                last_cleaned_display: String::new(),
                bytes_freed: 0,
                auto_clean_enabled: settings.auto_ram_clean,
                auto_clean_threshold: settings.ram_clean_threshold,
                auto_clean_interval: settings.auto_clean_interval,
                auto_clean_target: settings.auto_clean_target,
                auto_clean_exclusions: settings.auto_clean_exclusions.clone(),
                auto_clean_idle_only: settings.auto_clean_idle_only,
                auto_clean_notify: settings.auto_clean_notify,
                auto_clean_max_mb: settings.auto_clean_max_mb,
                is_cleaning: false,
                clean_count: 0,
            },
            startup_items: Vec::new(),
            startup_items_loaded: false,
            startup_items_loading: true,
            startup_items_share,
            startup_search: String::new(),
            startup_sort: StartupSortColumn::Impact,
            startup_sort_ascending: true,
            startup_filter_impact: None,
            startup_filter_signed: None,
            startup_filter_broken: false,
            startup_show_confirm: None,
            boot_diagnostics: None,
            boot_diagnostics_loaded: false,
            boot_diagnostics_share,
            show_shortcuts: false,
            suspend_process_pid: None,
            resume_process_pid: None,
            suspended_pids: std::collections::HashSet::new(),
            priority_change: None,
            service_search: String::new(),
            service_state_filter: None,
            pending_service_action: None,
            #[cfg(target_os = "windows")]
            tray_icon,
            #[cfg(target_os = "windows")]
            tray_menu_show_id,
            #[cfg(target_os = "windows")]
            tray_menu_quit_id,
            #[cfg(target_os = "windows")]
            tray_menu_clean_id,
            #[cfg(target_os = "windows")]
            tray_menu_procman_id,
            #[cfg(target_os = "windows")]
            tray_menu_pause_id,
            #[cfg(target_os = "windows")]
            tray_menu_pause_item,
            #[cfg(target_os = "windows")]
            tray_menu_handle,
            #[cfg(target_os = "windows")]
            tray_menu_power_item,
            #[cfg(target_os = "windows")]
            tray_menu_power_items,
            #[cfg(target_os = "windows")]
            tray_menu_power_guids,
            is_hidden: false,
            widget_open: settings.show_widget,
            start_minimized_applied: false,
        }
    }

    pub(crate) fn export_diagnostics(
        &self,
        destination: &std::path::Path,
    ) -> Result<std::path::PathBuf, std::io::Error> {
        let snapshot = self
            .latest_snapshot
            .as_ref()
            .cloned()
            .unwrap_or_else(|| snapshot_from_data(&self.data.lock()));
        persistence::diagnostics::export(destination, &snapshot, &self.settings)
    }

    fn export_to_csv(&self, data: &SystemData) -> Result<String, Box<dyn std::error::Error>> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Header
        wtr.write_record(["Category", "Metric", "Value"])?;

        // System info
        wtr.write_record(["System", "Timestamp", &data.last_update])?;
        wtr.write_record(["CPU", "Usage %", &format!("{:.2}", data.cpu_usage)])?;
        wtr.write_record(["Memory", "Total GB", &format!("{:.2}", bytes_to_gb(data.memory_total))])?;
        wtr.write_record(["Memory", "Used GB", &format!("{:.2}", bytes_to_gb(data.memory_used))])?;
        wtr.write_record(["Memory", "Usage %", &format!("{:.2}", data.memory_percentage)])?;

        // GPU
        for gpu in &data.gpu_info {
            wtr.write_record(["GPU", "Name", &gpu.name])?;
            wtr.write_record(["GPU", "Usage %", &format!("{:.2}", gpu.utilization)])?;
            if let Some(temp) = gpu.temperature {
                wtr.write_record(["GPU", "Temperature C", &format!("{}", temp)])?;
            }
            if let Some(clock) = gpu.clock_mhz {
                wtr.write_record(["GPU", "Clock MHz", &clock.to_string()])?;
            }
            if let Some(power) = gpu.power_watts {
                wtr.write_record(["GPU", "Power W", &format!("{:.1}", power)])?;
            }
            if let Some(fan) = gpu.fan_percent {
                wtr.write_record(["GPU", "Fan %", &fan.to_string()])?;
            }
        }

        // Top processes header
        wtr.write_record(["", "", ""])?; // Empty line
        wtr.write_record(["Process PID", "Name", "Memory MB", "CPU %"])?;
        for proc in &data.top_processes {
            wtr.write_record([
                &proc.pid.to_string(),
                &proc.name,
                &format!("{:.2}", bytes_to_mb(proc.memory)),
                &format!("{:.2}", proc.cpu_usage),
            ])?;
        }

        let csv_data = String::from_utf8(wtr.into_inner()?)?;
        Ok(csv_data)
    }

    fn export_data_to_json(&self, data: &SystemData) -> Result<String, Box<dyn std::error::Error>> {
        #[derive(Serialize)]
        struct ExportData {
            timestamp: String,
            cpu_usage: f32,
            memory_used: u64,
            memory_total: u64,
            memory_percentage: f32,
            gpu_info: Option<GpuInfo>,
            top_processes: Vec<ProcessInfo>,
            disk_info: Vec<DiskInfo>,
            network_info: Vec<NetworkInfo>,
            system_info: SystemInfo,
            startup_item_count: usize,
            high_impact_startup_count: usize,
            boot_diagnostics: Option<BootDiagnostics>,
            swap_info: SwapInfo,
            battery_info: Option<BatteryInfo>,
            network_download_history: Vec<DataPoint>,
            network_upload_history: Vec<DataPoint>,
            disk_read_history: Vec<DataPoint>,
            disk_write_history: Vec<DataPoint>,
        }

        let export = ExportData {
            timestamp: data.last_update.clone(),
            cpu_usage: data.cpu_usage,
            memory_used: data.memory_used,
            memory_total: data.memory_total,
            memory_percentage: data.memory_percentage,
            gpu_info: data.gpu_info.first().cloned(),
            top_processes: data.top_processes.clone(),
            disk_info: data.disk_info.clone(),
            network_info: data.network_info.clone(),
            system_info: data.system_info.clone(),
            startup_item_count: self.startup_items.len(),
            high_impact_startup_count: startup::high_impact_count(&self.startup_items),
            boot_diagnostics: self.boot_diagnostics.clone(),
            swap_info: data.swap_info.clone(),
            battery_info: data.battery_info.clone(),
            network_download_history: data.network_download_history.iter().copied().collect(),
            network_upload_history: data.network_upload_history.iter().copied().collect(),
            disk_read_history: data.disk_read_history.iter().copied().collect(),
            disk_write_history: data.disk_write_history.iter().copied().collect(),
        };

        Ok(serde_json::to_string_pretty(&export)?)
    }
}
impl eframe::App for SystemMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let ctx_clone = ctx.clone();
        self.data.lock().last_activity = Instant::now();
        while let Ok(event) = self.app_channels.event_receiver.try_recv() {
            match event {
                app::events::AppEvent::Snapshot(snapshot) => {
                    if let Err(error) = self.session_recorder.record(&snapshot) {
                        self.session_status = Some(format!("Session recording failed: {error}"));
                    }
                    self.latest_snapshot = Some(*snapshot);
                }
                app::events::AppEvent::AuditRecorded(record) => {
                    self.action_history
                        .push(app::actions::ActionHistoryEntry { record, undo: None });
                }
                app::events::AppEvent::ActionCompleted { command, record, undo } => {
                    self.action_pending = false;
                    self.action_status = Some(record.message.clone());
                    if matches!(command, app::commands::ActionCommand::CleanRam) {
                        self.ram_cleaner_state.is_cleaning = false;
                        self.ram_cleaner_state.last_cleaned = Some(Instant::now());
                        self.ram_cleaner_state.last_cleaned_display = Local::now().format("%H:%M:%S").to_string();
                        self.ram_cleaner_state.clean_count += 1;
                        if let Some(bytes) = record
                            .message
                            .strip_prefix("Freed ")
                            .and_then(|value| value.split_whitespace().next())
                            .and_then(|value| value.parse::<u64>().ok())
                        {
                            self.ram_cleaner_state.bytes_freed =
                                self.ram_cleaner_state.bytes_freed.saturating_add(bytes);
                            let mut data = self.data.lock();
                            data.ram_clean_freed_bytes = data.ram_clean_freed_bytes.saturating_add(bytes);
                        }
                    }
                    self.action_history
                        .push(app::actions::ActionHistoryEntry { record, undo });
                }
                app::events::AppEvent::ActionFailed { command, record } => {
                    self.action_pending = false;
                    self.action_status = Some(record.message.clone());
                    if matches!(command, app::commands::ActionCommand::CleanRam) {
                        self.ram_cleaner_state.is_cleaning = false;
                    }
                    self.action_history
                        .push(app::actions::ActionHistoryEntry { record, undo: None });
                    if self.settings.enable_sounds {
                        play_alert_sound();
                    }
                }
            }
        }
        {
            let mut data = self.data.lock();
            data.is_hidden = self.is_hidden;
            data.selected_tab = self.selected_tab;
            if let Some(items) = &*self.startup_items_share.lock() {
                data.high_impact_startup_count = startup::high_impact_count(items);
            }
        }
        // Apply start_minimized on the very first frame
        if !self.start_minimized_applied {
            self.start_minimized_applied = true;
            if self.settings.start_minimized {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                if self.settings.minimize_to_tray {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    self.is_hidden = true;
                }
            }
        }

        #[cfg(target_os = "windows")]
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if Some(&event.id) == self.tray_menu_quit_id.as_ref() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if Some(&event.id) == self.tray_menu_show_id.as_ref() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.is_hidden = false;
            } else if Some(&event.id) == self.tray_menu_clean_id.as_ref() {
                self.queue_action(app::commands::ActionCommand::CleanRam);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.is_hidden = false;
            } else if Some(&event.id) == self.tray_menu_procman_id.as_ref() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.is_hidden = false;
                let _ = self
                    .app_channels
                    .monitoring_sender
                    .send(app::commands::MonitoringCommand::SetHidden(false));
                self.show_process_manager = true;
            } else if Some(&event.id) == self.tray_menu_pause_id.as_ref() {
                let paused = {
                    let mut d = self.data.lock();
                    d.monitoring_paused = !d.monitoring_paused;
                    d.monitoring_paused
                };
                let _ = self
                    .app_channels
                    .monitoring_sender
                    .send(app::commands::MonitoringCommand::SetPaused(paused));
                if let Some(item) = &self.tray_menu_pause_item {
                    item.set_checked(paused);
                }
            } else if let Some(plan_guid) = self.tray_menu_power_guids.get(&event.id) {
                let plan_guid = plan_guid.clone();
                self.queue_action(app::commands::ActionCommand::SetPowerPlan(plan_guid));
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) && self.settings.minimize_to_tray {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.is_hidden = true;
            let _ = self
                .app_channels
                .monitoring_sender
                .send(app::commands::MonitoringCommand::SetHidden(true));
        }

        // Update tray tooltip with CPU/RAM usage
        #[cfg(target_os = "windows")]
        if let Some(tray) = &mut self.tray_icon {
            let data = self.data.lock();
            let tooltip = if data.monitoring_paused {
                format!(
                    "⏸ SysMon Paused — CPU {:.0}% | RAM {:.0}%",
                    data.cpu_usage, data.memory_percentage
                )
            } else {
                format!(
                    "SysMon: CPU {:.0}% | RAM {:.0}%",
                    data.cpu_usage, data.memory_percentage
                )
            };
            let _ = tray.set_tooltip(Some(tooltip));
        }

        // Ensure repaint for continuous updates but without CPU lock
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // Check for updates automatically (once every 24 hours)
        if self.update_check_time.is_none_or(|t| t.elapsed().as_secs() > 86400) {
            let mut updater = self.updater.clone();
            let update_info_share = self.update_info_share.clone();
            thread::Builder::new()
                .name("auto_updater_check".to_string())
                .stack_size(8 * 1024 * 1024)
                .spawn(move || {
                    if let Ok(update_info) = updater.check_for_updates() {
                        *update_info_share.lock() = Some(update_info.clone());
                    }
                })
                .expect("failed to spawn auto updater check thread");
            self.update_check_time = Some(Instant::now());
        }

        // Show update notification banner
        let update_info_opt = self.update_info_share.lock().clone();
        if let Some(update_info) = update_info_opt {
            if update_info.update_available && self.show_update_notification {
                let mut frame = egui::Frame::none().fill(ThemePalette::BG_SURFACE);
                frame.inner_margin = egui::Margin::symmetric(16.0, 12.0);

                egui::TopBottomPanel::top("update_notification")
                    .frame(frame)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                ThemePalette::ACCENT_PRIMARY,
                                egui::RichText::new("UPDATE AVAILABLE").strong(),
                            );
                            ui.add_space(8.0);
                            ui.label(format!(
                                "Version {} is ready. You are currently on v{}.",
                                update_info.latest_version, update_info.current_version
                            ));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Dismiss").clicked() {
                                    self.show_update_notification = false;
                                }
                                ui.add_space(8.0);
                                if ui.button(egui::RichText::new("Install Update").strong()).clicked() {
                                    let download_url = update_info.download_url.clone();
                                    thread::Builder::new()
                                        .name("updater_downloader".to_string())
                                        .stack_size(8 * 1024 * 1024)
                                        .spawn(move || {
                                            if let Err(e) =
                                                updater::Updater::new().download_and_install_update(&download_url)
                                            {
                                                eprintln!("Update failed: {}", e);
                                            }
                                        })
                                        .expect("failed to spawn updater downloader thread");
                                }
                            });
                        });
                    });
            }
        }

        // Keyboard shortcuts
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F5) {
                // Refresh (reset statistics)
                {
                    let mut data = self.data.lock();
                    data.cpu_history.clear();
                    data.memory_history.clear();
                    data.gpu_history.clear();
                }
            }
            if i.modifiers.ctrl {
                let mut new_tab = None;
                if i.key_pressed(egui::Key::Num1) {
                    new_tab = Some(Tab::Overview);
                }
                if i.key_pressed(egui::Key::Num2) {
                    new_tab = Some(Tab::Performance);
                }
                if i.key_pressed(egui::Key::Num3) {
                    new_tab = Some(Tab::Processes);
                }
                if i.key_pressed(egui::Key::Num4) {
                    new_tab = Some(Tab::CpuCores);
                }
                if i.key_pressed(egui::Key::Num5) {
                    new_tab = Some(Tab::Storage);
                }
                if i.key_pressed(egui::Key::Num6) {
                    new_tab = Some(Tab::Network);
                }
                if i.key_pressed(egui::Key::Num7) {
                    new_tab = Some(Tab::SystemInfo);
                }
                if i.key_pressed(egui::Key::Num8) {
                    new_tab = Some(Tab::Alerts);
                }
                if i.key_pressed(egui::Key::Num9) {
                    new_tab = Some(Tab::RamCleaner);
                }
                if i.key_pressed(egui::Key::Num0) {
                    new_tab = Some(Tab::StartupManager);
                }

                if let Some(tab) = new_tab {
                    if tab != Tab::CpuCores || self.settings.show_cpu_cores {
                        self.selected_tab = tab;
                    }
                }
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::E) {
                // Ctrl+E = Export
                self.show_export = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Comma) {
                // Ctrl+, = Settings
                self.show_settings = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::U) {
                // Ctrl+U = Check for updates manually
                let mut updater = self.updater.clone();
                let update_info_share = self.update_info_share.clone();
                let repaint_ctx = ctx_clone.clone();
                thread::Builder::new()
                    .name("manual_updater_check".to_string())
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move || {
                        if let Ok(update_info) = updater.check_for_updates() {
                            *update_info_share.lock() = Some(update_info);
                            repaint_ctx.request_repaint();
                        }
                    })
                    .expect("failed to spawn manual updater check thread");
            }
        });

        // Mirror details selection into shared state so the monitor thread computes details
        {
            let mut d = self.data.lock();
            if d.selected_process_pid != self.details_pid {
                d.selected_process_pid = self.details_pid;
                d.selected_process_details = None;
            }
        }

        let data = self.data.lock().clone();

        // Handle process kill actions
        if let Some(pid) = self.selected_process_pid.take() {
            self.queue_action(app::commands::ActionCommand::KillProcess(pid));
        }

        // Handle process tree kill actions (background thread; tree walk + kills can take seconds)
        if let Some(root) = self.kill_tree_pid.take() {
            self.queue_action(app::commands::ActionCommand::KillProcessTree(root));
        }

        // Handle process suspend actions
        if let Some(pid) = self.suspend_process_pid.take() {
            self.queue_action(app::commands::ActionCommand::SuspendProcess(pid));
        }

        // Handle process resume actions
        if let Some(pid) = self.resume_process_pid.take() {
            self.queue_action(app::commands::ActionCommand::ResumeProcess(pid));
        }

        // Handle process priority changes
        if let Some((pid, priority)) = self.priority_change.take() {
            self.queue_action(app::commands::ActionCommand::SetPriority { pid, priority });
        }

        // Auto RAM cleaning
        if self.ram_cleaner_state.auto_clean_enabled && !self.ram_cleaner_state.is_cleaning {
            let idle_ok = {
                let d = self.data.lock();
                !self.ram_cleaner_state.auto_clean_idle_only || d.last_activity.elapsed().as_secs() > 120
            };
            let should_clean = if let Some(last) = self.ram_cleaner_state.last_cleaned {
                last.elapsed().as_secs() >= self.ram_cleaner_state.auto_clean_interval
                    && data.memory_percentage >= self.ram_cleaner_state.auto_clean_threshold
            } else {
                data.memory_percentage >= self.ram_cleaner_state.auto_clean_threshold
            };
            if should_clean && idle_ok {
                self.ram_cleaner_state.is_cleaning = true;
                self.ram_cleaner_state.last_cleaned = Some(Instant::now());
                self.ram_cleaner_state.last_cleaned_display = Local::now().format("%H:%M:%S").to_string();
                self.ram_cleaner_state.clean_count += 1;
                let data_arc = Arc::clone(&self.data);
                let repaint_ctx = ctx_clone.clone();
                let enable_sounds = self.settings.enable_sounds;
                let target = self.ram_cleaner_state.auto_clean_target;
                let max_mb = self.ram_cleaner_state.auto_clean_max_mb;
                let notify = self.ram_cleaner_state.auto_clean_notify;
                let exclusions = self.ram_cleaner_state.auto_clean_exclusions.clone();
                let total_ram = data.memory_total;
                let auto_event_sender = self.app_channels.event_sender.clone();
                thread::Builder::new()
                    .name("ram_cleaner_auto".to_string())
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move || {
                        // ponytail: bounded passes + budget; a truly stuck
                        // process set just stops after 5 passes
                        let mut monitor = SystemMonitor::new();
                        let mut freed_total = 0u64;
                        for _pass in 0..5 {
                            let freed = monitor.clean_ram(&exclusions);
                            let budget_left = if max_mb == 0 {
                                u64::MAX
                            } else {
                                (max_mb * 1024 * 1024).saturating_sub(freed_total)
                            };
                            freed_total = freed_total.saturating_add(freed);
                            monitor.sys.refresh_memory();
                            let usage_pct = if total_ram > 0 {
                                monitor.sys.used_memory() as f64 / total_ram as f64 * 100.0
                            } else {
                                0.0
                            };
                            if should_stop_cleaning(usage_pct, target as f64, freed, budget_left) {
                                break;
                            }
                        }
                        if enable_sounds {
                            play_success_sound();
                        }
                        if notify {
                            let _ = notify_rust::Notification::new()
                                .summary("Auto RAM Clean")
                                .body(&format!("Freed {:.1} MB of RAM", freed_total as f64 / 1024.0 / 1024.0))
                                .timeout(notify_rust::Timeout::Milliseconds(5000))
                                .show();
                        }
                        let audit = app::actions::ActionAuditRecord::automatic(
                            "Automatic RAM working-set cleanup",
                            format!("Freed {freed_total} bytes using the configured cleanup policy"),
                        );
                        if let Err(error) = persistence::action_log::append(&audit) {
                            warn!(%error, "Failed to persist automatic action audit record");
                        }
                        let _ = auto_event_sender.send(app::events::AppEvent::AuditRecorded(audit));
                        // Store freed bytes in SystemData for the UI to pick up
                        {
                            let mut d = data_arc.lock();
                            d.ram_clean_freed_bytes += freed_total;
                            d.ram_clean_is_cleaning = false;
                        }
                        repaint_ctx.request_repaint();
                    })
                    .expect("failed to spawn auto ram cleaner thread");
                // Mark cleaning in shared data too
                {
                    let mut d = self.data.lock();
                    d.ram_clean_is_cleaning = true;
                }
            }
        }
        // Sync back from shared data
        {
            let d = self.data.lock();
            if !d.ram_clean_is_cleaning && self.ram_cleaner_state.is_cleaning {
                self.ram_cleaner_state.is_cleaning = false;
            }
            self.ram_cleaner_state.bytes_freed = d.ram_clean_freed_bytes;
        }

        // CSV Export window
        let mut show_export_csv = self.show_export_csv;
        if show_export_csv {
            let csv_result = self.export_to_csv(&data);
            egui::Window::new("Export to CSV")
                .open(&mut show_export_csv)
                .resizable(true)
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.heading("Export System Data to CSV");
                    ui.separator();

                    match csv_result {
                        Ok(csv_data) => {
                            ui.label("Data exported successfully. Copy the CSV below:");
                            ui.add_space(5.0);

                            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                ui.text_edit_multiline(&mut csv_data.as_str());
                            });

                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                if ui.button("📋 Copy to Clipboard").clicked() {
                                    ui.output_mut(|o| o.copied_text = csv_data.clone());
                                }
                                if ui.button("💾 Save to File...").clicked() {
                                    let date_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
                                    if let Some(path) = FileDialog::new()
                                        .set_file_name(format!("sysmon_export_{}.csv", date_str))
                                        .add_filter("CSV File", &["csv"])
                                        .save_file()
                                    {
                                        if std::fs::write(&path, &csv_data).is_ok() {
                                            #[cfg(target_os = "windows")]
                                            play_success_sound();
                                        }
                                    }
                                }
                            });

                            ui.add_space(5.0);
                            ui.label("Tip: Open in Excel or any spreadsheet application");
                        }
                        Err(e) => {
                            ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
                        }
                    }
                });
        }
        self.show_export_csv = show_export_csv;

        // JSON Export window
        let mut show_export = self.show_export;
        if show_export {
            let json_result = self.export_data_to_json(&data);
            egui::Window::new("Export Data")
                .open(&mut show_export)
                .resizable(true)
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.heading("Export System Data to JSON");
                    ui.separator();

                    match json_result {
                        Ok(json_data) => {
                            ui.label("Data exported successfully. Copy the JSON below:");
                            ui.add_space(5.0);

                            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                ui.text_edit_multiline(&mut json_data.as_str());
                            });

                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                if ui.button("📋 Copy to Clipboard").clicked() {
                                    ui.output_mut(|o| o.copied_text = json_data.clone());
                                }
                                if ui.button("💾 Save to File...").clicked() {
                                    let date_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
                                    if let Some(path) = FileDialog::new()
                                        .set_file_name(format!("sysmon_export_{}.json", date_str))
                                        .add_filter("JSON File", &["json"])
                                        .save_file()
                                    {
                                        if std::fs::write(&path, &json_data).is_ok() {
                                            #[cfg(target_os = "windows")]
                                            play_success_sound();
                                        }
                                    }
                                }
                            });

                            ui.add_space(5.0);
                            ui.label("Tip: You can paste this into a .json file");
                        }
                        Err(e) => {
                            ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
                        }
                    }
                });
        }
        self.show_export = show_export;

        // Alerts window
        let mut show_alerts = self.show_alerts;
        let mut clear_alerts = false;
        if show_alerts {
            egui::Window::new("System Alerts")
                .open(&mut show_alerts)
                .resizable(true)
                .default_width(600.0)
                .show(ctx, |ui| {
                    ui.heading("Active System Alerts");
                    ui.separator();

                    if data.alerts.is_empty() {
                        ui.label("✅ No active alerts. System is running normally.");
                    } else {
                        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                            for alert in &data.alerts {
                                ui.group(|ui| {
                                    let (icon, color) = match alert.alert_type {
                                        AlertType::CpuHigh => ("CPU", egui::Color32::YELLOW),
                                        AlertType::MemoryHigh => ("RAM", egui::Color32::YELLOW),
                                        AlertType::GpuTempHigh => ("GPU", egui::Color32::RED),
                                        AlertType::DiskSpaceLow => ("DISK", egui::Color32::RED),
                                        AlertType::StartupHighImpact => ("STARTUP", egui::Color32::YELLOW),
                                    };

                                    ui.horizontal(|ui| {
                                        ui.colored_label(color, icon);
                                        ui.colored_label(color, &alert.message);
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(&alert.timestamp);
                                        });
                                    });
                                });
                                ui.add_space(5.0);
                            }
                        });

                        ui.separator();
                        if ui.button("Clear All Alerts").clicked() {
                            clear_alerts = true;
                        }
                    }
                });
        }
        self.show_alerts = show_alerts;
        if clear_alerts {
            {
                let mut data = self.data.lock();
                data.alerts.clear();
            }
        }

        let sidebar_frame = egui::Frame::none()
            .fill(ThemePalette::BG_SURFACE)
            .stroke(egui::Stroke::new(1.0, ThemePalette::BORDER_LIGHT));

        // Modern sleek SidePanel for navigation
        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .exact_width(180.0)
            .frame(sidebar_frame)
            .show(ctx, |ui| {
                ui.add_space(16.0);

                // Brand Header
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    // Painted diamond glyph
                    let r = ui.label(egui::RichText::new(" ").size(18.0));
                    let cy = r.rect.center().y;
                    let cx = r.rect.left() + 2.0;
                    let sz = 8.0;
                    let pts = vec![
                        egui::pos2(cx, cy - sz),
                        egui::pos2(cx + sz * 0.65, cy),
                        egui::pos2(cx, cy + sz),
                        egui::pos2(cx - sz * 0.65, cy),
                    ];
                    ui.painter().add(egui::Shape::convex_polygon(
                        pts,
                        ThemePalette::ACCENT_PRIMARY,
                        egui::Stroke::NONE,
                    ));
                    ui.label(
                        egui::RichText::new("Sys")
                            .size(18.0)
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new("Mon")
                            .size(18.0)
                            .strong()
                            .color(ThemePalette::TEXT_PRIMARY),
                    );
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // Navigation Menu
                let tabs = [
                    (Tab::Overview, "Overview"),
                    (Tab::Performance, "Performance"),
                    (Tab::Processes, "Processes"),
                    (Tab::CpuCores, "CPU Cores"),
                    (Tab::Storage, "Storage"),
                    (Tab::Network, "Network"),
                    (Tab::Alerts, "Alerts"),
                    (Tab::SystemInfo, "System Info"),
                    (Tab::RamCleaner, "RAM Cleaner"),
                    (Tab::StartupManager, "Startup Apps"),
                    (Tab::Services, "Services"),
                    (Tab::Diagnostics, "Diagnostics"),
                ];

                ui.spacing_mut().item_spacing.y = 4.0;
                for (tab, label) in tabs {
                    if tab == Tab::CpuCores && !self.settings.show_cpu_cores {
                        continue;
                    }
                    let is_selected = self.selected_tab == tab;
                    let text = if is_selected {
                        egui::RichText::new(label).strong().color(ThemePalette::BG_DEEPEST)
                    } else {
                        egui::RichText::new(label).color(ThemePalette::TEXT_SECONDARY)
                    };

                    let btn = egui::Button::new(text)
                        .fill(if is_selected {
                            ThemePalette::ACCENT_ACTIVE
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .frame(is_selected);

                    if ui.add_sized([ui.available_width(), 32.0], btn).clicked() {
                        self.selected_tab = tab;
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(format!("Updated: {}", data.last_update))
                            .size(11.0)
                            .color(ThemePalette::TEXT_DIMMED),
                    );
                    ui.add_space(8.0);
                    if ui
                        .add_sized([ui.available_width(), 28.0], egui::Button::new("Settings"))
                        .clicked()
                    {
                        self.show_settings = true;
                    }
                    ui.add_space(4.0);
                    if ui
                        .add_sized([ui.available_width(), 28.0], egui::Button::new("Shortcuts"))
                        .clicked()
                    {
                        self.show_shortcuts = true;
                    }
                    ui.add_space(4.0);
                    if ui
                        .add_sized([ui.available_width(), 28.0], egui::Button::new("About"))
                        .clicked()
                    {
                        self.selected_tab = Tab::About;
                    }
                });
            });

        // Process Manager window
        if self.show_process_manager {
            crate::ui::windows::process_manager::show(self, ctx, &data);
        }

        // Keyboard Shortcuts dialog
        let mut show_shortcuts = self.show_shortcuts;
        if show_shortcuts {
            egui::Window::new("Keyboard Shortcuts")
                .open(&mut show_shortcuts)
                .resizable(false)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.heading("Available Shortcuts");
                    ui.separator();
                    egui::Grid::new("shortcuts_grid").spacing([20.0, 6.0]).show(ui, |ui| {
                        let shortcuts = [
                            ("F5", "Refresh / Reset statistics"),
                            ("Ctrl + E", "Export data to JSON"),
                            ("Ctrl + ,", "Open Settings"),
                            ("Ctrl + U", "Check for updates"),
                        ];
                        for (key, desc) in &shortcuts {
                            ui.label(egui::RichText::new(*key).strong().color(ThemePalette::ACCENT_PRIMARY));
                            ui.label(*desc);
                            ui.end_row();
                        }
                    });
                });
        }
        self.show_shortcuts = show_shortcuts;

        // Settings window
        if self.show_settings {
            let mut show_settings = self.show_settings;
            egui::Window::new("Settings")
                .open(&mut show_settings)
                .resizable(true)
                .default_width(600.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    crate::ui::pages::settings::show(self, ui);
                });
            self.show_settings = show_settings;
        }

        // Desktop mini-widget: a compact always-visible telemetry window
        if self.widget_open {
            egui::Window::new("SysMon Widget")
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(0.0, 0.0))
                .resizable(false)
                .title_bar(true)
                .collapsible(false)
                .show(ctx, |ui| {
                    self.render_widget(ui, &data);
                });
        }

        // Global always-visible status bar header
        let status_bar_frame = egui::Frame::none()
            .fill(ctx.style().visuals.extreme_bg_color)
            .inner_margin(egui::Margin::symmetric(16.0, 0.0))
            .stroke(egui::Stroke::new(
                1.0,
                ctx.style().visuals.widgets.noninteractive.bg_stroke.color,
            ));

        egui::TopBottomPanel::top("global_status_bar")
            .exact_height(48.0)
            .frame(status_bar_frame)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);

                    // Quick stats
                    let cpu_c = get_usage_color(data.cpu_usage);
                    ui.label("CPU: ");
                    ui.colored_label(cpu_c, egui::RichText::new(format!("{:.1}%", data.cpu_usage)).strong());

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(16.0);

                    let mem_c = get_usage_color(data.memory_percentage);
                    ui.label("RAM: ");
                    ui.colored_label(
                        mem_c,
                        egui::RichText::new(format!("{:.1}%", data.memory_percentage)).strong(),
                    );

                    if let Some(gpu) = data.gpu_info.first() {
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(16.0);
                        let gpu_c = get_usage_color(gpu.utilization);
                        ui.label("GPU: ");
                        ui.colored_label(gpu_c, egui::RichText::new(format!("{:.1}%", gpu.utilization)).strong());
                    }

                    // Alerts indicator
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        if !data.alerts.is_empty() {
                            let recent_alerts = data.alerts.len();
                            let btn = ui.button(
                                egui::RichText::new(format!("{} Alerts", recent_alerts))
                                    .color(ThemePalette::STATUS_WARNING),
                            );
                            if btn.clicked() {
                                self.selected_tab = Tab::Alerts;
                            }
                        } else {
                            ui.label(egui::RichText::new("All Good").color(ThemePalette::STATUS_HEALTHY));
                        }
                    });
                });
            });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| match self.selected_tab {
            Tab::Overview => crate::ui::pages::overview::show(self, ui, &data),
            Tab::Performance => crate::ui::pages::performance::show(self, ui, &data),
            Tab::Processes => crate::ui::pages::processes::show(self, ui, &data),
            Tab::CpuCores => crate::ui::pages::cpu_cores::show(self, ui, &data),
            Tab::Storage => crate::ui::pages::storage::show(self, ui, &data),
            Tab::Network => crate::ui::pages::network::show(self, ui, &data),
            Tab::SystemInfo => crate::ui::pages::system_info::show(self, ui, &data),
            Tab::Alerts => crate::ui::pages::alerts::show(self, ui, &data),
            Tab::RamCleaner => crate::ui::pages::ram_cleaner::show(self, ui, &data),
            Tab::StartupManager => crate::ui::pages::startup_manager::show(self, ui),
            Tab::Services => crate::ui::pages::services::show(self, ui, &data),
            Tab::Diagnostics => crate::ui::pages::diagnostics::show(self, ui, &data),
            Tab::About => crate::ui::pages::about::show(self, ui, &data),
        });
        self.render_action_confirmation(ctx);
        self.render_action_history(ctx);
    }
}

// ─── Custom UI helpers ───────────────────────────────────────────────

impl Drop for SystemMonitorApp {
    fn drop(&mut self) {
        let _ = self
            .app_channels
            .monitoring_sender
            .send(app::commands::MonitoringCommand::Shutdown);
        let _ = self.telemetry_commands.try_send(telemetry::HubCommand::Shutdown);
    }
}

impl SystemMonitorApp {
    pub(crate) fn queue_action(&mut self, command: app::commands::ActionCommand) {
        if self.action_pending || self.pending_action_plan.is_some() {
            self.action_status = Some("Another system action is already pending.".into());
            return;
        }
        self.pending_action_plan = Some(app::actions::ActionPlan::from_command(command));
    }

    fn render_action_confirmation(&mut self, ctx: &egui::Context) {
        let Some(plan) = self.pending_action_plan.clone() else {
            return;
        };
        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new("Confirm system action")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.heading(&plan.title);
                ui.label(&plan.summary);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.strong("Risk:");
                    let color = match plan.risk {
                        app::actions::RiskLevel::Low => ThemePalette::STATUS_HEALTHY,
                        app::actions::RiskLevel::Medium => ThemePalette::STATUS_WARNING,
                        app::actions::RiskLevel::High | app::actions::RiskLevel::Critical => {
                            ThemePalette::STATUS_CRITICAL
                        }
                    };
                    ui.colored_label(color, plan.risk.label());
                });
                ui.label(format!(
                    "Administrator privileges: {}",
                    if plan.requires_admin {
                        "usually required"
                    } else {
                        "not required"
                    }
                ));
                ui.label(format!(
                    "Undo available: {}",
                    if plan.reversible { "yes" } else { "no" }
                ));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                    if ui.button(egui::RichText::new("Confirm and run").strong()).clicked() {
                        confirm = true;
                    }
                });
            });

        if cancel {
            self.pending_action_plan = None;
        } else if confirm {
            self.pending_action_plan = None;
            if matches!(plan.command, app::commands::ActionCommand::CleanRam) {
                self.ram_cleaner_state.is_cleaning = true;
            }
            match self.app_channels.action_sender.send(plan.command) {
                Ok(()) => self.action_pending = true,
                Err(error) => self.action_status = Some(format!("Could not queue action: {error}")),
            }
        }
    }

    fn render_action_history(&mut self, ctx: &egui::Context) {
        if !self.show_action_history {
            return;
        }
        let mut open = self.show_action_history;
        let mut undo = None;
        egui::Window::new("System Action History")
            .open(&mut open)
            .default_width(620.0)
            .show(ctx, |ui| {
                ui.label("Persistent audit records are stored locally. Undo is offered only when the original state is known.");
                ui.separator();
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    for entry in self.action_history.iter().rev().take(100) {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(&entry.record.action);
                                ui.label(entry.record.risk.label());
                                if entry.record.succeeded {
                                    ui.colored_label(ThemePalette::STATUS_HEALTHY, "Succeeded");
                                } else {
                                    ui.colored_label(ThemePalette::STATUS_CRITICAL, "Failed");
                                }
                            });
                            ui.small(&entry.record.timestamp);
                            ui.small(format!("Initiated by {}", entry.record.initiator));
                            ui.label(&entry.record.message);
                            if let Some(command) = &entry.undo {
                                if ui.button("Undo this action").clicked() {
                                    undo = Some(command.clone());
                                }
                            }
                        });
                    }
                });
            });
        self.show_action_history = open;
        if let Some(command) = undo {
            self.queue_action(command);
        }
    }

    fn start_ram_clean(&mut self, _ctx: &egui::Context) {
        self.queue_action(app::commands::ActionCommand::CleanRam);
    }

    /// Render the compact desktop mini-widget telemetry panel.
    fn render_widget(&mut self, ui: &mut egui::Ui, data: &SystemData) {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
        ui.set_width(220.0);

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let cpu_c = get_usage_color(data.cpu_usage);
                ui.label("CPU");
                ui.colored_label(cpu_c, egui::RichText::new(format!("{:.1}%", data.cpu_usage)).strong());
                ui.weak(format!("{} cores", data.cpu_cores.len()));
            });
            ui.horizontal(|ui| {
                let mem_c = get_usage_color(data.memory_percentage);
                ui.label("RAM");
                ui.colored_label(
                    mem_c,
                    egui::RichText::new(format!("{:.1}%", data.memory_percentage)).strong(),
                );
                ui.weak(format!(
                    "{:.1} / {:.1} GB",
                    data.memory_used as f64 / 1024.0 / 1024.0 / 1024.0,
                    data.memory_total as f64 / 1024.0 / 1024.0 / 1024.0
                ));
            });

            if let Some(gpu) = data.gpu_info.first() {
                ui.horizontal(|ui| {
                    let gpu_c = get_usage_color(gpu.utilization);
                    ui.label("GPU");
                    ui.colored_label(gpu_c, egui::RichText::new(format!("{:.1}%", gpu.utilization)).strong());
                });
            }

            ui.separator();
            let dl: f64 = data.network_info.iter().map(|n| n.received_rate).sum();
            let ul: f64 = data.network_info.iter().map(|n| n.transmitted_rate).sum();
            ui.horizontal(|ui| {
                ui.colored_label(ThemePalette::ACCENT_PRIMARY, format!("↓ {:.0} K/s", dl));
                ui.add_space(8.0);
                ui.colored_label(ThemePalette::ACCENT_ACTIVE, format!("↑ {:.0} K/s", ul));
            });

            if let Some(temp) = data.cpu_temperature {
                ui.horizontal(|ui| {
                    ui.label("CPU Temp");
                    ui.strong(format!("{temp:.0}°C"));
                });
            }
        });

        ui.separator();
        ui.horizontal(|ui| {
            if ui.small_button("Hide").clicked() {
                self.widget_open = false;
                self.settings.show_widget = false;
                let _ = self.settings.save();
                {
                    let mut shared = self.shared_settings.lock();
                    *shared = self.settings.clone();
                }
            }
            ui.weak(&data.last_update);
        });
    }
}

use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Clone)]
pub(crate) enum ActionError {
    AccessDenied,
    // typed error contract; constructed once kill/service paths distinguish failures
    #[allow(dead_code)]
    NotFound,
    #[allow(dead_code)]
    Unavailable,
    Failed(String),
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessDenied => write!(f, "Access denied; administrator privileges may be required"),
            Self::NotFound => write!(f, "Process or service not found"),
            Self::Unavailable => write!(f, "Operation unavailable on this system"),
            Self::Failed(message) => f.write_str(message),
        }
    }
}

pub(crate) fn run_action_worker(
    commands: Receiver<app::commands::ActionCommand>,
    events: Sender<app::events::AppEvent>,
) {
    let mut monitor = SystemMonitor::new();
    while let Ok(command) = commands.recv() {
        let plan = app::actions::ActionPlan::from_command(command.clone());
        let result: Result<String, ActionError> = match command {
            app::commands::ActionCommand::KillProcess(pid) => monitor
                .kill_process(pid)
                .then_some(format!("Process {pid} killed"))
                .ok_or(ActionError::AccessDenied),
            app::commands::ActionCommand::SuspendProcess(pid) => monitor
                .suspend_process(pid)
                .then_some(format!("Process {pid} suspended"))
                .ok_or(ActionError::AccessDenied),
            app::commands::ActionCommand::ResumeProcess(pid) => monitor
                .resume_process(pid)
                .then_some(format!("Process {pid} resumed"))
                .ok_or(ActionError::AccessDenied),
            app::commands::ActionCommand::SetPriority { pid, priority } => {
                SystemMonitor::set_process_priority(pid, &priority)
                    .then_some(format!("Process {pid} priority set to {priority}"))
                    .ok_or(ActionError::AccessDenied)
            }
            app::commands::ActionCommand::CleanRam => Ok(format!("Freed {} bytes", monitor.clean_ram(&[]))),
            app::commands::ActionCommand::ControlService { name, action } => {
                services::send_service_control(&name, action)
                    .then_some(format!("Service {name} action completed"))
                    .ok_or(ActionError::Failed("Service action failed".into()))
            }
            app::commands::ActionCommand::SetPowerPlan(guid) => power::set_active_power_plan(&guid)
                .map(|_| "Power plan changed".into())
                .map_err(ActionError::Failed),
            app::commands::ActionCommand::KillProcessTree(root) => {
                monitor.sys.refresh_processes();
                let tree = processes::build_process_tree(&monitor.sys);
                let order = processes::kill_order(&tree, root);
                let total = order.len();
                let killed = order.into_iter().filter(|pid| monitor.kill_process(*pid)).count();
                if killed == total {
                    Ok(format!("Killed {killed} processes"))
                } else {
                    Err(ActionError::Failed(format!("Killed {killed} of {total} processes")))
                }
            }
        };
        let audit_result = result.map_err(|error| error.to_string());
        let record = app::actions::ActionAuditRecord::from_result(&plan, &audit_result);
        if let Err(error) = persistence::action_log::append(&record) {
            warn!(%error, "Failed to persist action audit record");
        }
        let event = match audit_result {
            Ok(_) => app::events::AppEvent::ActionCompleted {
                command: plan.command,
                record,
                undo: plan.undo,
            },
            Err(_) => app::events::AppEvent::ActionFailed {
                command: plan.command,
                record,
            },
        };
        let _ = events.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sysmon-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    #[test]
    fn validation_clamps_user_ranges() {
        let settings = AppSettings {
            refresh_interval: 0,
            process_count: 999,
            ram_clean_threshold: 1.0,
            ..Default::default()
        };
        let checked = crate::persistence::settings::validated(settings);
        assert_eq!(checked.refresh_interval, 1);
        assert_eq!(checked.process_count, 100);
        assert_eq!(checked.ram_clean_threshold, 50.0);
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_path("settings.json");
        let settings = AppSettings::default();
        crate::persistence::settings::save(&path, &settings).unwrap();
        let loaded = crate::persistence::settings::load(&path).unwrap();
        assert_eq!(loaded.refresh_interval, settings.refresh_interval);
        let _ = std::fs::remove_file(path);
    }
}
pub(crate) fn snapshot_from_data(data: &SystemData) -> monitoring::SystemSnapshot {
    let mut provider_status: std::collections::HashMap<_, _> = data
        .provider_status
        .iter()
        .map(|(name, available)| {
            (
                name.clone(),
                monitoring::snapshot::ProviderStatus {
                    available: *available,
                    stale: data.monitoring_paused,
                    error: None,
                },
            )
        })
        .collect();
    for (name, available) in [
        ("disk", !data.disk_info.is_empty()),
        ("network", !data.network_info.is_empty()),
        ("battery", data.battery_info.is_some()),
    ] {
        provider_status.insert(
            name.into(),
            monitoring::snapshot::ProviderStatus {
                available,
                stale: data.monitoring_paused,
                error: None,
            },
        );
    }
    monitoring::SystemSnapshot {
        sampled_at: std::time::SystemTime::now(),
        cpu_usage: data.cpu_usage,
        cpu_cores: data.cpu_cores.iter().map(|core| core.usage).collect(),
        cpu_temperature: data.cpu_temperature,
        memory_total: data.memory_total,
        memory_used: data.memory_used,
        memory_percentage: data.memory_percentage,
        swap: monitoring::snapshot::SwapSnapshot {
            total: data.swap_info.total,
            used: data.swap_info.used,
            percentage: data.swap_info.percentage,
        },
        gpus: data
            .gpu_info
            .iter()
            .map(|gpu| monitoring::snapshot::GpuSnapshot {
                name: gpu.name.clone(),
                utilization: gpu.utilization,
                memory_used: gpu.memory_used,
                memory_total: gpu.memory_total,
                temperature: gpu.temperature,
                clock_mhz: gpu.clock_mhz,
                power_watts: gpu.power_watts,
                fan_percent: gpu.fan_percent,
            })
            .collect(),
        disks: data
            .disk_info
            .iter()
            .map(|disk| monitoring::snapshot::DiskSnapshot {
                name: disk.name.clone(),
                mount_point: disk.mount_point.clone(),
                total_space: disk.total_space,
                available_space: disk.available_space,
                usage_percentage: disk.usage_percentage,
                file_system: disk.file_system.clone(),
                read_bytes_per_second: data.disk_read_rate,
                written_bytes_per_second: data.disk_write_rate,
            })
            .collect(),
        networks: data
            .network_info
            .iter()
            .map(|network| monitoring::snapshot::NetworkSnapshot {
                interface: network.interface.clone(),
                received: network.received,
                transmitted: network.transmitted,
                received_bytes_per_second: network.received_rate,
                transmitted_bytes_per_second: network.transmitted_rate,
            })
            .collect(),
        processes: data
            .top_processes
            .iter()
            .map(|process| monitoring::snapshot::ProcessSnapshot {
                pid: process.pid,
                name: process.name.clone(),
                cpu_usage: process.cpu_usage,
                memory: process.memory,
                status: process.status.clone(),
                disk_read_bytes: process.disk_read_bytes,
                disk_written_bytes: process.disk_written_bytes,
            })
            .collect(),
        battery: data
            .battery_info
            .as_ref()
            .map(|battery| monitoring::snapshot::BatterySnapshot {
                design_capacity: battery.design_capacity,
                full_charge_capacity: battery.full_charge_capacity,
                status: battery.status,
                discharge_state: battery.discharge_state.clone(),
                present: battery.present,
            }),
        system: monitoring::snapshot::SystemInfoSnapshot {
            os_name: data.system_info.os_name.clone(),
            os_version: data.system_info.os_version.clone(),
            kernel_version: data.system_info.kernel_version.clone(),
            hostname: data.system_info.hostname.clone(),
            uptime: data.system_info.uptime,
            cpu_count: data.system_info.cpu_count,
            cpu_brand: data.system_info.cpu_brand.clone(),
            motherboard: data.system_info.motherboard.clone(),
            bios_version: data.system_info.bios_version.clone(),
            gpu_driver: data.system_info.gpu_driver.clone(),
            os_build: data.system_info.os_build.clone(),
        },
        provider_status,
        paused: data.monitoring_paused,
    }
}

fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(icon_bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

#[cfg(target_os = "windows")]
fn load_tray_icon() -> Option<tray_icon::Icon> {
    let image = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    tray_icon::Icon::from_rgba(rgba, width, height).ok()
}

fn main() {
    // ── 1. Single-Instance Enforcement ──────────────────────────────────
    // Prevent multiple copies from running simultaneously using a Windows named mutex.
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn CreateMutexW(
                lp_mutex_attributes: *const std::ffi::c_void,
                b_initial_owner: i32,
                lp_name: *const u16,
            ) -> *mut std::ffi::c_void;
            fn GetLastError() -> u32;
        }

        let mutex_name: Vec<u16> = "Global\\SystemMonitorSingleInstance\0".encode_utf16().collect();
        let _handle = unsafe { CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr()) };
        let last_error = unsafe { GetLastError() };

        const ERROR_ALREADY_EXISTS: u32 = 183;
        if last_error == ERROR_ALREADY_EXISTS {
            use windows::core::PCWSTR;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

            let title: Vec<u16> = "System Monitor\0".encode_utf16().collect();
            let msg: Vec<u16> = "System Monitor is already running.\n\nCheck your system tray or taskbar.\0"
                .encode_utf16()
                .collect();
            unsafe {
                let _ = MessageBoxW(
                    None,
                    PCWSTR(msg.as_ptr()),
                    PCWSTR(title.as_ptr()),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            std::process::exit(0);
        }
    }

    // ── 2. Crash Report Directory ───────────────────────────────────────
    let log_dir = directories::ProjectDirs::from("com", "Xenonesis", "SystemMonitor")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("SystemMonitor"));
    let crash_dir = log_dir.join("crash-reports");
    let logs_dir = log_dir.join("logs");
    let _ = std::fs::create_dir_all(&crash_dir);
    let _ = std::fs::create_dir_all(&logs_dir);

    // ── 3. Global Panic Handler ─────────────────────────────────────────
    // On panic: write a crash report to disk and show a MessageBox.
    let crash_dir_clone = crash_dir.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let crash_file = crash_dir_clone.join(format!("crash_{}.log", timestamp));

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());

        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        let report = format!(
            "═══════════════════════════════════════════════\n\
             SYSTEM MONITOR — CRASH REPORT\n\
             ═══════════════════════════════════════════════\n\
             Version:   {}\n\
             Timestamp: {}\n\
             Location:  {}\n\
             \n\
             Error:\n\
             {}\n\
             \n\
             Please report this issue at:\n\
             https://github.com/Xenonesis/sysmon/issues\n\
             ═══════════════════════════════════════════════\n",
            APP_VERSION,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            location,
            payload,
        );

        let _ = std::fs::write(&crash_file, &report);

        // Show a MessageBox on Windows so the user sees feedback instead of silent crash
        #[cfg(target_os = "windows")]
        {
            use windows::core::PCWSTR;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

            let title: Vec<u16> = "System Monitor — Unexpected Error\0".encode_utf16().collect();
            let msg_text = format!(
                "System Monitor encountered an unexpected error and needs to close.\n\n\
                 Error: {}\n\
                 Location: {}\n\n\
                 A crash report has been saved to:\n{}\n\n\
                 Please report this issue on GitHub.\0",
                payload,
                location,
                crash_file.display()
            );
            let msg: Vec<u16> = msg_text.encode_utf16().collect();
            unsafe {
                MessageBoxW(None, PCWSTR(msg.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR);
            }
        }
    }));

    // ── 4. Structured Logging ───────────────────────────────────────────
    let file_appender = tracing_appender::rolling::daily(&logs_dir, "system-monitor.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .init();

    info!(version = APP_VERSION, "System Monitor starting — Enterprise Edition");
    info!("Log directory: {}", logs_dir.display());
    info!("Crash report directory: {}", crash_dir.display());

    // ── 5. Launch GUI ───────────────────────────────────────────────────
    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 800.0])
        .with_min_inner_size([900.0, 600.0])
        .with_title(format!("System Monitor v{}", APP_VERSION));

    if let Some(icon) = load_icon() {
        viewport_builder = viewport_builder.with_icon(std::sync::Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };

    info!("Launching GUI window");

    let result = eframe::run_native(
        "System Monitor",
        options,
        Box::new(|cc| {
            let app = SystemMonitorApp::new(cc);
            Ok(Box::new(app))
        }),
    );

    match result {
        Ok(()) => {
            info!("System Monitor shut down gracefully");
        }
        Err(e) => {
            error!("GUI failed to start: {}", e);

            #[cfg(target_os = "windows")]
            {
                use windows::core::PCWSTR;
                use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

                let title: Vec<u16> = "System Monitor — Startup Error\0".encode_utf16().collect();
                let msg_text = format!(
                    "System Monitor failed to start.\n\n\
                     Error: {}\n\n\
                     Please ensure your graphics drivers are up to date.\0",
                    e
                );
                let msg: Vec<u16> = msg_text.encode_utf16().collect();
                unsafe {
                    MessageBoxW(None, PCWSTR(msg.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR);
                }
            }

            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn test_battery_info_default() {
        let b = BatteryInfo::default();
        assert_eq!(b.design_capacity, 0);
        assert!(!b.present);
    }
}
#[cfg(test)]
mod ram_cleaner_tests {
    use super::*;

    fn ex(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exclusion_matches_case_insensitively() {
        assert!(is_excluded("Chrome.EXE", &ex(&["chrome.exe"])));
        assert!(is_excluded("firefox", &ex(&["FireFox"])));
        assert!(!is_excluded("notepad", &ex(&["chrome.exe"])));
        assert!(!is_excluded("chrome", &ex(&["chrome.exe"])));
    }

    #[test]
    fn stop_conditions_cover_target_budget_and_empty() {
        assert!(should_stop_cleaning(65.0, 70.0, 10, 100)); // under target
        assert!(!should_stop_cleaning(80.0, 70.0, 10, 100)); // still over target
        assert!(should_stop_cleaning(90.0, 70.0, 0, 100)); // nothing freed
        assert!(should_stop_cleaning(90.0, 70.0, 100, 100)); // budget exhausted
    }

    #[test]
    fn settings_defaults_and_clamps() {
        let s = AppSettings::default();
        assert_eq!(s.auto_clean_target, 70.0);
        assert!(s.auto_clean_notify);
        assert_eq!(s.auto_clean_max_mb, 0);
        let s2 = AppSettings {
            auto_clean_target: 10.0,
            auto_clean_max_mb: 99999,
            ..Default::default()
        };
        let c = crate::persistence::settings::validated(s2);
        assert_eq!(c.auto_clean_target, 30.0);
        assert_eq!(c.auto_clean_max_mb, 4096);
    }
}
