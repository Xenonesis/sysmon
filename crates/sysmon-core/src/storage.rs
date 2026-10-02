//! Physical storage identity and measured performance. Device Status is not a SMART test.
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

#[derive(Debug, Clone, Default)]
pub struct DiskPerfStats {
    pub name: String,
    pub read_latency_ms: Option<f64>,
    pub write_latency_ms: Option<f64>,
    pub queue_depth: Option<f64>,
    /// Disk busy time weighted by queued operations; may exceed 100%.
    pub disk_time_pct: Option<f64>,
    pub read_iops: Option<f64>,
    pub write_iops: Option<f64>,
    pub observed_at: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawDisk {
    name: String,
    avg_disk_sec_per_read: Option<u64>,
    #[serde(rename = "AvgDiskSecPerRead_Base")]
    read_base: Option<u32>,
    avg_disk_sec_per_write: Option<u64>,
    #[serde(rename = "AvgDiskSecPerWrite_Base")]
    write_base: Option<u32>,
    avg_disk_queue_length: Option<u64>,
    percent_disk_time: Option<u64>,
    disk_reads_persec: Option<u32>,
    disk_writes_persec: Option<u32>,
    #[serde(rename = "Timestamp_PerfTime")]
    timestamp: Option<u64>,
    #[serde(rename = "Frequency_PerfTime")]
    frequency: Option<u64>,
    #[serde(rename = "Timestamp_Sys100NS")]
    timestamp_100ns: Option<u64>,
}

fn delta(a: Option<u64>, b: Option<u64>) -> Option<f64> {
    Some(a?.checked_sub(b?)? as f64)
}
fn ratio(numerator: Option<f64>, denominator: Option<f64>) -> Option<f64> {
    let denominator = denominator?;
    (denominator > 0.0)
        .then(|| numerator.map(|n| n / denominator))
        .flatten()
}
fn performance(current: &RawDisk, previous: Option<&RawDisk>) -> DiskPerfStats {
    let mut result = DiskPerfStats {
        name: current.name.clone(),
        ..Default::default()
    };
    let Some(previous) = previous else { return result };
    if current.frequency != previous.frequency {
        return result;
    }
    let frequency = current.frequency.filter(|f| *f > 0).map(|f| f as f64);
    let elapsed = ratio(delta(current.timestamp, previous.timestamp), frequency);
    let elapsed_100ns = delta(current.timestamp_100ns, previous.timestamp_100ns);
    let latency = |n, old, base: Option<u32>, old_base: Option<u32>| {
        ratio(
            ratio(delta(n, old), frequency),
            delta(base.map(u64::from), old_base.map(u64::from)),
        )
        .map(|s| s * 1000.0)
    };
    result.read_latency_ms = latency(
        current.avg_disk_sec_per_read,
        previous.avg_disk_sec_per_read,
        current.read_base,
        previous.read_base,
    );
    result.write_latency_ms = latency(
        current.avg_disk_sec_per_write,
        previous.avg_disk_sec_per_write,
        current.write_base,
        previous.write_base,
    );
    result.queue_depth = ratio(
        delta(current.avg_disk_queue_length, previous.avg_disk_queue_length),
        elapsed_100ns,
    );
    result.disk_time_pct = ratio(
        delta(current.percent_disk_time, previous.percent_disk_time),
        elapsed_100ns,
    )
    .map(|v| v * 100.0);
    result.read_iops = ratio(
        delta(
            current.disk_reads_persec.map(u64::from),
            previous.disk_reads_persec.map(u64::from),
        ),
        elapsed,
    );
    result.write_iops = ratio(
        delta(
            current.disk_writes_persec.map(u64::from),
            previous.disk_writes_persec.map(u64::from),
        ),
        elapsed,
    );
    result.observed_at = Some(std::time::SystemTime::now());
    result
}

pub fn get_physical_disks() -> Result<Vec<PhysicalDiskHealth>, String> {
    #[cfg(not(target_os = "windows"))]
    {
        Err("Physical disk health requires Windows".into())
    }
    #[cfg(target_os = "windows")]
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Row {
            device_id: String,
            model: Option<String>,
            media_type: Option<String>,
            size: Option<u64>,
            status: Option<String>,
        }
        let connection = wmi::WMIConnection::new().map_err(|e| e.to_string())?;
        let rows: Vec<Row> = connection
            .raw_query("SELECT DeviceID, Model, MediaType, Size, Status FROM Win32_DiskDrive")
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|row| PhysicalDiskHealth {
                device_id: row.device_id,
                model: row.model.unwrap_or_else(|| "Unknown disk model".into()),
                // Win32_DiskDrive.MediaType=Fixed is not evidence of an HDD/SSD or bus type.
                media_type: row.media_type.unwrap_or_else(|| "Unknown media type".into()),
                size_bytes: row.size.unwrap_or_default(),
                status: row.status.unwrap_or_else(|| "Unknown device status".into()),
                smart_status: "Not acquired (device status is not SMART)".into(),
                temperature_c: None,
                wear_percentage: None,
            })
            .collect())
    }
}

/// Raw timer/base counters preserve sub-millisecond latency; first acquisition warms a baseline.
pub fn get_disk_perf() -> Result<Vec<DiskPerfStats>, String> {
    #[cfg(not(target_os = "windows"))]
    {
        Err("Disk performance counters require Windows".into())
    }
    #[cfg(target_os = "windows")]
    {
        use std::{cell::RefCell, collections::HashMap};
        thread_local! { static PREVIOUS: RefCell<HashMap<String, RawDisk>> = RefCell::new(HashMap::new()); }
        let connection = wmi::WMIConnection::new().map_err(|e| e.to_string())?;
        let rows: Vec<RawDisk> = connection.raw_query("SELECT Name, AvgDiskSecPerRead, AvgDiskSecPerRead_Base, AvgDiskSecPerWrite, AvgDiskSecPerWrite_Base, AvgDiskQueueLength, PercentDiskTime, DiskReadsPersec, DiskWritesPersec, Timestamp_PerfTime, Frequency_PerfTime, Timestamp_Sys100NS FROM Win32_PerfRawData_PerfDisk_PhysicalDisk").map_err(|e| e.to_string())?;
        PREVIOUS.with(|previous| {
            let mut previous = previous.borrow_mut();
            let mut current = HashMap::new();
            let mut output = Vec::new();
            for row in rows.into_iter().filter(|row| !row.name.eq_ignore_ascii_case("_Total")) {
                output.push(performance(&row, previous.get(&row.name)));
                current.insert(row.name.clone(), row);
            }
            *previous = current;
            Ok(output)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_latency_and_reset_are_not_manufactured() {
        let previous = RawDisk {
            name: "0 C:".into(),
            avg_disk_sec_per_read: Some(1000),
            read_base: Some(10),
            frequency: Some(10_000_000),
            timestamp: Some(10_000_000),
            ..Default::default()
        };
        let current = RawDisk {
            avg_disk_sec_per_read: Some(6000),
            read_base: Some(12),
            timestamp: Some(20_000_000),
            ..previous.clone()
        };
        assert_eq!(performance(&current, Some(&previous)).read_latency_ms, Some(0.25));
        assert_eq!(performance(&previous, Some(&current)).read_latency_ms, None);
        assert_eq!(performance(&current, None).read_latency_ms, None);
    }
}
