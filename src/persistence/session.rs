use crate::diagnostics::{ComparisonState, SignalSample, compare_to_baseline};
use crate::monitoring::{SystemSnapshot, snapshot::MetricState};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant, SystemTime};

pub const GUIDED_BASELINE_SAMPLES: usize = 15;
const MAX_SESSION_BYTES: u64 = 128 * 1024 * 1024;
const MAX_RECORD_BYTES: u64 = 64 * 1024 * 1024;
const MAX_LINE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SAMPLES: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RateUnits {
    BytesPerSecond,
    UnknownLegacy,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "record")]
enum SessionMetadata {
    Header {
        version: u32,
        rate_units: RateUnits,
        guided_baseline_samples: Option<usize>,
    },
    BaselineEnd {
        samples: usize,
        sampled_at: SystemTime,
    },
}

#[derive(Debug, Clone, Default, serde::Serialize, PartialEq)]
pub struct SessionSummary {
    pub sample_count: usize,
    pub duration_secs: u64,
    pub avg_cpu: f32,
    pub max_cpu: f32,
    pub avg_memory_pct: f32,
    pub max_memory_pct: f32,
    pub max_gpu_util: Option<f32>,
    pub total_net_recv_mb: Option<f64>,
    pub total_net_sent_mb: Option<f64>,
}
#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
pub enum DiagnosisState {
    Change,
    NoMeaningfulChange,
    Insufficient,
    Invalid,
    Stale,
}
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct SessionDiagnosis {
    pub state: DiagnosisState,
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
impl SessionDiagnosis {
    fn unavailable(state: DiagnosisState, count: usize, baseline: usize, reason: &str) -> Self {
        Self {
            state,
            sample_count: count,
            baseline_samples: baseline,
            incident_sample: 0,
            primary_signal: format!("{state:?} evidence"),
            summary: reason.into(),
            recommendation: "Capture a fresh guided session with at least 15 baseline and 3 reproduction samples."
                .into(),
            confidence: "not applicable".into(),
            evidence: Vec::new(),
            contributor: None,
        }
    }
}

#[derive(Default)]
struct SummaryAccumulator {
    summary: SessionSummary,
    first: Option<SystemTime>,
    last: Option<SystemTime>,
    counters: HashMap<String, (u64, u64)>,
    network_observed_at: Option<SystemTime>,
}
impl SummaryAccumulator {
    fn push(&mut self, snapshot: &SystemSnapshot) {
        let summary = &mut self.summary;
        summary.sample_count += 1;
        let n = summary.sample_count as f32;
        summary.avg_cpu += (snapshot.cpu_usage - summary.avg_cpu) / n;
        summary.avg_memory_pct += (snapshot.memory_percentage - summary.avg_memory_pct) / n;
        summary.max_cpu = summary.max_cpu.max(snapshot.cpu_usage);
        summary.max_memory_pct = summary.max_memory_pct.max(snapshot.memory_percentage);
        for value in snapshot.gpus.iter().filter_map(|gpu| gpu.utilization) {
            summary.max_gpu_util = Some(summary.max_gpu_util.map_or(value, |old| old.max(value)));
        }
        self.first.get_or_insert(snapshot.sampled_at);
        self.last = Some(snapshot.sampled_at);
        summary.duration_secs = snapshot
            .sampled_at
            .duration_since(self.first.unwrap())
            .unwrap_or_default()
            .as_secs();
        // Actual cumulative counters, not a sum of rates or an invented first interval.
        if let Some(observed) = snapshot
            .metric_status
            .get("network")
            .filter(|o| o.is_fresh())
            .and_then(|o| o.observed_at)
            && self.network_observed_at != Some(observed)
            && !snapshot.paused
        {
            let mut received = 0u64;
            let mut sent = 0u64;
            for interface in &snapshot.networks {
                if let Some((old_recv, old_sent)) = self.counters.get(&interface.interface) {
                    received = received.saturating_add(interface.received.saturating_sub(*old_recv));
                    sent = sent.saturating_add(interface.transmitted.saturating_sub(*old_sent));
                }
            }
            self.counters = snapshot
                .networks
                .iter()
                .map(|n| (n.interface.clone(), (n.received, n.transmitted)))
                .collect();
            summary.total_net_recv_mb = Some(summary.total_net_recv_mb.unwrap_or(0.0) + received as f64 / 1_048_576.0);
            summary.total_net_sent_mb = Some(summary.total_net_sent_mb.unwrap_or(0.0) + sent as f64 / 1_048_576.0);
            self.network_observed_at = Some(observed);
        }
    }
}

struct SessionData {
    snapshots: Vec<SystemSnapshot>,
    units: RateUnits,
    baseline: Option<usize>,
    guided: bool,
}
fn invalid(reason: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason.into())
}
fn read_session(path: &Path) -> io::Result<SessionData> {
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_SESSION_BYTES {
        return Err(invalid(
            "session exceeds the 128 MiB analysis limit; original file preserved",
        ));
    }
    let mut reader = BufReader::new(file);
    let mut data = SessionData {
        snapshots: Vec::new(),
        units: RateUnits::UnknownLegacy,
        baseline: None,
        guided: false,
    };
    let mut line = String::new();
    let mut line_number = 0;
    let mut header_seen = false;
    loop {
        line.clear();
        let read = reader.by_ref().take(MAX_LINE_BYTES + 1).read_line(&mut line)?;
        if read == 0 {
            break;
        }
        line_number += 1;
        if read as u64 > MAX_LINE_BYTES {
            return Err(invalid("session record exceeds 2 MiB"));
        }
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| invalid(format!("line {line_number}: {e}")))?;
        if value.get("record").is_some() {
            match serde_json::from_value::<SessionMetadata>(value).map_err(invalid_json)? {
                SessionMetadata::Header {
                    version,
                    rate_units,
                    guided_baseline_samples,
                } => {
                    if header_seen
                        || !data.snapshots.is_empty()
                        || version != 2
                        || rate_units != RateUnits::BytesPerSecond
                    {
                        return Err(invalid("unsupported or misplaced session schema header"));
                    }
                    if guided_baseline_samples.is_some_and(|n| n != GUIDED_BASELINE_SAMPLES) {
                        return Err(invalid("unsupported guided baseline target"));
                    }
                    header_seen = true;
                    data.units = rate_units;
                    data.guided = guided_baseline_samples.is_some();
                }
                SessionMetadata::BaselineEnd { samples, sampled_at } => {
                    if !data.guided
                        || data.baseline.is_some()
                        || samples != data.snapshots.len()
                        || samples != GUIDED_BASELINE_SAMPLES
                        || data.snapshots.last().is_none_or(|s| s.sampled_at != sampled_at)
                    {
                        return Err(invalid("baseline boundary does not match the captured samples"));
                    }
                    data.baseline = Some(samples);
                }
            }
        } else {
            if data.snapshots.len() >= MAX_SAMPLES {
                return Err(invalid(
                    "session exceeds 20,000-sample analysis limit; original file preserved",
                ));
            }
            let snapshot: SystemSnapshot = serde_json::from_value(value).map_err(invalid_json)?;
            if data
                .snapshots
                .last()
                .is_some_and(|old| snapshot.sampled_at <= old.sampled_at)
            {
                return Err(invalid("sample timestamps are not strictly increasing"));
            }
            data.snapshots.push(snapshot);
        }
    }
    Ok(data)
}
fn invalid_json(error: serde_json::Error) -> io::Error {
    invalid(error.to_string())
}

fn cadence(snapshots: &[SystemSnapshot]) -> Duration {
    let mut gaps: Vec<_> = snapshots
        .windows(2)
        .filter_map(|pair| pair[1].sampled_at.duration_since(pair[0].sampled_at).ok())
        .filter(|gap| !gap.is_zero())
        .collect();
    gaps.sort_unstable();
    gaps.get(gaps.len() / 2)
        .copied()
        .unwrap_or(Duration::from_secs(1))
        .saturating_mul(2)
        .clamp(Duration::from_secs(2), Duration::from_secs(30))
}
pub(crate) fn fresh_signal(snapshot: &SystemSnapshot, signal: &str, max_age: Duration) -> bool {
    !snapshot.paused
        && snapshot.metric_status.get(signal).is_some_and(|observation| {
            observation.state == MetricState::Available
                && observation
                    .observed_at
                    .and_then(|time| snapshot.sampled_at.duration_since(time).ok())
                    .is_some_and(|age| age <= max_age)
        })
}
fn signal_sample(snapshot: &SystemSnapshot, disk: bool, network: bool) -> SignalSample {
    SignalSample {
        cpu_pct: snapshot.cpu_usage as f64,
        memory_pct: snapshot.memory_percentage as f64,
        disk_bps: if disk {
            snapshot.disk_read_bytes_per_second.unwrap_or(f64::NAN)
                + snapshot.disk_written_bytes_per_second.unwrap_or(f64::NAN)
        } else {
            0.0
        },
        network_bps: if network {
            snapshot
                .networks
                .iter()
                .map(|n| n.received_bytes_per_second + n.transmitted_bytes_per_second)
                .sum()
        } else {
            0.0
        },
    }
}
fn analyze_data(data: &SessionData) -> SessionDiagnosis {
    let samples = &data.snapshots;
    let baseline_count = data.baseline.unwrap_or_else(|| {
        if data.guided {
            GUIDED_BASELINE_SAMPLES
        } else {
            (samples.len() / 3).clamp(3, 15)
        }
    });
    if samples.len() < baseline_count + 3 || (data.guided && data.baseline.is_none()) {
        return SessionDiagnosis::unavailable(
            DiagnosisState::Insufficient,
            samples.len(),
            baseline_count,
            "Insufficient baseline/reproduction samples or no recorded baseline boundary.",
        );
    }
    let age = cadence(samples);
    let baseline = &samples[..baseline_count];
    if baseline
        .iter()
        .any(|s| !fresh_signal(s, "cpu.global_usage", age) || !fresh_signal(s, "memory.used", age))
    {
        return SessionDiagnosis::unavailable(
            DiagnosisState::Stale,
            samples.len(),
            baseline_count,
            "Baseline CPU/memory acquisition timestamps are missing, stale, paused or unavailable.",
        );
    }
    let disk = data.units == RateUnits::BytesPerSecond
        && samples.iter().all(|s| {
            fresh_signal(s, "disk", age)
                && s.disk_read_bytes_per_second.is_some()
                && s.disk_written_bytes_per_second.is_some()
        });
    let network = data.units == RateUnits::BytesPerSecond && samples.iter().all(|s| fresh_signal(s, "network", age));
    let baseline_signals: Vec<_> = baseline.iter().map(|s| signal_sample(s, disk, network)).collect();
    let mut best = None;
    let mut stale = 0;
    for (index, incident) in samples.iter().enumerate().skip(baseline_count) {
        if !fresh_signal(incident, "cpu.global_usage", age) || !fresh_signal(incident, "memory.used", age) {
            stale += 1;
            continue;
        }
        let Some(comparison) = compare_to_baseline(&baseline_signals, signal_sample(incident, disk, network)) else {
            return SessionDiagnosis::unavailable(
                DiagnosisState::Invalid,
                samples.len(),
                baseline_count,
                "Non-finite, negative or out-of-range signal values cannot be compared.",
            );
        };
        if best
            .as_ref()
            .is_none_or(|(_, old): &(usize, crate::diagnostics::baseline::BaselineComparison)| {
                comparison.primary_score > old.primary_score
            })
        {
            best = Some((index, comparison));
        }
    }
    if samples.len() - baseline_count - stale < 3 {
        return SessionDiagnosis::unavailable(
            DiagnosisState::Stale,
            samples.len(),
            baseline_count,
            "Fewer than three reproduction samples have fresh CPU/memory evidence.",
        );
    }
    let Some((index, comparison)) = best else {
        return SessionDiagnosis::unavailable(
            DiagnosisState::Stale,
            samples.len(),
            baseline_count,
            "No fresh comparable incident samples.",
        );
    };
    let incident = &samples[index];
    let changed = comparison.state == ComparisonState::Change;
    let mut evidence = comparison.evidence;
    evidence.push(format!(
        "Recorded baseline boundary: {} samples; incident sample {}; maximum source age {:.1}s.",
        baseline_count,
        index + 1,
        age.as_secs_f64()
    ));
    if !data.guided {
        evidence.push("Generic recording: baseline uses the first third (3–15 samples), not a guided boundary.".into());
    }
    if data.units == RateUnits::UnknownLegacy {
        evidence.push("Legacy file: rate units unknown; I/O comparisons excluded. Original file is unchanged.".into());
    }
    if !disk {
        evidence.push("Aggregate disk I/O unavailable/stale: excluded, never summed across mounts.".into());
    }
    if !network {
        evidence.push("Network rates unavailable/stale: excluded.".into());
    }
    if stale > 0 {
        evidence.push(format!("{stale} stale reproduction samples excluded."));
    }
    let contributor = if changed && comparison.primary_signal != "Network" && fresh_signal(incident, "processes", age) {
        let score = |p: &crate::monitoring::snapshot::ProcessSnapshot| match comparison.primary_signal {
            "CPU" => p.cpu_usage as f64,
            "Memory" => p.memory as f64,
            "Disk I/O" => p.disk_read_bytes.saturating_add(p.disk_written_bytes) as f64,
            _ => 0.0,
        };
        let source_age = incident.metric_status["processes"]
            .observed_at
            .and_then(|time| incident.sampled_at.duration_since(time).ok())
            .unwrap_or_default();
        incident
            .processes
            .iter()
            .filter(|p| score(p).is_finite() && score(p) > 0.0)
            .max_by(|a, b| score(a).total_cmp(&score(b)))
            .map(|p| {
                let measured = match comparison.primary_signal {
                    "CPU" => format!("CPU {:.1}%", p.cpu_usage),
                    "Memory" => format!("working set {:.1} MiB", p.memory as f64 / 1_048_576.0),
                    _ => format!(
                        "process I/O {} bytes in its refresh interval",
                        p.disk_read_bytes.saturating_add(p.disk_written_bytes)
                    ),
                };
                format!(
                    "{} (PID {}): {measured}; source age {:.1}s (contemporaneous usage, not proof of causality)",
                    p.name,
                    p.pid,
                    source_age.as_secs_f64()
                )
            })
    } else {
        None
    };
    if changed && contributor.is_none() {
        evidence.push(if comparison.primary_signal == "Network" {
            "No per-process network transfer signal exists; no process network attribution is possible.".into()
        } else {
            "No fresh, positive incident-relevant process measurement; no contributor selected.".into()
        });
    }
    SessionDiagnosis {
        state: if changed {
            DiagnosisState::Change
        } else {
            DiagnosisState::NoMeaningfulChange
        },
        sample_count: samples.len(),
        baseline_samples: baseline_count,
        incident_sample: if changed { index + 1 } else { 0 },
        primary_signal: comparison.primary_signal.into(),
        summary: comparison.summary,
        recommendation: if changed {
            "Inspect the measured signal and contemporaneous evidence before taking any system-changing action."
        } else {
            "No contributor is recommended. Capture again while the slowdown is observable if symptoms continue."
        }
        .into(),
        confidence: comparison.confidence.into(),
        evidence,
        contributor,
    }
}
#[cfg(test)]
pub fn analyze_session_against_baseline(path: &Path) -> io::Result<SessionDiagnosis> {
    Ok(analyze_data(&read_session(path)?))
}

pub fn export_session_to_csv(input: &Path, output: &Path) -> io::Result<usize> {
    if input == output {
        return Err(invalid("export cannot overwrite the source session"));
    }
    let data = read_session(input)?;
    // Never overwrite an existing export or a path alias of the source.
    let mut writer = BufWriter::new(File::options().write(true).create_new(true).open(output)?);
    writeln!(
        writer,
        "Timestamp_UTC,CPU_Usage_Pct,Memory_Pct,GPU_Util_Pct,Rate_Units,Net_Recv_BytesPerSecond,Net_Sent_BytesPerSecond,Disk_Read_BytesPerSecond,Disk_Write_BytesPerSecond,CPU_State,Memory_State,Network_State,Processes_State"
    )?;
    for snapshot in &data.snapshots {
        let age = cadence(&data.snapshots);
        let show = |value: Option<f64>| {
            value
                .filter(|v| v.is_finite())
                .map(|v| v.to_string())
                .unwrap_or_default()
        };
        let state = |key: &str| {
            snapshot
                .metric_status
                .get(key)
                .map(|o| format!("{:?}", o.state))
                .unwrap_or_else(|| "Unknown".into())
        };
        let canonical = data.units == RateUnits::BytesPerSecond;
        writeln!(
            writer,
            "{},{},{},{},{:?},{},{},{},{},{},{},{},{}",
            chrono::DateTime::<chrono::Utc>::from(snapshot.sampled_at).to_rfc3339(),
            snapshot.cpu_usage,
            snapshot.memory_percentage,
            show(snapshot.gpus.first().and_then(|g| g.utilization).map(f64::from)),
            data.units,
            show(
                (canonical && fresh_signal(snapshot, "network", age)).then(|| snapshot
                    .networks
                    .iter()
                    .map(|n| n.received_bytes_per_second)
                    .sum())
            ),
            show(
                (canonical && fresh_signal(snapshot, "network", age)).then(|| snapshot
                    .networks
                    .iter()
                    .map(|n| n.transmitted_bytes_per_second)
                    .sum())
            ),
            show(
                snapshot
                    .disk_read_bytes_per_second
                    .filter(|_| canonical && fresh_signal(snapshot, "disk", age))
            ),
            show(
                snapshot
                    .disk_written_bytes_per_second
                    .filter(|_| canonical && fresh_signal(snapshot, "disk", age))
            ),
            state("cpu.global_usage"),
            state("memory.used"),
            state("network"),
            state("processes")
        )?;
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(data.snapshots.len())
}

pub fn list_recorded_sessions() -> io::Result<Vec<PathBuf>> {
    let root =
        crate::app_paths::sessions_dir().ok_or_else(|| io::Error::other("application data directory unavailable"))?;
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "jsonl") {
            paths.push(path);
        }
        if paths.len() > 20_000 {
            return Err(invalid(
                "session directory exceeds 20,000 entries; choose files externally",
            ));
        }
    }
    paths.sort_unstable_by(|a, b| b.cmp(a));
    Ok(paths)
}

#[derive(Clone, Debug)]
pub(crate) struct SessionView {
    pub(crate) path: PathBuf,
    pub(crate) summary: SessionSummary,
    pub(crate) diagnosis: SessionDiagnosis,
}
#[derive(Clone, PartialEq, Eq)]
struct FileIdentity {
    path: PathBuf,
    length: u64,
    modified: SystemTime,
    created: SystemTime,
}
fn file_identity(path: &Path) -> io::Result<FileIdentity> {
    let metadata = fs::metadata(path)?;
    Ok(FileIdentity {
        path: path.into(),
        length: metadata.len(),
        modified: metadata.modified()?,
        created: metadata.created()?,
    })
}
enum ReadJob {
    Inspect(Option<PathBuf>),
    Export(PathBuf, PathBuf),
}
enum ReadResult {
    View(Result<(Vec<PathBuf>, Option<Arc<SessionView>>), String>),
    Export(Result<String, String>),
}
struct SessionReader {
    tx: mpsc::SyncSender<ReadJob>,
    rx: mpsc::Receiver<ReadResult>,
    pending: bool,
    last_refresh: Option<Instant>,
}
impl SessionReader {
    fn spawn() -> io::Result<Self> {
        let (tx, jobs) = mpsc::sync_channel(4);
        let (results, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("session-analysis".into())
            .spawn(move || {
                let mut cache: Option<(FileIdentity, Arc<SessionView>)> = None;
                while let Ok(job) = jobs.recv() {
                    let result = match job {
                        ReadJob::Inspect(selected) => ReadResult::View(
                            (|| {
                                let paths = list_recorded_sessions()?;
                                let path = selected.or_else(|| paths.first().cloned());
                                let view = if let Some(path) = path {
                                    let identity = file_identity(&path)?;
                                    if cache.as_ref().is_none_or(|(old, _)| *old != identity) {
                                        let data = read_session(&path)?;
                                        let mut summary = SummaryAccumulator::default();
                                        for snapshot in &data.snapshots {
                                            summary.push(snapshot);
                                        }
                                        let view = Arc::new(SessionView {
                                            path,
                                            summary: summary.summary,
                                            diagnosis: analyze_data(&data),
                                        });
                                        if file_identity(&identity.path)? != identity {
                                            return Err(io::Error::other(
                                                "session changed during analysis; retry after capture finishes",
                                            ));
                                        }
                                        cache = Some((identity, view));
                                    }
                                    cache.as_ref().map(|(_, view)| Arc::clone(view))
                                } else {
                                    None
                                };
                                Ok((paths, view))
                            })()
                            .map_err(|error: io::Error| error.to_string()),
                        ),
                        ReadJob::Export(input, output) => ReadResult::Export(
                            export_session_to_csv(&input, &output)
                                .map(|count| format!("Exported {count} rows to {}", output.display()))
                                .map_err(|error| error.to_string()),
                        ),
                    };
                    if results.send(result).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            tx,
            rx,
            pending: false,
            last_refresh: None,
        })
    }
}

enum WriteJob {
    Sample(Box<SystemSnapshot>),
    Stop,
}
struct WriterHandle {
    tx: mpsc::SyncSender<WriteJob>,
    done: mpsc::Receiver<Result<(), String>>,
}
fn spawn_writer(path: PathBuf, guided: bool) -> io::Result<WriterHandle> {
    let (tx, jobs) = mpsc::sync_channel(64);
    let (complete, done) = mpsc::channel();
    std::thread::Builder::new().name("session-writer".into()).spawn(move || {
        let result = (|| -> io::Result<()> {
            fs::create_dir_all(path.parent().ok_or_else(|| invalid("missing session directory"))?)?;
            let mut writer = BufWriter::new(File::options().write(true).create_new(true).open(&path)?);
            serde_json::to_writer(&mut writer, &SessionMetadata::Header { version: 2, rate_units: RateUnits::BytesPerSecond,
                guided_baseline_samples: guided.then_some(GUIDED_BASELINE_SAMPLES) }).map_err(io::Error::other)?;
            writer.write_all(b"\n")?;
            let mut count = 0usize; let mut bytes = 0u64;
            while let Ok(job) = jobs.recv() {
                match job {
                    WriteJob::Sample(snapshot) => {
                        let line = serde_json::to_vec(&*snapshot).map_err(io::Error::other)?;
                        bytes += line.len() as u64 + 1;
                        if line.len() as u64 > MAX_LINE_BYTES || bytes > MAX_RECORD_BYTES || count >= MAX_SAMPLES {
                            writer.flush()?; writer.get_ref().sync_all()?;
                            return Err(io::Error::other("capture stopped at the 64 MiB / 20,000-sample safety limit; existing session preserved"));
                        }
                        writer.write_all(&line)?; writer.write_all(b"\n")?;
                        count += 1;
                        if guided && count == GUIDED_BASELINE_SAMPLES {
                            serde_json::to_writer(&mut writer, &SessionMetadata::BaselineEnd { samples: count, sampled_at: snapshot.sampled_at }).map_err(io::Error::other)?;
                            writer.write_all(b"\n")?;
                        }
                        writer.flush()?;
                    }
                    WriteJob::Stop => break,
                }
            }
            writer.flush()?; writer.get_ref().sync_all()
        })().map_err(|error| error.to_string());
        let _ = complete.send(result);
    })?;
    Ok(WriterHandle { tx, done })
}

#[derive(Default)]
pub(crate) struct SessionRecorder {
    writer: Option<WriterHandle>,
    stopping: bool,
    path: Option<PathBuf>,
    last_sample: Option<SystemTime>,
    accumulator: SummaryAccumulator,
    reader: Option<SessionReader>,
    pub(crate) view: Option<Arc<SessionView>>,
    pub(crate) sessions: Vec<PathBuf>,
    status: Option<String>,
}
impl SessionRecorder {
    pub(crate) fn start(&mut self) -> io::Result<PathBuf> {
        self.start_capture(false)
    }
    pub(crate) fn start_guided(&mut self) -> io::Result<PathBuf> {
        self.start_capture(true)
    }
    fn start_capture(&mut self, guided: bool) -> io::Result<PathBuf> {
        self.poll();
        if self.writer.is_some() {
            return Err(io::Error::other("previous recording is active or still flushing"));
        }
        let root = crate::app_paths::sessions_dir()
            .ok_or_else(|| io::Error::other("application data directory unavailable"))?;
        let path = root.join(format!(
            "session-{}-{}.jsonl",
            chrono::Utc::now().format("%Y%m%d-%H%M%S%.9f"),
            std::process::id()
        ));
        self.writer = Some(spawn_writer(path.clone(), guided)?);
        self.stopping = false;
        self.path = Some(path.clone());
        self.last_sample = None;
        self.accumulator = SummaryAccumulator::default();
        self.view = None;
        Ok(path)
    }
    pub(crate) fn record(&mut self, snapshot: &SystemSnapshot) -> io::Result<()> {
        if !self.is_recording() {
            return Ok(());
        }
        if let Some(last) = self.last_sample {
            let elapsed = snapshot
                .sampled_at
                .duration_since(last)
                .map_err(|_| invalid("recording timestamp moved backwards"))?;
            if elapsed < Duration::from_secs(1) {
                return Ok(());
            }
        }
        match self
            .writer
            .as_ref()
            .unwrap()
            .tx
            .try_send(WriteJob::Sample(Box::new(snapshot.clone())))
        {
            Ok(()) => {
                self.accumulator.push(snapshot);
                self.last_sample = Some(snapshot.sampled_at);
                Ok(())
            }
            Err(error) => {
                self.stopping = true;
                let _ = self.writer.as_ref().unwrap().tx.try_send(WriteJob::Stop);
                Err(io::Error::other(format!(
                    "recording queue unavailable; stopping rather than silently dropping evidence: {error}"
                )))
            }
        }
    }
    pub(crate) fn stop(&mut self) -> io::Result<Option<PathBuf>> {
        if let Some(writer) = &self.writer
            && !self.stopping
        {
            writer
                .tx
                .try_send(WriteJob::Stop)
                .map_err(|error| io::Error::other(format!("could not queue flush: {error}")))?;
            self.stopping = true;
        }
        Ok(self.path.clone())
    }
    pub(crate) fn toggle(&mut self) -> io::Result<Option<PathBuf>> {
        if self.is_recording() {
            self.stop()
        } else {
            self.start().map(Some)
        }
    }
    pub(crate) fn is_recording(&self) -> bool {
        self.writer.is_some() && !self.stopping
    }
    pub(crate) fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }
    pub(crate) fn take_status(&mut self) -> Option<String> {
        self.status.take()
    }
    pub(crate) fn sample_count(&self) -> u64 {
        self.accumulator.summary.sample_count as u64
    }
    pub(crate) fn refresh(&mut self) {
        self.poll();
        if self.writer.is_some() {
            return;
        }
        if self.reader.is_none() {
            match SessionReader::spawn() {
                Ok(reader) => self.reader = Some(reader),
                Err(error) => {
                    self.status = Some(error.to_string());
                    return;
                }
            }
        }
        let reader = self.reader.as_mut().unwrap();
        if reader.pending
            || reader
                .last_refresh
                .is_some_and(|time| time.elapsed() < Duration::from_secs(2))
        {
            return;
        }
        match reader.tx.try_send(ReadJob::Inspect(self.path.clone())) {
            Ok(()) => {
                reader.pending = true;
                reader.last_refresh = Some(Instant::now());
            }
            Err(error) => self.status = Some(format!("Analysis request failed: {error}")),
        }
    }
    pub(crate) fn request_export(&mut self, input: PathBuf) -> io::Result<()> {
        if self.writer.is_some() {
            return Err(io::Error::other(
                "wait for the capture to finish flushing before exporting",
            ));
        }
        if self.reader.is_none() {
            self.reader = Some(SessionReader::spawn()?);
        }
        let output = input.with_extension(format!("{}.csv", chrono::Utc::now().format("%Y%m%d-%H%M%S%.9f")));
        self.reader
            .as_ref()
            .unwrap()
            .tx
            .try_send(ReadJob::Export(input, output))
            .map_err(|error| io::Error::other(error.to_string()))
    }
    pub(crate) fn poll(&mut self) {
        if let Some(writer) = &self.writer {
            match writer.done.try_recv() {
                Ok(result) => {
                    self.status = Some(match result {
                        Ok(()) => "Session flushed and saved locally.".into(),
                        Err(error) => format!("Session recording/flush failed: {error}"),
                    });
                    self.writer = None;
                    self.stopping = false;
                    if let Some(reader) = &mut self.reader {
                        reader.last_refresh = None;
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status = Some("Session writer disconnected; durability is not confirmed.".into());
                    self.writer = None;
                    self.stopping = false;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    if self.stopping {
                        let _ = writer.tx.try_send(WriteJob::Stop);
                    }
                }
            }
        }
        if let Some(reader) = &mut self.reader {
            while let Ok(result) = reader.rx.try_recv() {
                match result {
                    ReadResult::View(result) => {
                        reader.pending = false;
                        match result {
                            Ok((paths, view)) => {
                                self.sessions = paths;
                                if self.writer.is_none()
                                    && view
                                        .as_ref()
                                        .is_none_or(|view| self.path.as_ref().is_none_or(|path| *path == view.path))
                                {
                                    self.view = view;
                                }
                            }
                            Err(error) => {
                                self.view = None;
                                self.status = Some(format!("Session analysis unavailable: {error}"));
                            }
                        }
                    }
                    ReadResult::Export(result) => {
                        self.status = Some(result.unwrap_or_else(|error| format!("Session export failed: {error}")))
                    }
                }
            }
        }
    }
    pub(crate) fn shutdown(&mut self, timeout: Duration) -> io::Result<()> {
        self.reader = None; // read-only work owns no pending writes and never blocks closing
        let Some(writer) = self.writer.take() else {
            return Ok(());
        };
        let WriterHandle { tx, done } = writer;
        drop(tx); // worker drains all owned snapshots, then flushes and syncs
        self.stopping = false;
        done.recv_timeout(timeout)
            .map_err(|error| {
                io::Error::other(format!("session flush not confirmed before shutdown deadline: {error}"))
            })?
            .map_err(io::Error::other)
    }
}
impl Drop for SessionRecorder {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown(Duration::from_millis(250)) {
            tracing::error!(%error, "Session shutdown durability not confirmed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(index: u64, cpu: f32) -> SystemSnapshot {
        let time = SystemTime::UNIX_EPOCH + Duration::from_secs(index * 2 + 100);
        SystemSnapshot {
            sampled_at: time,
            cpu_usage: cpu,
            memory_percentage: 40.0,
            memory_total: 1024,
            metric_status: ["cpu.global_usage", "memory.used", "processes", "network"]
                .into_iter()
                .map(|key| {
                    (
                        key.into(),
                        crate::monitoring::snapshot::MetricObservation::available(time),
                    )
                })
                .collect(),
            ..Default::default()
        }
    }
    fn data(count: usize) -> SessionData {
        SessionData {
            snapshots: (0..count).map(|n| snapshot(n as u64, 10.0)).collect(),
            units: RateUnits::BytesPerSecond,
            baseline: Some(15),
            guided: true,
        }
    }
    #[test]
    fn flat_thirty_samples_do_not_produce_a_culprit_or_high_confidence() {
        let diagnosis = analyze_data(&data(30));
        assert_eq!(diagnosis.state, DiagnosisState::NoMeaningfulChange);
        assert!(diagnosis.contributor.is_none());
        assert_ne!(diagnosis.confidence, "high");
    }
    #[test]
    fn eighteen_samples_keep_all_fifteen_baseline_samples_and_rank_cpu_only() {
        let mut session = data(18);
        session.snapshots[9].cpu_usage = 50.0;
        session.snapshots[16].cpu_usage = 90.0;
        session.snapshots[16].processes = vec![
            crate::monitoring::snapshot::ProcessSnapshot {
                name: "idle-large".into(),
                memory: u64::MAX,
                ..Default::default()
            },
            crate::monitoring::snapshot::ProcessSnapshot {
                name: "busy".into(),
                cpu_usage: 80.0,
                ..Default::default()
            },
        ];
        let diagnosis = analyze_data(&session);
        assert_eq!(diagnosis.baseline_samples, 15);
        assert_eq!(diagnosis.incident_sample, 17);
        assert_eq!(diagnosis.primary_signal, "CPU");
        assert!(diagnosis.contributor.unwrap().contains("busy"));
        session.snapshots[16]
            .metric_status
            .get_mut("processes")
            .unwrap()
            .observed_at = Some(SystemTime::UNIX_EPOCH);
        assert!(analyze_data(&session).contributor.is_none());
    }
    #[test]
    fn network_totals_use_counter_deltas_not_sample_count_and_handle_resets() {
        let mut accumulator = SummaryAccumulator::default();
        for (index, received) in [(0, 100), (1, 1100), (5, 3100), (6, 10)] {
            let mut snap = snapshot(index, 10.0);
            snap.networks.push(crate::monitoring::snapshot::NetworkSnapshot {
                interface: "test".into(),
                received,
                received_bytes_per_second: 999999.0,
                ..Default::default()
            });
            accumulator.push(&snap);
        }
        assert_eq!(accumulator.summary.duration_secs, 12);
        assert_eq!(accumulator.summary.total_net_recv_mb, Some(3000.0 / 1_048_576.0));
    }
    #[test]
    fn stale_empty_and_invalid_samples_are_not_healthy() {
        let mut session = data(18);
        session.snapshots[0].paused = true;
        assert_eq!(analyze_data(&session).state, DiagnosisState::Stale);
        assert_eq!(analyze_data(&data(16)).state, DiagnosisState::Insufficient);
        let mut session = data(18);
        session.snapshots[16].cpu_usage = f32::NAN;
        assert_eq!(analyze_data(&session).state, DiagnosisState::Invalid);
    }
    #[test]
    fn writer_persists_boundary_units_and_exports_without_rewriting_legacy_data() {
        let root = std::env::temp_dir().join(format!("sysmon-session-boundary-{}", std::process::id()));
        crate::app_paths::with_test_data_local_dir(root.clone(), || {
            let mut recorder = SessionRecorder::default();
            let path = recorder.start_guided().unwrap();
            for n in 0..18 {
                recorder
                    .record(&snapshot(n, if n == 16 { 90.0 } else { 10.0 }))
                    .unwrap();
            }
            recorder.shutdown(Duration::from_secs(5)).unwrap();
            let parsed = read_session(&path).unwrap();
            assert_eq!(parsed.baseline, Some(15));
            assert_eq!(parsed.units, RateUnits::BytesPerSecond);
            assert_eq!(analyze_session_against_baseline(&path).unwrap().incident_sample, 17);
            let legacy = root.join("legacy.jsonl");
            let original = format!("{}\n", serde_json::to_string(&snapshot(0, 10.0)).unwrap());
            fs::write(&legacy, &original).unwrap();
            assert_eq!(read_session(&legacy).unwrap().units, RateUnits::UnknownLegacy);
            let csv = root.join("legacy.csv");
            export_session_to_csv(&legacy, &csv).unwrap();
            assert!(fs::read_to_string(csv).unwrap().contains("UnknownLegacy"));
            assert_eq!(fs::read_to_string(legacy).unwrap(), original);
        });
        fs::remove_dir_all(root).unwrap();
    }
}
