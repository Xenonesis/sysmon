use super::*;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::BTreeSet;

pub(super) fn query_window(conn: &Connection, query: TimelineQuery) -> Result<TimelineWindow, String> {
    let query = query.validated();
    let mut metrics_statement = conn
        .prepare_cached(
            "SELECT timestamp_ms, cpu_pct, memory_pct, gpu_pct, cpu_temp_c, gpu_temp_c,
                    disk_read_bps, disk_write_bps, network_down_bps, network_up_bps, paused
             FROM metric_samples WHERE timestamp_ms BETWEEN ?1 AND ?2 ORDER BY timestamp_ms",
        )
        .map_err(|error| format!("Could not prepare timeline metric query: {error}"))?;
    let metrics = metrics_statement
        .query_map(params![query.start_ms, query.end_ms], |row| {
            Ok(TimelineMetricSample {
                timestamp_ms: row.get(0)?,
                cpu_pct: row.get(1)?,
                memory_pct: row.get(2)?,
                gpu_pct: row.get(3)?,
                cpu_temp_c: row.get(4)?,
                gpu_temp_c: row.get(5)?,
                disk_read_bps: row.get(6)?,
                disk_write_bps: row.get(7)?,
                network_down_bps: row.get(8)?,
                network_up_bps: row.get(9)?,
                paused: row.get(10)?,
            })
        })
        .map_err(|error| format!("Could not query timeline metrics: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode timeline metrics: {error}"))?;

    let mut event_statement = conn
        .prepare_cached(
            "SELECT id, timestamp_ms, kind, source, severity, summary, evidence
             FROM timeline_events WHERE timestamp_ms BETWEEN ?1 AND ?2
             ORDER BY timestamp_ms DESC LIMIT 500",
        )
        .map_err(|error| format!("Could not prepare timeline event query: {error}"))?;
    let events = event_statement
        .query_map(params![query.start_ms, query.end_ms], |row| {
            let kind: String = row.get(2)?;
            Ok(TimelineEvent {
                id: row.get(0)?,
                timestamp_ms: row.get(1)?,
                kind: TimelineEventKind::from_str(&kind),
                source: row.get(3)?,
                severity: row.get(4)?,
                summary: row.get(5)?,
                evidence: row.get(6)?,
            })
        })
        .map_err(|error| format!("Could not query timeline events: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not decode timeline events: {error}"))?;

    // A seven-day window can contain millions of process rows. The UI only needs
    // contributor evidence at the latest sample and around visible events, so
    // fetch those snapshots rather than transferring the full process history.
    let mut requested_timestamps = BTreeSet::from([query.end_ms]);
    requested_timestamps.extend(events.iter().map(|event| event.timestamp_ms));
    let mut sample_timestamps = BTreeSet::new();
    let mut nearest_statement = conn
        .prepare_cached(
            "SELECT timestamp_ms FROM process_samples
             WHERE timestamp_ms BETWEEN ?1 AND ?2
             ORDER BY ABS(timestamp_ms - ?3) LIMIT 1",
        )
        .map_err(|error| format!("Could not prepare nearest process query: {error}"))?;
    for timestamp_ms in requested_timestamps {
        let nearest = nearest_statement
            .query_row(
                params![
                    timestamp_ms.saturating_sub(10_000),
                    timestamp_ms.saturating_add(10_000),
                    timestamp_ms
                ],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| format!("Could not locate process evidence: {error}"))?;
        if let Some(nearest) = nearest {
            sample_timestamps.insert(nearest);
        }
    }

    let mut processes = Vec::new();
    let mut process_statement = conn
        .prepare_cached(
            "SELECT timestamp_ms, pid, start_time, name, cpu_pct, memory_bytes, disk_read_bytes, disk_write_bytes
             FROM process_samples WHERE timestamp_ms = ?1 ORDER BY cpu_pct DESC",
        )
        .map_err(|error| format!("Could not prepare timeline process query: {error}"))?;
    for timestamp_ms in sample_timestamps {
        let rows = process_statement
            .query_map([timestamp_ms], |row| {
                Ok(TimelineProcessSample {
                    timestamp_ms: row.get(0)?,
                    pid: row.get(1)?,
                    start_time: from_sql_i64(row.get(2)?),
                    name: row.get(3)?,
                    cpu_pct: row.get(4)?,
                    memory_bytes: from_sql_i64(row.get(5)?),
                    disk_read_bytes: from_sql_i64(row.get(6)?),
                    disk_write_bytes: from_sql_i64(row.get(7)?),
                })
            })
            .map_err(|error| format!("Could not query timeline processes: {error}"))?;
        processes.extend(
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Could not decode timeline processes: {error}"))?,
        );
    }

    Ok(TimelineWindow {
        query,
        metrics,
        processes,
        events,
    })
}

pub(crate) fn analyze_window(window: &TimelineWindow, timestamp_ms: i64) -> IncidentAnalysis {
    let Some(peak) = window
        .metrics
        .iter()
        .min_by_key(|sample| sample.timestamp_ms.abs_diff(timestamp_ms))
    else {
        return IncidentAnalysis {
            timestamp_ms,
            title: "Insufficient data".into(),
            summary: "No metric samples were recorded near the selected time.".into(),
            confidence: "none".into(),
            evidence: vec!["Enable timeline recording and reproduce the incident.".into()],
            contributors: Vec::new(),
        };
    };
    let baseline_start = peak.timestamp_ms.saturating_sub(5 * 60_000);
    let baseline: Vec<_> = window
        .metrics
        .iter()
        .filter(|sample| sample.timestamp_ms >= baseline_start && sample.timestamp_ms < peak.timestamp_ms)
        .collect();
    if baseline.len() < 3 {
        return IncidentAnalysis {
            timestamp_ms: peak.timestamp_ms,
            title: "Insufficient baseline".into(),
            summary: "At least three earlier samples are required before SysMon can compare this incident.".into(),
            confidence: "low".into(),
            evidence: vec![format!("Only {} baseline samples were available.", baseline.len())],
            contributors: contributors_near(window, peak.timestamp_ms),
        };
    }

    let baseline: Vec<_> = baseline
        .iter()
        .map(|sample| crate::diagnostics::SignalSample {
            cpu_pct: sample.cpu_pct,
            memory_pct: sample.memory_pct,
            disk_bps: sample.disk_read_bps + sample.disk_write_bps,
            network_bps: sample.network_down_bps + sample.network_up_bps,
        })
        .collect();
    let incident = crate::diagnostics::SignalSample {
        cpu_pct: peak.cpu_pct,
        memory_pct: peak.memory_pct,
        disk_bps: peak.disk_read_bps + peak.disk_write_bps,
        network_bps: peak.network_down_bps + peak.network_up_bps,
    };
    let Some(comparison) = crate::diagnostics::compare_to_baseline(&baseline, incident) else {
        return IncidentAnalysis {
            timestamp_ms: peak.timestamp_ms,
            title: "Invalid baseline".into(),
            summary: "Recorded samples could not be compared safely.".into(),
            confidence: "none".into(),
            evidence: vec!["The baseline contained invalid or non-finite metric values.".into()],
            contributors: contributors_near(window, peak.timestamp_ms),
        };
    };
    IncidentAnalysis {
        timestamp_ms: peak.timestamp_ms,
        title: format!("{} change near selected time", comparison.primary_signal),
        summary: comparison.summary,
        confidence: comparison.confidence.into(),
        evidence: comparison.evidence,
        contributors: contributors_near(window, peak.timestamp_ms),
    }
}

fn contributors_near(window: &TimelineWindow, timestamp_ms: i64) -> Vec<IncidentContributor> {
    let nearest = window
        .processes
        .iter()
        .min_by_key(|process| process.timestamp_ms.abs_diff(timestamp_ms))
        .map(|process| process.timestamp_ms);
    let Some(nearest) = nearest else {
        return Vec::new();
    };
    let mut contributors: Vec<_> = window
        .processes
        .iter()
        .filter(|process| process.timestamp_ms == nearest)
        .map(|process| IncidentContributor {
            name: process.name.clone(),
            pid: process.pid,
            start_time: process.start_time,
            cpu_pct: process.cpu_pct,
            memory_bytes: process.memory_bytes,
            disk_bytes: process.disk_read_bytes.saturating_add(process.disk_write_bytes),
        })
        .collect();
    contributors.sort_by(|a, b| {
        let a_score = a.cpu_pct + a.memory_bytes as f64 / 100_000_000.0 + a.disk_bytes as f64 / 1_000_000.0;
        let b_score = b.cpu_pct + b.memory_bytes as f64 / 100_000_000.0 + b.disk_bytes as f64 / 1_000_000.0;
        b_score.total_cmp(&a_score)
    });
    contributors.truncate(5);
    contributors
}
