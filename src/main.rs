#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use chrono::Local;
mod updater;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, System, Pid};

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

    fn suspend_process(&mut self, _pid: u32) -> bool {
        // Suspend process is complex on Windows and not implemented in this version
        // Would require additional Windows API calls
        false
    }

    #[cfg(target_os = "windows")]
    fn get_gpu_info(&self) -> Option<GpuInfo> {
        if let Some(ref nvml) = self.nvml {
            if let Ok(device_count) = nvml.device_count() {
                if device_count > 0 {
                    if let Ok(device) = nvml.device_by_index(0) {
                        let name = device.name().unwrap_or_else(|_| "Unknown GPU".to_string());
                        let utilization = device
                            .utilization_rates()
                            .map(|u| u.gpu)
                            .unwrap_or(0);
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
        
        let network_info: Vec<NetworkInfo> = self.networks
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
            cpu_brand: self.sys.cpus().first()
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
        }
    }
}

struct SystemMonitorApp {
    data: Arc<Mutex<SystemData>>,
    settings: AppSettings,
    selected_tab: Tab,
    show_settings: bool,
    show_export: bool,
    show_alerts: bool,
    show_process_manager: bool,
    show_cpu_cores: bool,
    selected_process_pid: Option<u32>,
    monitor_handle: Option<Arc<Mutex<SystemMonitor>>>,
    always_on_top: bool,
    process_search: String,
    process_sort_column: ProcessSortColumn,
    process_sort_ascending: bool,
    show_export_csv: bool,
    updater: updater::Updater,
    show_update_notification: bool,
    update_check_time: Option<Instant>,
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
    About,
}

impl SystemMonitorApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load settings
        let settings = AppSettings::load();

        // Configure fonts and style
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);

        // Typographic hierarchy
        use egui::{FontFamily, FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
            (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        ].into();

        // Apply theme — custom "Terminal Noir" dark or standard light
        if settings.theme_dark {
            let mut visuals = egui::Visuals::dark();
            // Deep charcoal backgrounds
            visuals.panel_fill = egui::Color32::from_rgb(13, 15, 20);         // bg-deep
            visuals.window_fill = egui::Color32::from_rgb(19, 22, 29);        // bg-surface
            visuals.extreme_bg_color = egui::Color32::from_rgb(8, 9, 12);     // bg-deepest

            // Cyan accent for selections and interactions
            visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(0, 229, 255, 60);
            visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 229, 255));
            visuals.hyperlink_color = egui::Color32::from_rgb(0, 229, 255);

            // Subtle borders
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(38, 42, 56));
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(26, 30, 40);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 40, 54);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 184, 212);
            
            // Rounding
            visuals.window_rounding = egui::Rounding::same(10.0);
            visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
            visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
            visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
            visuals.widgets.active.rounding = egui::Rounding::same(6.0);

            // Window chrome
            visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(0, 229, 255, 30));
            visuals.window_shadow = egui::epaint::Shadow {
                offset: egui::vec2(0.0, 8.0),
                blur: 24.0,
                spread: 0.0,
                color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 100),
            };

            cc.egui_ctx.set_visuals(visuals);
        } else {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
        }
        
        cc.egui_ctx.set_style(style);

        let data = Arc::new(Mutex::new(SystemData::default()));
        let data_clone = Arc::clone(&data);
        let refresh_interval = settings.refresh_interval;
        let settings_clone = settings.clone();

        // Background thread for monitoring
        thread::spawn(move || {
            let mut monitor = SystemMonitor::new();
            
            // Get system info once (doesn't change)
            let system_info = monitor.get_system_info();
            
            loop {
                thread::sleep(Duration::from_millis(500));
                monitor.refresh();

                let (total_mem, used_mem, mem_percentage) = monitor.get_memory_info();
                let cpu_usage = monitor.get_cpu_usage();
                let cpu_cores = monitor.get_cpu_cores_info();
                let gpu_info = monitor.get_gpu_info();
                let top_processes = monitor.get_top_processes(15);
                let disk_info = monitor.get_disk_info();
                let network_info = monitor.get_network_info();
                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

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

                    // Check for alerts
                    let new_alerts = monitor.check_alerts(&settings_clone, &data);
                    data.alerts.extend(new_alerts);

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

                    // Network history
                    data.network_download_history.push_back(DataPoint {
                        time: elapsed,
                        value: total_download_rate,
                    });
                    data.network_upload_history.push_back(DataPoint {
                        time: elapsed,
                        value: total_upload_rate,
                    });

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

                thread::sleep(Duration::from_millis(refresh_interval * 1000 - 500));
            }
        });

        Self {
            data,
            settings,
            selected_tab: Tab::Overview,
            show_settings: false,
            show_export: false,
            show_alerts: false,
            show_process_manager: false,
            show_cpu_cores: false,
            selected_process_pid: None,
            monitor_handle: None,
            always_on_top: false,
            process_search: String::new(),
            process_sort_column: ProcessSortColumn::Memory,
            process_sort_ascending: false,
            show_export_csv: false,
            updater: updater::Updater::new(),
            show_update_notification: false,
            update_check_time: None,
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
        egui::Color32::from_rgb(105, 240, 174)  // Mint green (#69f0ae)
    } else if percentage < 75.0 {
        egui::Color32::from_rgb(255, 171, 64)    // Amber (#ffab40)
    } else {
        egui::Color32::from_rgb(255, 82, 82)     // Saturated red (#ff5252)
    }
}

impl eframe::App for SystemMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request repaint for continuous updates
        ctx.request_repaint();

        // Handle always on top
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            if self.always_on_top {
                egui::viewport::WindowLevel::AlwaysOnTop
            } else {
                egui::viewport::WindowLevel::Normal
            }
        ));

        // Check for updates automatically (once every 24 hours)
        if self.update_check_time.is_none() || 
           self.update_check_time.unwrap().elapsed().as_secs() > 86400 {
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
                    ui.colored_label(egui::Color32::from_rgb(105, 240, 174), "🎉");
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

        // Handle process actions
        if let Some(pid) = self.selected_process_pid.take() {
            // Create a temporary system instance to kill the process
            let mut temp_sys = System::new();
            temp_sys.refresh_processes();
            
            if let Some(process) = temp_sys.process(Pid::from_u32(pid)) {
                let _ = process.kill();
            }
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
                            
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
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
                            
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
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
                        egui::ScrollArea::vertical()
                            .max_height(400.0)
                            .show(ui, |ui| {
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

        // Settings window
        let mut show_settings = self.show_settings;
        let mut save_settings = false;
        if show_settings {
            egui::Window::new("⚙️ Settings")
                .open(&mut show_settings)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Application Settings");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Refresh Interval (seconds):");
                        ui.add(egui::Slider::new(&mut self.settings.refresh_interval, 1..=10));
                    });

                    ui.separator();
                    ui.heading("Display Options");
                    ui.checkbox(&mut self.settings.show_graphs, "Show Performance Graphs");
                    ui.checkbox(&mut self.settings.show_gpu, "Show GPU Section");
                    ui.checkbox(&mut self.settings.show_processes, "Show Process List");

                    ui.separator();
                    ui.heading("Theme");
                    if ui.checkbox(&mut self.settings.theme_dark, "Dark Mode").changed() {
                        if self.settings.theme_dark {
                            let mut visuals = egui::Visuals::dark();
                            visuals.panel_fill = egui::Color32::from_rgb(13, 15, 20);
                            visuals.window_fill = egui::Color32::from_rgb(19, 22, 29);
                            visuals.extreme_bg_color = egui::Color32::from_rgb(8, 9, 12);
                            visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(0, 229, 255, 60);
                            visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 229, 255));
                            visuals.hyperlink_color = egui::Color32::from_rgb(0, 229, 255);
                            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(38, 42, 56));
                            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(26, 30, 40);
                            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 40, 54);
                            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 184, 212);
                            visuals.window_rounding = egui::Rounding::same(10.0);
                            visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
                            visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
                            visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
                            visuals.widgets.active.rounding = egui::Rounding::same(6.0);
                            visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(0, 229, 255, 30));
                            ctx.set_visuals(visuals);
                        } else {
                            ctx.set_visuals(egui::Visuals::light());
                        }
                    }

                    ui.separator();
                    ui.heading("Advanced Options");
                    ui.checkbox(&mut self.settings.show_per_core_cpu, "Show Per-Core CPU Usage");
                    ui.horizontal(|ui| {
                        ui.label("Process List Count:");
                        ui.add(egui::Slider::new(&mut self.settings.process_count, 5..=30));
                    });
                    ui.checkbox(&mut self.settings.auto_clear_alerts, "Auto-clear resolved alerts");

                    ui.separator();
                    ui.heading("Startup Options");
                    if ui.checkbox(&mut self.settings.auto_start, "Start with Windows").changed() {
                        let _ = self.settings.set_auto_start(self.settings.auto_start);
                    }
                    ui.checkbox(&mut self.settings.start_minimized, "Start minimized");
                    ui.label("⚠️ Auto-start requires administrator privileges");

                    ui.separator();
                    ui.heading("Notifications (Experimental)");
                    ui.checkbox(&mut self.settings.show_notifications, "Enable Notifications");
                    
                    if self.settings.show_notifications {
                        ui.horizontal(|ui| {
                            ui.label("CPU Threshold (%):");
                            ui.add(egui::Slider::new(&mut self.settings.notification_cpu_threshold, 50.0..=100.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Memory Threshold (%):");
                            ui.add(egui::Slider::new(&mut self.settings.notification_memory_threshold, 50.0..=100.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("GPU Temp Threshold (°C):");
                            ui.add(egui::Slider::new(&mut self.settings.notification_temp_threshold, 70..=100));
                        });
                    }

                    ui.separator();
                    if ui.button("💾 Save Settings").clicked() {
                        save_settings = true;
                    }
                });
        }
        self.show_settings = show_settings;
        if save_settings {
            if let Err(e) = self.settings.save() {
                eprintln!("Failed to save settings: {}", e);
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
                        // Will show shortcuts
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("🕒 {}", data.last_update));
                });
            });
        });

        // Side panel — branded nav with custom tabs
        egui::SidePanel::left("sidebar").min_width(190.0).max_width(210.0).show(ctx, |ui| {
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
                ui.painter().add(egui::Shape::convex_polygon(pts, egui::Color32::from_rgb(0, 229, 255), egui::Stroke::NONE));
                ui.label(egui::RichText::new("Sys").size(17.0).strong().color(egui::Color32::from_rgb(0, 229, 255)));
                ui.label(egui::RichText::new("Mon").size(17.0).strong().color(egui::Color32::from_rgb(220, 225, 240)));
                ui.label(egui::RichText::new("v1.0").size(9.5).color(egui::Color32::from_rgb(80, 90, 115)));
            });
            ui.add_space(6.0);
            // Thin accent line under brand
            {
                let r = ui.cursor();
                ui.painter().line_segment(
                    [egui::pos2(r.left() + 12.0, r.top()), egui::pos2(r.right() - 12.0, r.top())],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 35, 48)),
                );
            }
            ui.add_space(10.0);

            // ── Navigation label ──
            {
                let r = ui.cursor();
                ui.painter().text(
                    egui::pos2(r.left() + 14.0, r.top()),
                    egui::Align2::LEFT_TOP, "NAVIGATION",
                    egui::FontId::proportional(9.5),
                    egui::Color32::from_rgb(70, 78, 105),
                );
            }
            ui.add_space(18.0);

            // ── Tab items ──
            draw_nav_tab(ui, &mut self.selected_tab, Tab::Overview,    "Overview",    None);
            draw_nav_tab(ui, &mut self.selected_tab, Tab::Performance, "Performance", None);
            draw_nav_tab(ui, &mut self.selected_tab, Tab::Processes,   "Processes",   None);
            draw_nav_tab(ui, &mut self.selected_tab, Tab::CpuCores,   "CPU Cores",   None);
            draw_nav_tab(ui, &mut self.selected_tab, Tab::Storage,     "Storage",     None);
            draw_nav_tab(ui, &mut self.selected_tab, Tab::Network,     "Network",     None);
            draw_nav_tab(ui, &mut self.selected_tab, Tab::SystemInfo,  "System Info", None);

            ui.add_space(6.0);
            {
                let r = ui.cursor();
                ui.painter().line_segment(
                    [egui::pos2(r.left() + 12.0, r.top()), egui::pos2(r.right() - 12.0, r.top())],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 35, 48)),
                );
            }
            ui.add_space(6.0);

            let alert_count = data.alerts.len();
            draw_nav_tab(ui, &mut self.selected_tab, Tab::Alerts, "Alerts", if alert_count > 0 { Some(alert_count) } else { None });
            draw_nav_tab(ui, &mut self.selected_tab, Tab::About,  "About",  None);

            // ── Quick stats ──
            ui.add_space(14.0);
            {
                let r = ui.cursor();
                ui.painter().text(
                    egui::pos2(r.left() + 14.0, r.top()),
                    egui::Align2::LEFT_TOP, "QUICK STATS",
                    egui::FontId::proportional(9.5),
                    egui::Color32::from_rgb(70, 78, 105),
                );
            }
            ui.add_space(18.0);

            draw_mini_stat(ui, "CPU", data.cpu_usage);
            draw_mini_stat(ui, "RAM", data.memory_percentage);
            if let Some(ref gpu) = data.gpu_info {
                draw_mini_stat(ui, "GPU", gpu.utilization);
            }
        });

        // Process Manager window
        if self.show_process_manager {
            self.show_process_manager_window(ctx, &data);
        }

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Overview => self.show_overview_tab(ui, &data),
                Tab::Performance => self.show_performance_tab(ui, &data),
                Tab::Processes => self.show_processes_tab(ui, &data),
                Tab::CpuCores => self.show_cpu_cores_tab(ui, &data),
                Tab::Storage => self.show_storage_tab(ui, &data),
                Tab::Network => self.show_network_tab(ui, &data),
                Tab::SystemInfo => self.show_system_info_tab(ui, &data),
                Tab::Alerts => self.show_alerts_tab(ui, &data),
                Tab::About => self.show_about_tab(ui),
            }
        });
    }
}

// ─── Custom UI helpers ───────────────────────────────────────────────

/// Section header with cyan accent underline
fn paint_section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    let r = ui.label(egui::RichText::new(text)
        .size(18.0).strong()
        .color(egui::Color32::from_rgb(220, 225, 240)));
    let y = r.rect.bottom() + 2.0;
    ui.painter().line_segment(
        [egui::pos2(r.rect.left(), y), egui::pos2(r.rect.left() + 36.0, y)],
        egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 229, 255)),
    );
    ui.add_space(8.0);
}

/// Rounded pill progress bar with subtle track
fn paint_progress_bar(ui: &mut egui::Ui, fraction: f32, fill: egui::Color32, h: f32) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let rnd = h / 2.0;
    ui.painter().rect_filled(rect, rnd, egui::Color32::from_rgb(20, 24, 32));
    let frac = fraction.clamp(0.0, 1.0);
    if frac > 0.005 {
        let bar = egui::Rect::from_min_size(rect.min, egui::vec2(w * frac, h));
        ui.painter().rect_filled(bar, rnd, fill);
    }
}

/// Sidebar navigation tab with left accent bar + hover highlight
fn draw_nav_tab(ui: &mut egui::Ui, selected: &mut Tab, tab: Tab, label: &str, badge: Option<usize>) {
    let is_sel = *selected == tab;
    let size = egui::vec2(ui.available_width(), 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let p = ui.painter();
    if is_sel {
        p.rect_filled(rect, 6.0, egui::Color32::from_rgba_premultiplied(0, 229, 255, 15));
        p.rect_filled(
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height())),
            1.5, egui::Color32::from_rgb(0, 229, 255),
        );
    } else if resp.hovered() {
        p.rect_filled(rect, 6.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 6));
    }
    let mid = rect.center().y;
    let ic = if is_sel { egui::Color32::from_rgb(0, 229, 255) } else { egui::Color32::from_rgb(50, 58, 78) };
    let tc = if is_sel { egui::Color32::from_rgb(225, 230, 245) } else { egui::Color32::from_rgb(145, 155, 180) };
    // Painted dot indicator
    p.circle_filled(egui::pos2(rect.left() + 16.0, mid), 3.0, ic);
    p.text(egui::pos2(rect.left() + 30.0, mid), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(13.0), tc);
    if let Some(c) = badge {
        if c > 0 {
            let bx = rect.right() - 22.0;
            p.circle_filled(egui::pos2(bx, mid), 9.0, egui::Color32::from_rgb(255, 82, 82));
            p.text(egui::pos2(bx, mid), egui::Align2::CENTER_CENTER, &c.to_string(), egui::FontId::proportional(9.0), egui::Color32::WHITE);
        }
    }
    if resp.clicked() { *selected = tab; }
}

/// Compact stat row for sidebar: label, value %, mini bar
fn draw_mini_stat(ui: &mut egui::Ui, label: &str, value: f32) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 22.0), egui::Sense::hover());
    let p = ui.painter();
    let color = get_usage_color(value);
    p.text(egui::pos2(rect.left() + 12.0, rect.top() + 4.0), egui::Align2::LEFT_TOP, label, egui::FontId::proportional(11.0), egui::Color32::from_rgb(120, 130, 155));
    p.text(egui::pos2(rect.right() - 12.0, rect.top() + 4.0), egui::Align2::RIGHT_TOP, &format!("{:.0}%", value), egui::FontId::proportional(11.0), color);
    let bar_y = rect.bottom() - 3.0;
    let bar_w = w - 24.0;
    let track = egui::Rect::from_min_size(egui::pos2(rect.left() + 12.0, bar_y), egui::vec2(bar_w, 2.5));
    p.rect_filled(track, 1.0, egui::Color32::from_rgb(20, 24, 32));
    let fw = bar_w * (value / 100.0).clamp(0.0, 1.0);
    if fw > 0.5 {
        p.rect_filled(egui::Rect::from_min_size(track.min, egui::vec2(fw, 2.5)), 1.0, color);
    }
}

impl SystemMonitorApp {
    fn show_overview_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "System Overview");

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Metric cards row ──
            let card_bg = egui::Color32::from_rgb(25, 28, 38);
            let card_border = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 44, 60));
            let card_rnd = egui::Rounding::same(6.0);

            let full_avail = ui.available_width();
            let card_spacing = 8.0;
            let card_h = 100.0;
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
                } else { gpu.name.clone() };
                (format!("{:.1}%", gpu.utilization), sub, gpu.utilization / 100.0, c)
            } else {
                ("N/A".to_string(), "Not detected".to_string(), 0.0, egui::Color32::from_rgb(60, 65, 80))
            };

            let cards: [(egui::Color32, &str, &str, &str, f32, egui::Color32); 3] = [
                (egui::Color32::from_rgb(0, 229, 255), "CPU", &format!("{:.1}%", data.cpu_usage), &format!("{} cores", data.cpu_cores.len()), data.cpu_usage / 100.0, cpu_c),
                (egui::Color32::from_rgb(105, 240, 174), "MEMORY", &format!("{:.1}%", data.memory_percentage), &format!("{:.1} / {:.1} GB", bytes_to_gb(data.memory_used), bytes_to_gb(data.memory_total)), data.memory_percentage / 100.0, mem_c),
                (egui::Color32::from_rgb(255, 171, 64), "GPU", &gpu_val, &gpu_sub, gpu_frac, gpu_c),
            ];

            for (i, (accent, label, value, sub, frac, color)) in cards.iter().enumerate() {
                let x = row_rect.min.x + (card_w + card_spacing) * i as f32;
                let cr = egui::Rect::from_min_size(egui::pos2(x, row_rect.min.y), egui::vec2(card_w, card_h));
                p.rect_filled(cr, card_rnd, card_bg);
                p.rect_stroke(cr, card_rnd, card_border);
                let m = 10.0;
                p.circle_filled(egui::pos2(cr.left() + m + 4.0, cr.top() + m + 5.0), 3.0, *accent);
                p.text(egui::pos2(cr.left() + m + 14.0, cr.top() + m + 1.0), egui::Align2::LEFT_TOP, *label, egui::FontId::proportional(11.0), egui::Color32::from_rgb(120, 130, 160));
                // Value
                p.text(egui::pos2(cr.left() + m, cr.top() + m + 18.0), egui::Align2::LEFT_TOP, *value, egui::FontId::proportional(28.0), *color);
                // Sub
                p.text(egui::pos2(cr.left() + m, cr.top() + m + 50.0), egui::Align2::LEFT_TOP, *sub, egui::FontId::proportional(11.0), egui::Color32::from_rgb(100, 110, 140));
                // Progress bar
                let bar_y = cr.top() + m + 68.0;
                let bar_w = card_w - m * 2.0;
                let bar_rect = egui::Rect::from_min_size(egui::pos2(cr.left() + m, bar_y), egui::vec2(bar_w, 5.0));
                p.rect_filled(bar_rect, 2.5, egui::Color32::from_rgb(20, 24, 32));
                let f = frac.clamp(0.0, 1.0);
                if f > 0.005 { p.rect_filled(egui::Rect::from_min_size(bar_rect.min, egui::vec2(bar_w * f, 5.0)), 2.5, *color); }
            }

            ui.add_space(10.0);

            // ── Detail strip ──
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if let Some(ref gpu) = data.gpu_info {
                        if let Some(temp) = gpu.temperature {
                            let tc = if temp < 70 { egui::Color32::from_rgb(105, 240, 174) } else if temp < 85 { egui::Color32::from_rgb(255, 171, 64) } else { egui::Color32::from_rgb(255, 82, 82) };
                            ui.label(egui::RichText::new(format!("{}°C", temp)).strong().color(tc));
                            ui.separator();
                        }
                        ui.label(egui::RichText::new(&gpu.name).size(11.5).color(egui::Color32::from_rgb(130, 140, 165)));
                        ui.separator();
                    }
                    let d = data.system_info.uptime / 86400;
                    let h = (data.system_info.uptime % 86400) / 3600;
                    let m = (data.system_info.uptime % 3600) / 60;
                    ui.label(egui::RichText::new(format!("Uptime {}d {}h {}m", d, h, m)).size(11.5).color(egui::Color32::from_rgb(130, 140, 165)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(&data.last_update).size(11.0).color(egui::Color32::from_rgb(80, 90, 115)));
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
                        ui.label(egui::RichText::new("PROCESS").size(10.0).color(egui::Color32::from_rgb(80, 90, 115)));
                        ui.label(egui::RichText::new("MEMORY").size(10.0).color(egui::Color32::from_rgb(80, 90, 115)));
                        ui.label(egui::RichText::new("CPU").size(10.0).color(egui::Color32::from_rgb(80, 90, 115)));
                        ui.end_row();

                        for process in data.top_processes.iter().take(8) {
                            let mb = bytes_to_mb(process.memory);
                            let mc = if mb > 500.0 { egui::Color32::from_rgb(255, 82, 82) }
                                     else if mb > 200.0 { egui::Color32::from_rgb(255, 171, 64) }
                                     else { egui::Color32::from_rgb(105, 240, 174) };
                            let name = if process.name.len() > 32 {
                                format!("{}…", &process.name[..30])
                            } else { process.name.clone() };
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
                    ui.label(egui::RichText::new("CPU Usage History").size(15.0).strong().color(egui::Color32::from_rgb(0, 229, 255)));
                    let cpu_points: PlotPoints = data.cpu_history
                        .iter()
                        .map(|p| [p.time, p.value])
                        .collect();

                    let line = Line::new(cpu_points).color(egui::Color32::from_rgb(0, 229, 255));
                    
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
                    ui.label(egui::RichText::new("Memory Usage History").size(15.0).strong().color(egui::Color32::from_rgb(105, 240, 174)));
                    let mem_points: PlotPoints = data.memory_history
                        .iter()
                        .map(|p| [p.time, p.value])
                        .collect();

                    let line = Line::new(mem_points).color(egui::Color32::from_rgb(105, 240, 174));
                    
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
                        ui.label(egui::RichText::new("GPU Usage History").size(15.0).strong().color(egui::Color32::from_rgb(255, 171, 64)));
                        let gpu_points: PlotPoints = data.gpu_history
                            .iter()
                            .map(|p| [p.time, p.value])
                            .collect();

                        let line = Line::new(gpu_points).color(egui::Color32::from_rgb(255, 171, 64));
                        
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
        let filtered_processes: Vec<_> = if self.process_search.is_empty() {
            data.top_processes.clone()
        } else {
            data.top_processes.iter()
                .filter(|p| p.name.to_lowercase().contains(&self.process_search.to_lowercase()))
                .cloned()
                .collect()
        };

        ui.label(format!("Showing {} of {} processes", filtered_processes.len(), data.top_processes.len()));
        ui.add_space(5.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("full_process_grid")
                .striped(true)
                .spacing([10.0, 4.0])
                .min_col_width(80.0)
                .show(ui, |ui| {
                    // Header
                    ui.strong("PID");
                    ui.strong("Process Name");
                    ui.strong("Memory Usage");
                    ui.strong("CPU %");
                    ui.strong("Actions");
                    ui.end_row();

                    // Processes
                    for process in &filtered_processes {
                        let memory_mb = bytes_to_mb(process.memory);
                        let memory_color = if memory_mb > 500.0 {
                            egui::Color32::from_rgb(255, 82, 82)
                        } else if memory_mb > 200.0 {
                            egui::Color32::from_rgb(255, 171, 64)
                        } else {
                            egui::Color32::from_rgb(105, 240, 174)
                        };

                        ui.label(process.pid.to_string());
                        
                        let display_name = if process.name.len() > 40 {
                            format!("{}...", &process.name[..37])
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
                        ui.colored_label(egui::Color32::RED, 
                            format!("⚠️ Warning: Only {:.2} GB remaining!", bytes_to_gb(disk.available_space)));
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
                    ui.label(egui::RichText::new("Network Activity History").size(15.0).strong().color(egui::Color32::from_rgb(220, 225, 240)));
                    
                    // Download graph
                    ui.label(egui::RichText::new("▼ Download Rate (MB/s)").size(12.0).color(egui::Color32::from_rgb(105, 240, 174)));
                    let download_points: PlotPoints = data.network_download_history
                        .iter()
                        .map(|p| [p.time, p.value])
                        .collect();

                    let line = Line::new(download_points).color(egui::Color32::from_rgb(105, 240, 174));
                    
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
                    ui.label(egui::RichText::new("▲ Upload Rate (MB/s)").size(12.0).color(egui::Color32::from_rgb(0, 229, 255)));
                    let upload_points: PlotPoints = data.network_upload_history
                        .iter()
                        .map(|p| [p.time, p.value])
                        .collect();

                    let line = Line::new(upload_points).color(egui::Color32::from_rgb(0, 229, 255));
                    
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

    fn show_alerts_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
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
                ui.label(format!("  • CPU usage > {:.0}%", self.settings.notification_cpu_threshold));
                ui.label(format!("  • Memory usage > {:.0}%", self.settings.notification_memory_threshold));
                ui.label(format!("  • GPU temperature > {}°C", self.settings.notification_temp_threshold));
                ui.label("  • Disk usage > 90%");
                ui.add_space(5.0);
                if ui.button("⚙️ Configure Alert Thresholds").clicked() {
                    // Settings will be shown in main update
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
                    // Will be cleared in main update
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
                            ui.strong(format!("{:.0} MB / {:.0} MB", 
                                bytes_to_mb(used), bytes_to_mb(total)));
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
        });
    }

    fn show_cpu_cores_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "CPU Cores Monitoring");

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(format!("Total Cores: {} ({} logical processors)", 
                data.system_info.cpu_count, data.cpu_cores.len()));
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
            .default_width(700.0)
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
                                    egui::Color32::from_rgb(255, 82, 82)
                                } else if memory_mb > 200.0 {
                                    egui::Color32::from_rgb(255, 171, 64)
                                } else {
                                    egui::Color32::from_rgb(105, 240, 174)
                                };

                                ui.label(process.pid.to_string());
                                
                                let display_name = if process.name.len() > 25 {
                                    format!("{}...", &process.name[..22])
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
                                    ui.add_enabled_ui(false, |ui| {
                                        if ui.small_button("⏸️").on_hover_text("Suspend (Not implemented)").clicked() {
                                            // Suspend not implemented
                                        }
                                    });
                                });

                                ui.end_row();
                            }
                        });
                });

                ui.separator();
                ui.colored_label(egui::Color32::YELLOW, "⚠️ Warning: Killing processes may cause system instability!");
            });

        self.show_process_manager = show;
    }

    fn show_about_tab(&self, ui: &mut egui::Ui) {
        paint_section_header(ui, "About");

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            // Hero brand
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("◈").size(28.0).color(egui::Color32::from_rgb(0, 229, 255)));
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("System Monitor").size(22.0).strong().color(egui::Color32::from_rgb(220, 225, 240)));
                        ui.label(egui::RichText::new("v1.0.0 · Terminal Noir").size(12.0).color(egui::Color32::from_rgb(100, 110, 140)));
                    });
                });
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Professional system intelligence for Windows — built with Rust and egui.").size(13.0).color(egui::Color32::from_rgb(160, 170, 190)));
            });

            ui.add_space(10.0);

            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.label(egui::RichText::new("FEATURES").size(10.0).color(egui::Color32::from_rgb(0, 229, 255)));
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
                            ui.label(egui::RichText::new("›").color(egui::Color32::from_rgb(0, 229, 255)));
                            ui.label(egui::RichText::new(*item).size(12.5).color(egui::Color32::from_rgb(175, 185, 205)));
                        });
                    }
                });

                cols[1].group(|ui| {
                    ui.label(egui::RichText::new("TECHNICAL").size(10.0).color(egui::Color32::from_rgb(0, 229, 255)));
                    ui.add_space(6.0);
                    let specs = [
                        ("Framework", "egui + eframe"),
                        ("System", "sysinfo crate"),
                        ("GPU", "NVML (NVIDIA)"),
                        ("Refresh", "2 s interval"),
                        ("History", "60 data points"),
                        ("License", "MIT — open source"),
                    ];
                    for (k, v) in &specs {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(*k).size(11.5).color(egui::Color32::from_rgb(100, 110, 140)));
                            ui.label(egui::RichText::new(*v).size(12.0).color(egui::Color32::from_rgb(185, 195, 215)));
                        });
                    }
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("COLOR LEGEND").size(10.0).color(egui::Color32::from_rgb(0, 229, 255)));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(105, 240, 174), "●  Healthy < 50%");
                    ui.add_space(16.0);
                    ui.colored_label(egui::Color32::from_rgb(255, 171, 64), "●  Moderate 50-75%");
                    ui.add_space(16.0);
                    ui.colored_label(egui::Color32::from_rgb(255, 82, 82), "●  Critical > 75%");
                });
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("System Monitor v1.0.0"),
        ..Default::default()
    };

    eframe::run_native(
        "System Monitor",
        options,
        Box::new(|cc| Ok(Box::new(SystemMonitorApp::new(cc)))),
    )
}
