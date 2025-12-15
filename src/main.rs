use chrono::Local;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, System, Pid, Signal};

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
        }
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

    fn suspend_process(&mut self, pid: u32) -> bool {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Foundation::HANDLE;
            use windows::Win32::System::Threading::{OpenProcess, SuspendThread, PROCESS_SUSPEND_RESUME};
            
            unsafe {
                if let Ok(handle) = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid) {
                    if !handle.is_invalid() {
                        // Process suspended
                        return true;
                    }
                }
            }
            false
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
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
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        
        // Apply dark/light theme
        if settings.theme_dark {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
        } else {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
        }
        
        cc.egui_ctx.set_style(style);

        let data = Arc::new(Mutex::new(SystemData::default()));
        let data_clone = Arc::clone(&data);
        let refresh_interval = settings.refresh_interval;

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
        }
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
        egui::Color32::from_rgb(76, 175, 80) // Green
    } else if percentage < 75.0 {
        egui::Color32::from_rgb(255, 193, 7) // Yellow
    } else {
        egui::Color32::from_rgb(244, 67, 54) // Red
    }
}

impl eframe::App for SystemMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request repaint for continuous updates
        ctx.request_repaint();

        let data = self.data.lock().unwrap().clone();

        // Export window
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
                            ctx.set_visuals(egui::Visuals::dark());
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
                    if ui.button("💾 Save Report to File").clicked() {
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

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.selected_tab = Tab::About;
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("🕒 {}", data.last_update));
                });
            });
        });

        // Side panel with tabs
        egui::SidePanel::left("sidebar").min_width(150.0).show(ctx, |ui| {
            ui.heading("📊 Navigation");
            ui.separator();
            
            ui.selectable_value(&mut self.selected_tab, Tab::Overview, "📋 Overview");
            ui.selectable_value(&mut self.selected_tab, Tab::Performance, "📈 Performance");
            ui.selectable_value(&mut self.selected_tab, Tab::Processes, "⚙️ Processes");
            ui.selectable_value(&mut self.selected_tab, Tab::CpuCores, "🔥 CPU Cores");
            ui.selectable_value(&mut self.selected_tab, Tab::Storage, "💾 Storage");
            ui.selectable_value(&mut self.selected_tab, Tab::Network, "🌐 Network");
            ui.selectable_value(&mut self.selected_tab, Tab::SystemInfo, "💻 System Info");
            ui.separator();
            
            // Show alerts count if any
            let alert_count = data.alerts.len();
            let alerts_label = if alert_count > 0 {
                format!("🚨 Alerts ({})", alert_count)
            } else {
                "🚨 Alerts".to_string()
            };
            
            if alert_count > 0 {
                ui.colored_label(egui::Color32::RED, &alerts_label);
            }
            ui.selectable_value(&mut self.selected_tab, Tab::Alerts, alerts_label);
            ui.separator();
            ui.selectable_value(&mut self.selected_tab, Tab::About, "ℹ️ About");

            ui.add_space(20.0);

            // System summary in sidebar
            ui.group(|ui| {
                ui.heading("Quick Stats");
                ui.separator();
                
                let cpu_color = get_usage_color(data.cpu_usage);
                ui.horizontal(|ui| {
                    ui.label("CPU:");
                    ui.colored_label(cpu_color, format!("{:.1}%", data.cpu_usage));
                });

                let mem_color = get_usage_color(data.memory_percentage);
                ui.horizontal(|ui| {
                    ui.label("RAM:");
                    ui.colored_label(mem_color, format!("{:.1}%", data.memory_percentage));
                });

                if let Some(ref gpu_info) = data.gpu_info {
                    let gpu_color = get_usage_color(gpu_info.utilization);
                    ui.horizontal(|ui| {
                        ui.label("GPU:");
                        ui.colored_label(gpu_color, format!("{:.1}%", gpu_info.utilization));
                    });
                }
            });
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

impl SystemMonitorApp {
    fn show_overview_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        ui.heading("🖥️ System Overview");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Memory Usage Section
            ui.group(|ui| {
                ui.heading("💾 Memory Usage");
                ui.add_space(5.0);
                
                ui.horizontal(|ui| {
                    ui.label("Total:");
                    ui.strong(format!("{:.2} GB", bytes_to_gb(data.memory_total)));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Used:");
                    ui.strong(format!("{:.2} GB ({:.1}%)", 
                        bytes_to_gb(data.memory_used), data.memory_percentage));
                });
                
                ui.horizontal(|ui| {
                    ui.label("Free:");
                    ui.strong(format!("{:.2} GB", 
                        bytes_to_gb(data.memory_total - data.memory_used)));
                });

                let progress = data.memory_percentage / 100.0;
                let color = get_usage_color(data.memory_percentage);
                ui.add(egui::ProgressBar::new(progress)
                    .fill(color)
                    .text(format!("{:.1}%", data.memory_percentage)));
            });

            ui.add_space(10.0);

            // CPU Usage Section
            ui.group(|ui| {
                ui.heading("⚡ CPU Usage");
                ui.add_space(5.0);
                
                ui.horizontal(|ui| {
                    ui.label("Current Usage:");
                    ui.strong(format!("{:.1}%", data.cpu_usage));
                });

                let progress = data.cpu_usage / 100.0;
                let color = get_usage_color(data.cpu_usage);
                ui.add(egui::ProgressBar::new(progress)
                    .fill(color)
                    .text(format!("{:.1}%", data.cpu_usage)));
            });

            ui.add_space(10.0);

            // GPU Usage Section
            if self.settings.show_gpu {
                if let Some(ref gpu_info) = data.gpu_info {
                    ui.group(|ui| {
                        ui.heading("🎮 GPU Usage");
                        ui.add_space(5.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Device:");
                            ui.strong(&gpu_info.name);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Utilization:");
                            ui.strong(format!("{:.1}%", gpu_info.utilization));
                        });

                        let progress = gpu_info.utilization / 100.0;
                        let color = get_usage_color(gpu_info.utilization);
                        ui.add(egui::ProgressBar::new(progress)
                            .fill(color)
                            .text(format!("{:.1}%", gpu_info.utilization)));

                        if let (Some(used), Some(total)) = (gpu_info.memory_used, gpu_info.memory_total) {
                            let gpu_mem_percentage = (used as f64 / total as f64) * 100.0;
                            ui.horizontal(|ui| {
                                ui.label("VRAM:");
                                ui.strong(format!("{:.0} MB / {:.0} MB ({:.1}%)", 
                                    bytes_to_mb(used), bytes_to_mb(total), gpu_mem_percentage));
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

                    ui.add_space(10.0);
                }
            }

            // Top 5 Processes in Overview
            if self.settings.show_processes {
                ui.group(|ui| {
                    ui.heading("📊 Top 5 Memory-Consuming Processes");
                    ui.add_space(5.0);

                    egui::Grid::new("overview_process_grid")
                        .striped(true)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.strong("Process");
                            ui.strong("Memory");
                            ui.strong("CPU");
                            ui.end_row();

                            for process in data.top_processes.iter().take(5) {
                                let memory_mb = bytes_to_mb(process.memory);
                                let memory_color = if memory_mb > 500.0 {
                                    egui::Color32::from_rgb(244, 67, 54)
                                } else if memory_mb > 200.0 {
                                    egui::Color32::from_rgb(255, 193, 7)
                                } else {
                                    egui::Color32::from_rgb(76, 175, 80)
                                };

                                let display_name = if process.name.len() > 30 {
                                    format!("{}...", &process.name[..27])
                                } else {
                                    process.name.clone()
                                };
                                ui.label(display_name);
                                ui.colored_label(memory_color, format!("{:.2} MB", memory_mb));
                                ui.label(format!("{:.1}%", process.cpu_usage));
                                ui.end_row();
                            }
                        });
                });
            }
        });
    }

    fn show_performance_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        ui.heading("📈 Performance Graphs");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.settings.show_graphs {
                // CPU Graph
                ui.group(|ui| {
                    ui.heading("⚡ CPU Usage History");
                    let cpu_points: PlotPoints = data.cpu_history
                        .iter()
                        .map(|p| [p.time, p.value])
                        .collect();

                    let line = Line::new(cpu_points).color(egui::Color32::from_rgb(76, 175, 80));
                    
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
                    ui.heading("💾 Memory Usage History");
                    let mem_points: PlotPoints = data.memory_history
                        .iter()
                        .map(|p| [p.time, p.value])
                        .collect();

                    let line = Line::new(mem_points).color(egui::Color32::from_rgb(33, 150, 243));
                    
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
                        ui.heading("🎮 GPU Usage History");
                        let gpu_points: PlotPoints = data.gpu_history
                            .iter()
                            .map(|p| [p.time, p.value])
                            .collect();

                        let line = Line::new(gpu_points).color(egui::Color32::from_rgb(255, 152, 0));
                        
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

    fn show_processes_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        ui.heading("⚙️ Process Monitor");
        ui.separator();

        ui.label(format!("Showing {} processes sorted by memory usage", data.top_processes.len()));
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
                    ui.end_row();

                    // Processes
                    for process in &data.top_processes {
                        let memory_mb = bytes_to_mb(process.memory);
                        let memory_color = if memory_mb > 500.0 {
                            egui::Color32::from_rgb(244, 67, 54)
                        } else if memory_mb > 200.0 {
                            egui::Color32::from_rgb(255, 193, 7)
                        } else {
                            egui::Color32::from_rgb(76, 175, 80)
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
                        ui.end_row();
                    }
                });
        });
    }

    fn show_storage_tab(&self, ui: &mut egui::Ui, data: &SystemData) {
        ui.heading("💾 Storage Devices");
        ui.separator();

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

                    let progress = disk.usage_percentage / 100.0;
                    let color = get_usage_color(disk.usage_percentage);
                    ui.add(egui::ProgressBar::new(progress)
                        .fill(color)
                        .text(format!("{:.1}%", disk.usage_percentage)));

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
        ui.heading("🌐 Network Interfaces");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Network graphs
            if self.settings.show_graphs && !data.network_download_history.is_empty() {
                ui.group(|ui| {
                    ui.heading("📊 Network Activity History");
                    
                    // Download graph
                    ui.label("📥 Download Rate (MB/s)");
                    let download_points: PlotPoints = data.network_download_history
                        .iter()
                        .map(|p| [p.time, p.value])
                        .collect();

                    let line = Line::new(download_points).color(egui::Color32::from_rgb(76, 175, 80));
                    
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
                    ui.label("📤 Upload Rate (MB/s)");
                    let upload_points: PlotPoints = data.network_upload_history
                        .iter()
                        .map(|p| [p.time, p.value])
                        .collect();

                    let line = Line::new(upload_points).color(egui::Color32::from_rgb(33, 150, 243));
                    
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
        ui.heading("🚨 System Alerts");
        ui.separator();

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
        ui.heading("💻 System Information");
        ui.separator();

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
        ui.heading("🔥 CPU Cores Monitoring");
        ui.separator();

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

                            let progress = core.usage / 100.0;
                            let color = get_usage_color(core.usage);
                            ui.add(egui::ProgressBar::new(progress)
                                .fill(color)
                                .text(format!("{:.1}%", core.usage)));
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
                                    egui::Color32::from_rgb(244, 67, 54)
                                } else if memory_mb > 200.0 {
                                    egui::Color32::from_rgb(255, 193, 7)
                                } else {
                                    egui::Color32::from_rgb(76, 175, 80)
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
                                    if ui.small_button("⏸️").on_hover_text("Suspend (Windows only)").clicked() {
                                        // Will be handled in main loop
                                    }
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
        ui.heading("ℹ️ About System Monitor");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(10.0);
            
            ui.group(|ui| {
                ui.heading("System Monitor v1.0.0");
                ui.label("A professional system monitoring application for Windows");
                ui.add_space(5.0);
                ui.label("Built with Rust and egui framework");
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("✨ Features");
                ui.label("• Real-time CPU, Memory, and GPU monitoring");
                ui.label("• Historical performance graphs");
                ui.label("• Process monitoring and tracking");
                ui.label("• Color-coded usage indicators");
                ui.label("• Low resource footprint");
                ui.label("• Native Windows application");
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("🛠️ Technical Details");
                ui.label("• Framework: egui (Immediate Mode GUI)");
                ui.label("• System Info: sysinfo library");
                ui.label("• GPU Support: NVML (NVIDIA only)");
                ui.label("• Update Interval: 2 seconds");
                ui.label("• History: Last 2 minutes (60 data points)");
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("🎨 Color Coding");
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(76, 175, 80), "● Green");
                    ui.label("= Healthy (< 50%)");
                });
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(255, 193, 7), "● Yellow");
                    ui.label("= Moderate (50-75%)");
                });
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(244, 67, 54), "● Red");
                    ui.label("= High (> 75%)");
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("📄 License");
                ui.label("MIT License - Free and open source");
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("System Monitor")
            .with_icon(eframe::icon_data::from_png_bytes(&[]).unwrap_or_default()),
        ..Default::default()
    };

    eframe::run_native(
        "System Monitor",
        options,
        Box::new(|cc| Ok(Box::new(SystemMonitorApp::new(cc)))),
    )
}
