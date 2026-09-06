use super::query::analyze_window;
use super::*;
use chrono::Utc;
use std::path::{Path, PathBuf};

pub(crate) fn export_window(window: &TimelineWindow, destination: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(destination).map_err(|error| format!("Could not create export directory: {error}"))?;
    let directory = destination.join(format!("sysmon-incident-{}", Utc::now().format("%Y%m%d-%H%M%S")));
    std::fs::create_dir(&directory).map_err(|error| format!("Could not create incident directory: {error}"))?;

    let selected = window
        .events
        .first()
        .map_or(window.query.end_ms, |event| event.timestamp_ms);
    let analysis = analyze_window(window, selected);
    let summary =
        serde_json::to_vec_pretty(&analysis).map_err(|error| format!("Could not encode incident summary: {error}"))?;
    std::fs::write(directory.join("summary.json"), summary)
        .map_err(|error| format!("Could not write incident summary: {error}"))?;
    let events = serde_json::to_vec_pretty(&window.events)
        .map_err(|error| format!("Could not encode incident events: {error}"))?;
    std::fs::write(directory.join("events.json"), events)
        .map_err(|error| format!("Could not write incident events: {error}"))?;

    let mut metrics = csv::Writer::from_path(directory.join("metrics.csv"))
        .map_err(|error| format!("Could not create metric export: {error}"))?;
    metrics
        .write_record([
            "timestamp_utc",
            "cpu_pct",
            "memory_pct",
            "gpu_pct",
            "cpu_temp_c",
            "gpu_temp_c",
            "disk_read_bps",
            "disk_write_bps",
            "network_down_bps",
            "network_up_bps",
            "paused",
        ])
        .map_err(|error| format!("Could not write metric header: {error}"))?;
    for sample in &window.metrics {
        metrics
            .serialize((
                timestamp_rfc3339(sample.timestamp_ms),
                sample.cpu_pct,
                sample.memory_pct,
                sample.gpu_pct,
                sample.cpu_temp_c,
                sample.gpu_temp_c,
                sample.disk_read_bps,
                sample.disk_write_bps,
                sample.network_down_bps,
                sample.network_up_bps,
                sample.paused,
            ))
            .map_err(|error| format!("Could not write metric export: {error}"))?;
    }
    metrics
        .flush()
        .map_err(|error| format!("Could not finish metric export: {error}"))?;

    let mut processes = csv::Writer::from_path(directory.join("processes.csv"))
        .map_err(|error| format!("Could not create process export: {error}"))?;
    processes
        .write_record([
            "timestamp_utc",
            "pid",
            "start_time",
            "name",
            "cpu_pct",
            "memory_bytes",
            "disk_read_bytes",
            "disk_write_bytes",
        ])
        .map_err(|error| format!("Could not write process header: {error}"))?;
    for process in &window.processes {
        processes
            .serialize((
                timestamp_rfc3339(process.timestamp_ms),
                process.pid,
                process.start_time,
                &process.name,
                process.cpu_pct,
                process.memory_bytes,
                process.disk_read_bytes,
                process.disk_write_bytes,
            ))
            .map_err(|error| format!("Could not write process export: {error}"))?;
    }
    processes
        .flush()
        .map_err(|error| format!("Could not finish process export: {error}"))?;
    Ok(directory)
}

pub(super) fn timestamp_rfc3339(timestamp_ms: i64) -> String {
    chrono::DateTime::<Utc>::from_timestamp_millis(timestamp_ms)
        .unwrap_or(chrono::DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339()
}
