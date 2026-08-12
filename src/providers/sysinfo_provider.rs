//! Sysinfo-based telemetry provider for CPU, RAM, Disk, and Network metrics.

use super::{MetricValue, ProviderData, ProviderError, TelemetryProvider};
use std::time::Duration;
use sysinfo::System;

pub struct SysinfoProvider {
    system: System,
    available: bool,
}

impl SysinfoProvider {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self {
            system,
            available: true,
        }
    }
}

impl Default for SysinfoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryProvider for SysinfoProvider {
    fn name(&self) -> &str {
        "sysinfo"
    }

    fn poll_interval(&self) -> Duration {
        Duration::from_millis(200) // ~5 Hz
    }

    fn poll(&mut self) -> Result<ProviderData, ProviderError> {
        self.system.refresh_cpu();
        self.system.refresh_memory();

        let mut data = ProviderData::new();

        // CPU
        data.insert(
            "cpu.global_usage".into(),
            MetricValue::Float(self.system.global_cpu_info().cpu_usage() as f64),
        );
        data.insert(
            "cpu.core_count".into(),
            MetricValue::UInt(self.system.cpus().len() as u64),
        );

        // Per-core usage
        for (i, cpu) in self.system.cpus().iter().enumerate() {
            data.insert(
                format!("cpu.core.{}.usage", i),
                MetricValue::Float(cpu.cpu_usage() as f64),
            );
            data.insert(format!("cpu.core.{}.frequency", i), MetricValue::UInt(cpu.frequency()));
        }

        // Memory
        data.insert("memory.total".into(), MetricValue::UInt(self.system.total_memory()));
        data.insert("memory.used".into(), MetricValue::UInt(self.system.used_memory()));
        data.insert(
            "memory.available".into(),
            MetricValue::UInt(self.system.available_memory()),
        );
        data.insert("memory.total_swap".into(), MetricValue::UInt(self.system.total_swap()));
        data.insert("memory.used_swap".into(), MetricValue::UInt(self.system.used_swap()));

        Ok(data)
    }

    fn is_available(&self) -> bool {
        self.available
    }
}
