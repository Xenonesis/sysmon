use crate::app::models::{GpuInfo, SystemMonitor};
#[cfg(target_os = "windows")]
impl SystemMonitor {
    #[cfg(target_os = "windows")]
    pub(crate) fn get_gpu_info(&self, include_wmi: bool) -> Vec<GpuInfo> {
        let mut gpus = Vec::new();
        let mut nvml_names: Vec<String> = Vec::new();

        // Collect all NVML (NVIDIA) GPUs
        if let Some(ref nvml) = self.nvml
            && let Ok(device_count) = nvml.device_count()
        {
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

        // Also collect WMI GPUs (AMD/Intel) — skip any already covered by NVML
        if include_wmi && let Some(wmi_gpus) = self.get_gpu_info_wmi() {
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

        gpus
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn get_battery_wmi(&self) -> Option<wmi::WMIConnection> {
        wmi::WMIConnection::new().ok()
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn get_gpu_info_wmi(&self) -> Option<Vec<GpuInfo>> {
        let wmi = wmi::WMIConnection::new().ok()?;

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

        if gpus.is_empty() { None } else { Some(gpus) }
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn get_gpu_info(&self, _include_wmi: bool) -> Vec<GpuInfo> {
        Vec::new()
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn get_cpu_temperature_wmi(&self) -> Option<f32> {
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
    pub(crate) fn get_cpu_temperature_wmi(&self) -> Option<f32> {
        None
    }

    #[cfg(target_os = "windows")]
    /// One-time WMI queries for motherboard/BIOS/GPU-driver/OS-build details.
    /// Any failure returns `None` for the failed fields; WMI init failure returns all `None`.
    pub(crate) fn get_wmi_system_details() -> (Option<String>, Option<String>, Option<String>, Option<String>) {
        use wmi::{Variant, WMIConnection};
        let wmi = match WMIConnection::new() {
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
        let motherboard = one("SELECT Manufacturer, Product FROM Win32_BaseBoard", "Manufacturer")
            .map(|m| if m.trim().is_empty() { "N/A".to_string() } else { m });
        let bios_version = one("SELECT SMBIOSBIOSVersion FROM Win32_BIOS", "SMBIOSBIOSVersion");
        let gpu_driver = one("SELECT DriverVersion FROM Win32_VideoController", "DriverVersion");
        let os_build = one("SELECT BuildNumber FROM Win32_OperatingSystem", "BuildNumber");
        (motherboard, bios_version, gpu_driver, os_build)
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn get_wmi_system_details() -> (Option<String>, Option<String>, Option<String>, Option<String>) {
        (None, None, None, None)
    }
}
