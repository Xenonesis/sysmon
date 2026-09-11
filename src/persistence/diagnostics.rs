use std::fs;
use std::path::{Path, PathBuf};

use crate::{AppSettings, app::models::AppTheme, monitoring::SystemSnapshot};
use serde::Serialize;

#[derive(Serialize)]
struct Diagnostics {
    app_version: &'static str,
    snapshot: serde_json::Value,
    settings: RedactedSettings,
}

#[derive(Serialize)]
struct RedactedSettings {
    refresh_interval: u64,
    theme: AppTheme,
    show_notifications: bool,
}

pub(crate) fn export(
    destination: &Path,
    snapshot: &SystemSnapshot,
    settings: &AppSettings,
) -> Result<PathBuf, std::io::Error> {
    fs::create_dir_all(destination)?;
    let output = destination.join("sysmon-diagnostics.json");
    let document = Diagnostics {
        app_version: env!("CARGO_PKG_VERSION"),
        snapshot: redacted_snapshot(snapshot),
        settings: RedactedSettings {
            refresh_interval: settings.refresh_interval,
            theme: settings.theme,
            show_notifications: settings.show_notifications,
        },
    };
    let bytes = serde_json::to_vec_pretty(&document).map_err(std::io::Error::other)?;
    fs::write(&output, bytes)?;
    Ok(output)
}

fn redacted_snapshot(snapshot: &SystemSnapshot) -> serde_json::Value {
    // An explicit allowlist prevents newly added snapshot strings/errors leaking into exports.
    let status: serde_json::Map<String, serde_json::Value> =
        ["cpu.global_usage", "memory.used", "disk", "network", "processes"]
            .into_iter()
            .filter_map(|key| {
                snapshot.metric_status.get(key).map(|observation| {
                    (
                        key.into(),
                        serde_json::json!({"state": observation.state, "observed_at": observation.observed_at}),
                    )
                })
            })
            .collect();
    serde_json::json!({
        "sampled_at": snapshot.sampled_at,
        "paused": snapshot.paused,
        "metric_status": status,
        "cpu_usage": snapshot.cpu_usage,
        "cpu_cores": snapshot.cpu_cores,
        "cpu_temperature": snapshot.cpu_temperature,
        "memory_total": snapshot.memory_total,
        "memory_used": snapshot.memory_used,
        "memory_percentage": snapshot.memory_percentage,
        "disk_read_bytes_per_second": snapshot.disk_read_bytes_per_second,
        "disk_written_bytes_per_second": snapshot.disk_written_bytes_per_second,
        "gpus": snapshot.gpus.iter().map(|gpu| serde_json::json!({
            "utilization": gpu.utilization, "memory_used": gpu.memory_used,
            "memory_total": gpu.memory_total, "temperature": gpu.temperature,
            "clock_mhz": gpu.clock_mhz, "power_watts": gpu.power_watts, "fan_percent": gpu.fan_percent
        })).collect::<Vec<_>>(),
        "disks": snapshot.disks.iter().map(|disk| serde_json::json!({
            "total_space": disk.total_space, "available_space": disk.available_space,
            "usage_percentage": disk.usage_percentage,
            "read_bytes_per_second": disk.read_bytes_per_second,
            "written_bytes_per_second": disk.written_bytes_per_second
        })).collect::<Vec<_>>(),
        "networks": snapshot.networks.iter().map(|network| serde_json::json!({
            "received": network.received, "transmitted": network.transmitted,
            "received_bytes_per_second": network.received_bytes_per_second,
            "transmitted_bytes_per_second": network.transmitted_bytes_per_second
        })).collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_allowlist_omits_identifying_snapshot_strings_and_errors() {
        let mut snapshot = SystemSnapshot::default();
        snapshot.system.hostname = "private-marker".into();
        snapshot.metric_status.insert(
            "disk".into(),
            crate::monitoring::snapshot::MetricObservation {
                error: Some("C:\\Users\\private-marker\\failure".into()),
                ..Default::default()
            },
        );
        snapshot.networks.push(crate::monitoring::snapshot::NetworkSnapshot {
            interface: "private-marker".into(),
            received_bytes_per_second: 2048.0,
            ..Default::default()
        });
        let exported = redacted_snapshot(&snapshot);
        assert!(!exported.to_string().contains("private-marker"));
        assert_eq!(exported["networks"][0]["received_bytes_per_second"], 2048.0);
    }
}
