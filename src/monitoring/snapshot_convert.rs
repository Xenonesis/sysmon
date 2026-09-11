use crate::app::models::*;

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
                    observed_at: None,
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
                observed_at: None,
            },
        );
    }
    let now = std::time::SystemTime::now();
    crate::monitoring::SystemSnapshot {
        sampled_at: now,
        disk_read_bytes_per_second: Some(data.disk_read_rate),
        disk_written_bytes_per_second: Some(data.disk_write_rate),
        metric_status: std::collections::HashMap::new(),
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
                read_bytes_per_second: Some(data.disk_read_rate),
                written_bytes_per_second: Some(data.disk_write_rate),
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
            identity: process.identity,
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
            physical_core_count: data.system_info.physical_core_count,
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
