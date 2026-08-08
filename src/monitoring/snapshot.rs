use std::collections::HashMap;
use std::time::SystemTime;

use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProviderStatus {
    pub available: bool,
    pub stale: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GpuSnapshot {
    pub name: String,
    pub utilization: f32,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
    pub temperature: Option<u32>,
    pub clock_mhz: Option<u32>,
    pub power_watts: Option<f32>,
    pub fan_percent: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DiskSnapshot {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub usage_percentage: f32,
    pub file_system: String,
    pub read_bytes_per_second: f64,
    pub written_bytes_per_second: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct NetworkSnapshot {
    pub interface: String,
    pub received: u64,
    pub transmitted: u64,
    pub received_bytes_per_second: f64,
    pub transmitted_bytes_per_second: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub status: String,
    pub disk_read_bytes: u64,
    pub disk_written_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SwapSnapshot {
    pub total: u64,
    pub used: u64,
    pub percentage: f32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BatterySnapshot {
    pub design_capacity: u32,
    pub full_charge_capacity: u32,
    pub status: u16,
    pub discharge_state: Option<String>,
    pub present: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemInfoSnapshot {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
    pub uptime: u64,
    pub cpu_count: usize,
    pub cpu_brand: String,
    pub motherboard: Option<String>,
    pub bios_version: Option<String>,
    pub gpu_driver: Option<String>,
    pub os_build: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemSnapshot {
    pub sampled_at: SystemTime,
    pub cpu_usage: f32,
    pub cpu_cores: Vec<f32>,
    pub cpu_temperature: Option<f32>,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_percentage: f32,
    pub swap: SwapSnapshot,
    pub gpus: Vec<GpuSnapshot>,
    pub disks: Vec<DiskSnapshot>,
    pub networks: Vec<NetworkSnapshot>,
    pub processes: Vec<ProcessSnapshot>,
    pub battery: Option<BatterySnapshot>,
    pub system: SystemInfoSnapshot,
    pub provider_status: HashMap<String, ProviderStatus>,
    pub paused: bool,
}

impl Default for SystemSnapshot {
    fn default() -> Self {
        Self {
            sampled_at: SystemTime::UNIX_EPOCH,
            cpu_usage: 0.0,
            cpu_cores: Vec::new(),
            cpu_temperature: None,
            memory_total: 0,
            memory_used: 0,
            memory_percentage: 0.0,
            swap: SwapSnapshot::default(),
            gpus: Vec::new(),
            disks: Vec::new(),
            networks: Vec::new(),
            processes: Vec::new(),
            battery: None,
            system: SystemInfoSnapshot::default(),
            provider_status: HashMap::new(),
            paused: false,
        }
    }
}
