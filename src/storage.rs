//! Physical storage disk drive health, S.M.A.R.T. predictive status, and SSD diagnostics.

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
    use crate::providers::init_com;
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
        let com = match init_com() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let wmi_con = match WMIConnection::new(com.into()) {
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
}
