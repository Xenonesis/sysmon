#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use chrono::Local;
mod updater;
mod startup;
mod privilege;
use startup::{StartupItem, ImpactTier, Recommendation, StartupSortColumn, BootDiagnostics, StartupOptimizationEntry};
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use tracing::{error, info, warn};
use rfd::FileDialog;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

struct ThemePalette;
impl ThemePalette {
    // Primary Vibrant Accents -> Muted Minimalist Primary
    const ACCENT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(198, 198, 199); // #c6c6c7
    const ACCENT_ACTIVE: egui::Color32 = egui::Color32::from_rgb(226, 226, 226); // #e2e2e2

    // Sleek Dark Backgrounds -> Graphite Core
    const BG_DEEPEST: egui::Color32 = egui::Color32::from_rgb(14, 14, 14); // #0e0e0e
    const BG_DEEP: egui::Color32 = egui::Color32::from_rgb(14, 14, 14); 
    const BG_SURFACE: egui::Color32 = egui::Color32::from_rgb(19, 19, 19); // #131313
    const BG_CARD: egui::Color32 = egui::Color32::from_rgb(19, 19, 19);
    const BG_TRACK: egui::Color32 = egui::Color32::from_rgb(31, 32, 32); // #1f2020

    // Component states
    const WIDGET_INACTIVE: egui::Color32 = egui::Color32::from_rgb(31, 32, 32); // #1f2020
    const WIDGET_HOVERED: egui::Color32 = egui::Color32::from_rgb(37, 38, 38); // #252626
    const BORDER: egui::Color32 = egui::Color32::from_rgb(19, 19, 19); // Hidden in #131313
    const BORDER_LIGHT: egui::Color32 = egui::Color32::from_rgb(31, 32, 32); // Just slight edge

    // Modern Status Colors -> Minimalist Status
    const STATUS_HEALTHY: egui::Color32 = egui::Color32::from_rgb(230, 255, 244); // #e6fff4
    const STATUS_WARNING: egui::Color32 = egui::Color32::from_rgb(192, 191, 191); // Soft grey
    const STATUS_CRITICAL: egui::Color32 = egui::Color32::from_rgb(238, 125, 119); // #ee7d77

    // Gorgeous Typography hierarchy -> Crisp and Stark
    const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(255, 255, 255); // Stark white
    const TEXT_SELECTED: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
    const TEXT_FEATURE: egui::Color32 = egui::Color32::from_rgb(231, 229, 229); // #e7e5e5
    const TEXT_SUBTITLE: egui::Color32 = egui::Color32::from_rgb(172, 171, 170); // #acabaa
    const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(172, 171, 170); 
    const TEXT_LABEL: egui::Color32 = egui::Color32::from_rgb(118, 117, 117); // #767575
    const TEXT_LABEL_SUB: egui::Color32 = egui::Color32::from_rgb(118, 117, 117); 
    const TEXT_TERTIARY: egui::Color32 = egui::Color32::from_rgb(86, 85, 85); // #565555
    const TEXT_DIMMED: egui::Color32 = egui::Color32::from_rgb(86, 85, 85);

    const GPU_UNAVAILABLE: egui::Color32 = egui::Color32::from_rgb(86, 85, 85);
    const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(198, 198, 199); // Map purple to primary grey
    const ACCENT_CYAN: egui::Color32 = egui::Color32::from_rgb(198, 198, 199); // Map cyan to primary grey
}
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::sync::Arc;
use parking_lot::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, Pid, System};

#[cfg(target_os = "windows")]
use nvml_wrapper::Nvml;
#[cfg(target_os = "windows")]
use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}};

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
#[derive(Clone, Serialize)]
struct ProcessInfo {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory: u64,
    status: String,
}

#[derive(Clone)]
struct CpuCoreInfo {
    core_id: usize,
    usage: f32,
    #[allow(dead_code)]
    name: String,
}

#[derive(Clone, Serialize)]
struct GpuInfo {
    name: String,
    utilization: f32,
    memory_used: Option<u64>,
    memory_total: Option<u64>,
    temperature: Option<u32>,
}

#[derive(Clone, Serialize)]
struct DiskInfo {
    name: String,
    mount_point: String,
    total_space: u64,
    available_space: u64,
    usage_percentage: f32,
    file_system: String,
}

#[derive(Clone, Serialize)]
struct NetworkInfo {
    interface: String,
    received: u64,
    transmitted: u64,
    received_rate: f64,
    transmitted_rate: f64,
}

#[derive(Clone)]
struct AlertInfo {
    timestamp: String,
    alert_type: AlertType,
    message: String,
    value: f32,
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
struct SwapInfo {
    total: u64,
    used: u64,
    percentage: f32,
}

// Battery info
#[derive(Clone)]
struct BatteryInfo {
    percentage: f32,
    is_charging: bool,
    status_text: String,
}

// StartupItem is now in startup.rs module

// RAM Cleaner state
#[derive(Clone)]
struct RamCleanerState {
    last_cleaned: Option<Instant>,
    last_cleaned_display: String,
    bytes_freed: u64,
    auto_clean_enabled: bool,
    auto_clean_threshold: f32, // percentage threshold for auto-clean
    auto_clean_interval: u64,  // seconds between auto-cleans
    is_cleaning: bool,
    clean_count: u32,
}

#[derive(Clone, Serialize)]
struct SystemInfo {
    os_name: String,
    os_version: String,
    kernel_version: String,
    hostname: String,
    uptime: u64,
    cpu_count: usize,
    cpu_brand: String,
}

// Settings structure
#[derive(Serialize, Deserialize, Clone)]
struct AppSettings {
    refresh_interval: u64,
    show_graphs: bool,
    show_gpu: bool,
    show_processes: bool,
    show_notifications: bool,
    notification_cpu_threshold: f32,
    notification_memory_threshold: f32,
    notification_temp_threshold: u32,
    theme_dark: bool,
    show_per_core_cpu: bool,
    process_count: usize,
    auto_clear_alerts: bool,
    auto_start: bool,
    start_minimized: bool,
    minimize_to_tray: bool,
    #[serde(default = "default_auto_ram_clean")]
    auto_ram_clean: bool,
    #[serde(default = "default_ram_clean_threshold")]
    ram_clean_threshold: f32,
    #[serde(default = "default_enable_sounds")]
    enable_sounds: bool,
    #[serde(default)]
    startup_optimization_history: Vec<StartupOptimizationEntry>,
    #[serde(default)]
    last_boot_diagnostics: Option<BootDiagnostics>,
}

fn default_enable_sounds() -> bool {
    true
}

fn default_auto_ram_clean() -> bool {
    false
}
fn default_ram_clean_threshold() -> f32 {
    85.0
}

#[derive(PartialEq, Clone, Copy)]
enum ProcessSortColumn {
    Pid,
    Name,
    Memory,
    Cpu,
    #[allow(dead_code)]
    Status,
}

struct SystemMonitor {
    sys: System,
    disks: Disks,
    networks: Networks,
    #[cfg(target_os = "windows")]
    nvml: Option<Nvml>,
    #[cfg(target_os = "windows")]
    wmi_gpu_engine_class: Option<String>,
    #[cfg(target_os = "windows")]
    wmi_gpu_memory_class: Option<String>,
    last_network_update: Instant,
    last_disk_update: Instant,
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
            key.set_value("SystemMonitor", &exe_path.to_string_lossy().to_string())?;
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
            if let Ok(contents) = fs::read_to_string(config_path) {
                if let Ok(settings) = serde_json::from_str(&contents) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_dir) = directories::ProjectDirs::from("com", "Xenonesis", "SystemMonitor") {
            let config_path = config_dir.config_dir();
            fs::create_dir_all(config_path)?;
            let config_file = config_path.join("settings.json");
            let contents = serde_json::to_string_pretty(self)?;
            fs::write(config_file, contents)?;
        }
        Ok(())
    }
}

impl SystemMonitor {
    fn new() -> Self {
        println!("DBG: SystemMonitor::new starting");
        let mut sys = System::new_all();
        println!("DBG: sysinfo::System::new_all completed");
        sys.refresh_all();
        println!("DBG: sysinfo::System::refresh_all completed");

        let disks = Disks::new_with_refreshed_list();
        println!("DBG: sysinfo::Disks::new_with_refreshed_list completed");
        let networks = Networks::new_with_refreshed_list();
        println!("DBG: sysinfo::Networks::new_with_refreshed_list completed");

        #[cfg(target_os = "windows")]
        let nvml = {
            println!("DBG: Initializing NVML");
            let res = Nvml::init().ok();
            println!("DBG: NVML init done: {}", res.is_some());
            res
        };

        // Probe which WMI GPU performance counter class name is available on this system.
        // Windows versions differ: some use "GPUPerformanceMonitors", others use "GPUPerformanceCounters".
        #[cfg(target_os = "windows")]
        let (wmi_gpu_engine_class, wmi_gpu_memory_class) = {
            use wmi::{COMLibrary, WMIConnection, Variant};
            use std::rc::Rc;
            let mut engine_class = None;
            let mut memory_class = None;
            if let Some(com) = COMLibrary::new().ok() {
                if let Ok(wmi) = WMIConnection::new(Rc::new(com)) {
                    // Try GPUPerformanceCounters first (more common on modern Windows)
                    for prefix in &["GPUPerformanceCounters", "GPUPerformanceMonitors"] {
                        if engine_class.is_none() {
                            let q = format!("SELECT UtilizationPercentage FROM Win32_PerfFormattedData_{}_GPUEngine", prefix);
                            if wmi.raw_query::<std::collections::HashMap<String, Variant>>(&q).is_ok() {
                                engine_class = Some(format!("Win32_PerfFormattedData_{}_GPUEngine", prefix));
                            }
                        }
                        if memory_class.is_none() {
                            let q = format!("SELECT LocalUsage FROM Win32_PerfFormattedData_{}_GPULocalAdapterMemory", prefix);
                            if wmi.raw_query::<std::collections::HashMap<String, Variant>>(&q).is_ok() {
                                memory_class = Some(format!("Win32_PerfFormattedData_{}_GPULocalAdapterMemory", prefix));
                            }
                        }
                    }
                }
            }
            (engine_class, memory_class)
        };

        SystemMonitor {
            sys,
            disks,
            networks,
            #[cfg(target_os = "windows")]
            nvml,
            #[cfg(target_os = "windows")]
            wmi_gpu_engine_class,
            #[cfg(target_os = "windows")]
            wmi_gpu_memory_class,
            last_network_update: Instant::now(),
            last_disk_update: Instant::now(),
        }
    }

    fn refresh(&mut self) {
        self.sys.refresh_all();
        self.disks.refresh();
        self.networks.refresh();
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
                    cpu_usage: process.cpu_usage(),
                    memory: process.memory(),
                    status: format!("{:?}", process.status()),
                }
            })
            .collect();

        processes.sort_by(|a, b| b.memory.cmp(&a.memory));
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

    #[allow(dead_code)]
    fn kill_process(&mut self, pid: u32) -> bool {
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
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};
        use ntapi::ntpsapi::NtSuspendProcess;

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
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};
        use ntapi::ntpsapi::NtResumeProcess;

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
    fn get_battery_info(&self) -> Option<BatteryInfo> {
        // Use native Win32 API — no PowerShell process spawning
        use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
        let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
        let result = unsafe { GetSystemPowerStatus(&mut status) };
        if result.is_err() {
            return None;
        }
        // BatteryFlag 128 = no battery present
        if status.BatteryFlag == 128 {
            return None;
        }
        let percentage = if status.BatteryLifePercent <= 100 {
            status.BatteryLifePercent as f32
        } else {
            0.0 // 255 means unknown
        };
        // ACLineStatus: 0=Offline, 1=Online
        let is_charging = status.ACLineStatus == 1 && status.BatteryLifePercent < 100;
        let status_text = if status.ACLineStatus == 1 {
            if status.BatteryLifePercent >= 100 {
                "Fully Charged".to_string()
            } else {
                "Charging".to_string()
            }
        } else {
            if percentage <= 10.0 {
                "Critical".to_string()
            } else if percentage <= 25.0 {
                "Low".to_string()
            } else {
                "Discharging".to_string()
            }
        };
        Some(BatteryInfo {
            percentage,
            is_charging,
            status_text,
        })
    }

    #[cfg(not(target_os = "windows"))]
    fn get_battery_info(&self) -> Option<BatteryInfo> {
        None
    }

    #[cfg(target_os = "windows")]
    fn clean_ram(&mut self) -> u64 {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_ACCESS_RIGHTS,
        };

        info!("RAM clean operation initiated (native API)");
        let mem_before = self.sys.used_memory();
        let mut success_count = 0;
        let mut fail_count = 0;

        unsafe {
            for (pid, _) in self.sys.processes() {
                let pid_u32 = pid.as_u32();
                if let Ok(h) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_ACCESS_RIGHTS(0x0100), false, pid_u32) {
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
    fn clean_ram(&mut self) -> u64 {
        0
    }

    // Startup item collection and actions are now in startup.rs module

    #[cfg(target_os = "windows")]
    fn set_process_priority(pid: u32, priority: &str) -> bool {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{
            OpenProcess, SetPriorityClass, PROCESS_CREATION_FLAGS,
        };

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
            if let Ok(h) = OpenProcess(
                windows::Win32::System::Threading::PROCESS_SET_INFORMATION,
                false,
                pid,
            ) {
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

                        nvml_names.push(name.clone());
                        gpus.push(GpuInfo {
                            name,
                            utilization: utilization as f32,
                            memory_used: memory.as_ref().map(|m| m.used),
                            memory_total: memory.as_ref().map(|m| m.total),
                            temperature,
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
    fn get_gpu_info_wmi(&self) -> Option<Vec<GpuInfo>> {
        use wmi::{COMLibrary, WMIConnection, Variant};
        use std::rc::Rc;
        
        let com = Rc::new(COMLibrary::new().ok()?);
        let wmi = WMIConnection::new(com).ok()?;
        
        // Query all GPU controllers
        let results: Vec<std::collections::HashMap<String, Variant>> = wmi
            .raw_query("SELECT Name, DriverVersion, VideoProcessor, AdapterRAM FROM Win32_VideoController")
            .ok()?;
        
        if results.is_empty() {
            return None;
        }
        
        let mut gpus = Vec::new();

        for gpu_entry in &results {
            let name = gpu_entry.get("Name")
                .and_then(|v| match v { Variant::String(s) => Some(s.clone()), _ => None })
                .unwrap_or_else(|| "Unknown GPU".to_string());
            
            // Filter out generic/virtual adapters
            if name.contains("Microsoft Basic Display Adapter") || name.contains("Standard VGA") {
                continue;
            }

            // Get adapter RAM for this entry
            let adapter_ram = gpu_entry.get("AdapterRAM").and_then(|v| match v {
                Variant::UI4(n) => Some(*n as u64),
                Variant::UI8(n) => Some(*n),
                Variant::I4(n) => Some(*n as u64),
                _ => None,
            });
            
            // Query real-time GPU utilization using cached class name
            let mut utilization = 0.0;
            if let Some(ref engine_class) = self.wmi_gpu_engine_class {
                let q = format!("SELECT Name, UtilizationPercentage FROM {}", engine_class);
                if let Ok(perf_results) = wmi.raw_query::<std::collections::HashMap<String, Variant>>(&q) {
                    let mut max_util = 0u64;
                    for engine in perf_results {
                        if let Some(val) = engine.get("UtilizationPercentage") {
                            let u = match val {
                                Variant::UI1(n) => *n as u64,
                                Variant::UI2(n) => *n as u64,
                                Variant::UI4(n) => *n as u64,
                                Variant::UI8(n) => *n,
                                Variant::I1(n) => *n as u64,
                                Variant::I2(n) => *n as u64,
                                Variant::I4(n) => *n as u64,
                                Variant::I8(n) => *n as u64,
                                Variant::String(s) => s.parse().unwrap_or(0),
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

            // Query real-time VRAM usage using cached class name
            let mut memory_used = None;
            if let Some(ref mem_class) = self.wmi_gpu_memory_class {
                let q = format!("SELECT LocalUsage FROM {}", mem_class);
                if let Ok(mem_results) = wmi.raw_query::<std::collections::HashMap<String, Variant>>(&q) {
                    let mut total_used = 0u64;
                    for instance in mem_results {
                        if let Some(val) = instance.get("LocalUsage") {
                            let u = match val {
                                Variant::UI1(n) => *n as u64,
                                Variant::UI2(n) => *n as u64,
                                Variant::UI4(n) => *n as u64,
                                Variant::UI8(n) => *n,
                                Variant::I1(n) => *n as u64,
                                Variant::I2(n) => *n as u64,
                                Variant::I4(n) => *n as u64,
                                Variant::I8(n) => *n as u64,
                                Variant::String(s) => s.parse().unwrap_or(0),
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
            });
        }
        
        if gpus.is_empty() { None } else { Some(gpus) }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_gpu_info(&self, _include_wmi: bool) -> Vec<GpuInfo> {
        Vec::new()
    }

    #[cfg(target_os = "windows")]
    fn get_cpu_temperature_wmi() -> Option<f32> {
        use wmi::{COMLibrary, WMIConnection, Variant};
        use std::rc::Rc;
        
        let com = Rc::new(COMLibrary::new().ok()?);
        // The MSAcpi_ThermalZoneTemperature class is in the \root\wmi namespace
        let wmi = WMIConnection::with_namespace_path("ROOT\\WMI", com).ok()?;
        
        let results = wmi.raw_query::<std::collections::HashMap<String, Variant>>(
            "SELECT CurrentTemperature FROM MSAcpi_ThermalZoneTemperature"
        ).ok()?;

        if results.is_empty() {
            return None;
        }

        // Temperature is in tenths of degrees Kelvin
        if let Some(val) = results[0].get("CurrentTemperature") {
            let temp_k_tenths = match val {
                Variant::UI4(n) => *n as f32,
                Variant::I4(n) => *n as f32,
                Variant::UI8(n) => *n as f32,
                _ => return None,
            };
            
            // Convert to Celsius: (K / 10) - 273.15
            let temp_c = (temp_k_tenths / 10.0) - 273.15;
            return Some(temp_c.round());
        }
        
        None
    }

    #[cfg(not(target_os = "windows"))]
    fn get_cpu_temperature_wmi() -> Option<f32> {
        None
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

    fn get_disk_io(&mut self) -> (f64, f64) {
        let elapsed = self.last_disk_update.elapsed().as_secs_f64();
        let mut total_read = 0;
        let mut total_written = 0;

        for process in self.sys.processes().values() {
            let usage = process.disk_usage();
            total_read += usage.read_bytes;
            total_written += usage.written_bytes;
        }

        self.last_disk_update = Instant::now();

        let read_rate = if elapsed > 0.0 {
            total_read as f64 / elapsed / 1024.0 / 1024.0 // MB/s
        } else {
            0.0
        };

        let write_rate = if elapsed > 0.0 {
            total_written as f64 / elapsed / 1024.0 / 1024.0 // MB/s
        } else {
            0.0
        };

        (read_rate, write_rate)
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

        alerts
    }

    fn get_network_info(&mut self) -> Vec<NetworkInfo> {
        let elapsed = self.last_network_update.elapsed().as_secs_f64();

        let network_info: Vec<NetworkInfo> = self
            .networks
            .iter()
            .map(|(interface, data)| {
                let received_rate = if elapsed > 0.0 {
                    data.received() as f64 / elapsed / 1024.0 / 1024.0 // MB/s
                } else {
                    0.0
                };
                let transmitted_rate = if elapsed > 0.0 {
                    data.transmitted() as f64 / elapsed / 1024.0 / 1024.0 // MB/s
                } else {
                    0.0
                };

                NetworkInfo {
                    interface: interface.clone(),
                    received: data.received(),
                    transmitted: data.transmitted(),
                    received_rate,
                    transmitted_rate,
                }
            })
            .collect();

        self.last_network_update = Instant::now();
        network_info
    }

    fn get_system_info(&self) -> SystemInfo {
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
        }
    }
}

// Historical data point
#[derive(Clone, Copy)]
struct DataPoint {
    time: f64,
    value: f64,
}

// Shared state between threads
#[derive(Clone)]
struct SystemData {
    memory_total: u64,
    memory_used: u64,
    memory_percentage: f32,
    cpu_usage: f32,
    cpu_cores: Vec<CpuCoreInfo>,
    gpu_info: Vec<GpuInfo>,
    top_processes: Vec<ProcessInfo>,
    disk_info: Vec<DiskInfo>,
    network_info: Vec<NetworkInfo>,
    system_info: SystemInfo,
    cpu_temperature: Option<f32>,
    last_update: String,
    cpu_history: VecDeque<DataPoint>,
    memory_history: VecDeque<DataPoint>,
    gpu_history: VecDeque<DataPoint>,
    network_download_history: VecDeque<DataPoint>,
    network_upload_history: VecDeque<DataPoint>,
    alerts: Vec<AlertInfo>,
    start_time: Instant,
    swap_info: SwapInfo,
    battery_info: Option<BatteryInfo>,
    network_sample_count: u32,
    ram_clean_freed_bytes: u64,
    ram_clean_is_cleaning: bool,
    disk_read_rate: f64,
    disk_write_rate: f64,
    disk_read_history: VecDeque<DataPoint>,
    disk_write_history: VecDeque<DataPoint>,
    is_hidden: bool,
    selected_tab: Tab,
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
            },
            cpu_temperature: None,
            last_update: String::new(),
            cpu_history: VecDeque::new(),
            memory_history: VecDeque::new(),
            gpu_history: VecDeque::new(),
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
            ram_clean_freed_bytes: 0,
            ram_clean_is_cleaning: false,
            disk_read_rate: 0.0,
            disk_write_rate: 0.0,
            disk_read_history: VecDeque::new(),
            disk_write_history: VecDeque::new(),
            is_hidden: false,
            selected_tab: Tab::Overview,
        }
    }
}

struct SystemMonitorApp {
    data: Arc<Mutex<SystemData>>,
    settings: AppSettings,
    shared_settings: Arc<Mutex<AppSettings>>,
    selected_tab: Tab,
    show_settings: bool,
    show_export: bool,
    show_alerts: bool,
    show_process_manager: bool,
    #[allow(dead_code)]
    show_cpu_cores: bool,
    selected_process_pid: Option<u32>,
    process_search: String,
    process_sort_column: ProcessSortColumn,
    process_sort_ascending: bool,
    show_export_csv: bool,
    updater: updater::Updater,
    update_info_share: Arc<Mutex<Option<updater::UpdateInfo>>>,
    show_update_notification: bool,
    update_check_time: Option<Instant>,
    ram_cleaner_state: RamCleanerState,
    startup_items: Vec<StartupItem>,
    startup_items_loaded: bool,
    startup_items_loading: bool,
    startup_items_share: Arc<Mutex<Option<Vec<StartupItem>>>>,
    startup_search: String,
    startup_sort: StartupSortColumn,
    startup_sort_ascending: bool,
    startup_filter_impact: Option<ImpactTier>,
    startup_filter_signed: Option<bool>,
    startup_filter_broken: bool,
    startup_show_confirm: Option<usize>,
    boot_diagnostics: Option<BootDiagnostics>,
    boot_diagnostics_loaded: bool,
    boot_diagnostics_share: Arc<Mutex<Option<BootDiagnostics>>>,
    show_shortcuts: bool,
    suspend_process_pid: Option<u32>,
    resume_process_pid: Option<u32>,
    suspended_pids: std::collections::HashSet<u32>,
    priority_change: Option<(u32, String)>,
    #[allow(dead_code)]
    #[cfg(target_os = "windows")]
    tray_icon: Option<tray_icon::TrayIcon>,
    #[cfg(target_os = "windows")]
    tray_menu_show_id: Option<tray_icon::menu::MenuId>,
    #[cfg(target_os = "windows")]
    tray_menu_quit_id: Option<tray_icon::menu::MenuId>,
    is_hidden: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
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
    About,
}

impl SystemMonitorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        println!("DBG: SystemMonitorApp::new started");
        // Install image loaders for showing the logo
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Load settings
        let settings = AppSettings::load();
        println!("DBG: Settings loaded successfully");

        // Load Windows system fonts at runtime to support all standard symbols and checkmarks
        #[cfg(target_os = "windows")]
        {
            let mut fonts = egui::FontDefinitions::default();
            let mut proportional_loaded = false;
            let mut monospace_loaded = false;

            // Load Segoe UI for standard proportional text
            let font_paths = [
                "C:\\Windows\\Fonts\\segoeui.ttf",
                "C:\\Windows\\Fonts\\SegoeUI.ttf",
            ];
            for path in &font_paths {
                if let Ok(font_bytes) = std::fs::read(path) {
                    fonts.font_data.insert(
                        "segoe_ui".to_owned(),
                        egui::FontData::from_owned(font_bytes),
                    );
                    fonts.families.entry(egui::FontFamily::Proportional)
                        .or_default()
                        .insert(0, "segoe_ui".to_owned());
                    proportional_loaded = true;
                    break;
                }
            }

            // Load Consolas for monospace text
            let mono_paths = [
                "C:\\Windows\\Fonts\\consola.ttf",
                "C:\\Windows\\Fonts\\Consola.ttf",
            ];
            for path in &mono_paths {
                if let Ok(font_bytes) = std::fs::read(path) {
                    fonts.font_data.insert(
                        "consolas".to_owned(),
                        egui::FontData::from_owned(font_bytes),
                    );
                    fonts.families.entry(egui::FontFamily::Monospace)
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
        style.spacing.item_spacing = egui::vec2(12.0, 10.0);
        style.spacing.button_padding = egui::vec2(16.0, 8.0);
        style.spacing.window_margin = egui::Margin::same(16.0);
        style.spacing.menu_margin = egui::Margin::same(10.0);

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

            // Subtle borders & widgets (No borders)
            visuals.widgets.noninteractive.bg_fill = ThemePalette::BG_CARD;
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_PRIMARY);

            // Inactive
            visuals.widgets.inactive.bg_fill = ThemePalette::WIDGET_INACTIVE;
            visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SECONDARY);

            // Hovered
            visuals.widgets.hovered.bg_fill = ThemePalette::WIDGET_HOVERED;
            visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);

            // Active
            visuals.widgets.active.bg_fill = ThemePalette::ACCENT_ACTIVE;
            visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);

            // Rounding (Sharp minimal 4px)
            visuals.window_rounding = egui::Rounding::same(4.0);
            visuals.menu_rounding = egui::Rounding::same(4.0);
            visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
            visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
            visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
            visuals.widgets.active.rounding = egui::Rounding::same(4.0);

            // Window chrome and depth
            visuals.window_stroke = egui::Stroke::NONE;
            visuals.window_shadow = egui::epaint::Shadow {
                offset: egui::vec2(0.0, 8.0),
                blur: 40.0,
                spread: 0.0,
                color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 20), // Ambient 8%
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

        println!("DBG: Creating SystemData::default()");
        let data = Arc::new(Mutex::new(SystemData::default()));
        let data_clone = Arc::clone(&data);
        let shared_settings = Arc::new(Mutex::new(settings.clone()));
        let shared_settings_clone = Arc::clone(&shared_settings);

        // Background thread for monitoring
        println!("DBG: Spawning background monitoring thread");
        thread::spawn(move || {
            println!("DBG: Background monitoring thread started running");
            let mut monitor = SystemMonitor::new();
            println!("DBG: SystemMonitor::new finished in background thread");

            // Get system info once (doesn't change)
            let system_info = monitor.get_system_info();
            let mut battery_check_counter: u32 = 0;
            let mut last_alert_time: std::collections::HashMap<AlertType, Instant> = std::collections::HashMap::new();
            let mut last_hidden_tick = Instant::now();

            loop {
                thread::sleep(Duration::from_millis(500));

                // Read hidden status and current tab
                let (is_hidden, selected_tab) = {
                    let data = data_clone.lock();
                    (data.is_hidden, data.selected_tab)
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

                if is_hidden {
                    last_hidden_tick = Instant::now();
                }

                // If minimized refresh: only refresh CPU and RAM to check alerts & update tray tooltip
                if is_hidden {
                    monitor.sys.refresh_cpu();
                    monitor.sys.refresh_memory();
                } else {
                    monitor.refresh();
                }

                let (total_mem, used_mem, mem_percentage) = monitor.get_memory_info();
                let cpu_usage = monitor.get_cpu_usage();

                // Optimized queries
                let need_cpu_cores = !is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::CpuCores);
                let need_cpu_temp = !is_hidden && selected_tab == Tab::Overview;
                let need_gpu_wmi = !is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::Performance);
                let need_gpu_info = need_gpu_wmi || settings_snapshot.show_notifications || settings_snapshot.show_graphs;
                let need_processes = !is_hidden && selected_tab == Tab::Processes;
                let need_disks = (!is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::Storage)) || settings_snapshot.show_notifications || settings_snapshot.show_graphs;
                let need_network = !is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::Network || selected_tab == Tab::Performance);

                let cpu_cores = if need_cpu_cores {
                    monitor.get_cpu_cores_info()
                } else {
                    Vec::new()
                };

                let cpu_temperature = if need_cpu_temp {
                    SystemMonitor::get_cpu_temperature_wmi()
                } else {
                    None
                };

                let gpu_info = if need_gpu_info {
                    monitor.get_gpu_info(need_gpu_wmi)
                } else {
                    Vec::new()
                };

                let top_processes = if need_processes {
                    monitor.get_top_processes(process_count)
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

                let swap_info = monitor.get_swap_info();
                
                let (disk_read_rate, disk_write_rate) = if !is_hidden {
                    monitor.get_disk_io()
                } else {
                    (0.0, 0.0)
                };

                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

                // Get battery info every 15 ticks (~7.5s) — retain previous value if unavailable
                if battery_check_counter % 15 == 0 {
                    if let Some(bi) = monitor.get_battery_info() {
                        {
                            let mut data = data_clone.lock();
                            data.battery_info = Some(bi);
                        }
                    }
                }
                battery_check_counter = battery_check_counter.wrapping_add(1);

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

                    // Check for alerts
                    let new_alerts = monitor.check_alerts(&settings_snapshot, &data);

                    if !new_alerts.is_empty() && settings_snapshot.enable_sounds {
                        play_alert_sound();
                    }

                    // Send desktop notifications for new alerts with a 5-minute cooldown
                    if settings_snapshot.show_notifications {
                        for alert in &new_alerts {
                            let now = Instant::now();
                            let should_notify = last_alert_time.get(&alert.alert_type)
                                .map_or(true, |&last| now.duration_since(last).as_secs() > 300);
                            
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
                        data.alerts.retain(|alert| {
                            match alert.alert_type {
                                AlertType::CpuHigh => cpu_usage > settings_snapshot.notification_cpu_threshold,
                                AlertType::MemoryHigh => {
                                    mem_percentage > settings_snapshot.notification_memory_threshold
                                }
                                AlertType::GpuTempHigh => {
                                    temp_gpu_info.first()
                                        .and_then(|g| g.temperature)
                                        .map_or(false, |t| t > settings_snapshot.notification_temp_threshold)
                                }
                                AlertType::DiskSpaceLow => true, // disk alerts don't auto-clear
                                AlertType::StartupHighImpact => true,
                            }
                        });
                    }

                    // Keep only last 10 alerts
                    while data.alerts.len() > 10 {
                        data.alerts.remove(0);
                    }

                    // Update history (keep last 60 data points = 2 minutes)
                    data.cpu_history.push_back(DataPoint {
                        time: elapsed,
                        value: cpu_usage as f64,
                    });
                    data.memory_history.push_back(DataPoint {
                        time: elapsed,
                        value: mem_percentage as f64,
                    });

                    if need_gpu_info {
                        let gpu_util = data.gpu_info.first().map(|gpu| gpu.utilization as f64);
                        if let Some(val) = gpu_util {
                            data.gpu_history.push_back(DataPoint {
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

                    // Keep only last 60 points
                    while data.cpu_history.len() > 60 {
                        data.cpu_history.pop_front();
                    }
                    while data.memory_history.len() > 60 {
                        data.memory_history.pop_front();
                    }
                    while data.gpu_history.len() > 60 {
                        data.gpu_history.pop_front();
                    }
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

                if is_hidden {
                    // Minimized: sleep 10s
                    thread::sleep(Duration::from_millis(10000));
                } else {
                    let sleep_ms = (refresh_interval * 1000).saturating_sub(500);
                    thread::sleep(Duration::from_millis(sleep_ms));
                }
            }
        });

        let mut tray_icon = None;
        let mut tray_menu_show_id = None;
        let mut tray_menu_quit_id = None;

        #[cfg(target_os = "windows")]
        if let Some(icon) = load_tray_icon() {
            let tray_menu = Menu::new();
            let show_i = MenuItem::new("Show Dashboard", true, None);
            let quit_i = MenuItem::new("Quit System Monitor", true, None);
            
            tray_menu_show_id = Some(show_i.id().clone());
            tray_menu_quit_id = Some(quit_i.id().clone());

            let _ = tray_menu.append_items(&[&show_i, &quit_i]);

            if let Ok(tray) = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("System Monitor")
                .with_icon(icon)
                .build()
            {
                tray_icon = Some(tray);
            }
        }

        Self {
            data,
            settings,
            shared_settings,
            selected_tab: Tab::Overview,
            show_settings: false,
            show_export: false,
            show_alerts: false,
            show_process_manager: false,
            show_cpu_cores: false,
            selected_process_pid: None,
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
                auto_clean_enabled: false,
                auto_clean_threshold: 85.0,
                auto_clean_interval: 300,
                is_cleaning: false,
                clean_count: 0,
            },
            startup_items: Vec::new(),
            startup_items_loaded: false,
            startup_items_loading: false,
            startup_items_share: Arc::new(Mutex::new(None)),
            startup_search: String::new(),
            startup_sort: StartupSortColumn::Impact,
            startup_sort_ascending: true,
            startup_filter_impact: None,
            startup_filter_signed: None,
            startup_filter_broken: false,
            startup_show_confirm: None,
            boot_diagnostics: None,
            boot_diagnostics_loaded: false,
            boot_diagnostics_share: Arc::new(Mutex::new(None)),
            show_shortcuts: false,
            suspend_process_pid: None,
            resume_process_pid: None,
            suspended_pids: std::collections::HashSet::new(),
            priority_change: None,
            #[cfg(target_os = "windows")]
            tray_icon,
            #[cfg(target_os = "windows")]
            tray_menu_show_id,
            #[cfg(target_os = "windows")]
            tray_menu_quit_id,
            is_hidden: false,
        }
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
        };

        Ok(serde_json::to_string_pretty(&export)?)
    }
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

fn format_rate(mb_per_sec: f64) -> String {
    let bytes_per_sec = mb_per_sec * 1024.0 * 1024.0;
    if bytes_per_sec >= 1_073_741_824.0 {
        format!("{:.2} GB/s", bytes_per_sec / 1_073_741_824.0)
    } else if bytes_per_sec >= 1_048_576.0 {
        format!("{:.2} MB/s", bytes_per_sec / 1_048_576.0)
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.0} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

fn get_usage_color(percentage: f32) -> egui::Color32 {
    if percentage < 50.0 {
        ThemePalette::STATUS_HEALTHY // Mint green (#69f0ae)
    } else if percentage < 75.0 {
        ThemePalette::STATUS_WARNING // Amber (#ffab40)
    } else {
        ThemePalette::STATUS_CRITICAL // Saturated red (#ff5252)
    }
}

impl eframe::App for SystemMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        {
            let mut data = self.data.lock();
            data.is_hidden = self.is_hidden;
            data.selected_tab = self.selected_tab;
        }

        #[cfg(target_os = "windows")]
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if Some(&event.id) == self.tray_menu_quit_id.as_ref() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if Some(&event.id) == self.tray_menu_show_id.as_ref() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.is_hidden = false;
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            if self.settings.minimize_to_tray {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                self.is_hidden = true;
            }
        }

        // Update tray tooltip with CPU/RAM usage
        #[cfg(target_os = "windows")]
        if let Some(tray) = &mut self.tray_icon {
            let data = self.data.lock();
            let tooltip = format!("SysMon: CPU {:.0}% | RAM {:.0}%", data.cpu_usage, data.memory_percentage);
            let _ = tray.set_tooltip(Some(tooltip));
        }

        // Ensure repaint for continuous updates but without CPU lock
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // Check for updates automatically (once every 24 hours)
        if self.update_check_time.is_none() || self.update_check_time.unwrap().elapsed().as_secs() > 86400 {
            let mut updater = self.updater.clone();
            let ctx_clone = ctx.clone();
            let update_info_share = self.update_info_share.clone();
            thread::spawn(move || {
                if let Ok(update_info) = updater.check_for_updates() {
                    *update_info_share.lock() = Some(update_info.clone());
                    if update_info.update_available {
                        ctx_clone.request_repaint();
                    }
                }
            });
            self.update_check_time = Some(Instant::now());
        }

        // Show update notification banner
        let update_available = {
            let info = self.update_info_share.lock();
            info.as_ref().map_or(false, |i| i.update_available)
        };
        if update_available && self.show_update_notification {
            let update_info = {
                let info = self.update_info_share.lock();
                info.as_ref().unwrap().clone()
            };
            egui::TopBottomPanel::top("update_notification").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(ThemePalette::STATUS_HEALTHY, "🎉");
                    ui.label(format!(
                        "New version {} is available! Current: {}",
                        update_info.latest_version,
                        update_info.current_version
                    ));
                    if ui.button("⬇️ Download & Install").clicked() {
                        let download_url = update_info.download_url.clone();
                        thread::spawn(move || {
                            if let Err(e) = updater::Updater::new().download_and_install_update(&download_url) {
                                eprintln!("Update failed: {}", e);
                            }
                        });
                    }
                    if ui.button("✖").clicked() {
                        self.show_update_notification = false;
                    }
                });
            });
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
                let ctx_clone = ctx.clone();
                thread::spawn(move || {
                    if let Ok(update_info) = updater.check_for_updates() {
                        *update_info_share.lock() = Some(update_info);
                        ctx_clone.request_repaint();
                    }
                });
            }
        });

        let data = self.data.lock().clone();

        // Handle process kill actions
        if let Some(pid) = self.selected_process_pid.take() {
            let mut temp_sys = System::new();
            temp_sys.refresh_processes();
            if let Some(process) = temp_sys.process(Pid::from_u32(pid)) {
                let success = process.kill();
                if success {
                    if self.settings.enable_sounds { play_success_sound(); }
                } else {
                    if self.settings.enable_sounds { play_alert_sound(); }
                    let _ = notify_rust::Notification::new()
                        .summary("Action Failed")
                        .body("Failed to kill process. Access Denied (requires Administrator privileges).")
                        .timeout(notify_rust::Timeout::Milliseconds(5000))
                        .show();
                }
            }
        }

        // Handle process suspend actions
        if let Some(pid) = self.suspend_process_pid.take() {
            let mut monitor = SystemMonitor::new();
            if monitor.suspend_process(pid) {
                self.suspended_pids.insert(pid);
                if self.settings.enable_sounds { play_success_sound(); }
            } else {
                if self.settings.enable_sounds { play_alert_sound(); }
                let _ = notify_rust::Notification::new()
                    .summary("Action Failed")
                    .body("Failed to suspend process. Access Denied (requires Administrator privileges).")
                    .timeout(notify_rust::Timeout::Milliseconds(5000))
                    .show();
            }
        }

        // Handle process resume actions
        if let Some(pid) = self.resume_process_pid.take() {
            let mut monitor = SystemMonitor::new();
            if monitor.resume_process(pid) {
                self.suspended_pids.remove(&pid);
                if self.settings.enable_sounds { play_success_sound(); }
            } else {
                if self.settings.enable_sounds { play_alert_sound(); }
                let _ = notify_rust::Notification::new()
                    .summary("Action Failed")
                    .body("Failed to resume process. Access Denied (requires Administrator privileges).")
                    .timeout(notify_rust::Timeout::Milliseconds(5000))
                    .show();
            }
        }

        // Handle process priority changes
        if let Some((pid, priority)) = self.priority_change.take() {
            if SystemMonitor::set_process_priority(pid, &priority) {
                if self.settings.enable_sounds { play_success_sound(); }
            } else {
                if self.settings.enable_sounds { play_alert_sound(); }
                let _ = notify_rust::Notification::new()
                    .summary("Action Failed")
                    .body("Failed to set process priority. Access Denied (requires Administrator privileges).")
                    .timeout(notify_rust::Timeout::Milliseconds(5000))
                    .show();
            }
        }

        // Auto RAM cleaning
        if self.ram_cleaner_state.auto_clean_enabled && !self.ram_cleaner_state.is_cleaning {
            let should_clean = if let Some(last) = self.ram_cleaner_state.last_cleaned {
                last.elapsed().as_secs() >= self.ram_cleaner_state.auto_clean_interval
                    && data.memory_percentage >= self.ram_cleaner_state.auto_clean_threshold
            } else {
                data.memory_percentage >= self.ram_cleaner_state.auto_clean_threshold
            };
            if should_clean {
                self.ram_cleaner_state.is_cleaning = true;
                self.ram_cleaner_state.last_cleaned = Some(Instant::now());
                self.ram_cleaner_state.last_cleaned_display = Local::now().format("%H:%M:%S").to_string();
                self.ram_cleaner_state.clean_count += 1;
                let data_arc = Arc::clone(&self.data);
                let ctx_clone = ctx.clone();
                let enable_sounds = self.settings.enable_sounds;
                thread::spawn(move || {
                    let mut monitor = SystemMonitor::new();
                    let freed = monitor.clean_ram();
                    if enable_sounds { play_success_sound(); }
                    // Store freed bytes in SystemData for the UI to pick up
                    {
                        let mut d = data_arc.lock();
                        d.ram_clean_freed_bytes += freed;
                        d.ram_clean_is_cleaning = false;
                    }
                    ctx_clone.request_repaint();
                });
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
                                        .set_file_name(&format!("sysmon_export_{}.csv", date_str))
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
                                        .set_file_name(&format!("sysmon_export_{}.json", date_str))
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
                ];

                ui.spacing_mut().item_spacing.y = 4.0;
                for (tab, label) in tabs {
                    let is_selected = self.selected_tab == tab;
                    let text = if is_selected {
                        egui::RichText::new(label).strong().color(ThemePalette::BG_DEEPEST)
                    } else {
                        egui::RichText::new(label).color(ThemePalette::TEXT_SECONDARY)
                    };
                    
                    let btn = egui::Button::new(text)
                        .fill(if is_selected { ThemePalette::ACCENT_ACTIVE } else { egui::Color32::TRANSPARENT })
                        .frame(is_selected);
                        
                    if ui.add_sized([ui.available_width(), 32.0], btn).clicked() {
                        self.selected_tab = tab;
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new(format!("Updated: {}", data.last_update)).size(11.0).color(ThemePalette::TEXT_DIMMED));
                    ui.add_space(8.0);
                    if ui.add_sized([ui.available_width(), 28.0], egui::Button::new("Settings")).clicked() { self.show_settings = true; }
                    ui.add_space(4.0);
                    if ui.add_sized([ui.available_width(), 28.0], egui::Button::new("About")).clicked() { self.selected_tab = Tab::About; }
                });
            });

        // Process Manager window
        if self.show_process_manager {
            self.show_process_manager_window(ctx, &data);
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
                    self.show_settings_tab(ui);
                });
            self.show_settings = show_settings;
        }

        // Global always-visible status bar header
        let status_bar_frame = egui::Frame::none()
            .fill(ctx.style().visuals.extreme_bg_color)
            .inner_margin(egui::Margin::symmetric(16.0, 0.0))
            .stroke(egui::Stroke::new(1.0, ctx.style().visuals.widgets.noninteractive.bg_stroke.color));

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
                    ui.colored_label(mem_c, egui::RichText::new(format!("{:.1}%", data.memory_percentage)).strong());
                    
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
                            let btn = ui.button(egui::RichText::new(format!("{} Alerts", recent_alerts)).color(ThemePalette::STATUS_WARNING));
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
            Tab::Overview => self.show_overview_tab(ui, &data),
            Tab::Performance => self.show_performance_tab(ui, &data),
            Tab::Processes => self.show_processes_tab(ui, &data),
            Tab::CpuCores => self.show_cpu_cores_tab(ui, &data),
            Tab::Storage => self.show_storage_tab(ui, &data),
            Tab::Network => self.show_network_tab(ui, &data),
            Tab::SystemInfo => self.show_system_info_tab(ui, &data),
            Tab::Alerts => self.show_alerts_tab(ui, &data),
            Tab::RamCleaner => self.show_ram_cleaner_tab(ui, &data),
            Tab::StartupManager => self.show_startup_manager_tab(ui),
            Tab::About => self.show_about_tab(ui, &data),
        });
    }
}

// ─── Custom UI helpers ───────────────────────────────────────────────

/// Section header with sleek gradient-like accent underline
fn paint_section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    let r = ui.label(
        egui::RichText::new(text)
            .text_style(egui::TextStyle::Heading)
            .strong()
            .color(ThemePalette::TEXT_PRIMARY),
    );
    let y = r.rect.bottom() + 4.0;

    // Modern thick rounded line highlight
    let underline_w = r.rect.width();
    ui.painter().line_segment(
        [egui::pos2(r.rect.left(), y), egui::pos2(r.rect.left() + underline_w, y)],
        egui::Stroke::new(3.5, ThemePalette::ACCENT_PRIMARY),
    );
    ui.add_space(12.0);
}

/// Rounded pill progress bar with subtle track
fn paint_progress_bar(ui: &mut egui::Ui, fraction: f32, fill: egui::Color32, h: f32) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let rnd = h / 2.0;

    // Track background
    ui.painter().rect_filled(rect, rnd, ThemePalette::BG_DEEPEST);
    ui.painter()
        .rect_stroke(rect, rnd, egui::Stroke::new(1.0, ThemePalette::BG_TRACK));

    let frac = fraction.clamp(0.0, 1.0);
    if frac > 0.005 {
        let bar = egui::Rect::from_min_size(rect.min, egui::vec2(w * frac, h));
        ui.painter().rect_filled(bar, rnd, fill);
    }
}

/// Circular animated gauge for premium UI
fn paint_circular_gauge(ui: &mut egui::Ui, center: egui::Pos2, radius: f32, fraction: f32, color: egui::Color32, label: &str) {
    let p = ui.painter();
    let track_color = ThemePalette::BG_TRACK;
    
    // Track
    p.circle_stroke(center, radius, egui::Stroke::new(6.0, track_color));
    
    // Animate fraction if we had time context, but for now we draw the arc
    let frac = fraction.clamp(0.0, 1.0);
    if frac > 0.005 {
        use std::f32::consts::PI;
        // Start from top (-PI/2), sweep clockwise
        let start_angle = -PI / 2.0;
        let end_angle = start_angle + (frac * 2.0 * PI);
        
        let path: Vec<egui::Pos2> = (0..=30).map(|i| {
            let t = i as f32 / 30.0;
            let angle = start_angle + (end_angle - start_angle) * t;
            center + egui::vec2(angle.cos() * radius, angle.sin() * radius)
        }).collect();

        // Outer glow
        p.add(egui::Shape::line(
            path.clone(),
            egui::Stroke::new(12.0, color.linear_multiply(0.15)),
        ));

        // Main arc
        p.add(egui::Shape::line(
            path,
            egui::Stroke::new(6.0, color),
        ));
    }
    
    // Label in center
    let text_color = ThemePalette::TEXT_PRIMARY;
    p.text(
        center,
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(16.0),
        text_color,
    );
}

impl SystemMonitorApp {
    fn show_overview_tab(&mut self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "System Overview");

        // Show loading state until first data arrives
        if data.memory_total == 0 {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Collecting system data...")
                        .size(18.0)
                        .color(ThemePalette::TEXT_SUBTITLE),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Please wait a moment.")
                        .size(13.0)
                        .color(ThemePalette::TEXT_DIMMED),
                );
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Metric cards row ──
            let card_bg = ThemePalette::BG_CARD;
            let card_border = egui::Stroke::new(1.0, ThemePalette::BORDER);
            let card_rnd = egui::Rounding::same(12.0); // Premium smooth rounding

            let full_avail = ui.available_width();
            let card_spacing = 16.0;
            let card_h = 120.0;
            let (row_rect, _) = ui.allocate_exact_size(egui::vec2(full_avail, card_h), egui::Sense::hover());

            // Account for HiDPI: at ppp>1, available_width can exceed visible area
            let ppp = ui.ctx().pixels_per_point();
            let visible_w = if ppp > 1.01 {
                let screen_w = ui.ctx().screen_rect().width();
                (screen_w / ppp - row_rect.min.x).max(200.0)
            } else {
                full_avail
            };
            let card_w = ((visible_w - card_spacing * 4.0) / 5.0).max(80.0);

            // Prepare card data
            let cpu_c = get_usage_color(data.cpu_usage);
            let mem_c = get_usage_color(data.memory_percentage);

            let net_total_rate = data.network_info.iter().map(|n| n.received_rate + n.transmitted_rate).sum::<f64>();
            let net_download_rate = data.network_info.iter().map(|n| n.received_rate).sum::<f64>();
            let net_upload_rate = data.network_info.iter().map(|n| n.transmitted_rate).sum::<f64>();
            let net_c = if net_total_rate > 5_000_000.0 {
                ThemePalette::STATUS_WARNING
            } else if net_total_rate > 1_000_000.0 {
                ThemePalette::STATUS_HEALTHY
            } else {
                ThemePalette::TEXT_LABEL_SUB
            };

            let (gpu_val, gpu_sub, gpu_frac, gpu_c) = if let Some(gpu) = data.gpu_info.first() {
                let c = get_usage_color(gpu.utilization);
                let sub = if let (Some(u), Some(t)) = (gpu.memory_used, gpu.memory_total) {
                    format!("{:.0}/{:.0} MB", bytes_to_mb(u), bytes_to_mb(t))
                } else {
                    gpu.name.clone()
                };
                (format!("{:.1}%", gpu.utilization), sub, gpu.utilization / 100.0, c)
            } else {
                (
                    "N/A".to_string(),
                    "Not detected".to_string(),
                    0.0,
                    ThemePalette::GPU_UNAVAILABLE,
                )
            };

            let cards = [
                (
                    ThemePalette::ACCENT_PRIMARY,
                    "CPU",
                    format!("{:.1}%", data.cpu_usage),
                    if let Some(temp) = data.cpu_temperature {
                        format!("{} cores • {}°C", data.cpu_cores.len(), temp)
                    } else {
                        format!("{} cores", data.cpu_cores.len())
                    },
                    data.cpu_usage / 100.0,
                    cpu_c,
                ),
                (
                    ThemePalette::ACCENT_ACTIVE,
                    "MEMORY",
                    format!("{:.1}%", data.memory_percentage),
                    format!(
                        "{:.1} / {:.1} GB",
                        bytes_to_gb(data.memory_used),
                        bytes_to_gb(data.memory_total)
                    ),
                    data.memory_percentage / 100.0,
                    mem_c,
                ),
                (ThemePalette::ACCENT_PURPLE, "GPU", gpu_val, gpu_sub, gpu_frac, gpu_c),
                (
                    ThemePalette::TEXT_LABEL_SUB,
                    "DISK I/O",
                    format_rate(data.disk_read_rate + data.disk_write_rate),
                    format!("R: {}  W: {}", format_rate(data.disk_read_rate), format_rate(data.disk_write_rate)),
                    ((data.disk_read_rate + data.disk_write_rate) / 200.0).clamp(0.0, 1.0) as f32,
                    ThemePalette::TEXT_LABEL_SUB,
                ),
                (
                    ThemePalette::ACCENT_CYAN,
                    "NETWORK",
                    format_rate(net_total_rate),
                    format!("D: {}  U: {}", format_rate(net_download_rate), format_rate(net_upload_rate)),
                    (net_total_rate / 10_000_000.0).clamp(0.0, 1.0) as f32,
                    net_c,
                ),
            ];

            for (i, (accent, label, value, sub, frac, color)) in cards.iter().enumerate() {
                let x = row_rect.min.x + (card_w + card_spacing) * i as f32;
                let cr = egui::Rect::from_min_size(egui::pos2(x, row_rect.min.y), egui::vec2(card_w, card_h));

                // Deep card background with subtle inner border
                ui.painter().rect_filled(cr, card_rnd, ThemePalette::BG_DEEPEST);
                ui.painter().rect_filled(cr.shrink(1.0), card_rnd, card_bg);
                ui.painter().rect_stroke(cr, card_rnd, card_border);

                // Accent dot
                ui.painter().circle_filled(
                    cr.min + egui::vec2(16.0, 18.0),
                    3.0,
                    *accent,
                );

                // Title
                ui.painter().text(
                    cr.min + egui::vec2(26.0, 12.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::FontId::proportional(12.0),
                    ThemePalette::TEXT_LABEL_SUB,
                );

                // Circular Gauge
                let radius = 28.0;
                let center = cr.min + egui::vec2(card_w / 2.0, card_h / 2.0 - 4.0);
                paint_circular_gauge(ui, center, radius, *frac, *color, "");

                // Value Text inside gauge
                ui.painter().text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    value,
                    egui::FontId::new(14.0, egui::FontFamily::Monospace),
                    ThemePalette::TEXT_PRIMARY,
                );

                // Subtitle
                ui.painter().text(
                    cr.min + egui::vec2(card_w / 2.0, card_h - 18.0),
                    egui::Align2::CENTER_BOTTOM,
                    sub,
                    egui::FontId::proportional(11.0),
                    ThemePalette::TEXT_DIMMED,
                );
            }

            ui.add_space(16.0);

            // ── Detail strip ──
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if let Some(gpu) = data.gpu_info.first() {
                        if let Some(temp) = gpu.temperature {
                            let tc = if temp < 70 {
                                ThemePalette::STATUS_HEALTHY
                            } else if temp < 85 {
                                ThemePalette::STATUS_WARNING
                            } else {
                                ThemePalette::STATUS_CRITICAL
                            };
                            ui.label(egui::RichText::new(format!("{}°C", temp)).strong().color(tc));
                            ui.separator();
                        }
                        ui.label(
                            egui::RichText::new(&gpu.name)
                                .size(11.5)
                                .color(ThemePalette::TEXT_LABEL_SUB),
                        );
                        ui.separator();
                    }
                    let d = data.system_info.uptime / 86400;
                    let h = (data.system_info.uptime % 86400) / 3600;
                    let m = (data.system_info.uptime % 3600) / 60;
                    ui.label(
                        egui::RichText::new(format!("Uptime {}d {}h {}m", d, h, m))
                            .size(11.5)
                            .color(ThemePalette::TEXT_LABEL_SUB),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(&data.last_update)
                                .size(11.0)
                                .color(ThemePalette::TEXT_DIMMED),
                        );
                    });
                });
            });

            ui.add_space(12.0);

            // ── Startup Health ──
            {
                let high = startup::high_impact_count(&self.startup_items);
                let total = self.startup_items.len();
                let boot_text = self.boot_diagnostics.as_ref()
                    .and_then(|bd| bd.boot_duration_ms)
                    .map(|ms| format!("{:.1}s", ms as f64 / 1000.0));

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("STARTUP HEALTH")
                            .size(10.0).color(ThemePalette::TEXT_DIMMED));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("View All >").clicked() {
                                self.selected_tab = Tab::StartupManager;
                            }
                        });
                    });

                    ui.horizontal(|ui| {
                        if let Some(ref bt) = boot_text {
                            let boot_ms = self.boot_diagnostics.as_ref().and_then(|b| b.boot_duration_ms).unwrap_or(0);
                            let c = if boot_ms < 30000 { ThemePalette::STATUS_HEALTHY }
                                    else if boot_ms < 60000 { ThemePalette::STATUS_WARNING }
                                    else { ThemePalette::STATUS_CRITICAL };
                            ui.colored_label(c, egui::RichText::new(format!("Boot: {}", bt)).strong());
                            ui.separator();
                        }
                        if high > 0 {
                            ui.colored_label(ThemePalette::STATUS_CRITICAL,
                                format!("{} high-impact", high));
                        } else {
                            ui.colored_label(ThemePalette::STATUS_HEALTHY, "Healthy");
                        }
                        ui.separator();
                        ui.label(egui::RichText::new(format!("{} startup items", total))
                            .color(ThemePalette::TEXT_LABEL_SUB));
                    });
                });
            }

            ui.add_space(12.0);

            // ── Top processes ──
            if self.settings.show_processes {
                paint_section_header(ui, "Top Processes");

                egui::Grid::new("overview_process_grid")
                    .striped(true)
                    .spacing([14.0, 5.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("PROCESS")
                                .size(10.0)
                                .color(ThemePalette::TEXT_DIMMED),
                        );
                        ui.label(
                            egui::RichText::new("MEMORY")
                                .size(10.0)
                                .color(ThemePalette::TEXT_DIMMED),
                        );
                        ui.label(egui::RichText::new("CPU").size(10.0).color(ThemePalette::TEXT_DIMMED));
                        ui.end_row();

                        for process in data.top_processes.iter().take(8) {
                            let mb = bytes_to_mb(process.memory);
                            let mc = if mb > 500.0 {
                                ThemePalette::STATUS_CRITICAL
                            } else if mb > 200.0 {
                                ThemePalette::STATUS_WARNING
                            } else {
                                ThemePalette::STATUS_HEALTHY
                            };
                            let name = if process.name.chars().count() > 32 {
                                let truncated: String = process.name.chars().take(30).collect();
                                format!("{}…", truncated)
                            } else {
                                process.name.clone()
                            };
                            ui.label(egui::RichText::new(name).size(12.5));
                            ui.colored_label(mc, format!("{:.1} MB", mb));
                            ui.label(format!("{:.1}%", process.cpu_usage));
                            ui.end_row();
                        }
                    });
            }
        });
    }

    fn show_performance_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "Performance Graphs");

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.settings.show_graphs {
                ui.columns(2, |cols| {
                    // CPU Graph
                    cols[0].group(|ui| {
                        ui.label(
                            egui::RichText::new("CPU Usage History")
                                .size(15.0)
                                .strong()
                                .color(ThemePalette::ACCENT_PRIMARY),
                        );
                        let cpu_points: PlotPoints = data.cpu_history.iter().map(|p| [p.time, p.value]).collect();

                        let line = Line::new(cpu_points).color(ThemePalette::ACCENT_PRIMARY);

                        Plot::new("cpu_plot")
                            .height(200.0)
                            .allow_zoom(false)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .include_y(0.0)
                            .include_y(100.0)
                            .y_axis_label("CPU %")
                            .show(ui, |plot_ui| {
                                plot_ui.line(line);
                            });
                    });

                    // Memory Graph
                    cols[1].group(|ui| {
                        ui.label(
                            egui::RichText::new("Memory Usage History")
                                .size(15.0)
                                .strong()
                                .color(ThemePalette::STATUS_HEALTHY),
                        );
                        let mem_points: PlotPoints = data.memory_history.iter().map(|p| [p.time, p.value]).collect();

                        let line = Line::new(mem_points).color(ThemePalette::STATUS_HEALTHY);

                        Plot::new("memory_plot")
                            .height(200.0)
                            .allow_zoom(false)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .include_y(0.0)
                            .include_y(100.0)
                            .y_axis_label("Memory %")
                            .show(ui, |plot_ui| {
                                plot_ui.line(line);
                            });
                    });
                });

                ui.add_space(10.0);

                ui.columns(2, |cols| {
                    // GPU Graph
                    if !data.gpu_history.is_empty() {
                        cols[0].group(|ui| {
                            ui.label(
                                egui::RichText::new("GPU Usage History")
                                    .size(15.0)
                                    .strong()
                                    .color(ThemePalette::STATUS_WARNING),
                            );
                            let gpu_points: PlotPoints = data.gpu_history.iter().map(|p| [p.time, p.value]).collect();

                            let line = Line::new(gpu_points).color(ThemePalette::STATUS_WARNING);

                            Plot::new("gpu_plot")
                                .height(200.0)
                                .allow_zoom(false)
                                .allow_drag(false)
                                .allow_scroll(false)
                                .include_y(0.0)
                                .include_y(100.0)
                                .y_axis_label("GPU %")
                                .show(ui, |plot_ui| {
                                    plot_ui.line(line);
                                });
                        });
                    }

                    // Disk I/O Graph
                    cols[1].group(|ui| {
                        ui.label(
                            egui::RichText::new("Disk I/O History")
                                .size(15.0)
                                .strong()
                                .color(ThemePalette::TEXT_LABEL_SUB),
                        );
                        let read_points: PlotPoints = data.disk_read_history.iter().map(|p| [p.time, p.value]).collect();
                        let write_points: PlotPoints = data.disk_write_history.iter().map(|p| [p.time, p.value]).collect();

                        let line_r = Line::new(read_points).name("Read MB/s").color(ThemePalette::STATUS_HEALTHY);
                        let line_w = Line::new(write_points).name("Write MB/s").color(ThemePalette::STATUS_WARNING);

                        Plot::new("disk_plot")
                            .height(200.0)
                            .allow_zoom(false)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .legend(egui_plot::Legend::default())
                            .y_axis_label("MB/s")
                            .show(ui, |plot_ui| {
                                plot_ui.line(line_r);
                                plot_ui.line(line_w);
                            });
                    });
                });
            } else {
                ui.label("Performance graphs are disabled. Enable them in View menu.");
            }
        });
    }

    fn show_processes_tab(&mut self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "Process Monitor");

        // Search box
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.add(egui::TextEdit::singleline(&mut self.process_search).hint_text("Filter processes...").desired_width(200.0));
            if ui.button("x").clicked() {
                self.process_search.clear();
            }
        });

        ui.add_space(5.0);

        // Filter processes
        let mut filtered_processes: Vec<_> = if self.process_search.is_empty() {
            data.top_processes.clone()
        } else {
            let query = self.process_search.to_lowercase();
            data.top_processes
                .iter()
                .filter(|p| {
                    p.name.to_lowercase().contains(&query)
                        || p.pid.to_string().contains(&query)
                })
                .cloned()
                .collect()
        };

        // Sort processes
        let ascending = self.process_sort_ascending;
        match self.process_sort_column {
            ProcessSortColumn::Pid => {
                filtered_processes.sort_by(|a, b| {
                    if ascending {
                        a.pid.cmp(&b.pid)
                    } else {
                        b.pid.cmp(&a.pid)
                    }
                });
            }
            ProcessSortColumn::Name => {
                filtered_processes.sort_by(|a, b| {
                    let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                    if ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
            ProcessSortColumn::Memory => {
                filtered_processes.sort_by(|a, b| {
                    if ascending {
                        a.memory.cmp(&b.memory)
                    } else {
                        b.memory.cmp(&a.memory)
                    }
                });
            }
            ProcessSortColumn::Cpu => {
                filtered_processes.sort_by(|a, b| {
                    let cmp = a
                        .cpu_usage
                        .partial_cmp(&b.cpu_usage)
                        .unwrap_or(std::cmp::Ordering::Equal);
                    if ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
            ProcessSortColumn::Status => {
                // Status sorting not applicable for basic process info, fall back to memory
                filtered_processes.sort_by(|a, b| {
                    if ascending {
                        a.memory.cmp(&b.memory)
                    } else {
                        b.memory.cmp(&a.memory)
                    }
                });
            }
        }

        ui.label(format!(
            "Showing {} of {} processes",
            filtered_processes.len(),
            data.top_processes.len()
        ));
        ui.add_space(5.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("full_process_grid")
                .striped(true)
                .spacing([10.0, 4.0])
                .min_col_width(80.0)
                .show(ui, |ui| {
                    // Clickable sort headers
                    let sort_arrow = |col: ProcessSortColumn, current: ProcessSortColumn, asc: bool| -> &'static str {
                        if col == current {
                            if asc {
                                " ^"
                            } else {
                                " v"
                            }
                        } else {
                            ""
                        }
                    };

                    let sort_col = self.process_sort_column;
                    let sort_asc = self.process_sort_ascending;

                    if ui
                        .button(format!("PID{}", sort_arrow(ProcessSortColumn::Pid, sort_col, sort_asc)))
                        .clicked()
                    {
                        if self.process_sort_column == ProcessSortColumn::Pid {
                            self.process_sort_ascending = !self.process_sort_ascending;
                        } else {
                            self.process_sort_column = ProcessSortColumn::Pid;
                            self.process_sort_ascending = true;
                        }
                    }
                    if ui
                        .button(format!(
                            "Process Name{}",
                            sort_arrow(ProcessSortColumn::Name, sort_col, sort_asc)
                        ))
                        .clicked()
                    {
                        if self.process_sort_column == ProcessSortColumn::Name {
                            self.process_sort_ascending = !self.process_sort_ascending;
                        } else {
                            self.process_sort_column = ProcessSortColumn::Name;
                            self.process_sort_ascending = true;
                        }
                    }
                    if ui
                        .button(format!(
                            "Memory{}",
                            sort_arrow(ProcessSortColumn::Memory, sort_col, sort_asc)
                        ))
                        .clicked()
                    {
                        if self.process_sort_column == ProcessSortColumn::Memory {
                            self.process_sort_ascending = !self.process_sort_ascending;
                        } else {
                            self.process_sort_column = ProcessSortColumn::Memory;
                            self.process_sort_ascending = false; // default descending for memory
                        }
                    }
                    if ui
                        .button(format!(
                            "CPU %{}",
                            sort_arrow(ProcessSortColumn::Cpu, sort_col, sort_asc)
                        ))
                        .clicked()
                    {
                        if self.process_sort_column == ProcessSortColumn::Cpu {
                            self.process_sort_ascending = !self.process_sort_ascending;
                        } else {
                            self.process_sort_column = ProcessSortColumn::Cpu;
                            self.process_sort_ascending = false; // default descending for CPU
                        }
                    }
                    ui.strong("Actions");
                    ui.end_row();

                    // Processes
                    for process in &filtered_processes {
                        let memory_mb = bytes_to_mb(process.memory);

                        let mut text_color = ui.visuals().text_color();
                        if memory_mb > 500.0 || process.cpu_usage > 20.0 {
                            text_color = ThemePalette::STATUS_CRITICAL;
                        } else if memory_mb > 200.0 || process.cpu_usage > 10.0 {
                            text_color = ThemePalette::STATUS_WARNING;
                        }

                        ui.label(egui::RichText::new(process.pid.to_string()).color(text_color));

                        let display_name = if process.name.chars().count() > 40 {
                            let truncated: String = process.name.chars().take(37).collect();
                            format!("{}...", truncated)
                        } else {
                            process.name.clone()
                        };
                        ui.label(egui::RichText::new(display_name).color(text_color));

                        ui.label(egui::RichText::new(format!("{:.2} MB", memory_mb)).color(text_color));
                        ui.label(egui::RichText::new(format!("{:.1}%", process.cpu_usage)).color(text_color));

                        // Action buttons
                        ui.horizontal(|ui| {
                            if ui.small_button("PID").on_hover_text("Copy PID").clicked() {
                                ui.output_mut(|o| o.copied_text = process.pid.to_string());
                            }
                            if ui.small_button("Name").on_hover_text("Copy Name").clicked() {
                                ui.output_mut(|o| o.copied_text = process.name.clone());
                            }
                        });

                        ui.end_row();
                    }
                });
        });
    }

    fn show_storage_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "Storage Devices");

        egui::ScrollArea::vertical().show(ui, |ui| {
            for disk in &data.disk_info {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(&disk.name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let color = get_usage_color(disk.usage_percentage);
                            ui.colored_label(color, format!("{:.1}%", disk.usage_percentage));

                            // Warning icon for high usage
                            if disk.usage_percentage > 90.0 {
                                ui.colored_label(egui::Color32::RED, "[!]");
                            }
                        });
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Mount Point:");
                        ui.strong(&disk.mount_point);
                    });

                    ui.horizontal(|ui| {
                        ui.label("File System:");
                        ui.strong(&disk.file_system);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Total Space:");
                        ui.strong(format!("{:.2} GB", bytes_to_gb(disk.total_space)));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Available:");
                        ui.strong(format!("{:.2} GB", bytes_to_gb(disk.available_space)));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Used:");
                        let used = disk.total_space.saturating_sub(disk.available_space);
                        ui.strong(format!("{:.2} GB", bytes_to_gb(used)));
                    });

                    let color = get_usage_color(disk.usage_percentage);
                    paint_progress_bar(ui, disk.usage_percentage / 100.0, color, 6.0);

                    // Show warning for low disk space
                    if disk.usage_percentage > 90.0 {
                        ui.add_space(5.0);
                        ui.colored_label(
                            egui::Color32::RED,
                            format!(
                                "Warning: Only {:.2} GB remaining!",
                                bytes_to_gb(disk.available_space)
                            ),
                        );
                    }
                });

                ui.add_space(10.0);
            }

            if data.disk_info.is_empty() {
                ui.label("No storage devices detected.");
            }
        });
    }

    fn show_network_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "Network Interfaces");

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Network graphs
            if self.settings.show_graphs && !data.network_download_history.is_empty() {
                ui.group(|ui| {
                    ui.label(
                        egui::RichText::new("Network Activity History")
                            .size(15.0)
                            .strong()
                            .color(ThemePalette::TEXT_PRIMARY),
                    );

                    // Download graph
                    ui.label(
                        egui::RichText::new("Download Rate (MB/s)")
                            .size(12.0)
                            .color(ThemePalette::STATUS_HEALTHY),
                    );
                    let download_points: PlotPoints = data
                        .network_download_history
                        .iter()
                        .map(|p| [p.time, p.value])
                        .collect();

                    let line = Line::new(download_points).color(ThemePalette::STATUS_HEALTHY);

                    Plot::new("network_download_plot")
                        .height(150.0)
                        .allow_zoom(false)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .y_axis_label("MB/s")
                        .show(ui, |plot_ui| {
                            plot_ui.line(line);
                        });

                    ui.add_space(10.0);

                    // Upload graph
                    ui.label(
                        egui::RichText::new("Upload Rate (MB/s)")
                            .size(12.0)
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    let upload_points: PlotPoints =
                        data.network_upload_history.iter().map(|p| [p.time, p.value]).collect();

                    let line = Line::new(upload_points).color(ThemePalette::ACCENT_PRIMARY);

                    Plot::new("network_upload_plot")
                        .height(150.0)
                        .allow_zoom(false)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .y_axis_label("MB/s")
                        .show(ui, |plot_ui| {
                            plot_ui.line(line);
                        });
                });

                ui.add_space(10.0);
            }

            // Network interfaces list
            for network in &data.network_info {
                ui.group(|ui| {
                    ui.heading(&network.interface);
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Total Received:");
                        ui.strong(format!("{:.2} MB", bytes_to_mb(network.received)));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Total Transmitted:");
                        ui.strong(format!("{:.2} MB", bytes_to_mb(network.transmitted)));
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Download Rate:");
                        let color = if network.received_rate > 10.0 {
                            ThemePalette::STATUS_CRITICAL
                        } else if network.received_rate > 1.0 {
                            ThemePalette::STATUS_WARNING
                        } else {
                            ThemePalette::TEXT_TERTIARY
                        };
                        ui.colored_label(color, format_rate(network.received_rate));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Upload Rate:");
                        let color = if network.transmitted_rate > 10.0 {
                            ThemePalette::STATUS_CRITICAL
                        } else if network.transmitted_rate > 1.0 {
                            ThemePalette::STATUS_WARNING
                        } else {
                            ThemePalette::TEXT_TERTIARY
                        };
                        ui.colored_label(color, format_rate(network.transmitted_rate));
                    });
                });

                ui.add_space(10.0);
            }

            if data.network_info.is_empty() {
                ui.label("No network interfaces detected.");
            }
        });
    }

    fn show_alerts_tab(&mut self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "System Alerts");

        if data.alerts.is_empty() {
            ui.group(|ui| {
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.colored_label(egui::Color32::GREEN, "OK");
                    ui.heading("All Systems Normal");
                });
                ui.add_space(10.0);
                ui.label("No alerts detected. Your system is running smoothly.");
                ui.add_space(20.0);
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("Alert Configuration");
                ui.separator();
                ui.label("Alerts are triggered when:");
                ui.label(format!(
                    "  • CPU usage > {:.0}%",
                    self.settings.notification_cpu_threshold
                ));
                ui.label(format!(
                    "  • Memory usage > {:.0}%",
                    self.settings.notification_memory_threshold
                ));
                ui.label(format!(
                    "  • GPU temperature > {}°C",
                    self.settings.notification_temp_threshold
                ));
                ui.label("  • Disk usage > 90%");
                ui.add_space(5.0);
                if ui.button("Configure Alert Thresholds").clicked() {
                    self.show_settings = true;
                }
            });
        } else {
            ui.label(format!("{} active alert(s)", data.alerts.len()));
            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, alert) in data.alerts.iter().enumerate() {
                    ui.group(|ui| {
                        let (icon, color, severity) = match alert.alert_type {
                            AlertType::CpuHigh => ("CPU", egui::Color32::YELLOW, "WARNING"),
                            AlertType::MemoryHigh => ("RAM", egui::Color32::YELLOW, "WARNING"),
                            AlertType::GpuTempHigh => ("GPU", egui::Color32::RED, "CRITICAL"),
                            AlertType::DiskSpaceLow => ("DISK", egui::Color32::RED, "CRITICAL"),
                            AlertType::StartupHighImpact => ("STARTUP", egui::Color32::YELLOW, "INFO"),
                        };

                        ui.horizontal(|ui| {
                            ui.colored_label(color, icon);
                            ui.colored_label(color, severity);
                            ui.separator();
                            ui.strong(&alert.message);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Time:");
                            ui.label(&alert.timestamp);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("Value: {:.1}", alert.value));
                            });
                        });
                    });

                    if i < data.alerts.len() - 1 {
                        ui.add_space(5.0);
                    }
                }
            });

            ui.add_space(10.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Clear All Alerts").clicked() {
                    {
                        let mut data = self.data.lock();
                        data.alerts.clear();
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Tip: Configure alert thresholds in Settings");
                });
            });
        }
    }

    fn show_system_info_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "System Information");

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.heading("Operating System");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("OS Name:");
                    ui.strong(&data.system_info.os_name);
                });

                ui.horizontal(|ui| {
                    ui.label("OS Version:");
                    ui.strong(&data.system_info.os_version);
                });

                ui.horizontal(|ui| {
                    ui.label("Kernel Version:");
                    ui.strong(&data.system_info.kernel_version);
                });

                ui.horizontal(|ui| {
                    ui.label("Hostname:");
                    ui.strong(&data.system_info.hostname);
                });

                ui.horizontal(|ui| {
                    ui.label("Uptime:");
                    let days = data.system_info.uptime / 86400;
                    let hours = (data.system_info.uptime % 86400) / 3600;
                    let minutes = (data.system_info.uptime % 3600) / 60;
                    ui.strong(format!("{}d {}h {}m", days, hours, minutes));
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("Processor");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("CPU Brand:");
                    ui.strong(&data.system_info.cpu_brand);
                });

                ui.horizontal(|ui| {
                    ui.label("CPU Cores:");
                    ui.strong(format!("{}", data.system_info.cpu_count));
                });

                ui.horizontal(|ui| {
                    ui.label("Current Usage:");
                    let color = get_usage_color(data.cpu_usage);
                    ui.colored_label(color, format!("{:.1}%", data.cpu_usage));
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("Memory");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Total RAM:");
                    ui.strong(format!("{:.2} GB", bytes_to_gb(data.memory_total)));
                });

                ui.horizontal(|ui| {
                    ui.label("Used RAM:");
                    ui.strong(format!("{:.2} GB", bytes_to_gb(data.memory_used)));
                });

                ui.horizontal(|ui| {
                    ui.label("Free RAM:");
                    ui.strong(format!("{:.2} GB", bytes_to_gb(data.memory_total - data.memory_used)));
                });

                ui.horizontal(|ui| {
                    ui.label("Usage:");
                    let color = get_usage_color(data.memory_percentage);
                    ui.colored_label(color, format!("{:.1}%", data.memory_percentage));
                });
            });

            ui.add_space(10.0);

            if data.gpu_info.is_empty() {
                ui.label(
                    egui::RichText::new("No supported GPU detected.")
                        .italics()
                        .color(ThemePalette::TEXT_DIMMED),
                );
            } else {
                for gpu_info in &data.gpu_info {
                    ui.group(|ui| {
                        ui.heading("Graphics Card");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("GPU:");
                            ui.strong(&gpu_info.name);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Utilization:");
                            let color = get_usage_color(gpu_info.utilization);
                            ui.colored_label(color, format!("{:.1}%", gpu_info.utilization));
                        });

                        if let (Some(used), Some(total)) = (gpu_info.memory_used, gpu_info.memory_total) {
                            ui.horizontal(|ui| {
                                ui.label("VRAM:");
                                let used_mb = bytes_to_mb(used);
                                let total_mb = bytes_to_mb(total);
                                if total_mb >= 1024.0 {
                                    ui.strong(format!("{:.1} / {:.1} GB", used_mb / 1024.0, total_mb / 1024.0));
                                } else {
                                    ui.strong(format!("{:.0} / {:.0} MB", used_mb, total_mb));
                                }
                            });
                        }

                        if let Some(temp) = gpu_info.temperature {
                            ui.horizontal(|ui| {
                                ui.label("Temperature:");
                                let temp_color = if temp < 70 {
                                    ThemePalette::STATUS_HEALTHY
                                } else if temp < 85 {
                                    ThemePalette::STATUS_WARNING
                                } else {
                                    ThemePalette::STATUS_CRITICAL
                                };
                                ui.colored_label(temp_color, format!("🌡️ {}°C", temp));
                            });
                        }
                    });
                }
            }

            ui.add_space(10.0);

            // Swap / Page File info
            if data.swap_info.total > 0 {
                ui.group(|ui| {
                    ui.heading("Swap / Page File");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Total Swap:");
                        ui.strong(format!("{:.2} GB", bytes_to_gb(data.swap_info.total)));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Used Swap:");
                        ui.strong(format!("{:.2} GB", bytes_to_gb(data.swap_info.used)));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Usage:");
                        let color = get_usage_color(data.swap_info.percentage);
                        ui.colored_label(color, format!("{:.1}%", data.swap_info.percentage));
                    });

                    let color = get_usage_color(data.swap_info.percentage);
                    paint_progress_bar(ui, data.swap_info.percentage / 100.0, color, 5.0);
                });
            }

            ui.add_space(10.0);

            // Battery info
            if let Some(ref battery) = data.battery_info {
                ui.group(|ui| {
                    ui.heading("Battery");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Charge:");
                        let color = if battery.percentage > 50.0 {
                            ThemePalette::STATUS_HEALTHY
                        } else if battery.percentage > 20.0 {
                            ThemePalette::STATUS_WARNING
                        } else {
                            ThemePalette::STATUS_CRITICAL
                        };
                        let icon = if battery.is_charging { "AC" } else { "BAT" };
                        ui.colored_label(color, format!("{} {:.0}%", icon, battery.percentage));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Status:");
                        ui.strong(&battery.status_text);
                    });

                    let color = if battery.percentage > 50.0 {
                        ThemePalette::STATUS_HEALTHY
                    } else if battery.percentage > 20.0 {
                        ThemePalette::STATUS_WARNING
                    } else {
                        ThemePalette::STATUS_CRITICAL
                    };
                    paint_progress_bar(ui, battery.percentage / 100.0, color, 5.0);
                });
            }
        });
    }

    fn show_cpu_cores_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "CPU Cores Monitoring");

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(format!(
                "Total Cores: {} ({} logical processors)",
                data.system_info.cpu_count,
                data.cpu_cores.len()
            ));
            ui.add_space(10.0);

            // Grid layout for cores
            let cores_per_row = 4;
            let mut core_index = 0;

            while core_index < data.cpu_cores.len() {
                ui.horizontal(|ui| {
                    for _ in 0..cores_per_row {
                        if core_index >= data.cpu_cores.len() {
                            break;
                        }

                        let core = &data.cpu_cores[core_index];
                        ui.group(|ui| {
                            ui.set_min_width(180.0);

                            ui.horizontal(|ui| {
                                ui.strong(format!("Core {}", core.core_id));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let color = get_usage_color(core.usage);
                                    ui.colored_label(color, format!("{:.1}%", core.usage));
                                });
                            });

                            let color = get_usage_color(core.usage);
                            paint_progress_bar(ui, core.usage / 100.0, color, 5.0);
                        });

                        core_index += 1;
                    }
                });
                ui.add_space(5.0);
            }

            ui.add_space(10.0);

            // Summary statistics
            ui.group(|ui| {
                ui.heading("Core Statistics");
                ui.separator();

                let avg_usage: f32 = data.cpu_cores.iter().map(|c| c.usage).sum::<f32>() / data.cpu_cores.len() as f32;
                let max_usage = data.cpu_cores.iter().map(|c| c.usage).fold(0.0f32, f32::max);
                let min_usage = data.cpu_cores.iter().map(|c| c.usage).fold(100.0f32, f32::min);

                ui.horizontal(|ui| {
                    ui.label("Average Usage:");
                    let color = get_usage_color(avg_usage);
                    ui.colored_label(color, format!("{:.1}%", avg_usage));
                });

                ui.horizontal(|ui| {
                    ui.label("Maximum Core:");
                    let color = get_usage_color(max_usage);
                    ui.colored_label(color, format!("{:.1}%", max_usage));
                });

                ui.horizontal(|ui| {
                    ui.label("Minimum Core:");
                    ui.label(format!("{:.1}%", min_usage));
                });

                ui.horizontal(|ui| {
                    ui.label("Cores Above 50%:");
                    let high_cores = data.cpu_cores.iter().filter(|c| c.usage > 50.0).count();
                    ui.label(format!("{} / {}", high_cores, data.cpu_cores.len()));
                });
            });
        });
    }

    fn show_process_manager_window(&mut self, ctx: &egui::Context, data: &SystemData) {
        let mut show = self.show_process_manager;

        egui::Window::new("Process Manager")
            .open(&mut show)
            .resizable(true)
            .default_width(800.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.heading("Running Processes");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(format!("Total processes: {}", data.top_processes.len()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Refresh").clicked() {
                            // Refresh happens automatically
                        }
                    });
                });

                ui.add_space(5.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("process_manager_grid")
                        .striped(true)
                        .spacing([10.0, 4.0])
                        .min_col_width(60.0)
                        .show(ui, |ui| {
                            // Header
                            ui.strong("PID");
                            ui.strong("Process Name");
                            ui.strong("Memory");
                            ui.strong("CPU %");
                            ui.strong("Status");
                            ui.strong("Actions");
                            ui.end_row();

                            // Processes
                            for process in &data.top_processes {
                                let memory_mb = bytes_to_mb(process.memory);
                                let memory_color = if memory_mb > 500.0 {
                                    ThemePalette::STATUS_CRITICAL
                                } else if memory_mb > 200.0 {
                                    ThemePalette::STATUS_WARNING
                                } else {
                                    ThemePalette::STATUS_HEALTHY
                                };

                                ui.label(process.pid.to_string());

                                // Safe truncation using char boundaries
                                let display_name = if process.name.chars().count() > 25 {
                                    let truncated: String = process.name.chars().take(22).collect();
                                    format!("{}...", truncated)
                                } else {
                                    process.name.clone()
                                };
                                ui.label(display_name);

                                ui.colored_label(memory_color, format!("{:.2} MB", memory_mb));
                                ui.label(format!("{:.1}%", process.cpu_usage));
                                ui.label(&process.status);

                                ui.horizontal(|ui| {
                                    if ui.small_button("Kill").on_hover_text("Kill Process").clicked() {
                                        self.selected_process_pid = Some(process.pid);
                                    }
                                    let is_suspended = self.suspended_pids.contains(&process.pid);
                                    if is_suspended {
                                        if ui.small_button("Resume").on_hover_text("Resume Process").clicked() {
                                            self.resume_process_pid = Some(process.pid);
                                        }
                                    } else {
                                        if ui.small_button("Suspend").on_hover_text("Suspend Process").clicked() {
                                            self.suspend_process_pid = Some(process.pid);
                                        }
                                    }
                                    // Priority menu
                                    ui.menu_button("Priority", |ui| {
                                        ui.label("Set Priority:");
                                        for priority in &["High", "AboveNormal", "Normal", "BelowNormal", "Idle"] {
                                            if ui.button(*priority).clicked() {
                                                self.priority_change = Some((process.pid, priority.to_string()));
                                                ui.close_menu();
                                            }
                                        }
                                    })
                                    .response
                                    .on_hover_text("Set Priority");
                                });

                                ui.end_row();
                            }
                        });
                });

                ui.separator();
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "Warning: Killing/suspending processes may cause system instability!",
                );
                if !self.suspended_pids.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 165, 0),
                        format!("{} process(s) suspended", self.suspended_pids.len()),
                    );
                }
            });

        self.show_process_manager = show;
    }

    fn show_ram_cleaner_tab(&mut self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "RAM Cleaner");

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Current Memory Status
            ui.group(|ui| {
                ui.heading("Memory Overview");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Total RAM:");
                    ui.strong(format!("{:.2} GB", bytes_to_gb(data.memory_total)));
                });
                ui.horizontal(|ui| {
                    ui.label("Used RAM:");
                    let color = get_usage_color(data.memory_percentage);
                    ui.colored_label(
                        color,
                        format!(
                            "{:.2} GB ({:.1}%)",
                            bytes_to_gb(data.memory_used),
                            data.memory_percentage
                        ),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Free RAM:");
                    ui.strong(format!(
                        "{:.2} GB",
                        bytes_to_gb(data.memory_total.saturating_sub(data.memory_used))
                    ));
                });
                let color = get_usage_color(data.memory_percentage);
                paint_progress_bar(ui, data.memory_percentage / 100.0, color, 8.0);
            });

            ui.add_space(10.0);

            // Manual Clean button
            ui.group(|ui| {
                ui.heading("Manual Clean");
                ui.separator();
                ui.label("Frees up unused RAM by emptying process working sets.");
                ui.label("This is safe and Windows will reload memory as needed.");
                ui.add_space(5.0);

                if privilege::is_app_elevated() {
                    ui.colored_label(ThemePalette::STATUS_HEALTHY, "Running as Administrator: Full memory cleaning enabled.");
                } else {
                    ui.colored_label(ThemePalette::STATUS_WARNING, "Standard Privileges: User processes only. Run as Admin to clean system memory.");
                }
                ui.add_space(5.0);

                let is_cleaning = self.ram_cleaner_state.is_cleaning;
                ui.add_enabled_ui(!is_cleaning, |ui| {
                    if ui
                        .button(egui::RichText::new("🧹 Clean RAM Now").size(16.0).strong())
                        .clicked()
                    {
                        self.ram_cleaner_state.is_cleaning = true;
                        self.ram_cleaner_state.last_cleaned = Some(Instant::now());
                        self.ram_cleaner_state.last_cleaned_display = Local::now().format("%H:%M:%S").to_string();
                        self.ram_cleaner_state.clean_count += 1;
                        let data_arc = Arc::clone(&self.data);
                        let ctx_clone = ui.ctx().clone();
                        let enable_sounds = self.settings.enable_sounds;
                        thread::spawn(move || {
                            let mut monitor = SystemMonitor::new();
                            let freed = monitor.clean_ram();
                            if enable_sounds { play_success_sound(); }
                            {
                                let mut d = data_arc.lock();
                                d.ram_clean_freed_bytes += freed;
                                d.ram_clean_is_cleaning = false;
                            }
                            ctx_clone.request_repaint();
                        });
                        {
                            let mut d = self.data.lock();
                            d.ram_clean_is_cleaning = true;
                        }
                    }
                });

                if is_cleaning {
                    ui.colored_label(ThemePalette::ACCENT_PRIMARY, "Cleaning in progress...");
                }
            });

            ui.add_space(10.0);

            // Auto Clean settings
            ui.group(|ui| {
                ui.heading("Auto Clean");
                ui.separator();

                ui.checkbox(
                    &mut self.ram_cleaner_state.auto_clean_enabled,
                    "Enable automatic RAM cleaning",
                );

                if self.ram_cleaner_state.auto_clean_enabled {
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.label("Clean when RAM usage exceeds:");
                        ui.add(
                            egui::Slider::new(&mut self.ram_cleaner_state.auto_clean_threshold, 50.0..=95.0)
                                .suffix("%"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Minimum interval between cleans:");
                        ui.add(
                            egui::Slider::new(&mut self.ram_cleaner_state.auto_clean_interval, 60..=1800)
                                .suffix(" sec"),
                        );
                    });
                }
            });

            ui.add_space(10.0);

            // Statistics
            ui.group(|ui| {
                ui.heading("Cleaning Statistics");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Total cleans this session:");
                    ui.strong(format!("{}", self.ram_cleaner_state.clean_count));
                });

                ui.horizontal(|ui| {
                    ui.label("Total RAM freed this session:");
                    ui.strong(format!("{:.2} MB", bytes_to_mb(self.ram_cleaner_state.bytes_freed)));
                });

                ui.horizontal(|ui| {
                    ui.label("Last cleaned:");
                    if self.ram_cleaner_state.last_cleaned.is_some() {
                        ui.strong(&self.ram_cleaner_state.last_cleaned_display);
                    } else {
                        ui.label("Never");
                    }
                });
            });
        });
    }

    fn show_startup_manager_tab(&mut self, ui: &mut egui::Ui) {
        paint_section_header(ui, "Startup Programs");

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Load data lazily in a background thread ──
            if !self.startup_items_loaded && !self.startup_items_loading {
                self.startup_items_loading = true;
                let ctx = ui.ctx().clone();
                let startup_items_share = Arc::clone(&self.startup_items_share);
                let boot_diagnostics_share = Arc::clone(&self.boot_diagnostics_share);
                thread::spawn(move || {
                    let items = startup::get_startup_items();
                    let diag = startup::get_boot_diagnostics();
                    {
                        let mut share = startup_items_share.lock();
                        *share = Some(items);
                    }
                    {
                        let mut share = boot_diagnostics_share.lock();
                        *share = diag;
                    }
                    ctx.request_repaint();
                });
            }

            // Sync loaded data to app state
            let is_loading = {
                let share = self.startup_items_share.lock();
                if let Some(ref items) = *share {
                    self.startup_items = items.clone();
                    self.startup_items_loaded = true;
                    self.startup_items_loading = false;
                    false
                } else {
                    true
                }
            };

            if let Some(ref diag) = *self.boot_diagnostics_share.lock() {
                self.boot_diagnostics = Some(diag.clone());
                self.boot_diagnostics_loaded = true;
            }

            if is_loading {
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Analyzing startup configuration...").strong().color(ThemePalette::TEXT_SECONDARY));
                });
                return;
            }

            // ── Header card with boot info ──
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("Startup Items");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🔄 Refresh").clicked() {
                            self.startup_items_loaded = false;
                            self.boot_diagnostics_loaded = false;
                            *self.startup_items_share.lock() = None;
                            *self.boot_diagnostics_share.lock() = None;
                            self.startup_show_confirm = None;
                        }
                    });
                });
                ui.separator();

                // Boot diagnostics summary
                ui.horizontal(|ui| {
                    let total = self.startup_items.len();
                    let high = startup::high_impact_count(&self.startup_items);

                    let mut boot_shown = false;
                    if let Some(ref bd) = self.boot_diagnostics {
                        if let Some(ms) = bd.boot_duration_ms {
                            let secs = ms as f64 / 1000.0;
                            let c = if secs < 30.0 { ThemePalette::STATUS_HEALTHY }
                                    else if secs < 60.0 { ThemePalette::STATUS_WARNING }
                                    else { ThemePalette::STATUS_CRITICAL };
                            ui.colored_label(c, format!("Boot: {:.1}s", secs));
                            ui.separator();
                            boot_shown = true;
                        }
                    }
                    if !boot_shown {
                        if privilege::is_app_elevated() {
                            ui.colored_label(ThemePalette::STATUS_WARNING, "Boot: Unknown");
                            ui.separator();
                        } else {
                            ui.colored_label(ThemePalette::STATUS_WARNING, "Boot: (Requires Admin)")
                                .on_hover_text("Reading boot diagnostics event logs requires Administrator privileges");
                            ui.separator();
                        }
                    }
                    if high > 0 {
                        ui.colored_label(ThemePalette::STATUS_CRITICAL, format!("{} high-impact", high));
                    } else {
                        ui.colored_label(ThemePalette::STATUS_HEALTHY, "No high-impact items");
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(format!("{} total", total)).color(ThemePalette::TEXT_LABEL));
                });
            });

            ui.add_space(8.0);

            // ── Search & Filter toolbar ──
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.add(egui::TextEdit::singleline(&mut self.startup_search)
                        .hint_text("Search by name, command, publisher...")
                        .desired_width(250.0));

                    ui.separator();

                    // Impact filter
                    egui::ComboBox::from_id_source("impact_filter")
                        .selected_text(match &self.startup_filter_impact {
                            Some(ImpactTier::High) => "High",
                            Some(ImpactTier::Medium) => "Medium",
                            Some(ImpactTier::Low) => "Low",
                            _ => "Impact: All",
                        })
                        .show_ui(ui, |ui: &mut egui::Ui| {
                            if ui.selectable_label(self.startup_filter_impact.is_none(), "All").clicked() {
                                self.startup_filter_impact = None;
                            }
                            if ui.selectable_label(self.startup_filter_impact == Some(ImpactTier::High), "High").clicked() {
                                self.startup_filter_impact = Some(ImpactTier::High);
                            }
                            if ui.selectable_label(self.startup_filter_impact == Some(ImpactTier::Medium), "Medium").clicked() {
                                self.startup_filter_impact = Some(ImpactTier::Medium);
                            }
                            if ui.selectable_label(self.startup_filter_impact == Some(ImpactTier::Low), "Low").clicked() {
                                self.startup_filter_impact = Some(ImpactTier::Low);
                            }
                        });

                    // Signed filter
                    egui::ComboBox::from_id_source("signed_filter")
                        .selected_text(match self.startup_filter_signed {
                            Some(true) => "Signed",
                            Some(false) => "Unsigned",
                            None => "Signed: All",
                        })
                        .show_ui(ui, |ui: &mut egui::Ui| {
                            if ui.selectable_label(self.startup_filter_signed.is_none(), "All").clicked() {
                                self.startup_filter_signed = None;
                            }
                            if ui.selectable_label(self.startup_filter_signed == Some(true), "Signed").clicked() {
                                self.startup_filter_signed = Some(true);
                            }
                            if ui.selectable_label(self.startup_filter_signed == Some(false), "Unsigned").clicked() {
                                self.startup_filter_signed = Some(false);
                            }
                        });

                    ui.checkbox(&mut self.startup_filter_broken, "Broken only");
                });

                // Sort controls
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Sort:").color(ThemePalette::TEXT_LABEL).small());

                    let sorts = [
                        (StartupSortColumn::Impact, "Impact"),
                        (StartupSortColumn::Name, "Name"),
                        (StartupSortColumn::Source, "Source"),
                        (StartupSortColumn::Publisher, "Publisher"),
                    ];
                    for (col, label) in &sorts {
                        let is_active = self.startup_sort == *col;
                        let text = if is_active {
                            let arrow = if self.startup_sort_ascending { "^" } else { "v" };
                            format!("{} {}", label, arrow)
                        } else {
                            label.to_string()
                        };
                        if ui.selectable_label(is_active,
                            egui::RichText::new(text).small()
                        ).clicked() {
                            if is_active {
                                self.startup_sort_ascending = !self.startup_sort_ascending;
                            } else {
                                self.startup_sort = *col;
                                self.startup_sort_ascending = true;
                            }
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // ── Apply filters and sort ──
            let search_lower = self.startup_search.to_lowercase();
            let mut filtered_indices: Vec<usize> = self.startup_items.iter().enumerate()
                .filter(|(_, item)| {
                    // Search filter
                    if !search_lower.is_empty() {
                        let matches = item.name.to_lowercase().contains(&search_lower)
                            || item.command.to_lowercase().contains(&search_lower)
                            || item.publisher.as_ref().map(|p| p.to_lowercase().contains(&search_lower)).unwrap_or(false);
                        if !matches { return false; }
                    }
                    // Impact filter
                    if let Some(ref filter) = self.startup_filter_impact {
                        if item.impact_tier != *filter { return false; }
                    }
                    // Signed filter
                    if let Some(filter_signed) = self.startup_filter_signed {
                        if item.is_signed != Some(filter_signed) { return false; }
                    }
                    // Broken filter
                    if self.startup_filter_broken && item.exe_exists { return false; }
                    true
                })
                .map(|(i, _)| i)
                .collect();

            // Sort the filtered view
            {
                let items_ref = &self.startup_items;
                let sort_col = self.startup_sort;
                let ascending = self.startup_sort_ascending;
                filtered_indices.sort_by(|a, b| {
                    let ia = &items_ref[*a];
                    let ib = &items_ref[*b];
                    let cmp = match sort_col {
                        StartupSortColumn::Name => ia.name.to_lowercase().cmp(&ib.name.to_lowercase()),
                        StartupSortColumn::Impact => ia.impact_tier.sort_key().cmp(&ib.impact_tier.sort_key()),
                        StartupSortColumn::Source => ia.source.cmp(&ib.source),
                        StartupSortColumn::Publisher => {
                            let pa = ia.publisher.as_deref().unwrap_or("zzz").to_lowercase();
                            let pb = ib.publisher.as_deref().unwrap_or("zzz").to_lowercase();
                            pa.cmp(&pb)
                        }
                    };
                    if ascending { cmp } else { cmp.reverse() }
                });
            }

            if filtered_indices.is_empty() {
                ui.group(|ui| {
                    ui.add_space(20.0);
                    if self.startup_items.is_empty() {
                        ui.label("No startup items found.");
                    } else {
                        ui.label("No items match the current filters.");
                    }
                    ui.add_space(20.0);
                });
            } else {
                ui.label(egui::RichText::new(format!("Showing {} of {} item(s)", filtered_indices.len(), self.startup_items.len()))
                    .small().color(ThemePalette::TEXT_LABEL));
                ui.add_space(4.0);

                let mut action: Option<(usize, &str)> = None;

                for &idx in &filtered_indices {
                    let item = &self.startup_items[idx];
                    let is_confirming = self.startup_show_confirm == Some(idx);

                    ui.group(|ui| {
                        // ── Row 1: Impact badge + Name + Source ──
                        ui.horizontal(|ui| {
                            // Impact badge
                            let (badge_text, badge_color) = match item.impact_tier {
                                ImpactTier::High => ("HIGH", ThemePalette::STATUS_CRITICAL),
                                ImpactTier::Medium => ("MED", ThemePalette::STATUS_WARNING),
                                ImpactTier::Low => ("LOW", ThemePalette::STATUS_HEALTHY),
                                ImpactTier::Unknown => ("?", ThemePalette::TEXT_DIMMED),
                            };
                            ui.colored_label(badge_color,
                                egui::RichText::new(badge_text).size(11.0).strong());
                            ui.separator();
                            if item.enabled {
                                ui.strong(&item.name);
                            } else {
                                ui.label(egui::RichText::new(&item.name).strong().strikethrough().color(ThemePalette::TEXT_DIMMED));
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.colored_label(ThemePalette::TEXT_TERTIARY,
                                    egui::RichText::new(&item.source).small());
                            });
                        });

                        // ── Row 2: Command path ──
                        let cmd_display = if item.command.chars().count() > 90 {
                            let truncated: String = item.command.chars().take(87).collect();
                            format!("{}...", truncated)
                        } else {
                            item.command.clone()
                        };
                        ui.label(egui::RichText::new(cmd_display).small().color(ThemePalette::TEXT_DIMMED));

                        // ── Row 3: Publisher + Signed status ──
                        ui.horizontal(|ui| {
                            if let Some(ref pub_name) = item.publisher {
                                ui.label(egui::RichText::new(format!("Publisher: {}", pub_name))
                                    .small().color(ThemePalette::TEXT_LABEL));
                            }
                            match item.is_signed {
                                Some(true) => { ui.colored_label(ThemePalette::STATUS_HEALTHY,
                                    egui::RichText::new("Signed").small()); }
                                Some(false) => { ui.colored_label(ThemePalette::STATUS_CRITICAL,
                                    egui::RichText::new("Unsigned").small()); }
                                None => {}
                            }
                            if !item.exe_exists && item.exe_path.is_some() {
                                ui.colored_label(ThemePalette::STATUS_CRITICAL,
                                    egui::RichText::new("File missing").small());
                            }
                        });

                        // ── Row 4: Recommendation + Reason ──
                        ui.horizontal(|ui| {
                            let rec_color = match item.recommendation {
                                Recommendation::Keep => ThemePalette::STATUS_HEALTHY,
                                Recommendation::Review => ThemePalette::STATUS_WARNING,
                                Recommendation::Disable => ThemePalette::STATUS_CRITICAL,
                                Recommendation::Cleanup => ThemePalette::STATUS_CRITICAL,
                            };
                            ui.colored_label(rec_color,
                                egui::RichText::new(format!("> {}", item.recommendation.label()))
                                    .small().strong());
                            ui.label(egui::RichText::new(format!("— {}", item.reason))
                                .small().color(ThemePalette::TEXT_LABEL_SUB));
                        });

                        // ── Row 5: Actions ──
                        if is_confirming {
                            // Confirmation dialog
                            ui.horizontal(|ui| {
                                ui.colored_label(ThemePalette::STATUS_WARNING,
                                    egui::RichText::new(format!("Disable \"{}\" from startup?", item.name)).strong());
                                if ui.button("Yes, disable").clicked() {
                                    action = Some((idx, "disable"));
                                    self.startup_show_confirm = None;
                                }
                                if ui.button("Cancel").clicked() {
                                    self.startup_show_confirm = None;
                                }
                            });
                        } else {
                            ui.horizontal(|ui| {
                                let is_elevated = privilege::is_app_elevated();
                                let can_modify = item.source.contains("HKCU")
                                    || item.source.contains("Startup Folder")
                                    || (is_elevated && (item.source.contains("HKLM") || item.source.contains("Task Scheduler")));
                                let is_keep = item.recommendation == Recommendation::Keep;

                                // Disable/Enable button
                                if item.enabled {
                                    ui.add_enabled_ui(can_modify && !is_keep, |ui| {
                                        if ui.button("Disable").on_hover_text(
                                            if is_keep { "System component — disabling not recommended" }
                                            else if !can_modify { "Requires Administrator privileges" }
                                            else { "Disable this startup item (reversible)" }
                                        ).clicked() {
                                            self.startup_show_confirm = Some(idx);
                                        }
                                    });
                                } else {
                                    ui.add_enabled_ui(can_modify, |ui| {
                                        if ui.button("Enable").on_hover_text(
                                            if !can_modify { "Requires Administrator privileges" }
                                            else { "Re-enable this startup item" }
                                        ).clicked() {
                                            action = Some((idx, "enable"));
                                        }
                                    });
                                }

                                // Open location
                                if let Some(ref path) = item.exe_path {
                                    if item.exe_exists {
                                        let path_clone = path.clone();
                                        if ui.button("Open").on_hover_text("Open file location in Explorer").clicked() {
                                            startup::open_file_location(&path_clone);
                                        }
                                    }
                                }

                                // Copy command
                                if ui.button("Copy").on_hover_text("Copy full command to clipboard").clicked() {
                                    ui.output_mut(|o| o.copied_text = item.command.clone());
                                }

                                // Search online
                                let name_clone = item.name.clone();
                                if ui.button("Search").on_hover_text("Search online for info about this item").clicked() {
                                    startup::search_online(&name_clone);
                                }

                                // Remove button (permanent delete for HKCU/Startup Folder/HKLM/Task Scheduler items)
                                if can_modify && !item.enabled {
                                    if ui.button("Remove").on_hover_text("Permanently remove this startup item").clicked() {
                                        action = Some((idx, "remove"));
                                    }
                                }

                                // Admin message for HKLM/Task Scheduler items when not elevated
                                if !can_modify {
                                    ui.colored_label(ThemePalette::TEXT_DIMMED,
                                        egui::RichText::new("(Requires Admin)").small());
                                }
                            });
                        }
                    });
                    ui.add_space(3.0);
                }

                // Process actions
                if let Some((idx, act)) = action {
                    let item = &self.startup_items[idx];
                    let item_name = item.name.clone();
                    let item_source = item.source.clone();
                    let item_command = item.command.clone();
                    let tier_before = item.impact_tier.label().to_string();
                    let high_before = startup::high_impact_count(&self.startup_items);

                    let success = match act {
                        "disable" => startup::disable_startup_item(&item_name, &item_source, &item_command),
                        "enable" => startup::reenable_startup_item(&item_name, &item_source),
                        "remove" => startup::remove_startup_item(&item_name, &item_source),
                        _ => false,
                    };

                    if success {
                        let high_after = if self.startup_items[idx].impact_tier == ImpactTier::High {
                            if act == "disable" { high_before.saturating_sub(1) } else { high_before + 1 }
                        } else {
                            high_before
                        };

                        self.settings.startup_optimization_history.push(StartupOptimizationEntry {
                            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                            action: act.to_string(),
                            item_name: item_name.clone(),
                            item_source,
                            impact_tier_before: tier_before,
                            high_impact_count_before: high_before,
                            high_impact_count_after: high_after,
                        });
                        let _ = self.settings.save();
                        self.startup_items_loaded = false;
                        *self.startup_items_share.lock() = None;
                        *self.boot_diagnostics_share.lock() = None;
                    }
                }
            }

            // ── Optimization History ──
            if !self.settings.startup_optimization_history.is_empty() {
                ui.add_space(16.0);
                ui.group(|ui| {
                    ui.heading("Optimization History");
                    ui.separator();

                    let history = &self.settings.startup_optimization_history;
                    let show_count = history.len().min(10);
                    for entry in history.iter().rev().take(show_count) {
                        ui.horizontal(|ui| {
                            ui.colored_label(ThemePalette::TEXT_LABEL,
                                egui::RichText::new(&entry.timestamp).small());
                            ui.label(egui::RichText::new(format!("{} \"{}\"",
                                entry.action, entry.item_name)).small());
                            let delta = entry.high_impact_count_before as i32 - entry.high_impact_count_after as i32;
                            if delta > 0 {
                                ui.colored_label(ThemePalette::STATUS_HEALTHY,
                                    egui::RichText::new(format!("-{} high", delta)).small());
                            }
                        });
                    }
                });
            }
        });
    }

    fn show_settings_tab(&mut self, ui: &mut egui::Ui) {
        paint_section_header(ui, "Application Settings");

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut changed = false;
            let mut theme_changed = false;

            // --- General Group ---
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.heading("General");
                });
                ui.add_space(8.0);

                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        changed |= ui
                            .checkbox(&mut self.settings.show_graphs, "Show Performance Graphs")
                            .changed();
                        changed |= ui
                            .checkbox(&mut self.settings.show_gpu, "Show GPU Information")
                            .changed();
                        changed |= ui
                            .checkbox(&mut self.settings.show_processes, "Show Process List")
                            .changed();
                    });

                    cols[1].vertical(|ui| {
                        changed |= ui
                            .checkbox(&mut self.settings.show_notifications, "Enable Desktop Notifications")
                            .changed();
                        changed |= ui
                            .checkbox(&mut self.settings.enable_sounds, "Enable System Event Sounds")
                            .changed();
                        if ui
                            .checkbox(&mut self.settings.theme_dark, "Dark Theme (Terminal Noir)")
                            .changed()
                        {
                            changed = true;
                            theme_changed = true;
                        }
                        changed |= ui
                            .checkbox(&mut self.settings.auto_clear_alerts, "Auto-clear resolved alerts")
                            .changed();
                    });
                });
            });
            ui.add_space(12.0);

            // --- Monitoring Group ---
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.heading("Monitoring");
                });
                ui.add_space(8.0);

                egui::Grid::new("monitoring_grid")
                    .num_columns(2)
                    .spacing([24.0, 12.0])
                    .min_col_width(200.0)
                    .show(ui, |ui| {
                        ui.label("Data refresh interval:");
                        changed |= ui
                            .add(egui::Slider::new(&mut self.settings.refresh_interval, 1..=10).suffix("s"))
                            .changed();
                        ui.end_row();

                        ui.label("Number of processes to show:");
                        changed |= ui
                            .add(egui::Slider::new(&mut self.settings.process_count, 5..=100))
                            .changed();
                        ui.end_row();
                    });
            });
            ui.add_space(12.0);

            // --- Alert Thresholds Group ---
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.heading("Alert Thresholds");
                });
                ui.add_space(8.0);

                egui::Grid::new("alert_thresholds_grid")
                    .num_columns(2)
                    .spacing([24.0, 12.0])
                    .min_col_width(200.0)
                    .show(ui, |ui| {
                        ui.label("CPU Usage % Alert:");
                        changed |= ui
                            .add(egui::Slider::new(&mut self.settings.notification_cpu_threshold, 50.0..=100.0).suffix("%"))
                            .changed();
                        ui.end_row();

                        ui.label("Memory Usage % Alert:");
                        changed |= ui
                            .add(egui::Slider::new(&mut self.settings.notification_memory_threshold, 50.0..=100.0).suffix("%"))
                            .changed();
                        ui.end_row();

                        ui.label("Temperature °C Alert:");
                        changed |= ui
                            .add(egui::Slider::new(&mut self.settings.notification_temp_threshold, 60..=105).suffix("°C"))
                            .changed();
                        ui.end_row();
                    });
            });
            ui.add_space(12.0);

            // --- Windows Integration Group ---
            #[cfg(target_os = "windows")]
            {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.heading("Windows Integration");
                    });
                    ui.add_space(8.0);

                    if ui
                        .checkbox(&mut self.settings.auto_start, "Start with Windows")
                        .changed()
                    {
                        changed = true;
                        let _ = self.settings.set_auto_start(self.settings.auto_start);
                    }
                    changed |= ui
                        .checkbox(&mut self.settings.minimize_to_tray, "Minimize to system tray on close")
                        .changed();
                });
                ui.add_space(12.0);
            }

            if changed {
                let _ = self.settings.save();
                // Sync settings to the background thread
                {
                    let mut shared = self.shared_settings.lock();
                    *shared = self.settings.clone();
                }
            }

            // Apply theme change live
            if theme_changed {
                if self.settings.theme_dark {
                    let mut visuals = egui::Visuals::dark();
                    visuals.panel_fill = ThemePalette::BG_DEEP;
                    visuals.window_fill = ThemePalette::BG_SURFACE;
                    visuals.extreme_bg_color = ThemePalette::BG_DEEPEST;
                    visuals.selection.bg_fill = ThemePalette::ACCENT_PRIMARY;
                    visuals.selection.stroke = egui::Stroke::new(1.0, ThemePalette::ACCENT_ACTIVE);
                    visuals.hyperlink_color = ThemePalette::ACCENT_PRIMARY;
                    visuals.widgets.noninteractive.bg_fill = ThemePalette::BG_CARD;
                    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
                    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_PRIMARY);
                    visuals.widgets.inactive.bg_fill = ThemePalette::WIDGET_INACTIVE;
                    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
                    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SECONDARY);
                    visuals.widgets.hovered.bg_fill = ThemePalette::WIDGET_HOVERED;
                    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER_LIGHT);
                    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);
                    visuals.widgets.active.bg_fill = ThemePalette::ACCENT_ACTIVE;
                    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ThemePalette::ACCENT_PRIMARY);
                    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);
                    visuals.window_rounding = egui::Rounding::same(12.0);
                    visuals.menu_rounding = egui::Rounding::same(10.0);
                    visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
                    visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
                    visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
                    visuals.widgets.active.rounding = egui::Rounding::same(8.0);
                    visuals.window_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER_LIGHT);
                    visuals.window_shadow = egui::epaint::Shadow {
                        offset: egui::vec2(0.0, 12.0),
                        blur: 32.0,
                        spread: -4.0,
                        color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 180),
                    };
                    visuals.popup_shadow = egui::epaint::Shadow {
                        offset: egui::vec2(0.0, 8.0),
                        blur: 24.0,
                        spread: -2.0,
                        color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 150),
                    };
                    ui.ctx().set_visuals(visuals);
                } else {
                    ui.ctx().set_visuals(egui::Visuals::light());
                }
            }
        });
    }
    fn show_about_tab(&self, ui: &mut egui::Ui, _data: &SystemData) {
        paint_section_header(ui, "About");

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            // Hero brand
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Image::new(egui::include_image!("../assets/icon.png"))
                            .max_width(40.0)
                            .max_height(40.0),
                    );
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("System Monitor")
                                .size(22.0)
                                .strong()
                                .color(ThemePalette::TEXT_PRIMARY),
                        );
                        ui.label(
                            egui::RichText::new(format!("v{} · Terminal Noir", APP_VERSION))
                                .size(12.0)
                                .color(ThemePalette::TEXT_TERTIARY),
                        );
                    });
                });
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Professional system intelligence for Windows — built with Rust and egui.")
                        .size(13.0)
                        .color(ThemePalette::TEXT_SUBTITLE),
                );
            });

            ui.add_space(10.0);

            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.label(
                        egui::RichText::new("FEATURES")
                            .size(10.0)
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.add_space(6.0);
                    for item in &[
                        "Real-time CPU, Memory & GPU",
                        "Historical performance graphs",
                        "Process monitoring & management",
                        "Color-coded usage indicators",
                        "Per-core CPU breakdown",
                        "Smart alerts system",
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("›").color(ThemePalette::ACCENT_PRIMARY));
                            ui.label(egui::RichText::new(*item).size(12.5).color(ThemePalette::TEXT_FEATURE));
                        });
                    }
                });

                cols[1].group(|ui| {
                    ui.label(
                        egui::RichText::new("TECHNICAL")
                            .size(10.0)
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.add_space(6.0);
                    let refresh_str = format!("{} s interval", self.settings.refresh_interval);
                    let specs: Vec<(&str, &str)> = vec![
                        ("Framework", "egui + eframe"),
                        ("System", "sysinfo crate"),
                        ("GPU", "NVML (NVIDIA)"),
                        ("Refresh", &refresh_str),
                        ("History", "60 data points"),
                        ("License", "MIT — open source"),
                    ];
                    for (k, v) in &specs {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(*k).size(11.5).color(ThemePalette::TEXT_TERTIARY));
                            ui.label(
                                egui::RichText::new(*v)
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(185, 195, 215)),
                            );
                        });
                    }
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("COLOR LEGEND")
                        .size(10.0)
                        .color(ThemePalette::ACCENT_PRIMARY),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.colored_label(ThemePalette::STATUS_HEALTHY, "●  Healthy < 50%");
                    ui.add_space(16.0);
                    ui.colored_label(ThemePalette::STATUS_WARNING, "●  Moderate 50-75%");
                    ui.add_space(16.0);
                    ui.colored_label(ThemePalette::STATUS_CRITICAL, "●  Critical > 75%");
                });
            });
        });
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
    let image = image::load_from_memory(include_bytes!("../assets/icon.png")).ok()?.into_rgba8();
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

        let mutex_name: Vec<u16> = "Global\\SystemMonitorSingleInstance\0"
            .encode_utf16()
            .collect();
        let _handle = unsafe { CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr()) };
        let last_error = unsafe { GetLastError() };

        const ERROR_ALREADY_EXISTS: u32 = 183;
        if last_error == ERROR_ALREADY_EXISTS {
            use windows::core::PCWSTR;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

            let title: Vec<u16> = "System Monitor\0".encode_utf16().collect();
            let msg: Vec<u16> =
                "System Monitor is already running.\n\nCheck your system tray or taskbar.\0"
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

            let title: Vec<u16> = "System Monitor — Unexpected Error\0"
                .encode_utf16()
                .collect();
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

    info!(
        version = APP_VERSION,
        "System Monitor starting — Enterprise Edition"
    );
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
    println!("DBG: Launching GUI window via eframe::run_native");

    let result = eframe::run_native(
        "System Monitor",
        options,
        Box::new(|cc| {
            println!("DBG: eframe application creator closure running");
            let app = SystemMonitorApp::new(cc);
            println!("DBG: SystemMonitorApp::new completed successfully");
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

                let title: Vec<u16> = "System Monitor — Startup Error\0"
                    .encode_utf16()
                    .collect();
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
