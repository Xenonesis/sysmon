use crate::app::models::{CpuCoreInfo, DiskInfo, NetworkInfo, SwapInfo, SystemInfo, SystemMonitor};

use std::time::Instant;
use sysinfo::System;

impl SystemMonitor {
    pub(crate) fn get_memory_info(&self) -> (u64, u64, f32) {
        let total = self.sys.total_memory();
        let used = self.sys.used_memory();
        let percentage = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        (total, used, percentage as f32)
    }

    pub(crate) fn get_cpu_usage(&mut self) -> f32 {
        self.sys.global_cpu_usage()
    }

    pub(crate) fn get_top_processes(&self, count: usize) -> Vec<crate::processes::ProcessInfo> {
        let vram_map = crate::providers::windows_gpu_provider::query_process_memory().unwrap_or_default();

        let cpu_count = self.sys.cpus().len().max(1) as f32;
        let mut processes: Vec<_> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, process)| {
                // Try to use the exe path's file name if `name()` is empty or not helpful
                let mut name_str = process.name().to_string_lossy().into_owned();
                if name_str.is_empty()
                    && let Some(exe_path) = process.exe()
                    && let Some(file_name) = exe_path.file_name()
                {
                    name_str = file_name.to_string_lossy().into_owned();
                }

                let identity = crate::processes::process_identity(pid.as_u32()).ok();
                crate::processes::ProcessInfo {
                    pid: pid.as_u32(),
                    start_time: process.start_time(),
                    identity,
                    name: name_str,
                    parent_pid: process.parent().map(|p| p.as_u32()),
                    cpu_usage: process.cpu_usage() / cpu_count,
                    memory: process.memory(),
                    vram_bytes: identity.and_then(|identity| vram_map.get(&identity).copied()),
                    status: format!("{:?}", process.status()),
                    disk_read_bytes: process.disk_usage().total_read_bytes,
                    disk_written_bytes: process.disk_usage().total_written_bytes,
                    disk_read_bytes_per_second: None,
                    disk_written_bytes_per_second: None,
                }
            })
            .collect();

        processes.sort_by_key(|process| std::cmp::Reverse(process.memory));
        processes.truncate(count);
        processes
    }

    pub(crate) fn get_timeline_processes(&self, per_metric: usize) -> Vec<crate::processes::ProcessInfo> {
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

    pub(crate) fn get_cpu_cores_info(&self) -> Vec<CpuCoreInfo> {
        self.sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(id, cpu)| CpuCoreInfo {
                core_id: id,
                usage: cpu.cpu_usage(),
                frequency_mhz: (cpu.frequency() > 0).then_some(cpu.frequency()),
                name: cpu.name().to_string(),
            })
            .collect()
    }

    pub(crate) fn get_swap_info(&self) -> SwapInfo {
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

    pub(crate) fn get_disk_info(&self) -> Vec<DiskInfo> {
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

    pub(crate) fn get_disk_io(&mut self, _refresh_interval: u64) -> (f64, f64) {
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(self.last_disk_update);
        let mut current = std::collections::HashMap::new();
        let mut rates = (0.0, 0.0);
        for (pid, process) in self.sys.processes() {
            let identity = (pid.as_u32(), process.start_time());
            let usage = process.disk_usage();
            let totals = (usage.total_read_bytes, usage.total_written_bytes);
            let previous = self.previous_disk_totals.get(&identity);
            rates.0 +=
                crate::monitoring::rates::counter_rate(previous.map(|p| p.0), totals.0, elapsed).value_per_second;
            rates.1 +=
                crate::monitoring::rates::counter_rate(previous.map(|p| p.1), totals.1, elapsed).value_per_second;
            current.insert(identity, totals);
        }
        self.previous_disk_totals = current;
        self.last_disk_update = now;
        rates
    }

    pub(crate) fn get_network_info(&mut self) -> Vec<NetworkInfo> {
        let elapsed = self.last_network_update.elapsed();
        let mut current_totals = std::collections::HashMap::new();
        let network_info = self
            .networks
            .iter()
            .map(|(interface, data)| {
                let current = (data.total_received(), data.total_transmitted());
                let previous = self.previous_network_totals.get(interface).copied();
                current_totals.insert(interface.clone(), current);
                let received_rate = crate::monitoring::rates::counter_rate(previous.map(|p| p.0), current.0, elapsed);
                let transmitted_rate =
                    crate::monitoring::rates::counter_rate(previous.map(|p| p.1), current.1, elapsed);
                NetworkInfo {
                    interface: interface.clone(),
                    received: current.0,
                    transmitted: current.1,
                    received_rate: received_rate.value_per_second,
                    transmitted_rate: transmitted_rate.value_per_second,
                }
            })
            .collect();
        self.previous_network_totals = current_totals;
        self.last_network_update = Instant::now();
        network_info
    }

    pub(crate) fn get_system_info(&self) -> SystemInfo {
        let (motherboard, bios_version, gpu_driver, os_build) = Self::get_wmi_system_details();
        SystemInfo {
            os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            uptime: System::uptime(),
            cpu_count: self.sys.cpus().len(),
            physical_core_count: System::physical_core_count(),
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
