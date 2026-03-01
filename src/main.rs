#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use chrono::Local;
mod updater;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

struct ThemePalette;
impl ThemePalette {
    // Primary Vibrant Accents (Indigo)
    const ACCENT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(99, 102, 241); // Indigo 500
    const ACCENT_ACTIVE: egui::Color32 = egui::Color32::from_rgb(129, 140, 248); // Indigo 400

    // Sleek Dark Backgrounds (Zinc)
    const BG_DEEPEST: egui::Color32 = egui::Color32::from_rgb(9, 9, 11); // Zinc 950
    const BG_DEEP: egui::Color32 = egui::Color32::from_rgb(15, 15, 18);
    const BG_SURFACE: egui::Color32 = egui::Color32::from_rgb(24, 24, 27); // Zinc 900
    const BG_CARD: egui::Color32 = egui::Color32::from_rgb(28, 28, 32);
    const BG_TRACK: egui::Color32 = egui::Color32::from_rgb(39, 39, 42); // Zinc 800

    // Component states
    const WIDGET_INACTIVE: egui::Color32 = egui::Color32::from_rgb(39, 39, 42);
    const WIDGET_HOVERED: egui::Color32 = egui::Color32::from_rgb(63, 63, 70); // Zinc 700
    const BORDER: egui::Color32 = egui::Color32::from_rgb(39, 39, 42); // Zinc 800
    const BORDER_LIGHT: egui::Color32 = egui::Color32::from_rgb(63, 63, 70);
    const ACCENT_LINE: egui::Color32 = egui::Color32::from_rgb(39, 39, 42);

    // Modern Status Colors (Vibrant but accessible)
    const STATUS_HEALTHY: egui::Color32 = egui::Color32::from_rgb(52, 211, 153); // Emerald 400
    const STATUS_WARNING: egui::Color32 = egui::Color32::from_rgb(251, 191, 36); // Amber 400
    const STATUS_CRITICAL: egui::Color32 = egui::Color32::from_rgb(248, 113, 113); // Red 400

    // Gorgeous Typography hierarchy
    const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(250, 250, 250); // Zinc 50
    const TEXT_SELECTED: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
    const TEXT_FEATURE: egui::Color32 = egui::Color32::from_rgb(228, 228, 231); // Zinc 200
    const TEXT_SUBTITLE: egui::Color32 = egui::Color32::from_rgb(161, 161, 170); // Zinc 400
    const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(161, 161, 170); // Zinc 400
    const TEXT_LABEL: egui::Color32 = egui::Color32::from_rgb(113, 113, 122); // Zinc 500
    const TEXT_LABEL_SUB: egui::Color32 = egui::Color32::from_rgb(113, 113, 122); // Zinc 500
    const TEXT_TERTIARY: egui::Color32 = egui::Color32::from_rgb(82, 82, 91); // Zinc 600
    const TEXT_DIMMED: egui::Color32 = egui::Color32::from_rgb(82, 82, 91); // Zinc 600

    // Sidebar colors
    const TEXT_NAV: egui::Color32 = egui::Color32::from_rgb(161, 161, 170);
    const TEXT_ICON_INACTIVE: egui::Color32 = egui::Color32::from_rgb(113, 113, 122);
    const GPU_UNAVAILABLE: egui::Color32 = egui::Color32::from_rgb(63, 63, 70);
    const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(168, 85, 247); // Purple 500
}
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, Pid, System};

#[cfg(target_os = "windows")]
use nvml_wrapper::Nvml;

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

#[derive(Clone, PartialEq)]
enum AlertType {
    CpuHigh,
    MemoryHigh,
    GpuTempHigh,
    DiskSpaceLow,
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

// Startup item info
#[derive(Clone)]
struct StartupItem {
    name: String,
    command: String,
    enabled: bool,
    source: String, // "Registry" or "Startup Folder"
}

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
    Status,
}

struct SystemMonitor {
    sys: System,
    disks: Disks,
    networks: Networks,
    #[cfg(target_os = "windows")]
    nvml: Option<Nvml>,
    last_network_update: Instant,
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
            theme_dark: false,
            show_per_core_cpu: false,
            process_count: 15,
            auto_clear_alerts: false,
            auto_start: false,
            start_minimized: false,
            minimize_to_tray: false,
            auto_ram_clean: false,
            ram_clean_threshold: 85.0,
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
        if let Some(config_dir) = directories::ProjectDirs::from("com", "SystemMonitor", "SystemMonitor") {
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
        if let Some(config_dir) = directories::ProjectDirs::from("com", "SystemMonitor", "SystemMonitor") {
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
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        #[cfg(target_os = "windows")]
        let nvml = Nvml::init().ok();

        SystemMonitor {
            sys,
            disks,
            networks,
            #[cfg(target_os = "windows")]
            nvml,
            last_network_update: Instant::now(),
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
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
                status: format!("{:?}", process.status()),
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

    fn kill_process(&mut self, pid: u32) -> bool {
        if let Some(process) = self.sys.process(Pid::from_u32(pid)) {
            process.kill()
        } else {
            false
        }
    }

    #[cfg(target_os = "windows")]
    fn suspend_process(&mut self, pid: u32) -> bool {
        use std::process::Command;
        // Use pssuspend-like approach via powershell to suspend a process
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(format!(
                "try {{ $proc = Get-Process -Id {} -ErrorAction Stop; $proc.Suspend(); $true }} catch {{ try {{ $code = @'\nAdd-Type -Name Suspender -Namespace Win32 -MemberDefinition @\"\n[DllImport(\"ntdll.dll\", SetLastError=true)] public static extern int NtSuspendProcess(IntPtr hProcess);\n[DllImport(\"kernel32.dll\", SetLastError=true)] public static extern IntPtr OpenProcess(int access, bool inherit, int pid);\n[DllImport(\"kernel32.dll\")] public static extern bool CloseHandle(IntPtr h);\n\"@\n'@; Invoke-Expression $code; $h = [Win32.Suspender]::OpenProcess(0x0800, $false, {}); if($h -ne [IntPtr]::Zero) {{ [Win32.Suspender]::NtSuspendProcess($h); [Win32.Suspender]::CloseHandle($h); $true }} else {{ $false }} }} catch {{ $false }} }}",
                pid, pid
            ))
            .output();
        match output {
            Ok(o) => String::from_utf8_lossy(&o.stdout).trim().contains("True"),
            Err(_) => false,
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn suspend_process(&mut self, _pid: u32) -> bool {
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
        use std::process::Command;
        let output = Command::new("powershell")
            .arg("-Command")
            .arg("Get-CimInstance Win32_Battery | Select-Object EstimatedChargeRemaining, BatteryStatus | ConvertTo-Json")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let json_str = String::from_utf8_lossy(&output.stdout);
        let json_str = json_str.trim();
        if json_str.is_empty() || json_str == "null" {
            return None;
        }
        // Parse minimal JSON
        let percentage: f32 = json_str
            .split("EstimatedChargeRemaining")
            .nth(1)
            .and_then(|s| s.split([',', '}'].as_ref()).next())
            .and_then(|s| s.trim().trim_start_matches([':', ' '].as_ref()).parse().ok())
            .unwrap_or(0.0);
        let status_num: u32 = json_str
            .split("BatteryStatus")
            .nth(1)
            .and_then(|s| s.split([',', '}'].as_ref()).next())
            .and_then(|s| s.trim().trim_start_matches([':', ' '].as_ref()).parse().ok())
            .unwrap_or(0);
        let is_charging = status_num == 2 || status_num == 6 || status_num == 7 || status_num == 8 || status_num == 9;
        let status_text = match status_num {
            1 => "Discharging".to_string(),
            2 => "Plugged In".to_string(),
            3 => "Fully Charged".to_string(),
            4 => "Low".to_string(),
            5 => "Critical".to_string(),
            6 | 7 | 8 | 9 => "Charging".to_string(),
            _ => "Unknown".to_string(),
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
        use std::process::Command;
        // Get memory before
        let mem_before = self.sys.used_memory();

        // Use PowerShell to clean working sets of all user-accessible processes
        let _ = Command::new("powershell")
            .arg("-Command")
            .arg(
                r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class MemCleaner {
    [DllImport("psapi.dll")] public static extern bool EmptyWorkingSet(IntPtr hProcess);
    [DllImport("kernel32.dll")] public static extern IntPtr OpenProcess(int access, bool inherit, int pid);
    [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
}
"@
Get-Process | ForEach-Object {
    try {
        $h = [MemCleaner]::OpenProcess(0x1F0FFF, $false, $_.Id)
        if ($h -ne [IntPtr]::Zero) {
            [MemCleaner]::EmptyWorkingSet($h) | Out-Null
            [MemCleaner]::CloseHandle($h) | Out-Null
        }
    } catch {}
}
"#,
            )
            .output();

        // Refresh system info to get new memory reading
        self.sys.refresh_memory();
        let mem_after = self.sys.used_memory();
        mem_before.saturating_sub(mem_after)
    }

    #[cfg(not(target_os = "windows"))]
    fn clean_ram(&mut self) -> u64 {
        0
    }

    #[cfg(target_os = "windows")]
    fn get_startup_items(&self) -> Vec<StartupItem> {
        use std::process::Command;
        let mut items = Vec::new();

        // Read from HKCU Run key
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(r#"Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue | ForEach-Object { $_.PSObject.Properties | Where-Object { $_.Name -notlike 'PS*' } | ForEach-Object { "$($_.Name)|$($_.Value)" } }"#)
            .output();
        if let Ok(o) = output {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                if parts.len() == 2 {
                    items.push(StartupItem {
                        name: parts[0].trim().to_string(),
                        command: parts[1].trim().to_string(),
                        enabled: true,
                        source: "Registry (HKCU)".to_string(),
                    });
                }
            }
        }

        // Read from HKLM Run key
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(r#"Get-ItemProperty -Path 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue | ForEach-Object { $_.PSObject.Properties | Where-Object { $_.Name -notlike 'PS*' } | ForEach-Object { "$($_.Name)|$($_.Value)" } }"#)
            .output();
        if let Ok(o) = output {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                if parts.len() == 2 {
                    items.push(StartupItem {
                        name: parts[0].trim().to_string(),
                        command: parts[1].trim().to_string(),
                        enabled: true,
                        source: "Registry (HKLM)".to_string(),
                    });
                }
            }
        }

        // Read from startup folder
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(r#"Get-ChildItem "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup" -ErrorAction SilentlyContinue | ForEach-Object { "$($_.BaseName)|$($_.FullName)" }"#)
            .output();
        if let Ok(o) = output {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                if parts.len() == 2 {
                    items.push(StartupItem {
                        name: parts[0].trim().to_string(),
                        command: parts[1].trim().to_string(),
                        enabled: true,
                        source: "Startup Folder".to_string(),
                    });
                }
            }
        }

        items
    }

    #[cfg(not(target_os = "windows"))]
    fn get_startup_items(&self) -> Vec<StartupItem> {
        Vec::new()
    }

    #[cfg(target_os = "windows")]
    fn remove_startup_item(name: &str, source: &str) -> bool {
        use std::process::Command;
        if source.contains("HKCU") {
            Command::new("powershell")
                .arg("-Command")
                .arg(format!(
                    "Remove-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -Name '{}' -ErrorAction SilentlyContinue",
                    name
                ))
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        } else if source.contains("Startup Folder") {
            // Remove shortcut from startup folder
            Command::new("powershell")
                .arg("-Command")
                .arg(format!(
                    "Remove-Item \"$env:APPDATA\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\{}*\" -ErrorAction SilentlyContinue",
                    name
                ))
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        } else {
            false // HKLM requires admin
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn remove_startup_item(_name: &str, _source: &str) -> bool {
        false
    }

    #[cfg(target_os = "windows")]
    fn set_process_priority(pid: u32, priority: &str) -> bool {
        use std::process::Command;
        let priority_class = match priority {
            "Realtime" => "RealTime",
            "High" => "High",
            "AboveNormal" => "AboveNormal",
            "Normal" => "Normal",
            "BelowNormal" => "BelowNormal",
            "Idle" => "Idle",
            _ => return false,
        };
        Command::new("powershell")
            .arg("-Command")
            .arg(format!(
                "try {{ (Get-Process -Id {}).PriorityClass = '{}'; $true }} catch {{ $false }}",
                pid, priority_class
            ))
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().contains("True"))
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    fn set_process_priority(_pid: u32, _priority: &str) -> bool {
        false
    }

    #[cfg(target_os = "windows")]
    fn get_gpu_info(&self) -> Option<GpuInfo> {
        if let Some(ref nvml) = self.nvml {
            if let Ok(device_count) = nvml.device_count() {
                if device_count > 0 {
                    if let Ok(device) = nvml.device_by_index(0) {
                        let name = device.name().unwrap_or_else(|_| "Unknown GPU".to_string());
                        let utilization = device.utilization_rates().map(|u| u.gpu).unwrap_or(0);
                        let memory = device.memory_info().ok();
                        let temperature = device
                            .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                            .ok();

                        return Some(GpuInfo {
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
        None
    }

    #[cfg(not(target_os = "windows"))]
    fn get_gpu_info(&self) -> Option<GpuInfo> {
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
            if let Some(ref gpu) = data.gpu_info {
                if let Some(temp) = gpu.temperature {
                    if temp > settings.notification_temp_threshold {
                        alerts.push(AlertInfo {
                            timestamp: timestamp.clone(),
                            alert_type: AlertType::GpuTempHigh,
                            message: format!("GPU temperature is high: {}°C", temp),
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
    gpu_info: Option<GpuInfo>,
    top_processes: Vec<ProcessInfo>,
    disk_info: Vec<DiskInfo>,
    network_info: Vec<NetworkInfo>,
    system_info: SystemInfo,
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
}

impl Default for SystemData {
    fn default() -> Self {
        Self {
            memory_total: 0,
            memory_used: 0,
            memory_percentage: 0.0,
            cpu_usage: 0.0,
            cpu_cores: Vec::new(),
            gpu_info: None,
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
    show_cpu_cores: bool,
    selected_process_pid: Option<u32>,
    always_on_top: bool,
    process_search: String,
    process_sort_column: ProcessSortColumn,
    process_sort_ascending: bool,
    show_export_csv: bool,
    updater: updater::Updater,
    show_update_notification: bool,
    update_check_time: Option<Instant>,
    ram_cleaner_state: RamCleanerState,
    startup_items: Vec<StartupItem>,
    startup_items_loaded: bool,
    show_shortcuts: bool,
    suspend_process_pid: Option<u32>,
    priority_change: Option<(u32, String)>,
}

#[derive(PartialEq)]
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
        // Install image loaders for showing the logo
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Load settings
        let settings = AppSettings::load();

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
            visuals.selection.stroke = egui::Stroke::new(1.0, ThemePalette::ACCENT_ACTIVE);
            visuals.hyperlink_color = ThemePalette::ACCENT_PRIMARY;

            // Subtle borders & widgets
            visuals.widgets.noninteractive.bg_fill = ThemePalette::BG_CARD;
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_PRIMARY);

            // Inactive
            visuals.widgets.inactive.bg_fill = ThemePalette::WIDGET_INACTIVE;
            visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SECONDARY);

            // Hovered
            visuals.widgets.hovered.bg_fill = ThemePalette::WIDGET_HOVERED;
            visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER_LIGHT);
            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);

            // Active
            visuals.widgets.active.bg_fill = ThemePalette::ACCENT_ACTIVE;
            visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ThemePalette::ACCENT_PRIMARY);
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);

            // Rounding (Modern smooth)
            visuals.window_rounding = egui::Rounding::same(12.0);
            visuals.menu_rounding = egui::Rounding::same(10.0);
            visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
            visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
            visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
            visuals.widgets.active.rounding = egui::Rounding::same(8.0);

            // Window chrome and depth
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

            cc.egui_ctx.set_visuals(visuals);
        } else {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
        }

        cc.egui_ctx.set_style(style);

        let data = Arc::new(Mutex::new(SystemData::default()));
        let data_clone = Arc::clone(&data);
        let shared_settings = Arc::new(Mutex::new(settings.clone()));
        let shared_settings_clone = Arc::clone(&shared_settings);

        // Background thread for monitoring
        thread::spawn(move || {
            let mut monitor = SystemMonitor::new();

            // Get system info once (doesn't change)
            let system_info = monitor.get_system_info();
            let mut battery_check_counter: u32 = 0;

            loop {
                thread::sleep(Duration::from_millis(500));
                monitor.refresh();

                // Read current settings from shared state
                let (refresh_interval, process_count, settings_snapshot) = {
                    let s = shared_settings_clone.lock().unwrap();
                    (s.refresh_interval, s.process_count, s.clone())
                };

                let (total_mem, used_mem, mem_percentage) = monitor.get_memory_info();
                let cpu_usage = monitor.get_cpu_usage();
                let cpu_cores = monitor.get_cpu_cores_info();
                let gpu_info = monitor.get_gpu_info();
                let top_processes = monitor.get_top_processes(process_count);
                let disk_info = monitor.get_disk_info();
                let network_info = monitor.get_network_info();
                let swap_info = monitor.get_swap_info();
                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

                // Get battery info every 30 seconds (it's slow via WMI)
                let battery_info = if battery_check_counter % 15 == 0 {
                    monitor.get_battery_info()
                } else {
                    None
                };
                battery_check_counter = battery_check_counter.wrapping_add(1);

                // Calculate total network rates
                let total_download_rate: f64 = network_info.iter().map(|n| n.received_rate).sum();
                let total_upload_rate: f64 = network_info.iter().map(|n| n.transmitted_rate).sum();

                if let Ok(mut data) = data_clone.lock() {
                    let elapsed = data.start_time.elapsed().as_secs_f64();

                    // Update current values
                    data.memory_total = total_mem;
                    data.memory_used = used_mem;
                    data.memory_percentage = mem_percentage;
                    data.cpu_usage = cpu_usage;
                    data.cpu_cores = cpu_cores;
                    data.gpu_info = gpu_info.clone();
                    data.top_processes = top_processes;
                    data.disk_info = disk_info;
                    data.network_info = network_info;
                    data.system_info = system_info.clone();
                    data.last_update = timestamp;
                    data.swap_info = swap_info;
                    if let Some(bi) = battery_info {
                        data.battery_info = Some(bi);
                    }
                    data.network_sample_count += 1;

                    // Check for alerts
                    let new_alerts = monitor.check_alerts(&settings_snapshot, &data);

                    // Send desktop notifications for new alerts
                    if settings_snapshot.show_notifications {
                        for alert in &new_alerts {
                            let _ = notify_rust::Notification::new()
                                .summary("System Monitor Alert")
                                .body(&alert.message)
                                .timeout(notify_rust::Timeout::Milliseconds(5000))
                                .show();
                        }
                    }

                    data.alerts.extend(new_alerts);

                    // Auto-clear resolved alerts
                    if settings_snapshot.auto_clear_alerts {
                        data.alerts.retain(|alert| {
                            match alert.alert_type {
                                AlertType::CpuHigh => cpu_usage > settings_snapshot.notification_cpu_threshold,
                                AlertType::MemoryHigh => {
                                    mem_percentage > settings_snapshot.notification_memory_threshold
                                }
                                AlertType::GpuTempHigh => {
                                    if let Some(ref g) = gpu_info {
                                        g.temperature
                                            .map_or(false, |t| t > settings_snapshot.notification_temp_threshold)
                                    } else {
                                        false
                                    }
                                }
                                AlertType::DiskSpaceLow => true, // disk alerts don't auto-clear
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

                    if let Some(ref gpu) = gpu_info {
                        data.gpu_history.push_back(DataPoint {
                            time: elapsed,
                            value: gpu.utilization as f64,
                        });
                    }

                    // Network history — skip first sample (inflated rates)
                    if data.network_sample_count > 1 {
                        data.network_download_history.push_back(DataPoint {
                            time: elapsed,
                            value: total_download_rate,
                        });
                        data.network_upload_history.push_back(DataPoint {
                            time: elapsed,
                            value: total_upload_rate,
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
                }

                let sleep_ms = (refresh_interval * 1000).saturating_sub(500);
                thread::sleep(Duration::from_millis(sleep_ms));
            }
        });

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
            always_on_top: false,
            process_search: String::new(),
            process_sort_column: ProcessSortColumn::Memory,
            process_sort_ascending: false,
            show_export_csv: false,
            updater: updater::Updater::new(),
            show_update_notification: false,
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
            show_shortcuts: false,
            suspend_process_pid: None,
            priority_change: None,
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
        if let Some(ref gpu) = data.gpu_info {
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
        }

        let export = ExportData {
            timestamp: data.last_update.clone(),
            cpu_usage: data.cpu_usage,
            memory_used: data.memory_used,
            memory_total: data.memory_total,
            memory_percentage: data.memory_percentage,
            gpu_info: data.gpu_info.clone(),
            top_processes: data.top_processes.clone(),
            disk_info: data.disk_info.clone(),
            network_info: data.network_info.clone(),
            system_info: data.system_info.clone(),
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
        // Request repaint for continuous updates
        ctx.request_repaint();

        // Handle always on top
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.always_on_top {
            egui::viewport::WindowLevel::AlwaysOnTop
        } else {
            egui::viewport::WindowLevel::Normal
        }));

        // Check for updates automatically (once every 24 hours)
        if self.update_check_time.is_none() || self.update_check_time.unwrap().elapsed().as_secs() > 86400 {
            let mut updater = self.updater.clone();
            let ctx_clone = ctx.clone();
            thread::spawn(move || {
                if let Ok(update_info) = updater.check_for_updates() {
                    if update_info.update_available {
                        ctx_clone.request_repaint();
                    }
                }
            });
            self.update_check_time = Some(Instant::now());
        }

        // Show update notification banner
        if self.updater.get_update_info().update_available {
            egui::TopBottomPanel::top("update_notification").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(ThemePalette::STATUS_HEALTHY, "🎉");
                    ui.label(format!(
                        "New version {} is available! Current: {}",
                        self.updater.get_update_info().latest_version,
                        self.updater.get_update_info().current_version
                    ));
                    if ui.button("⬇️ Download & Install").clicked() {
                        let download_url = self.updater.get_update_info().download_url.clone();
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
                if let Ok(mut data) = self.data.lock() {
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
                thread::spawn(move || {
                    let _ = updater.check_for_updates();
                });
            }
        });

        let data = self.data.lock().unwrap().clone();

        // Handle process kill actions
        if let Some(pid) = self.selected_process_pid.take() {
            let mut temp_sys = System::new();
            temp_sys.refresh_processes();
            if let Some(process) = temp_sys.process(Pid::from_u32(pid)) {
                let _ = process.kill();
            }
        }

        // Handle process suspend actions
        if let Some(pid) = self.suspend_process_pid.take() {
            let mut monitor = SystemMonitor::new();
            monitor.suspend_process(pid);
        }

        // Handle process priority changes
        if let Some((pid, priority)) = self.priority_change.take() {
            SystemMonitor::set_process_priority(pid, &priority);
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
                thread::spawn(move || {
                    let mut monitor = SystemMonitor::new();
                    let freed = monitor.clean_ram();
                    // Store freed bytes in SystemData for the UI to pick up
                    if let Ok(mut d) = data_arc.lock() {
                        d.ram_clean_freed_bytes += freed;
                        d.ram_clean_is_cleaning = false;
                    }
                    ctx_clone.request_repaint();
                });
                // Mark cleaning in shared data too
                if let Ok(mut d) = self.data.lock() {
                    d.ram_clean_is_cleaning = true;
                }
            }
        }
        // Sync back from shared data
        if let Ok(d) = self.data.lock() {
            if !d.ram_clean_is_cleaning && self.ram_cleaner_state.is_cleaning {
                self.ram_cleaner_state.is_cleaning = false;
            }
            self.ram_cleaner_state.bytes_freed = d.ram_clean_freed_bytes;
        }

        // CSV Export window
        let mut show_export_csv = self.show_export_csv;
        if show_export_csv {
            let csv_result = self.export_to_csv(&data);
            egui::Window::new("📊 Export to CSV")
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
                            if ui.button("📋 Copy to Clipboard").clicked() {
                                ui.output_mut(|o| o.copied_text = csv_data.clone());
                            }

                            ui.add_space(5.0);
                            ui.label("💡 Tip: Open in Excel or any spreadsheet application");
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
            egui::Window::new("💾 Export Data")
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
                            if ui.button("📋 Copy to Clipboard").clicked() {
                                ui.output_mut(|o| o.copied_text = json_data.clone());
                            }

                            ui.add_space(5.0);
                            ui.label("💡 Tip: You can paste this into a .json file");
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
            egui::Window::new("🚨 System Alerts")
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
                                        AlertType::CpuHigh => ("⚡", egui::Color32::YELLOW),
                                        AlertType::MemoryHigh => ("💾", egui::Color32::YELLOW),
                                        AlertType::GpuTempHigh => ("🔥", egui::Color32::RED),
                                        AlertType::DiskSpaceLow => ("💽", egui::Color32::RED),
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
                        if ui.button("🗑️ Clear All Alerts").clicked() {
                            clear_alerts = true;
                        }
                    }
                });
        }
        self.show_alerts = show_alerts;
        if clear_alerts {
            if let Ok(mut data) = self.data.lock() {
                data.alerts.clear();
            }
        }

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.settings.show_graphs, "Show Performance Graphs");
                    ui.checkbox(&mut self.settings.show_gpu, "Show GPU Section");
                    ui.checkbox(&mut self.settings.show_processes, "Show Process List");
                    ui.separator();
                    if ui.button("⚙️ Settings").clicked() {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("💾 Export Data to JSON").clicked() {
                        self.show_export = true;
                        ui.close_menu();
                    }
                    if ui.button("📊 Export to CSV").clicked() {
                        self.show_export_csv = true;
                        ui.close_menu();
                    }
                    if ui.button("💾 Save JSON to File").clicked() {
                        // Save to file with file picker
                        if let Ok(json) = self.export_data_to_json(&data) {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("system-report.json")
                                .add_filter("JSON", &["json"])
                                .save_file()
                            {
                                let _ = std::fs::write(path, json);
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("📊 Save CSV to File").clicked() {
                        // Save CSV to file
                        if let Ok(csv) = self.export_to_csv(&data) {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("system-report.csv")
                                .add_filter("CSV", &["csv"])
                                .save_file()
                            {
                                let _ = std::fs::write(path, csv);
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🔄 Reset Statistics").clicked() {
                        if let Ok(mut data) = self.data.lock() {
                            data.cpu_history.clear();
                            data.memory_history.clear();
                            data.gpu_history.clear();
                            data.network_download_history.clear();
                            data.network_upload_history.clear();
                            data.alerts.clear();
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🚨 View Alerts").clicked() {
                        self.show_alerts = true;
                        ui.close_menu();
                    }
                    if ui.button("⚙️ Process Manager").clicked() {
                        self.show_process_manager = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Window", |ui| {
                    if ui.checkbox(&mut self.always_on_top, "Always on Top").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("🔄 Restart Application").clicked() {
                        std::process::Command::new(std::env::current_exe().unwrap())
                            .spawn()
                            .ok();
                        std::process::exit(0);
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.selected_tab = Tab::About;
                        ui.close_menu();
                    }
                    if ui.button("⌨️ Keyboard Shortcuts").clicked() {
                        self.show_shortcuts = true;
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("🕒 {}", data.last_update));
                });
            });
        });

        // Side panel — branded nav with custom tabs
        egui::SidePanel::left("sidebar")
            .min_width(190.0)
            .max_width(210.0)
            .show(ctx, |ui| {
                // ── Brand header ──
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    // Painted diamond glyph
                    let r = ui.label(egui::RichText::new(" ").size(17.0));
                    let cy = r.rect.center().y;
                    let cx = r.rect.left() + 2.0;
                    let sz = 7.0;
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
                            .size(17.0)
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new("Mon")
                            .size(17.0)
                            .strong()
                            .color(ThemePalette::TEXT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(format!("v{}", APP_VERSION))
                            .size(9.5)
                            .color(ThemePalette::TEXT_DIMMED),
                    );
                });
                ui.add_space(6.0);
                // Thin accent line under brand
                {
                    let r = ui.cursor();
                    ui.painter().line_segment(
                        [
                            egui::pos2(r.left() + 12.0, r.top()),
                            egui::pos2(r.right() - 12.0, r.top()),
                        ],
                        egui::Stroke::new(1.0, ThemePalette::ACCENT_LINE),
                    );
                }
                ui.add_space(10.0);

                // ── Navigation label ──
                {
                    let r = ui.cursor();
                    ui.painter().text(
                        egui::pos2(r.left() + 14.0, r.top()),
                        egui::Align2::LEFT_TOP,
                        "NAVIGATION",
                        egui::FontId::proportional(9.5),
                        ThemePalette::TEXT_NAV,
                    );
                }
                ui.add_space(18.0);

                // ── Tab items ──
                draw_nav_tab(ui, &mut self.selected_tab, Tab::Overview, "Overview", None);
                draw_nav_tab(ui, &mut self.selected_tab, Tab::Performance, "Performance", None);
                draw_nav_tab(ui, &mut self.selected_tab, Tab::Processes, "Processes", None);
                draw_nav_tab(ui, &mut self.selected_tab, Tab::CpuCores, "CPU Cores", None);
                draw_nav_tab(ui, &mut self.selected_tab, Tab::Storage, "Storage", None);
                draw_nav_tab(ui, &mut self.selected_tab, Tab::Network, "Network", None);
                draw_nav_tab(ui, &mut self.selected_tab, Tab::SystemInfo, "System Info", None);

                ui.add_space(6.0);
                {
                    let r = ui.cursor();
                    ui.painter().line_segment(
                        [
                            egui::pos2(r.left() + 12.0, r.top()),
                            egui::pos2(r.right() - 12.0, r.top()),
                        ],
                        egui::Stroke::new(1.0, ThemePalette::ACCENT_LINE),
                    );
                }
                ui.add_space(6.0);

                let alert_count = data.alerts.len();
                draw_nav_tab(
                    ui,
                    &mut self.selected_tab,
                    Tab::Alerts,
                    "Alerts",
                    if alert_count > 0 { Some(alert_count) } else { None },
                );
                draw_nav_tab(ui, &mut self.selected_tab, Tab::RamCleaner, "RAM Cleaner", None);
                draw_nav_tab(ui, &mut self.selected_tab, Tab::StartupManager, "Startup Apps", None);
                draw_nav_tab(ui, &mut self.selected_tab, Tab::About, "About", None);

                // ── Quick stats ──
                ui.add_space(14.0);
                {
                    let r = ui.cursor();
                    ui.painter().text(
                        egui::pos2(r.left() + 14.0, r.top()),
                        egui::Align2::LEFT_TOP,
                        "QUICK STATS",
                        egui::FontId::proportional(9.5),
                        ThemePalette::TEXT_NAV,
                    );
                }
                ui.add_space(18.0);

                draw_mini_stat(ui, "CPU", data.cpu_usage);
                draw_mini_stat(ui, "RAM", data.memory_percentage);
                if let Some(ref gpu) = data.gpu_info {
                    draw_mini_stat(ui, "GPU", gpu.utilization);
                }
                if data.swap_info.total > 0 {
                    draw_mini_stat(ui, "SWAP", data.swap_info.percentage);
                }
            });

        // Process Manager window
        if self.show_process_manager {
            self.show_process_manager_window(ctx, &data);
        }

        // Keyboard Shortcuts dialog
        let mut show_shortcuts = self.show_shortcuts;
        if show_shortcuts {
            egui::Window::new("⌨️ Keyboard Shortcuts")
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
            egui::Window::new("⚙️ Settings")
                .open(&mut show_settings)
                .resizable(true)
                .default_width(600.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    self.show_settings_tab(ui);
                });
            self.show_settings = show_settings;
        }

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
            Tab::About => self.show_about_tab(ui),
        });
    }
}

// ─── Custom UI helpers ───────────────────────────────────────────────

/// Section header with sleek gradient-like accent underline
fn paint_section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    let r = ui.label(
        egui::RichText::new(text)
            .size(24.0)
            .strong()
            .color(ThemePalette::TEXT_PRIMARY),
    );
    let y = r.rect.bottom() + 4.0;

    // Modern thick rounded line highlight
    ui.painter().line_segment(
        [egui::pos2(r.rect.left(), y), egui::pos2(r.rect.left() + 48.0, y)],
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

/// Sidebar navigation tab with left accent bar + hover highlight
fn draw_nav_tab(ui: &mut egui::Ui, selected: &mut Tab, tab: Tab, label: &str, badge: Option<usize>) {
    let is_sel = *selected == tab;
    let size = egui::vec2(ui.available_width(), 38.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let p = ui.painter();

    // Pill selection hover/active states
    if is_sel {
        // Indigo 500 with low opacity
        p.rect_filled(rect, 8.0, egui::Color32::from_rgba_premultiplied(99, 102, 241, 24));

        // Left accent bar
        p.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.top() + 8.0),
                egui::vec2(4.0, rect.height() - 16.0),
            ),
            2.0,
            ThemePalette::ACCENT_PRIMARY,
        );
    } else if resp.hovered() {
        p.rect_filled(rect, 8.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 8));
    }

    let mid = rect.center().y;
    let ic = if is_sel {
        ThemePalette::ACCENT_PRIMARY
    } else {
        ThemePalette::TEXT_ICON_INACTIVE
    };
    let tc = if is_sel {
        ThemePalette::TEXT_SELECTED
    } else {
        ThemePalette::TEXT_SECONDARY
    };

    // Painted dot indicator
    p.circle_filled(egui::pos2(rect.left() + 20.0, mid), 3.5, ic);

    p.text(
        egui::pos2(rect.left() + 38.0, mid),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(15.0),
        tc,
    );

    if let Some(c) = badge {
        if c > 0 {
            let bx = rect.right() - 24.0;
            p.circle_filled(egui::pos2(bx, mid), 10.0, ThemePalette::STATUS_CRITICAL);
            p.text(
                egui::pos2(bx, mid),
                egui::Align2::CENTER_CENTER,
                &c.to_string(),
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
        }
    }

    if resp.clicked() {
        *selected = tab;
    }
}

/// Compact stat row for sidebar: label, value %, mini bar
fn draw_mini_stat(ui: &mut egui::Ui, label: &str, value: f32) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 24.0), egui::Sense::hover());
    let p = ui.painter();
    let color = get_usage_color(value);

    p.text(
        egui::pos2(rect.left() + 16.0, rect.top() + 4.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(12.0),
        ThemePalette::TEXT_LABEL,
    );
    p.text(
        egui::pos2(rect.right() - 16.0, rect.top() + 4.0),
        egui::Align2::RIGHT_TOP,
        &format!("{:.0}%", value),
        egui::FontId::proportional(12.0),
        color,
    );

    let bar_y = rect.bottom() - 4.0;
    let bar_w = w - 32.0;
    let track = egui::Rect::from_min_size(egui::pos2(rect.left() + 16.0, bar_y), egui::vec2(bar_w, 3.0));
    p.rect_filled(track, 1.5, ThemePalette::BG_DEEPEST);

    let fw = bar_w * (value / 100.0).clamp(0.0, 1.0);
    if fw > 0.5 {
        p.rect_filled(egui::Rect::from_min_size(track.min, egui::vec2(fw, 3.0)), 1.5, color);
    }
}

impl SystemMonitorApp {
    fn show_overview_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "System Overview");

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Metric cards row ──
            let card_bg = ThemePalette::BG_CARD;
            let card_border = egui::Stroke::new(1.0, ThemePalette::BORDER);
            let card_rnd = egui::Rounding::same(12.0); // Premium smooth rounding

            let full_avail = ui.available_width();
            let card_spacing = 16.0;
            let card_h = 120.0;
            let (row_rect, _) = ui.allocate_exact_size(egui::vec2(full_avail, card_h), egui::Sense::hover());
            let p = ui.painter();

            // Account for HiDPI: at ppp>1, available_width can exceed visible area
            let ppp = ui.ctx().pixels_per_point();
            let visible_w = if ppp > 1.01 {
                let screen_w = ui.ctx().screen_rect().width();
                (screen_w / ppp - row_rect.min.x).max(200.0)
            } else {
                full_avail
            };
            let card_w = (visible_w - card_spacing * 2.0) / 3.0;

            // Prepare card data
            let cpu_c = get_usage_color(data.cpu_usage);
            let mem_c = get_usage_color(data.memory_percentage);

            let (gpu_val, gpu_sub, gpu_frac, gpu_c) = if let Some(ref gpu) = data.gpu_info {
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
                    format!("{} cores", data.cpu_cores.len()),
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
            ];

            for (i, (accent, label, value, sub, frac, color)) in cards.iter().enumerate() {
                let x = row_rect.min.x + (card_w + card_spacing) * i as f32;
                let cr = egui::Rect::from_min_size(egui::pos2(x, row_rect.min.y), egui::vec2(card_w, card_h));

                // Deep card background with subtle inner border
                p.rect_filled(cr, card_rnd, ThemePalette::BG_DEEPEST);
                p.rect_filled(cr.shrink(1.0), card_rnd, card_bg);
                p.rect_stroke(cr, card_rnd, card_border);

                let m = 16.0;

                // Card header section
                p.circle_filled(egui::pos2(cr.left() + m + 4.0, cr.top() + m + 6.0), 3.5, *accent);
                p.text(
                    egui::pos2(cr.left() + m + 14.0, cr.top() + m),
                    egui::Align2::LEFT_TOP,
                    *label,
                    egui::FontId::proportional(12.0),
                    ThemePalette::TEXT_LABEL,
                );

                // Value text
                p.text(
                    egui::pos2(cr.left() + m, cr.top() + m + 22.0),
                    egui::Align2::LEFT_TOP,
                    value,
                    egui::FontId::proportional(32.0),
                    *color,
                );

                // Subtitle
                p.text(
                    egui::pos2(cr.left() + m, cr.top() + m + 60.0),
                    egui::Align2::LEFT_TOP,
                    sub,
                    egui::FontId::proportional(12.0),
                    ThemePalette::TEXT_TERTIARY,
                );

                // Progress bar track
                let bar_y = cr.top() + m + 80.0;
                let bar_w = card_w - m * 2.0;
                let bar_rect = egui::Rect::from_min_size(egui::pos2(cr.left() + m, bar_y), egui::vec2(bar_w, 6.0));

                p.rect_filled(bar_rect, 3.0, ThemePalette::BG_DEEPEST);
                let f = frac.clamp(0.0, 1.0);
                if f > 0.005 {
                    p.rect_filled(
                        egui::Rect::from_min_size(bar_rect.min, egui::vec2(bar_w * f, 6.0)),
                        3.0,
                        *color,
                    );
                }
            }

            ui.add_space(16.0);

            // ── Detail strip ──
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if let Some(ref gpu) = data.gpu_info {
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
                // CPU Graph
                ui.group(|ui| {
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
                        .y_axis_label("CPU %")
                        .show(ui, |plot_ui| {
                            plot_ui.line(line);
                        });
                });

                ui.add_space(10.0);

                // Memory Graph
                ui.group(|ui| {
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
                        .y_axis_label("Memory %")
                        .show(ui, |plot_ui| {
                            plot_ui.line(line);
                        });
                });

                ui.add_space(10.0);

                // GPU Graph
                if !data.gpu_history.is_empty() {
                    ui.group(|ui| {
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
                            .y_axis_label("GPU %")
                            .show(ui, |plot_ui| {
                                plot_ui.line(line);
                            });
                    });
                }
            } else {
                ui.label("Performance graphs are disabled. Enable them in View menu.");
            }
        });
    }

    fn show_processes_tab(&mut self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "Process Monitor");

        // Search box
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.process_search);
            if ui.button("✖").clicked() {
                self.process_search.clear();
            }
        });

        ui.add_space(5.0);

        // Filter processes
        let mut filtered_processes: Vec<_> = if self.process_search.is_empty() {
            data.top_processes.clone()
        } else {
            data.top_processes
                .iter()
                .filter(|p| p.name.to_lowercase().contains(&self.process_search.to_lowercase()))
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
                                " ▲"
                            } else {
                                " ▼"
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
                        let memory_color = if memory_mb > 500.0 {
                            ThemePalette::STATUS_CRITICAL
                        } else if memory_mb > 200.0 {
                            ThemePalette::STATUS_WARNING
                        } else {
                            ThemePalette::STATUS_HEALTHY
                        };

                        ui.label(process.pid.to_string());

                        let display_name = if process.name.chars().count() > 40 {
                            let truncated: String = process.name.chars().take(37).collect();
                            format!("{}...", truncated)
                        } else {
                            process.name.clone()
                        };
                        ui.label(display_name);

                        ui.colored_label(memory_color, format!("{:.2} MB", memory_mb));
                        ui.label(format!("{:.1}%", process.cpu_usage));

                        // Action buttons
                        ui.horizontal(|ui| {
                            if ui.small_button("📋").on_hover_text("Copy PID").clicked() {
                                ui.output_mut(|o| o.copied_text = process.pid.to_string());
                            }
                            if ui.small_button("📄").on_hover_text("Copy Name").clicked() {
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
                                ui.colored_label(egui::Color32::RED, "⚠️");
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
                                "⚠️ Warning: Only {:.2} GB remaining!",
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
                        egui::RichText::new("▼ Download Rate (MB/s)")
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
                        egui::RichText::new("▲ Upload Rate (MB/s)")
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
                        ui.label("📥 Download Rate:");
                        let color = if network.received_rate > 10.0 {
                            egui::Color32::GREEN
                        } else if network.received_rate > 1.0 {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::GRAY
                        };
                        ui.colored_label(color, format!("{:.2} MB/s", network.received_rate));
                    });

                    ui.horizontal(|ui| {
                        ui.label("📤 Upload Rate:");
                        let color = if network.transmitted_rate > 10.0 {
                            egui::Color32::GREEN
                        } else if network.transmitted_rate > 1.0 {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::GRAY
                        };
                        ui.colored_label(color, format!("{:.2} MB/s", network.transmitted_rate));
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
                    ui.colored_label(egui::Color32::GREEN, "✅");
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
                if ui.button("⚙️ Configure Alert Thresholds").clicked() {
                    self.selected_tab = Tab::About; // temp, will switch below
                    self.show_settings = true;
                }
            });
        } else {
            ui.label(format!("⚠️ {} active alert(s)", data.alerts.len()));
            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, alert) in data.alerts.iter().enumerate() {
                    ui.group(|ui| {
                        let (icon, color, severity) = match alert.alert_type {
                            AlertType::CpuHigh => ("⚡", egui::Color32::YELLOW, "WARNING"),
                            AlertType::MemoryHigh => ("💾", egui::Color32::YELLOW, "WARNING"),
                            AlertType::GpuTempHigh => ("🔥", egui::Color32::RED, "CRITICAL"),
                            AlertType::DiskSpaceLow => ("💽", egui::Color32::RED, "CRITICAL"),
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
                if ui.button("🗑️ Clear All Alerts").clicked() {
                    if let Ok(mut data) = self.data.lock() {
                        data.alerts.clear();
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("💡 Tip: Configure alert thresholds in Settings");
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

            if let Some(ref gpu_info) = data.gpu_info {
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
                            ui.strong(format!("{:.0} MB / {:.0} MB", bytes_to_mb(used), bytes_to_mb(total)));
                        });
                    }

                    if let Some(temp) = gpu_info.temperature {
                        ui.horizontal(|ui| {
                            ui.label("Temperature:");
                            let temp_color = if temp < 70 {
                                egui::Color32::GREEN
                            } else if temp < 85 {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(temp_color, format!("🌡️ {}°C", temp));
                        });
                    }
                });
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
                        let icon = if battery.is_charging { "🔌" } else { "🔋" };
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
                ui.heading("📊 Core Statistics");
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

        egui::Window::new("⚙️ Process Manager")
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
                        if ui.button("🔄 Refresh").clicked() {
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
                                    if ui.small_button("🗑️").on_hover_text("Kill Process").clicked() {
                                        self.selected_process_pid = Some(process.pid);
                                    }
                                    if ui.small_button("⏸️").on_hover_text("Suspend Process").clicked() {
                                        self.suspend_process_pid = Some(process.pid);
                                    }
                                    // Priority menu
                                    ui.menu_button("⚡", |ui| {
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
                    "⚠️ Warning: Killing/suspending processes may cause system instability!",
                );
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
                        thread::spawn(move || {
                            let mut monitor = SystemMonitor::new();
                            let freed = monitor.clean_ram();
                            if let Ok(mut d) = data_arc.lock() {
                                d.ram_clean_freed_bytes += freed;
                                d.ram_clean_is_cleaning = false;
                            }
                            ctx_clone.request_repaint();
                        });
                        if let Ok(mut d) = self.data.lock() {
                            d.ram_clean_is_cleaning = true;
                        }
                    }
                });

                if is_cleaning {
                    ui.colored_label(ThemePalette::ACCENT_PRIMARY, "⏳ Cleaning in progress...");
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
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("Startup Items");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🔄 Refresh").clicked() {
                            self.startup_items_loaded = false;
                        }
                    });
                });
                ui.separator();
                ui.label("Programs that run automatically when Windows starts.");
                ui.label("⚠️ Only user-level (HKCU) items can be removed. System-level items require Administrator privileges.");
            });

            // Load startup items lazily
            if !self.startup_items_loaded {
                let monitor = SystemMonitor::new();
                self.startup_items = monitor.get_startup_items();
                self.startup_items_loaded = true;
            }

            ui.add_space(10.0);

            if self.startup_items.is_empty() {
                ui.group(|ui| {
                    ui.add_space(20.0);
                    ui.label("No startup items found.");
                    ui.add_space(20.0);
                });
            } else {
                ui.label(format!("Found {} startup item(s)", self.startup_items.len()));
                ui.add_space(5.0);

                let mut item_to_remove: Option<usize> = None;

                for (i, item) in self.startup_items.iter().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong(&item.name);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.colored_label(ThemePalette::TEXT_TERTIARY, &item.source);
                            });
                        });

                        // Safe truncation of command path
                        let cmd_display = if item.command.chars().count() > 80 {
                            let truncated: String = item.command.chars().take(77).collect();
                            format!("{}...", truncated)
                        } else {
                            item.command.clone()
                        };
                        ui.label(egui::RichText::new(cmd_display).small().color(ThemePalette::TEXT_DIMMED));

                        ui.horizontal(|ui| {
                            // Only allow removal of HKCU and Startup Folder items
                            let can_remove = item.source.contains("HKCU") || item.source.contains("Startup Folder");
                            ui.add_enabled_ui(can_remove, |ui| {
                                if ui.button("🗑️ Remove from Startup").clicked() {
                                    if SystemMonitor::remove_startup_item(&item.name, &item.source) {
                                        item_to_remove = Some(i);
                                    }
                                }
                            });
                            if !can_remove {
                                ui.colored_label(ThemePalette::TEXT_DIMMED, "(Requires Admin)");
                            }
                        });
                    });
                    ui.add_space(3.0);
                }

                // Remove item if requested
                if let Some(idx) = item_to_remove {
                    self.startup_items.remove(idx);
                }
            }
        });
    }

    fn show_settings_tab(&mut self, ui: &mut egui::Ui) {
        paint_section_header(ui, "Application Settings");

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.heading("General");
                ui.add_space(8.0);

                let mut changed = false;
                let mut theme_changed = false;

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

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                ui.heading("Monitoring");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("Data refresh interval (seconds):");
                    changed |= ui
                        .add(egui::Slider::new(&mut self.settings.refresh_interval, 1..=10))
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Number of processes to show:");
                    changed |= ui
                        .add(egui::Slider::new(&mut self.settings.process_count, 5..=100))
                        .changed();
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                ui.heading("Alert Thresholds");
                ui.add_space(8.0);
                changed |= ui
                    .add(
                        egui::Slider::new(&mut self.settings.notification_cpu_threshold, 50.0..=100.0)
                            .text("CPU Usage % Alert"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::Slider::new(&mut self.settings.notification_memory_threshold, 50.0..=100.0)
                            .text("Memory Usage % Alert"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::Slider::new(&mut self.settings.notification_temp_threshold, 60..=105)
                            .text("Temperature °C Alert"),
                    )
                    .changed();

                #[cfg(target_os = "windows")]
                {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.heading("Windows Integration");
                    ui.add_space(8.0);
                    if ui
                        .checkbox(&mut self.settings.auto_start, "Start with Windows")
                        .changed()
                    {
                        changed = true;
                        let _ = self.settings.set_auto_start(self.settings.auto_start);
                    }
                }

                if changed {
                    let _ = self.settings.save();
                    // Sync settings to the background thread
                    if let Ok(mut shared) = self.shared_settings.lock() {
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
        });
    }
    fn show_about_tab(&self, ui: &mut egui::Ui) {
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

fn main() -> Result<(), eframe::Error> {
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

    eframe::run_native(
        "System Monitor",
        options,
        Box::new(|cc| Ok(Box::new(SystemMonitorApp::new(cc)))),
    )
}
