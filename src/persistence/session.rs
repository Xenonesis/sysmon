use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::monitoring::SystemSnapshot;

#[derive(Default)]
pub(crate) struct SessionRecorder {
    writer: Option<BufWriter<File>>,
    path: Option<PathBuf>,
    last_sample: Option<SystemTime>,
    sample_count: u64,
}

impl SessionRecorder {
    pub(crate) fn start(&mut self) -> Result<PathBuf, std::io::Error> {
        let root = crate::app_paths::sessions_dir()
            .ok_or_else(|| std::io::Error::other("application data directory unavailable"))?;
        fs::create_dir_all(&root)?;
        let name = format!(
            "session-{}-{}.jsonl",
            chrono::Utc::now().format("%Y%m%d-%H%M%S%.3f"),
            std::process::id()
        );
        let path = root.join(name);
        self.writer = Some(BufWriter::new(File::create(&path)?));
        self.path = Some(path.clone());
        self.last_sample = None;
        self.sample_count = 0;
        Ok(path)
    }

    pub(crate) fn record(&mut self, snapshot: &SystemSnapshot) -> Result<(), std::io::Error> {
        let Some(writer) = self.writer.as_mut() else {
            return Ok(());
        };
        if self
            .last_sample
            .and_then(|last| snapshot.sampled_at.duration_since(last).ok())
            .is_some_and(|elapsed| elapsed < Duration::from_secs(1))
        {
            return Ok(());
        }
        serde_json::to_writer(&mut *writer, snapshot).map_err(std::io::Error::other)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        self.last_sample = Some(snapshot.sampled_at);
        self.sample_count += 1;
        Ok(())
    }

    pub(crate) fn stop(&mut self) -> Result<Option<PathBuf>, std::io::Error> {
        if let Some(mut writer) = self.writer.take() {
            writer.flush()?;
            writer.get_ref().sync_all()?;
        }
        Ok(self.path.clone())
    }

    pub(crate) fn toggle(&mut self) -> Result<Option<PathBuf>, std::io::Error> {
        if self.is_recording() {
            self.stop()
        } else {
            self.start().map(Some)
        }
    }

    pub(crate) fn is_recording(&self) -> bool {
        self.writer.is_some()
    }

    pub(crate) fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub(crate) fn sample_count(&self) -> u64 {
        self.sample_count
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct SessionSummary {
    pub sample_count: usize,
    pub duration_secs: u64,
    pub avg_cpu: f32,
    pub max_cpu: f32,
    pub avg_memory_pct: f32,
    pub max_memory_pct: f32,
    pub max_gpu_util: f32,
    pub total_net_recv_mb: f64,
    pub total_net_sent_mb: f64,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct SessionDiagnosis {
    pub sample_count: usize,
    pub baseline_samples: usize,
    pub incident_sample: usize,
    pub primary_signal: String,
    pub summary: String,
    pub recommendation: String,
    pub confidence: String,
    pub evidence: Vec<String>,
    pub contributor: Option<String>,
}

/// Export a recorded .jsonl session file to standard multi-column CSV.
pub fn export_session_to_csv(
    jsonl_path: &std::path::Path,
    output_csv: &std::path::Path,
) -> Result<usize, std::io::Error> {
    let file = File::open(jsonl_path)?;
    let reader = std::io::BufReader::new(file);
    use std::io::BufRead;

    let out_file = File::create(output_csv)?;
    let mut writer = BufWriter::new(out_file);

    // Header
    writer.write_all(
        b"Timestamp_UTC,CPU_Usage_Pct,Memory_Used_MB,Memory_Total_MB,Memory_Pct,GPU_Util_Pct,Net_Recv_KBps,Net_Sent_KBps,Disk_Read_KBps,Disk_Write_KBps\n",
    )?;

    let mut count = 0;
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(snap) = serde_json::from_str::<SystemSnapshot>(&line) {
            let dt = chrono::DateTime::<chrono::Utc>::from(snap.sampled_at);
            let time_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();
            let mem_used_mb = (snap.memory_used as f64) / (1024.0 * 1024.0);
            let mem_total_mb = (snap.memory_total as f64) / (1024.0 * 1024.0);
            let gpu_util = snap.gpus.first().map_or(0.0, |g| g.utilization);
            let net_recv = snap
                .networks
                .iter()
                .map(|n| n.received_bytes_per_second / 1024.0)
                .sum::<f64>();
            let net_sent = snap
                .networks
                .iter()
                .map(|n| n.transmitted_bytes_per_second / 1024.0)
                .sum::<f64>();
            let disk_read = snap.disks.iter().map(|d| d.read_bytes_per_second / 1024.0).sum::<f64>();
            let disk_write = snap
                .disks
                .iter()
                .map(|d| d.written_bytes_per_second / 1024.0)
                .sum::<f64>();

            let row = format!(
                "{},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1}\n",
                time_str,
                snap.cpu_usage,
                mem_used_mb,
                mem_total_mb,
                snap.memory_percentage,
                gpu_util,
                net_recv,
                net_sent,
                disk_read,
                disk_write,
            );
            writer.write_all(row.as_bytes())?;
            count += 1;
        }
    }

    writer.flush()?;
    Ok(count)
}

/// Calculate aggregate statistics over a recorded JSONL telemetry session.
pub fn calculate_session_summary(jsonl_path: &std::path::Path) -> Result<SessionSummary, std::io::Error> {
    let file = File::open(jsonl_path)?;
    let reader = std::io::BufReader::new(file);
    use std::io::BufRead;

    let mut count = 0;
    let mut sum_cpu = 0.0;
    let mut max_cpu = 0.0f32;
    let mut sum_mem = 0.0;
    let mut max_mem = 0.0f32;
    let mut max_gpu = 0.0f32;
    let mut total_recv_bytes = 0.0;
    let mut total_sent_bytes = 0.0;
    let mut first_time: Option<SystemTime> = None;
    let mut last_time: Option<SystemTime> = None;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(snap) = serde_json::from_str::<SystemSnapshot>(&line) {
            if first_time.is_none() {
                first_time = Some(snap.sampled_at);
            }
            last_time = Some(snap.sampled_at);

            sum_cpu += snap.cpu_usage;
            max_cpu = max_cpu.max(snap.cpu_usage);
            sum_mem += snap.memory_percentage;
            max_mem = max_mem.max(snap.memory_percentage);
            if let Some(gpu) = snap.gpus.first() {
                max_gpu = max_gpu.max(gpu.utilization);
            }
            let net_r: f64 = snap.networks.iter().map(|n| n.received_bytes_per_second).sum();
            let net_s: f64 = snap.networks.iter().map(|n| n.transmitted_bytes_per_second).sum();
            total_recv_bytes += net_r;
            total_sent_bytes += net_s;
            count += 1;
        }
    }

    let duration_secs = match (first_time, last_time) {
        (Some(start), Some(end)) => end.duration_since(start).unwrap_or_default().as_secs(),
        _ => 0,
    };

    Ok(SessionSummary {
        sample_count: count,
        duration_secs,
        avg_cpu: if count > 0 { sum_cpu / count as f32 } else { 0.0 },
        max_cpu,
        avg_memory_pct: if count > 0 { sum_mem / count as f32 } else { 0.0 },
        max_memory_pct: max_mem,
        max_gpu_util: max_gpu,
        total_net_recv_mb: total_recv_bytes / (1024.0 * 1024.0),
        total_net_sent_mb: total_sent_bytes / (1024.0 * 1024.0),
    })
}

/// Compare the first stable portion of a recorded session with the strongest
/// later deviation. The workflow intentionally requires a baseline so SysMon
/// does not guess from one isolated sample.
pub fn analyze_session_against_baseline(jsonl_path: &std::path::Path) -> Result<SessionDiagnosis, std::io::Error> {
    let snapshots = read_session_snapshots(jsonl_path)?;
    if snapshots.len() < 6 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "at least 6 valid samples are required (3 baseline and 3 incident)",
        ));
    }

    let baseline_count = (snapshots.len() / 3).clamp(3, 15);
    let baseline: Vec<_> = snapshots[..baseline_count].iter().map(signal_sample).collect();
    let (incident_index, comparison) = snapshots[baseline_count..]
        .iter()
        .enumerate()
        .filter_map(|(index, snapshot)| {
            crate::diagnostics::compare_to_baseline(&baseline, signal_sample(snapshot))
                .map(|comparison| (baseline_count + index, comparison))
        })
        .max_by(|(_, left), (_, right)| left.primary_score.total_cmp(&right.primary_score))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "session metrics are not comparable"))?;

    let incident = &snapshots[incident_index];
    let contributor = incident
        .processes
        .iter()
        .max_by(|left, right| process_score(left).total_cmp(&process_score(right)))
        .map(|process| {
            format!(
                "{} (PID {}) — CPU {:.1}%, RAM {:.1} MB",
                process.name,
                process.pid,
                process.cpu_usage,
                process.memory as f64 / 1_048_576.0
            )
        });
    let recommendation = match comparison.primary_signal {
        "CPU" => "Inspect the leading process and its child processes before changing priority or power settings.",
        "Memory" => {
            "Check the leading process for sustained growth; working-set trimming is only a temporary diagnostic step."
        }
        "Disk I/O" => "Inspect per-process disk throughput and storage health before stopping a service or process.",
        "Network" => "Inspect active sockets and the owning process before blocking or terminating anything.",
        _ => "Review the ranked evidence before taking a system-changing action.",
    };

    Ok(SessionDiagnosis {
        sample_count: snapshots.len(),
        baseline_samples: baseline_count,
        incident_sample: incident_index + 1,
        primary_signal: comparison.primary_signal.into(),
        summary: comparison.summary,
        recommendation: recommendation.into(),
        confidence: comparison.confidence.into(),
        evidence: comparison.evidence,
        contributor,
    })
}

fn read_session_snapshots(jsonl_path: &std::path::Path) -> Result<Vec<SystemSnapshot>, std::io::Error> {
    use std::io::BufRead;

    let reader = std::io::BufReader::new(File::open(jsonl_path)?);
    Ok(reader
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<SystemSnapshot>(&line).ok())
        .collect())
}

fn signal_sample(snapshot: &SystemSnapshot) -> crate::diagnostics::SignalSample {
    crate::diagnostics::SignalSample {
        cpu_pct: snapshot.cpu_usage as f64,
        memory_pct: snapshot.memory_percentage as f64,
        disk_bps: snapshot
            .disks
            .iter()
            .map(|disk| disk.read_bytes_per_second + disk.written_bytes_per_second)
            .sum(),
        network_bps: snapshot
            .networks
            .iter()
            .map(|network| network.received_bytes_per_second + network.transmitted_bytes_per_second)
            .sum(),
    }
}

fn process_score(process: &crate::monitoring::snapshot::ProcessSnapshot) -> f64 {
    process.cpu_usage as f64
        + process.memory as f64 / 100_000_000.0
        + process.disk_read_bytes.saturating_add(process.disk_written_bytes) as f64 / 1_000_000.0
}

/// List all recorded session paths sorted by newest first.
pub fn list_recorded_sessions() -> Vec<PathBuf> {
    let Some(root) = crate::app_paths::sessions_dir() else {
        return Vec::new();
    };

    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut sessions: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "jsonl"))
        .collect();

    sessions.sort_by(|a, b| b.cmp(a));
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_recorder_toggle() {
        let root = std::env::temp_dir().join(format!("sysmon-recorder-test-{}", std::process::id()));
        crate::app_paths::with_test_data_local_dir(root.clone(), || {
            let mut recorder = SessionRecorder::default();
            assert!(!recorder.is_recording());
            assert_eq!(recorder.sample_count(), 0);

            let start_res = recorder.toggle();
            assert!(start_res.is_ok());
            assert!(recorder.is_recording());

            let stop_res = recorder.toggle();
            assert!(stop_res.is_ok());
            assert!(!recorder.is_recording());
        });
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn session_export_to_csv_and_summary() {
        let temp_dir = std::env::temp_dir().join(format!("sysmon_test_session_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let jsonl_file = temp_dir.join("test_session.jsonl");
        let csv_file = temp_dir.join("test_session.csv");

        let snap = SystemSnapshot {
            cpu_usage: 42.5,
            memory_percentage: 60.0,
            memory_used: 8 * 1024 * 1024 * 1024,
            memory_total: 16 * 1024 * 1024 * 1024,
            ..Default::default()
        };

        let line = serde_json::to_string(&snap).unwrap();
        fs::write(&jsonl_file, format!("{line}\n")).unwrap();

        let rows = export_session_to_csv(&jsonl_file, &csv_file).unwrap();
        assert_eq!(rows, 1);

        let csv_content = fs::read_to_string(&csv_file).unwrap();
        assert!(csv_content.contains("Timestamp_UTC"));
        assert!(csv_content.contains("42.5"));

        let summary = calculate_session_summary(&jsonl_file).unwrap();
        assert_eq!(summary.sample_count, 1);
        assert_eq!(summary.max_cpu, 42.5);
        assert_eq!(summary.avg_memory_pct, 60.0);

        let _ = fs::remove_file(jsonl_file);
        let _ = fs::remove_file(csv_file);
        let _ = fs::remove_dir(temp_dir);
    }

    #[test]
    fn session_diagnosis_compares_baseline_and_names_contributor() {
        let temp_dir = std::env::temp_dir().join(format!("sysmon_diagnosis_test_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let jsonl_file = temp_dir.join("diagnosis.jsonl");
        let mut lines = String::new();
        for index in 0..9 {
            let mut snapshot = SystemSnapshot {
                cpu_usage: if index < 3 { 10.0 } else { 20.0 },
                memory_percentage: 40.0,
                ..Default::default()
            };
            if index == 7 {
                snapshot.cpu_usage = 95.0;
                snapshot.processes.push(crate::monitoring::snapshot::ProcessSnapshot {
                    pid: 42,
                    name: "encoder.exe".into(),
                    cpu_usage: 80.0,
                    memory: 512 * 1_048_576,
                    ..Default::default()
                });
            }
            lines.push_str(&serde_json::to_string(&snapshot).unwrap());
            lines.push('\n');
        }
        fs::write(&jsonl_file, lines).unwrap();

        let diagnosis = analyze_session_against_baseline(&jsonl_file).unwrap();
        assert_eq!(diagnosis.primary_signal, "CPU");
        assert!(diagnosis
            .contributor
            .as_deref()
            .is_some_and(|value| value.contains("encoder.exe")));
        assert_eq!(diagnosis.baseline_samples, 3);

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn session_diagnosis_refuses_to_guess_without_baseline() {
        let temp_dir = std::env::temp_dir().join(format!("sysmon_short_diagnosis_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        let jsonl_file = temp_dir.join("short.jsonl");
        fs::write(
            &jsonl_file,
            format!("{}\n", serde_json::to_string(&SystemSnapshot::default()).unwrap()),
        )
        .unwrap();

        let error = analyze_session_against_baseline(&jsonl_file).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
