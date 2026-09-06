use super::*;
use crate::monitoring::SystemSnapshot;
use rusqlite::{Connection, params};
use std::collections::{BTreeMap, HashMap};

pub(super) fn write_snapshot(conn: &mut Connection, snapshot: &SystemSnapshot) -> Result<(), String> {
    let timestamp_ms = system_time_ms(snapshot.sampled_at);
    let metric = metric_from_snapshot(snapshot);
    let processes = select_process_union(snapshot, timestamp_ms, 10);
    let transaction = conn
        .transaction()
        .map_err(|error| format!("Could not begin timeline write: {error}"))?;
    transaction
        .execute(
            "INSERT OR REPLACE INTO metric_samples
             (timestamp_ms, cpu_pct, memory_pct, gpu_pct, cpu_temp_c, gpu_temp_c,
              disk_read_bps, disk_write_bps, network_down_bps, network_up_bps, paused)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                timestamp_ms,
                metric.cpu_pct,
                metric.memory_pct,
                metric.gpu_pct,
                metric.cpu_temp_c,
                metric.gpu_temp_c,
                metric.disk_read_bps,
                metric.disk_write_bps,
                metric.network_down_bps,
                metric.network_up_bps,
                metric.paused
            ],
        )
        .map_err(|error| format!("Could not write timeline metrics: {error}"))?;
    transaction
        .execute("DELETE FROM process_samples WHERE timestamp_ms = ?1", [timestamp_ms])
        .map_err(|error| format!("Could not replace timeline process samples: {error}"))?;
    {
        let mut statement = transaction
            .prepare_cached(
                "INSERT INTO process_samples
                 (timestamp_ms, pid, start_time, name, cpu_pct, memory_bytes, disk_read_bytes, disk_write_bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .map_err(|error| format!("Could not prepare timeline process write: {error}"))?;
        for process in processes {
            statement
                .execute(params![
                    process.timestamp_ms,
                    process.pid,
                    to_sql_i64(process.start_time),
                    process.name,
                    process.cpu_pct,
                    to_sql_i64(process.memory_bytes),
                    to_sql_i64(process.disk_read_bytes),
                    to_sql_i64(process.disk_write_bytes)
                ])
                .map_err(|error| format!("Could not write timeline process sample: {error}"))?;
        }
    }
    transaction
        .commit()
        .map_err(|error| format!("Could not commit timeline sample: {error}"))
}

fn metric_from_snapshot(snapshot: &SystemSnapshot) -> TimelineMetricSample {
    TimelineMetricSample {
        timestamp_ms: system_time_ms(snapshot.sampled_at),
        cpu_pct: snapshot.cpu_usage as f64,
        memory_pct: snapshot.memory_percentage as f64,
        gpu_pct: snapshot.gpus.iter().map(|gpu| gpu.utilization as f64).reduce(f64::max),
        cpu_temp_c: snapshot.cpu_temperature.map(f64::from),
        gpu_temp_c: snapshot
            .gpus
            .iter()
            .filter_map(|gpu| gpu.temperature.map(f64::from))
            .reduce(f64::max),
        disk_read_bps: snapshot.disks.first().map_or(0.0, |disk| disk.read_bytes_per_second),
        disk_write_bps: snapshot.disks.first().map_or(0.0, |disk| disk.written_bytes_per_second),
        network_down_bps: snapshot
            .networks
            .iter()
            .map(|network| network.received_bytes_per_second)
            .sum(),
        network_up_bps: snapshot
            .networks
            .iter()
            .map(|network| network.transmitted_bytes_per_second)
            .sum(),
        paused: snapshot.paused,
    }
}

pub(super) fn select_process_union(
    snapshot: &SystemSnapshot,
    timestamp_ms: i64,
    limit: usize,
) -> Vec<TimelineProcessSample> {
    let mut selected: BTreeMap<(u32, u64), &crate::monitoring::snapshot::ProcessSnapshot> = BTreeMap::new();
    let mut by_cpu: Vec<_> = snapshot.processes.iter().collect();
    let mut by_memory = by_cpu.clone();
    let mut by_disk = by_cpu.clone();
    by_cpu.sort_by(|a, b| b.cpu_usage.total_cmp(&a.cpu_usage));
    by_memory.sort_by_key(|process| std::cmp::Reverse(process.memory));
    by_disk
        .sort_by_key(|process| std::cmp::Reverse(process.disk_read_bytes.saturating_add(process.disk_written_bytes)));
    for process in by_cpu
        .into_iter()
        .take(limit)
        .chain(by_memory.into_iter().take(limit))
        .chain(by_disk.into_iter().take(limit))
    {
        selected.insert((process.pid, process.start_time), process);
    }
    selected
        .into_values()
        .map(|process| TimelineProcessSample {
            timestamp_ms,
            pid: process.pid,
            start_time: process.start_time,
            name: sanitize_text(process.name.clone(), 260),
            cpu_pct: process.cpu_usage as f64,
            memory_bytes: process.memory,
            disk_read_bytes: process.disk_read_bytes,
            disk_write_bytes: process.disk_written_bytes,
        })
        .collect()
}

pub(super) fn record_derived_events(
    conn: &Connection,
    snapshot: &SystemSnapshot,
    previous_providers: &mut HashMap<String, bool>,
    previous_paused: &mut Option<bool>,
    previous_power: &mut Option<String>,
) -> Result<(), String> {
    for (name, provider) in &snapshot.provider_status {
        if let Some(previous) = previous_providers.insert(name.clone(), provider.available)
            && previous != provider.available
        {
            insert_event(
                conn,
                &TimelineEvent {
                    id: None,
                    timestamp_ms: system_time_ms(snapshot.sampled_at),
                    kind: if provider.available {
                        TimelineEventKind::ProviderRecovered
                    } else {
                        TimelineEventKind::ProviderUnavailable
                    },
                    source: sanitize_text(name.clone(), 128),
                    severity: if provider.available { "info" } else { "warning" }.into(),
                    summary: if provider.available {
                        format!("{name} telemetry recovered")
                    } else {
                        format!("{name} telemetry became unavailable")
                    },
                    evidence: provider
                        .error
                        .clone()
                        .unwrap_or_else(|| "Provider availability changed".into()),
                },
            )?;
        }
    }

    if let Some(previous) = previous_paused.replace(snapshot.paused)
        && previous != snapshot.paused
    {
        insert_event(
            conn,
            &TimelineEvent {
                id: None,
                timestamp_ms: system_time_ms(snapshot.sampled_at),
                kind: if snapshot.paused {
                    TimelineEventKind::MonitoringPaused
                } else {
                    TimelineEventKind::MonitoringResumed
                },
                source: "monitoring".into(),
                severity: "info".into(),
                summary: if snapshot.paused {
                    "Monitoring paused".into()
                } else {
                    "Monitoring resumed".into()
                },
                evidence: "User-visible monitoring state changed".into(),
            },
        )?;
    }

    let power = snapshot.battery.as_ref().map(|battery| {
        format!(
            "{}:{}",
            battery.status,
            battery.discharge_state.as_deref().unwrap_or("unknown")
        )
    });
    if let (Some(previous), Some(current)) = (previous_power.as_ref(), power.as_ref())
        && previous != current
    {
        insert_event(
            conn,
            &TimelineEvent {
                id: None,
                timestamp_ms: system_time_ms(snapshot.sampled_at),
                kind: TimelineEventKind::PowerChanged,
                source: "power".into(),
                severity: "info".into(),
                summary: "Power state changed".into(),
                evidence: format!("Battery status changed from {previous} to {current}"),
            },
        )?;
    }
    if power.is_some() {
        *previous_power = power;
    }
    Ok(())
}

pub(super) fn insert_event(conn: &Connection, event: &TimelineEvent) -> Result<(), String> {
    conn.execute(
        "INSERT INTO timeline_events (timestamp_ms, kind, source, severity, summary, evidence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            event.timestamp_ms,
            event.kind.as_str(),
            sanitize_text(event.source.clone(), 128),
            sanitize_text(event.severity.clone(), 32),
            sanitize_text(event.summary.clone(), 512),
            sanitize_text(event.evidence.clone(), 2_048)
        ],
    )
    .map(|_| ())
    .map_err(|error| format!("Could not write timeline event: {error}"))
}
