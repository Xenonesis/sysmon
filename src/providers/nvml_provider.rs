//! NVIDIA GPU telemetry provider using NVML.
//!
//! Gracefully handles systems without NVIDIA GPUs — returns
//! `ProviderError::Unavailable` instead of crashing.

use super::{MetricValue, ProviderData, ProviderError, TelemetryProvider};
use std::time::Duration;

pub struct NvmlProvider {
    nvml: Option<nvml_wrapper::Nvml>,
    available: bool,
    device_count: u32,
}

impl NvmlProvider {
    pub fn new() -> Self {
        match nvml_wrapper::Nvml::init() {
            Ok(nvml) => {
                let device_count = nvml.device_count().unwrap_or(0);
                Self {
                    nvml: Some(nvml),
                    available: device_count > 0,
                    device_count,
                }
            }
            Err(_) => Self {
                nvml: None,
                available: false,
                device_count: 0,
            },
        }
    }
}

impl Default for NvmlProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryProvider for NvmlProvider {
    fn name(&self) -> &str {
        "nvml"
    }

    fn poll_interval(&self) -> Duration {
        Duration::from_millis(200) // ~5 Hz
    }

    fn poll(&mut self) -> Result<ProviderData, ProviderError> {
        let nvml = self
            .nvml
            .as_ref()
            .ok_or_else(|| ProviderError::Unavailable("NVML not initialized".into()))?;

        let mut data = ProviderData::new();
        data.insert("gpu.device_count".into(), MetricValue::UInt(self.device_count as u64));

        for i in 0..self.device_count {
            let prefix = format!("gpu.{}", i);
            match nvml.device_by_index(i) {
                Ok(device) => {
                    // Name
                    if let Ok(name) = device.name() {
                        data.insert(format!("{}.name", prefix), MetricValue::Text(name));
                    }

                    // Utilization
                    if let Ok(util) = device.utilization_rates() {
                        data.insert(format!("{}.utilization", prefix), MetricValue::UInt(util.gpu as u64));
                        data.insert(format!("{}.memory_util", prefix), MetricValue::UInt(util.memory as u64));
                    }

                    // Memory
                    if let Ok(mem) = device.memory_info() {
                        data.insert(format!("{}.vram_used", prefix), MetricValue::UInt(mem.used));
                        data.insert(format!("{}.vram_total", prefix), MetricValue::UInt(mem.total));
                        data.insert(format!("{}.vram_free", prefix), MetricValue::UInt(mem.free));
                    }

                    // Temperature
                    if let Ok(temp) = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu) {
                        data.insert(format!("{}.temperature", prefix), MetricValue::UInt(temp as u64));
                    }

                    // Fan speed
                    if let Ok(fan) = device.fan_speed(0) {
                        data.insert(format!("{}.fan_speed", prefix), MetricValue::UInt(fan as u64));
                    }

                    // Clocks
                    if let Ok(clock) = device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics) {
                        data.insert(format!("{}.clock_graphics", prefix), MetricValue::UInt(clock as u64));
                    }
                    if let Ok(clock) = device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory) {
                        data.insert(format!("{}.clock_memory", prefix), MetricValue::UInt(clock as u64));
                    }

                    // Power
                    if let Ok(power) = device.power_usage() {
                        data.insert(format!("{}.power_draw_mw", prefix), MetricValue::UInt(power as u64));
                    }
                    if let Ok(limit) = device.power_management_limit() {
                        data.insert(format!("{}.power_limit_mw", prefix), MetricValue::UInt(limit as u64));
                    }
                }
                Err(e) => {
                    data.insert(format!("{}.error", prefix), MetricValue::Text(format!("{}", e)));
                }
            }
        }

        Ok(data)
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn shutdown(&mut self) {
        self.nvml = None;
        self.available = false;
    }
}
