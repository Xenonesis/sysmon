use crate::app::models::*;
use crate::{monitoring, power, processes, services};
use chrono::Local;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, Pid, System};
use tracing::{info, warn};

#[cfg(target_os = "windows")]
use nvml_wrapper::Nvml;
#[cfg(target_os = "windows")]
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    TrayIconBuilder,
};

impl SystemMonitor {
    pub fn new() -> Self {
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
            let com = crate::providers::init_com().ok().map(std::rc::Rc::new);
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
        let percentage = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        (total, used, percentage as f32)
    }

    fn get_cpu_usage(&mut self) -> f32 {
        self.sys.global_cpu_info().cpu_usage()
    }

    fn get_top_processes(&self, count: usize) -> Vec<crate::processes::ProcessInfo> {
        #[cfg(target_os = "windows")]
        let vram_map = if let Some(nvml) = &self.nvml {
            crate::processes::query_process_vram_from_nvml(nvml)
        } else {
            std::collections::HashMap::new()
        };
        #[cfg(not(target_os = "windows"))]
        let vram_map = std::collections::HashMap::new();

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

                crate::processes::ProcessInfo {
                    pid: pid.as_u32(),
                    start_time: process.start_time(),
                    name: name_str,
                    parent_pid: process.parent().map(|p| p.as_u32()),
                    cpu_usage: process.cpu_usage() / cpu_count,
                    memory: process.memory(),
                    vram_bytes: vram_map.get(&pid.as_u32()).copied(),
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

    fn get_timeline_processes(&self, per_metric: usize) -> Vec<crate::processes::ProcessInfo> {
        let processes = self.get_top_processes(usize::MAX);
        let mut selected = std::collections::BTreeMap::new();
        let mut by_cpu = processes.iter().collect::<Vec<_>>();
        let mut by_memory = by_cpu.clone();
        let mut by_disk = by_cpu.clone();
        by_cpu.sort_by(|a, b| b.cpu_usage.total_cmp(&a.cpu_usage));
        by_memory.sort_by_key(|process| std::cmp::Reverse(process.memory));
        by_disk.sort_by_key(|process| {
            std::cmp::Reverse(process.disk_read_bytes.saturating_add(process.disk_written_bytes))
        });
        for process in by_cpu
            .into_iter()
            .take(per_metric)
            .chain(by_memory.into_iter().take(per_metric))
            .chain(by_disk.into_iter().take(per_metric))
        {
            selected.insert((process.pid, process.start_time), process.clone());
        }
        selected.into_values().collect()
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

    pub(crate) fn kill_process(&mut self, pid: u32) -> bool {
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
    pub(crate) fn suspend_process(&mut self, pid: u32) -> bool {
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
    pub(crate) fn resume_process(&mut self, pid: u32) -> bool {
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
    pub(crate) fn suspend_process(&mut self, _pid: u32) -> bool {
        false
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn resume_process(&mut self, _pid: u32) -> bool {
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
    pub fn clean_ram(&mut self, exclusions: &[String], smart_only: bool) -> u64 {
        use windows::Win32::Foundation::{CloseHandle, E_ACCESSDENIED};
        use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA};

        info!(
            excluded = exclusions.len(),
            "RAM clean operation initiated (native API)"
        );
        let mem_before = self.sys.used_memory();
        let mut trimmed = 0u32;
        let mut access_denied = 0u32;
        let mut errored = 0u32;

        let mut foreground_pid = 0;
        if smart_only {
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
                let hwnd = GetForegroundWindow();
                if hwnd.0 != 0 {
                    GetWindowThreadProcessId(hwnd, Some(&mut foreground_pid));
                }
            }
        }

        unsafe {
            for (pid, process) in self.sys.processes() {
                if is_excluded(process.name(), exclusions) {
                    continue;
                }
                let pid_u32 = pid.as_u32();
                if smart_only && pid_u32 == foreground_pid {
                    continue;
                }
                match OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, false, pid_u32) {
                    Ok(h) if !h.is_invalid() => {
                        match EmptyWorkingSet(h) {
                            Ok(()) => trimmed += 1,
                            Err(e) if e.code() == E_ACCESSDENIED => access_denied += 1,
                            Err(_) => errored += 1,
                        }
                        let _ = CloseHandle(h);
                    }
                    Err(e) if e.code() == E_ACCESSDENIED => access_denied += 1,
                    _ => errored += 1,
                }
            }
        }

        self.sys.refresh_memory();
        let mem_after = self.sys.used_memory();
        let freed = mem_before.saturating_sub(mem_after);
        info!(
            freed_mb = freed / 1024 / 1024,
            trimmed = trimmed,
            access_denied = access_denied,
            errored = errored,
            "RAM clean complete"
        );
        freed
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn clean_ram(&mut self, _exclusions: &[String], _smart_only: bool) -> u64 {
        0
    }

    // Startup item collection and actions are now in startup.rs module

    #[cfg(target_os = "windows")]
    pub(crate) fn set_process_priority(pid: u32, priority: &str) -> bool {
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
    pub(crate) fn set_process_priority(_pid: u32, _priority: &str) -> bool {
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
        use wmi::{Variant, WMIConnection};
        let com = match crate::providers::init_com() {
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

    fn check_alerts(settings: &AppSettings, data: &SystemData) -> Vec<AlertInfo> {
        let mut alerts = Vec::new();
        let timestamp = Local::now().format("%H:%M:%S").to_string();

        // CPU alert
        if data.cpu_usage > settings.notification_cpu_threshold {
            alerts.push(AlertInfo {
                timestamp: timestamp.clone(),
                alert_type: AlertType::CpuHigh,
                source: AlertSource::Cpu,
                message: format!("CPU usage is high: {:.1}%", data.cpu_usage),
                value: data.cpu_usage,
            });
        }

        // Memory alert
        if data.memory_percentage > settings.notification_memory_threshold {
            alerts.push(AlertInfo {
                timestamp: timestamp.clone(),
                alert_type: AlertType::MemoryHigh,
                source: AlertSource::Memory,
                message: format!("Memory usage is high: {:.1}%", data.memory_percentage),
                value: data.memory_percentage,
            });
        }

        // GPU temperature alert
        for (index, gpu) in data.gpu_info.iter().enumerate() {
            if let Some(temp) = gpu.temperature {
                if temp > settings.notification_temp_threshold {
                    alerts.push(AlertInfo {
                        timestamp: timestamp.clone(),
                        alert_type: AlertType::GpuTempHigh,
                        source: AlertSource::Gpu {
                            index,
                            name: gpu.name.clone(),
                        },
                        message: format!("GPU temperature is high: {}°C ({})", temp, gpu.name),
                        value: temp as f32,
                    });
                }
            }
        }

        // Disk space alerts
        for disk in &data.disk_info {
            if disk.usage_percentage > settings.notification_disk_threshold {
                alerts.push(AlertInfo {
                    timestamp: timestamp.clone(),
                    alert_type: AlertType::DiskSpaceLow,
                    source: AlertSource::Disk {
                        mount_point: disk.mount_point.clone(),
                        name: disk.name.clone(),
                    },
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
                source: AlertSource::Startup,
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

pub(crate) struct SystemMonitorApp {
    pub(crate) data: Arc<RwLock<SystemData>>,
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
    pub(crate) selected_process_pid: Option<u32>,
    pub(crate) details_pid: Option<u32>,
    pub(crate) kill_tree_pid: Option<u32>,
    pub(crate) service_page: crate::app::page_state::ServicePageState,
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
    pub(crate) update_result_share: Arc<Mutex<Option<Result<(), String>>>>,
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
    pub(crate) suspend_process_pid: Option<u32>,
    pub(crate) resume_process_pid: Option<u32>,
    pub(crate) suspended_pids: std::collections::HashSet<u32>,
    pub(crate) priority_change: Option<(u32, String)>,
    pub(crate) process_tree_view: bool,
    pub(crate) affinity_change: Option<(u32, usize)>,
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
    Timeline,
    About,
}

impl SystemMonitorApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Install image loaders for showing the logo
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Load settings
        let settings = AppSettings::load();
        let timeline =
            crate::timeline::TimelineHandle::start(settings.timeline_enabled, settings.timeline_retention_days);

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

            // Load Segoe UI Symbol and Emoji fonts for crisp system symbols and icons
            let sym_paths = [
                ("segoe_symbol", "C:\\Windows\\Fonts\\seguisym.ttf"),
                ("segoe_symbol", "C:\\Windows\\Fonts\\SeguiSym.ttf"),
                ("segoe_emoji", "C:\\Windows\\Fonts\\seguiemj.ttf"),
                ("segoe_emoji", "C:\\Windows\\Fonts\\SeguiEmj.ttf"),
            ];
            for (key, path) in &sym_paths {
                if let Ok(font_bytes) = std::fs::read(path) {
                    fonts
                        .font_data
                        .insert(key.to_string(), egui::FontData::from_owned(font_bytes));
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .push(key.to_string());
                    fonts
                        .families
                        .entry(egui::FontFamily::Monospace)
                        .or_default()
                        .push(key.to_string());
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

        // Apply theme — custom "Terminal Noir" dark or clean light
        let is_dark = crate::ui::theme::ThemePalette::is_dark_mode(settings.theme);
        if is_dark {
            let mut visuals = egui::Visuals::dark();
            // Deep charcoal backgrounds
            visuals.panel_fill = crate::ui::theme::ThemePalette::BG_DEEP;
            visuals.window_fill = crate::ui::theme::ThemePalette::BG_SURFACE;
            visuals.extreme_bg_color = crate::ui::theme::ThemePalette::BG_DEEPEST;

            // Accent for selections and interactions
            visuals.selection.bg_fill = crate::ui::theme::ThemePalette::ACCENT_PRIMARY;
            visuals.selection.stroke = egui::Stroke::NONE;
            visuals.hyperlink_color = crate::ui::theme::ThemePalette::ACCENT_PRIMARY;

            // Subtle borders & widgets
            visuals.widgets.noninteractive.bg_fill = crate::ui::theme::ThemePalette::BG_CARD;
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::BORDER);
            visuals.widgets.noninteractive.fg_stroke =
                egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::TEXT_PRIMARY);

            // Inactive
            visuals.widgets.inactive.bg_fill = crate::ui::theme::ThemePalette::WIDGET_INACTIVE;
            visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::TEXT_SECONDARY);

            // Hovered
            visuals.widgets.hovered.bg_fill = crate::ui::theme::ThemePalette::WIDGET_HOVERED;
            visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::BORDER_LIGHT);
            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::TEXT_SELECTED);

            // Active
            visuals.widgets.active.bg_fill = crate::ui::theme::ThemePalette::ACCENT_ACTIVE;
            visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::TEXT_SELECTED);

            // Rounding (Terminal Noir Minimal 4px)
            visuals.window_rounding = egui::Rounding::same(4.0);
            visuals.menu_rounding = egui::Rounding::same(4.0);
            visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
            visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
            visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
            visuals.widgets.active.rounding = egui::Rounding::same(4.0);

            // Window chrome and depth
            visuals.window_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::BORDER);
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
            visuals.selection.bg_fill = crate::ui::theme::ThemePalette::ACCENT_PRIMARY;
            visuals.selection.stroke = egui::Stroke::NONE;

            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(250, 250, 250);
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 220, 225));
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 45));

            visuals.window_rounding = egui::Rounding::same(8.0);
            visuals.menu_rounding = egui::Rounding::same(8.0);

            cc.egui_ctx.set_visuals(visuals);
        }

        cc.egui_ctx.set_style(style);

        let data = Arc::new(RwLock::new(SystemData::default()));
        let data_clone = Arc::clone(&data);
        let shared_settings = Arc::new(Mutex::new(settings.clone()));
        let shared_settings_clone = Arc::clone(&shared_settings);
        let mut app_channels = crate::app::AppChannels::new();
        let monitoring_receiver = app_channels
            .monitoring_receiver
            .take()
            .expect("monitoring receiver missing");
        let action_receiver = app_channels.action_receiver.take().expect("action receiver missing");
        let action_events = app_channels.event_sender.clone();
        let monitoring_events = app_channels.event_sender.clone();

        thread::Builder::new()
            .name("actions".to_string())
            .spawn(move || crate::app::run_action_worker(action_receiver, action_events))
            .expect("failed to spawn action worker");

        let (mut telemetry_hub, mut telemetry_reader, telemetry_commands) = crate::telemetry::TelemetryHub::new();
        telemetry_hub.add_provider(Box::new(crate::providers::sysinfo_provider::SysinfoProvider::new()));
        telemetry_hub.add_provider(Box::new(crate::providers::nvml_provider::NvmlProvider::new()));
        telemetry_hub.add_provider(Box::new(crate::providers::wmi_provider::WmiProvider::new()));
        telemetry_hub.add_provider(Box::new(
            crate::providers::windows_gpu_provider::WindowsGpuProvider::new(),
        ));
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
                let mut disk_smart_check_counter: u32 = 0;
                let mut sockets_check_counter: u32 = 0;
                let mut power_plans_check_counter: u32 = 0;
                let mut last_alert_time: std::collections::HashMap<AlertType, Instant> =
                    std::collections::HashMap::new();
                let mut last_hidden_tick = Instant::now();
                let mut last_selected_tab = data_clone.read().selected_tab;
                let mut latest_telemetry = crate::telemetry::TelemetrySnapshot::default();

                loop {
                    let mut force_refresh = false;
                    while let Ok(command) = monitoring_receiver.try_recv() {
                        match command {
                            crate::app::commands::MonitoringCommand::SetSettings(new_settings) => {
                                *shared_settings_clone.lock() = *new_settings;
                            }
                            crate::app::commands::MonitoringCommand::SetPaused(paused) => {
                                data_clone.write().monitoring_paused = paused
                            }
                            crate::app::commands::MonitoringCommand::SetHidden(hidden) => {
                                data_clone.write().is_hidden = hidden;
                                let _ = telemetry_commands_for_monitor
                                    .try_send(crate::telemetry::HubCommand::SetBackgroundMode(hidden));
                            }
                            crate::app::commands::MonitoringCommand::RefreshNow => {
                                force_refresh = true;
                                let _ =
                                    telemetry_commands_for_monitor.try_send(crate::telemetry::HubCommand::ForceRefresh);
                            }
                            crate::app::commands::MonitoringCommand::Shutdown => {
                                let _ = telemetry_commands_for_monitor.try_send(crate::telemetry::HubCommand::Shutdown);
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
                        let data = data_clone.read();
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
                    let need_gpu_info = need_gpu_wmi
                        || settings_snapshot.show_notifications
                        || settings_snapshot.show_graphs
                        || settings_snapshot.timeline_enabled;
                    // Fetch processes for both Processes tab (all) and Overview tab (top N summary)
                    let need_processes =
                        !is_hidden && (selected_tab == Tab::Processes || selected_tab == Tab::Overview);
                    let need_disks = (!is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::Storage))
                        || settings_snapshot.show_notifications
                        || settings_snapshot.show_graphs
                        || settings_snapshot.timeline_enabled;
                    let need_network = settings_snapshot.timeline_enabled
                        || (!is_hidden
                            && (selected_tab == Tab::Overview
                                || selected_tab == Tab::Network
                                || selected_tab == Tab::Performance));

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
                    let timeline_processes = if settings_snapshot.timeline_enabled {
                        monitor.get_timeline_processes(10)
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

                    let (disk_read_rate, disk_write_rate) = if !is_hidden || settings_snapshot.timeline_enabled {
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
                            let mut data = data_clone.write();
                            data.battery_info = Some(bi);
                        }
                    }
                    battery_check_counter = battery_check_counter.wrapping_add(1);

                    // Poll services every 60 ticks (~30s) — WMI queries are expensive
                    if !is_hidden && selected_tab == Tab::Services {
                        let services_list = if last_selected_tab != Tab::Services
                            || data_clone.read().services.is_empty()
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
                            let mut data = data_clone.write();
                            data.services = services_list;
                        }
                    }
                    service_check_counter = service_check_counter.wrapping_add(1);

                    // Poll physical disk SMART health in background (~every 30s or when entering tab)
                    if !is_hidden
                        && (selected_tab == Tab::Storage || selected_tab == Tab::Overview)
                        && (disk_smart_check_counter % 60 == 0 || data_clone.read().physical_disks.is_empty())
                    {
                        let drives = crate::storage::get_physical_disks();
                        if !drives.is_empty() {
                            let mut data = data_clone.write();
                            data.physical_disks = drives;
                        }
                    }
                    disk_smart_check_counter = disk_smart_check_counter.wrapping_add(1);

                    // Poll active socket connections in background (~every 2s on Network tab)
                    if !is_hidden
                        && selected_tab == Tab::Network
                        && (sockets_check_counter % 4 == 0 || data_clone.read().socket_connections.is_empty())
                    {
                        let process_map: std::collections::HashMap<u32, String> = monitor
                            .sys
                            .processes()
                            .iter()
                            .map(|(pid, p)| (pid.as_u32(), p.name().to_string()))
                            .collect();
                        let conns = crate::network::get_active_connections(&process_map);
                        let mut data = data_clone.write();
                        data.socket_connections = conns;
                    }
                    sockets_check_counter = sockets_check_counter.wrapping_add(1);

                    // Poll power plans & battery health in background (~every 10s on SystemInfo tab or startup)
                    if !is_hidden
                        && (selected_tab == Tab::SystemInfo || selected_tab == Tab::Overview)
                        && (power_plans_check_counter % 20 == 0 || data_clone.read().power_plans.is_empty())
                    {
                        let plans = crate::power::get_power_plans();
                        let bat_health = crate::power::get_battery_health();
                        let mut data = data_clone.write();
                        data.power_plans = plans;
                        data.battery_health = bat_health;
                    }
                    power_plans_check_counter = power_plans_check_counter.wrapping_add(1);

                    last_selected_tab = selected_tab;
                    // Calculate total network rates
                    let total_download_rate: f64 = network_info.iter().map(|n| n.received_rate).sum();
                    let total_upload_rate: f64 = network_info.iter().map(|n| n.transmitted_rate).sum();

                    {
                        let mut data = data_clone.write();
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
                        if settings_snapshot.timeline_enabled {
                            data.timeline_processes = timeline_processes;
                        } else {
                            data.timeline_processes.clear();
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
                        if !is_hidden || settings_snapshot.timeline_enabled {
                            data.disk_read_rate = disk_read_rate;
                            data.disk_write_rate = disk_write_rate;
                        }
                        data.network_sample_count += 1;
                        data.telemetry_history_stats = latest_telemetry.history_stats.clone();
                        data.provider_status = latest_telemetry.provider_status.clone();

                        // Check for alerts
                        let mut new_alerts = SystemMonitor::check_alerts(&settings_snapshot, &data);
                        let active_keys: std::collections::HashSet<String> =
                            data.alerts.iter().map(AlertInfo::key).collect();
                        new_alerts.retain(|alert| !active_keys.contains(&alert.key()));

                        if !new_alerts.is_empty()
                            && settings_snapshot.enable_alert_sound
                            && settings_snapshot.enable_sounds
                        {
                            play_alert_sound();
                        }

                        // Send desktop notifications for new alerts with a 5-minute cooldown
                        if settings_snapshot.show_notifications {
                            for alert in &new_alerts {
                                let now = Instant::now();
                                let should_notify = last_alert_time
                                    .get(&alert.alert_type)
                                    .is_none_or(|&last| now.saturating_duration_since(last).as_secs() > 300);

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
                            let disk_info = data.disk_info.clone();
                            let high_impact_count = data.high_impact_startup_count;
                            data.alerts.retain(|alert| match alert.alert_type {
                                AlertType::CpuHigh => cpu_usage > settings_snapshot.notification_cpu_threshold,
                                AlertType::MemoryHigh => {
                                    mem_percentage > settings_snapshot.notification_memory_threshold
                                }
                                AlertType::GpuTempHigh => match &alert.source {
                                    AlertSource::Gpu { index, name } => temp_gpu_info.get(*index).is_some_and(|gpu| {
                                        gpu.name == *name
                                            && gpu.temperature.is_some_and(|temperature| {
                                                temperature > settings_snapshot.notification_temp_threshold
                                            })
                                    }),
                                    _ => false,
                                },
                                AlertType::DiskSpaceLow => match &alert.source {
                                    AlertSource::Disk { mount_point, .. } => disk_info.iter().any(|disk| {
                                        disk.mount_point == *mount_point
                                            && disk.usage_percentage > settings_snapshot.notification_disk_threshold
                                    }),
                                    _ => false,
                                },
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

                    let snapshot = crate::snapshot_from_data(&data_clone.read());
                    let _ = monitoring_events.send(crate::app::events::AppEvent::Snapshot(Box::new(snapshot)));

                    // Process details for the selected row (recompute only when selection changed)
                    let selected_pid = {
                        let d = data_clone.read();
                        d.selected_process_pid
                    };
                    if let Some(pid) = selected_pid {
                        let cached = {
                            let d = data_clone.read();
                            d.selected_process_details.as_ref().map(|(p, _)| *p)
                        };
                        if cached != Some(pid) {
                            if let Some(details) = processes::lookup_details(&monitor.sys, pid) {
                                let mut d = data_clone.write();
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

        let mut _hotkey_manager = None;
        let mut clean_ram_hotkey = None;
        #[cfg(target_os = "windows")]
        {
            if let Ok(manager) = global_hotkey::GlobalHotKeyManager::new() {
                let hotkey = global_hotkey::hotkey::HotKey::new(
                    Some(global_hotkey::hotkey::Modifiers::CONTROL | global_hotkey::hotkey::Modifiers::ALT),
                    global_hotkey::hotkey::Code::KeyC,
                );
                if manager.register(hotkey).is_ok() {
                    _hotkey_manager = Some(manager);
                    clean_ram_hotkey = Some(hotkey);
                }
            }
        }

        #[cfg(target_os = "windows")]
        if let Some(icon) = crate::load_tray_icon() {
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
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(crate::startup::get_startup_data));
                match result {
                    Ok((items, diag)) => {
                        *startup_share_clone.lock() = Some(items);
                        *boot_share_clone.lock() = diag;
                    }
                    Err(_) => {
                        *startup_share_clone.lock() = Some(Vec::new());
                    }
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
            action_history: crate::persistence::action_log::load_recent(100)
                .into_iter()
                .map(|record| {
                    let undo = record
                        .quarantine_id
                        .as_ref()
                        .filter(|quarantine_id| crate::startup::quarantine_exists(quarantine_id))
                        .map(|quarantine_id| crate::app::commands::ActionCommand::RestoreStartup {
                            item_name: record.action.clone(),
                            quarantine_id: quarantine_id.clone(),
                        });
                    crate::app::actions::ActionHistoryEntry { record, undo }
                })
                .collect(),
            show_action_history: false,
            session_recorder: crate::persistence::session::SessionRecorder::default(),
            session_status: None,
            timeline,
            timeline_ui: crate::timeline::TimelineUiState::default(),
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
            process_sort_column: crate::processes::ProcessSortColumn::Memory,
            process_sort_ascending: false,
            show_export_csv: false,
            updater: crate::updater::Updater::new(),
            update_info_share: Arc::new(Mutex::new(None)),
            show_update_notification: true,
            update_check_time: None,
            update_downloading: false,
            update_error: None,
            update_result_share: Arc::new(Mutex::new(None)),
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
                auto_clean_smart_only: settings.auto_clean_smart_only,
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
            startup_sort: crate::startup::StartupSortColumn::Impact,
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
            process_tree_view: false,
            affinity_change: None,
            network_socket_search: String::new(),
            service_page: crate::app::page_state::ServicePageState::default(),
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
            #[cfg(target_os = "windows")]
            _hotkey_manager,
            #[cfg(target_os = "windows")]
            clean_ram_hotkey,
            is_hidden: false,
            widget_open: settings.show_widget,
            start_minimized_applied: false,
        }
    }
    #[cfg(test)]
    pub(crate) fn test_app() -> Self {
        let settings = AppSettings::default();
        let data = Arc::new(parking_lot::RwLock::new(SystemData::default()));
        let shared_settings = Arc::new(Mutex::new(settings.clone()));
        let app_channels = crate::app::AppChannels::new();
        let (telemetry_commands, _) = std::sync::mpsc::sync_channel(16);
        let timeline = crate::timeline::TimelineHandle::start(false, 7);

        Self {
            app_channels,
            latest_snapshot: None,
            action_pending: false,
            action_status: None,
            pending_action_plan: None,
            action_history: Vec::new(),
            show_action_history: false,
            session_recorder: crate::persistence::session::SessionRecorder::default(),
            session_status: None,
            timeline,
            timeline_ui: crate::timeline::TimelineUiState::default(),
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
            process_sort_column: crate::processes::ProcessSortColumn::Memory,
            process_sort_ascending: false,
            show_export_csv: false,
            updater: crate::updater::Updater::new(),
            update_info_share: Arc::new(Mutex::new(None)),
            show_update_notification: false,
            update_check_time: None,
            update_downloading: false,
            update_error: None,
            update_result_share: Arc::new(Mutex::new(None)),
            ram_cleaner_state: RamCleanerState {
                last_cleaned: None,
                last_cleaned_display: String::new(),
                bytes_freed: 0,
                auto_clean_enabled: false,
                auto_clean_threshold: 80.0,
                auto_clean_interval: 60,
                auto_clean_target: 60.0,
                auto_clean_exclusions: Vec::new(),
                auto_clean_idle_only: false,
                auto_clean_smart_only: true,
                auto_clean_notify: false,
                auto_clean_max_mb: 0,
                is_cleaning: false,
                clean_count: 0,
            },
            startup_items: Vec::new(),
            startup_items_loaded: false,
            startup_items_loading: false,
            startup_items_share: Arc::new(Mutex::new(None)),
            startup_search: String::new(),
            startup_sort: crate::startup::StartupSortColumn::Impact,
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
            process_tree_view: false,
            affinity_change: None,
            network_socket_search: String::new(),
            service_page: crate::app::page_state::ServicePageState::default(),
            #[cfg(target_os = "windows")]
            tray_icon: None,
            #[cfg(target_os = "windows")]
            tray_menu_show_id: None,
            #[cfg(target_os = "windows")]
            tray_menu_quit_id: None,
            #[cfg(target_os = "windows")]
            tray_menu_clean_id: None,
            #[cfg(target_os = "windows")]
            tray_menu_procman_id: None,
            #[cfg(target_os = "windows")]
            tray_menu_pause_id: None,
            #[cfg(target_os = "windows")]
            tray_menu_pause_item: None,
            #[cfg(target_os = "windows")]
            tray_menu_handle: None,
            #[cfg(target_os = "windows")]
            tray_menu_power_item: None,
            #[cfg(target_os = "windows")]
            tray_menu_power_items: std::collections::HashMap::new(),
            #[cfg(target_os = "windows")]
            tray_menu_power_guids: std::collections::HashMap::new(),
            #[cfg(target_os = "windows")]
            _hotkey_manager: None,
            #[cfg(target_os = "windows")]
            clean_ram_hotkey: None,
            is_hidden: false,
            widget_open: false,
            start_minimized_applied: true,
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
            .unwrap_or_else(|| crate::snapshot_from_data(&self.data.read()));
        crate::persistence::diagnostics::export(destination, &snapshot, &self.settings)
    }

    pub(crate) fn export_to_csv(&self, data: &SystemData) -> Result<String, Box<dyn std::error::Error>> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Header
        wtr.write_record(["Category", "Metric", "Value"])?;

        // System info
        wtr.write_record(["System", "Timestamp", &data.last_update])?;
        wtr.write_record(["CPU", "Usage %", &format!("{:.2}", data.cpu_usage)])?;
        wtr.write_record([
            "Memory",
            "Total GB",
            &format!("{:.2}", crate::ui::components::bytes_to_gb(data.memory_total)),
        ])?;
        wtr.write_record([
            "Memory",
            "Used GB",
            &format!("{:.2}", crate::ui::components::bytes_to_gb(data.memory_used)),
        ])?;
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
        wtr.write_record(["Process PID", "Name", "Memory MB", "VRAM MB", "CPU %"])?;
        for proc in &data.top_processes {
            let vram_str = proc
                .vram_bytes
                .map(|b| format!("{:.2}", crate::ui::components::bytes_to_mb(b)))
                .unwrap_or_else(|| "-".to_string());
            wtr.write_record([
                &proc.pid.to_string(),
                &proc.name,
                &format!("{:.2}", crate::ui::components::bytes_to_mb(proc.memory)),
                &vram_str,
                &format!("{:.2}", proc.cpu_usage),
            ])?;
        }

        let csv_data = String::from_utf8(wtr.into_inner()?)?;
        Ok(csv_data)
    }

    pub(crate) fn export_data_to_json(&self, data: &SystemData) -> Result<String, Box<dyn std::error::Error>> {
        use serde::Serialize;
        #[derive(Serialize)]
        struct ExportData {
            timestamp: String,
            cpu_usage: f32,
            memory_used: u64,
            memory_total: u64,
            memory_percentage: f32,
            gpu_info: Option<GpuInfo>,
            top_processes: Vec<crate::processes::ProcessInfo>,
            disk_info: Vec<DiskInfo>,
            network_info: Vec<NetworkInfo>,
            system_info: SystemInfo,
            startup_item_count: usize,
            high_impact_startup_count: usize,
            boot_diagnostics: Option<crate::startup::BootDiagnostics>,
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
            high_impact_startup_count: crate::startup::high_impact_count(&self.startup_items),
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

    pub fn queue_action(&mut self, command: crate::app::commands::ActionCommand) {
        if self.action_pending || self.pending_action_plan.is_some() {
            self.action_status = Some("Another system action is already pending.".into());
            return;
        }
        self.pending_action_plan = Some(crate::app::actions::ActionPlan::from_command(command));
    }

    pub fn start_ram_clean(&mut self, _ctx: &egui::Context) {
        self.queue_action(crate::app::commands::ActionCommand::CleanRam);
    }
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

pub(crate) fn snapshot_from_data(data: &SystemData) -> crate::monitoring::SystemSnapshot {
    let mut provider_status: std::collections::HashMap<_, _> = data
        .provider_status
        .iter()
        .map(|(name, available)| {
            (
                name.clone(),
                crate::monitoring::snapshot::ProviderStatus {
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
            crate::monitoring::snapshot::ProviderStatus {
                available,
                stale: data.monitoring_paused,
                error: None,
            },
        );
    }
    crate::monitoring::SystemSnapshot {
        sampled_at: std::time::SystemTime::now(),
        cpu_usage: data.cpu_usage,
        cpu_cores: data.cpu_cores.iter().map(|core| core.usage).collect(),
        cpu_temperature: data.cpu_temperature,
        memory_total: data.memory_total,
        memory_used: data.memory_used,
        memory_percentage: data.memory_percentage,
        swap: crate::monitoring::snapshot::SwapSnapshot {
            total: data.swap_info.total,
            used: data.swap_info.used,
            percentage: data.swap_info.percentage,
        },
        gpus: data
            .gpu_info
            .iter()
            .map(|gpu| crate::monitoring::snapshot::GpuSnapshot {
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
            .map(|disk| crate::monitoring::snapshot::DiskSnapshot {
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
            .map(|network| crate::monitoring::snapshot::NetworkSnapshot {
                interface: network.interface.clone(),
                received: network.received,
                transmitted: network.transmitted,
                received_bytes_per_second: network.received_rate,
                transmitted_bytes_per_second: network.transmitted_rate,
            })
            .collect(),
        processes: (if data.timeline_processes.is_empty() {
            &data.top_processes
        } else {
            &data.timeline_processes
        })
        .iter()
        .map(|process| crate::monitoring::snapshot::ProcessSnapshot {
            pid: process.pid,
            start_time: process.start_time,
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
            .map(|battery| crate::monitoring::snapshot::BatterySnapshot {
                design_capacity: battery.design_capacity,
                full_charge_capacity: battery.full_charge_capacity,
                status: battery.status,
                discharge_state: battery.discharge_state.clone(),
                present: battery.present,
            }),
        system: crate::monitoring::snapshot::SystemInfoSnapshot {
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

pub fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../../assets/icon.png");
    let image = image::load_from_memory(icon_bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

#[cfg(target_os = "windows")]
pub fn load_tray_icon() -> Option<tray_icon::Icon> {
    let image = image::load_from_memory(include_bytes!("../../assets/icon.png"))
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    tray_icon::Icon::from_rgba(rgba, width, height).ok()
}

#[cfg(test)]
mod alert_tests {
    use super::*;

    #[test]
    fn in_app_alerts_do_not_require_desktop_notifications() {
        let settings = AppSettings {
            show_notifications: false,
            notification_cpu_threshold: 80.0,
            ..Default::default()
        };
        let data = SystemData {
            cpu_usage: 85.0,
            ..Default::default()
        };
        let alerts = SystemMonitor::check_alerts(&settings, &data);
        assert!(alerts.iter().any(|alert| alert.source == AlertSource::Cpu));
    }

    #[test]
    fn disk_alert_uses_configured_threshold_and_typed_source() {
        let settings = AppSettings {
            notification_disk_threshold: 75.0,
            ..Default::default()
        };
        let data = SystemData {
            disk_info: vec![DiskInfo {
                name: "Data".into(),
                mount_point: "D:\\".into(),
                total_space: 100,
                available_space: 20,
                usage_percentage: 80.0,
                file_system: "NTFS".into(),
            }],
            ..Default::default()
        };
        let alerts = SystemMonitor::check_alerts(&settings, &data);
        assert_eq!(alerts.len(), 1);
        assert_eq!(
            alerts[0].source,
            AlertSource::Disk {
                mount_point: "D:\\".into(),
                name: "Data".into(),
            }
        );
        assert_eq!(alerts[0].key(), "disk:D:\\");
    }
}
