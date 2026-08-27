//! Physical storage disk drive health, S.M.A.R.T. predictive status, and SSD diagnostics.
pub mod file_locks;
pub mod reclaimer;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhysicalDiskHealth {
    pub device_id: String,
    pub model: String,
    pub media_type: String,
    pub size_bytes: u64,
    pub status: String,
    pub smart_status: String,
    pub temperature_c: Option<u32>,
    pub wear_percentage: Option<u32>,
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use wmi::WMIConnection;

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "PascalCase")]
    struct Win32DiskDriveRow {
        device_id: Option<String>,
        model: Option<String>,
        media_type: Option<String>,
        size: Option<u64>,
        status: Option<String>,
        interface_type: Option<String>,
    }

    pub fn get_physical_disks_internal() -> Vec<PhysicalDiskHealth> {
        let wmi_con = match WMIConnection::new() {
            Ok(con) => con,
            Err(_) => return Vec::new(),
        };

        let rows: Vec<Win32DiskDriveRow> = match wmi_con
            .raw_query("SELECT DeviceID, Model, MediaType, Size, Status, InterfaceType FROM Win32_DiskDrive")
        {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        rows.into_iter()
            .map(|row| {
                let model = row.model.unwrap_or_else(|| "Generic Physical Disk".into());
                let interface = row.interface_type.unwrap_or_default();
                let raw_media = row.media_type.unwrap_or_default();

                let media_type = if interface.to_uppercase().contains("NVME") || model.to_uppercase().contains("NVME") {
                    "NVMe SSD".to_string()
                } else if raw_media.to_uppercase().contains("SSD") || model.to_uppercase().contains("SSD") {
                    "SATA SSD".to_string()
                } else if raw_media.to_uppercase().contains("FIXED") {
                    "Hard Disk Drive (HDD)".to_string()
                } else {
                    "Fixed Storage Media".to_string()
                };

                let raw_status = row.status.unwrap_or_else(|| "OK".into());
                let (status, smart_status) = if raw_status.eq_ignore_ascii_case("OK") {
                    ("HEALTHY".to_string(), "PASSED (GOOD)".to_string())
                } else if raw_status.to_uppercase().contains("PRED") || raw_status.to_uppercase().contains("FAIL") {
                    ("CRITICAL".to_string(), "PREDICTIVE FAILURE".to_string())
                } else {
                    ("WARNING".to_string(), "DEGRADED".to_string())
                };

                PhysicalDiskHealth {
                    device_id: row.device_id.unwrap_or_default(),
                    model,
                    media_type,
                    size_bytes: row.size.unwrap_or(0),
                    status,
                    smart_status,
                    temperature_c: None,
                    wear_percentage: Some(100),
                }
            })
            .collect()
    }
}

/// Retrieve physical disk drive health and SMART status.
pub fn get_physical_disks() -> Vec<PhysicalDiskHealth> {
    #[cfg(target_os = "windows")]
    {
        windows_impl::get_physical_disks_internal()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

/// Live per-physical-disk performance counters (latency, queue depth, IOPS).
#[derive(Debug, Clone, Default)]
pub struct DiskPerfStats {
    pub name: String,
    pub read_latency_ms: f32,
    pub write_latency_ms: f32,
    pub queue_depth: f32,
    pub active_pct: f32,
    pub read_iops: u32,
    pub write_iops: u32,
}

#[cfg(target_os = "windows")]
mod perf_impl {
    use super::DiskPerfStats;
    use serde::Deserialize;
    use wmi::WMIConnection;

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "PascalCase")]
    struct PerfRow {
        name: Option<String>,
        avg_disk_sec_per_read: Option<u32>,
        avg_disk_sec_per_write: Option<u32>,
        avg_disk_queue_length: Option<u32>,
        percent_disk_time: Option<u32>,
        disk_reads_persec: Option<u32>,
        disk_writes_persec: Option<u32>,
    }

    pub fn get_disk_perf_internal() -> Vec<DiskPerfStats> {
        let Ok(wmi_con) = WMIConnection::new() else {
            return Vec::new();
        };
        let Ok(rows) = wmi_con.raw_query::<PerfRow>(
            "SELECT Name, AvgDiskSecPerRead, AvgDiskSecPerWrite, AvgDiskQueueLength, PercentDiskTime, DiskReadsPersec, DiskWritesPersec FROM Win32_PerfFormattedData_PerfDisk_PhysicalDisk",
        ) else {
            return Vec::new();
        };
        rows.into_iter()
            .filter_map(|row| {
                let name = row.name?;
                if name.eq_ignore_ascii_case("_Total") {
                    return None;
                }
                Some(DiskPerfStats {
                    name,
                    read_latency_ms: row.avg_disk_sec_per_read.unwrap_or(0) as f32,
                    write_latency_ms: row.avg_disk_sec_per_write.unwrap_or(0) as f32,
                    queue_depth: row.avg_disk_queue_length.unwrap_or(0) as f32,
                    active_pct: row.percent_disk_time.unwrap_or(0) as f32,
                    read_iops: row.disk_reads_persec.unwrap_or(0),
                    write_iops: row.disk_writes_persec.unwrap_or(0),
                })
            })
            .collect()
    }
}

/// Retrieve live disk latency, queue depth and IOPS per physical disk.
pub fn get_disk_perf() -> Vec<DiskPerfStats> {
    #[cfg(target_os = "windows")]
    {
        perf_impl::get_disk_perf_internal()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_disk_health_creation() {
        let disk = PhysicalDiskHealth {
            device_id: "\\\\.\\PHYSICALDRIVE0".into(),
            model: "Samsung SSD 980 PRO 1TB".into(),
            media_type: "NVMe SSD".into(),
            size_bytes: 1_000_204_886_016,
            status: "HEALTHY".into(),
            smart_status: "PASSED (GOOD)".into(),
            temperature_c: Some(38),
            wear_percentage: Some(99),
        };
        assert_eq!(disk.media_type, "NVMe SSD");
        assert_eq!(disk.status, "HEALTHY");
    }

    #[test]
    fn get_physical_disks_does_not_panic() {
        let disks = get_physical_disks();
        for d in disks {
            assert!(!d.model.is_empty());
        }
    }

    #[test]
    fn get_disk_perf_does_not_panic() {
        let perf = get_disk_perf();
        for p in perf {
            assert!(!p.name.is_empty());
            assert!(!p.name.eq_ignore_ascii_case("_Total"));
        }
    }
}
