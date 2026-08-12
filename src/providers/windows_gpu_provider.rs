//! Vendor-neutral Windows GPU utilization via formatted WMI performance counters.

use super::{MetricValue, ProviderData, ProviderError, TelemetryProvider};
use std::time::Duration;

#[derive(Default)]
pub struct WindowsGpuProvider {
    engine_class: Option<String>,
    memory_class: Option<String>,
    available: bool,
}

impl WindowsGpuProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(target_os = "windows")]
fn numeric(value: &wmi::Variant) -> u64 {
    match value {
        wmi::Variant::UI1(value) => *value as u64,
        wmi::Variant::UI2(value) => *value as u64,
        wmi::Variant::UI4(value) => *value as u64,
        wmi::Variant::UI8(value) => *value,
        wmi::Variant::I1(value) => (*value).max(0) as u64,
        wmi::Variant::I2(value) => (*value).max(0) as u64,
        wmi::Variant::I4(value) => (*value).max(0) as u64,
        wmi::Variant::I8(value) => (*value).max(0) as u64,
        wmi::Variant::R4(value) => value.max(0.0) as u64,
        wmi::Variant::R8(value) => value.max(0.0) as u64,
        wmi::Variant::String(value) => value.parse().unwrap_or_default(),
        _ => 0,
    }
}

impl TelemetryProvider for WindowsGpuProvider {
    fn name(&self) -> &str {
        "windows_gpu"
    }

    fn poll_interval(&self) -> Duration {
        Duration::from_secs(1)
    }

    fn poll(&mut self) -> Result<ProviderData, ProviderError> {
        #[cfg(not(target_os = "windows"))]
        {
            return Err(ProviderError::Unavailable(
                "Windows GPU counters require Windows".into(),
            ));
        }

        #[cfg(target_os = "windows")]
        {
            use std::collections::HashMap;
            use wmi::{COMLibrary, Variant, WMIConnection};

            let com = COMLibrary::new().map_err(|error| ProviderError::InitFailed(error.to_string()))?;
            let connection =
                WMIConnection::new(com.into()).map_err(|error| ProviderError::InitFailed(error.to_string()))?;

            if self.engine_class.is_none() || self.memory_class.is_none() {
                for prefix in ["GPUPerformanceCounters", "GPUPerformanceMonitors"] {
                    if self.engine_class.is_none() {
                        let class = format!("Win32_PerfFormattedData_{prefix}_GPUEngine");
                        let query = format!("SELECT UtilizationPercentage FROM {class}");
                        if connection.raw_query::<HashMap<String, Variant>>(&query).is_ok() {
                            self.engine_class = Some(class);
                        }
                    }
                    if self.memory_class.is_none() {
                        let class = format!("Win32_PerfFormattedData_{prefix}_GPULocalAdapterMemory");
                        let query = format!("SELECT LocalUsage FROM {class}");
                        if connection.raw_query::<HashMap<String, Variant>>(&query).is_ok() {
                            self.memory_class = Some(class);
                        }
                    }
                }
            }

            let engine_class = self
                .engine_class
                .as_ref()
                .ok_or_else(|| ProviderError::Unavailable("GPU engine performance counters are unavailable".into()))?;
            let engine_query = format!("SELECT UtilizationPercentage FROM {engine_class}");
            let engines: Vec<HashMap<String, Variant>> = connection
                .raw_query(&engine_query)
                .map_err(|error| ProviderError::PollFailed(error.to_string()))?;
            let utilization = engines
                .iter()
                .filter_map(|row| row.get("UtilizationPercentage"))
                .map(numeric)
                .max()
                .unwrap_or_default()
                .min(100);

            let mut data = ProviderData::new();
            data.insert("gpu.generic.utilization".into(), MetricValue::UInt(utilization));
            if let Some(memory_class) = &self.memory_class {
                let query = format!("SELECT LocalUsage FROM {memory_class}");
                if let Ok(rows) = connection.raw_query::<HashMap<String, Variant>>(&query) {
                    let used = rows
                        .iter()
                        .filter_map(|row| row.get("LocalUsage"))
                        .map(numeric)
                        .fold(0_u64, u64::saturating_add);
                    data.insert("gpu.generic.vram_used".into(), MetricValue::UInt(used));
                }
            }
            self.available = true;
            Ok(data)
        }
    }

    fn is_available(&self) -> bool {
        self.available
    }
}
