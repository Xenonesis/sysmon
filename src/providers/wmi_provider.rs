//! WMI-based telemetry provider for hardware identity (Motherboard, BIOS, OS).
//!
//! This is a "slow poller" — fetched once at startup and on manual refresh.

use super::{MetricValue, ProviderData, ProviderError, TelemetryProvider};
use std::collections::HashMap;
use std::time::Duration;

pub struct WmiProvider {
    available: bool,
    cached_data: Option<ProviderData>,
}

impl WmiProvider {
    pub fn new() -> Self {
        Self {
            available: true,
            cached_data: None,
        }
    }

    /// Perform a full WMI query and cache the results.
    fn query_hardware_info(&mut self) -> Result<ProviderData, ProviderError> {
        let mut data = ProviderData::new();

        #[cfg(target_os = "windows")]
        {
            use wmi::{Variant, WMIConnection};

            let com = super::init_com().map_err(|e| ProviderError::InitFailed(format!("COM init: {}", e)))?;
            let wmi_con = WMIConnection::new(com.into())
                .map_err(|e| ProviderError::InitFailed(format!("WMI connection: {}", e)))?;

            // Motherboard
            let board_query: Result<Vec<HashMap<String, Variant>>, _> =
                wmi_con.raw_query("SELECT Manufacturer, Product FROM Win32_BaseBoard");
            if let Ok(results) = board_query
                && let Some(row) = results.first()
            {
                if let Some(Variant::String(v)) = row.get("Manufacturer") {
                    data.insert("board.manufacturer".into(), MetricValue::Text(v.clone()));
                }
                if let Some(Variant::String(v)) = row.get("Product") {
                    data.insert("board.product".into(), MetricValue::Text(v.clone()));
                }
            }

            // BIOS
            let bios_query: Result<Vec<HashMap<String, Variant>>, _> =
                wmi_con.raw_query("SELECT Manufacturer, SMBIOSBIOSVersion FROM Win32_BIOS");
            if let Ok(results) = bios_query
                && let Some(row) = results.first()
            {
                if let Some(Variant::String(v)) = row.get("Manufacturer") {
                    data.insert("bios.manufacturer".into(), MetricValue::Text(v.clone()));
                }
                if let Some(Variant::String(v)) = row.get("SMBIOSBIOSVersion") {
                    data.insert("bios.version".into(), MetricValue::Text(v.clone()));
                }
            }

            // Processor identity
            let cpu_query: Result<Vec<HashMap<String, Variant>>, _> = wmi_con
                .raw_query("SELECT Name, MaxClockSpeed, NumberOfCores, NumberOfLogicalProcessors FROM Win32_Processor");
            if let Ok(results) = cpu_query
                && let Some(row) = results.first()
            {
                if let Some(Variant::String(v)) = row.get("Name") {
                    data.insert("cpu.name".into(), MetricValue::Text(v.clone()));
                }
                if let Some(Variant::UI4(v)) = row.get("MaxClockSpeed") {
                    data.insert("cpu.max_clock_mhz".into(), MetricValue::UInt(*v as u64));
                }
                if let Some(Variant::UI4(v)) = row.get("NumberOfCores") {
                    data.insert("cpu.physical_cores".into(), MetricValue::UInt(*v as u64));
                }
                if let Some(Variant::UI4(v)) = row.get("NumberOfLogicalProcessors") {
                    data.insert("cpu.logical_processors".into(), MetricValue::UInt(*v as u64));
                }
            }

            // Vendor-neutral GPU identity for AMD, Intel, NVIDIA and virtual adapters.
            let gpu_query: Result<Vec<HashMap<String, Variant>>, _> =
                wmi_con.raw_query("SELECT Name, AdapterRAM, DriverVersion, PNPDeviceID FROM Win32_VideoController");
            if let Ok(results) = gpu_query {
                data.insert("gpu.generic_count".into(), MetricValue::UInt(results.len() as u64));
                for (index, row) in results.iter().enumerate() {
                    let prefix = format!("gpu.generic.{index}");
                    if let Some(Variant::String(value)) = row.get("Name") {
                        data.insert(format!("{prefix}.name"), MetricValue::Text(value.clone()));
                    }
                    if let Some(Variant::String(value)) = row.get("DriverVersion") {
                        data.insert(format!("{prefix}.driver_version"), MetricValue::Text(value.clone()));
                    }
                    if let Some(Variant::String(value)) = row.get("PNPDeviceID") {
                        data.insert(format!("{prefix}.pnp_device_id"), MetricValue::Text(value.clone()));
                    }
                    let adapter_ram = match row.get("AdapterRAM") {
                        Some(Variant::UI4(value)) => Some(*value as u64),
                        Some(Variant::UI8(value)) => Some(*value),
                        _ => None,
                    };
                    if let Some(value) = adapter_ram {
                        data.insert(format!("{prefix}.vram_total"), MetricValue::UInt(value));
                    }
                }
            }
        }

        self.cached_data = Some(data.clone());
        Ok(data)
    }
}

impl Default for WmiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryProvider for WmiProvider {
    fn name(&self) -> &str {
        "wmi"
    }

    fn poll_interval(&self) -> Duration {
        Duration::from_secs(3600) // Startup-only, refresh on demand
    }

    fn poll(&mut self) -> Result<ProviderData, ProviderError> {
        // Return cached data if available (WMI data is static hardware info)
        if let Some(ref cached) = self.cached_data {
            return Ok(cached.clone());
        }
        self.query_hardware_info()
    }

    fn is_available(&self) -> bool {
        self.available
    }
}
