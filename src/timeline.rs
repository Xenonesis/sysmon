//! Privacy-first, local diagnostic timeline.
//!
//! SQLite is owned by a dedicated worker thread. The UI and monitoring loop only
//! exchange small commands and immutable result snapshots with that worker.

use crate::monitoring::SystemSnapshot;
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 1;
const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);
const MAX_DATABASE_BYTES: u64 = 512 * 1024 * 1024;
const DAY_MS: i64 = 86_400_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TimelineRange {
    FifteenMinutes,
    OneHour,
    SixHours,
    OneDay,
    SevenDays,
}

impl TimelineRange {
    pub(crate) const ALL: [Self; 5] = [
        Self::FifteenMinutes,
        Self::OneHour,
        Self::SixHours,
        Self::OneDay,
        Self::SevenDays,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::FifteenMinutes => "15m",
            Self::OneHour => "1h",
            Self::SixHours => "6h",
            Self::OneDay => "24h",
            Self::SevenDays => "7d",
        }
    }

    fn duration_ms(self) -> i64 {
        match self {
            Self::FifteenMinutes => 15 * 60_000,
            Self::OneHour => 60 * 60_000,
            Self::SixHours => 6 * 60 * 60_000,
            Self::OneDay => DAY_MS,
            Self::SevenDays => 7 * DAY_MS,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TimelineQuery {
    pub(crate) start_ms: i64,
    pub(crate) end_ms: i64,
}

impl TimelineQuery {
    pub(crate) fn latest(range: TimelineRange) -> Self {
        let end_ms = now_ms();
        Self {
            start_ms: end_ms.saturating_sub(range.duration_ms()),
            end_ms,
        }
    }

    fn validated(self) -> Self {
        if self.start_ms <= self.end_ms {
            self
        } else {
            Self {
                start_ms: self.end_ms,
                end_ms: self.start_ms,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TimelineEventKind {
    AlertTriggered,
    AlertResolved,
    ActionSucceeded,
    ActionFailed,
    ProviderUnavailable,
    ProviderRecovered,
    MonitoringPaused,
    MonitoringResumed,
    PowerChanged,
    ServiceChanged,
    StartupChanged,
    System,
}

impl TimelineEventKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::AlertTriggered => "alert_triggered",
            Self::AlertResolved => "alert_resolved",
            Self::ActionSucceeded => "action_succeeded",
            Self::ActionFailed => "action_failed",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::ProviderRecovered => "provider_recovered",
            Self::MonitoringPaused => "monitoring_paused",
            Self::MonitoringResumed => "monitoring_resumed",
            Self::PowerChanged => "power_changed",
            Self::ServiceChanged => "service_changed",
            Self::StartupChanged => "startup_changed",
            Self::System => "system",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "alert_triggered" => Self::AlertTriggered,
            "alert_resolved" => Self::AlertResolved,
            "action_succeeded" => Self::ActionSucceeded,
            "action_failed" => Self::ActionFailed,
            "provider_unavailable" => Self::ProviderUnavailable,
            "provider_recovered" => Self::ProviderRecovered,
            "monitoring_paused" => Self::MonitoringPaused,
            "monitoring_resumed" => Self::MonitoringResumed,
            "power_changed" => Self::PowerChanged,
            "service_changed" => Self::ServiceChanged,
            "startup_changed" => Self::StartupChanged,
            _ => Self::System,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TimelineEvent {
    pub(crate) id: Option<i64>,
    pub(crate) timestamp_ms: i64,
    pub(crate) kind: TimelineEventKind,
    pub(crate) source: String,
    pub(crate) severity: String,
    pub(crate) summary: String,
    pub(crate) evidence: String,
}

impl TimelineEvent {
    pub(crate) fn new(
        kind: TimelineEventKind,
        source: impl Into<String>,
        severity: impl Into<String>,
        summary: impl Into<String>,
        evidence: impl Into<String>,
    ) -> Self {
        Self {
            id: None,
            timestamp_ms: now_ms(),
            kind,
            source: sanitize_text(source.into(), 128),
            severity: sanitize_text(severity.into(), 32),
            summary: sanitize_text(summary.into(), 512),
            evidence: sanitize_text(evidence.into(), 2_048),
        }
    }

    pub(crate) fn from_audit(record: &crate::app::actions::ActionAuditRecord) -> Self {
        let timestamp_ms = chrono::DateTime::parse_from_rfc3339(&record.timestamp)
            .map(|timestamp| timestamp.timestamp_millis())
            .unwrap_or_else(|_| now_ms());
        Self {
            id: None,
            timestamp_ms,
            kind: if record.succeeded {
                TimelineEventKind::ActionSucceeded
            } else {
                TimelineEventKind::ActionFailed
            },
            source: "guarded_action".into(),
            severity: record.risk.label().to_ascii_lowercase(),
            summary: sanitize_text(record.action.clone(), 512),
            evidence: sanitize_text(record.message.clone(), 2_048),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct TimelineMetricSample {
    pub(crate) timestamp_ms: i64,
    pub(crate) cpu_pct: f64,
    pub(crate) memory_pct: f64,
    pub(crate) gpu_pct: Option<f64>,
    pub(crate) cpu_temp_c: Option<f64>,
    pub(crate) gpu_temp_c: Option<f64>,
    pub(crate) disk_read_bps: f64,
    pub(crate) disk_write_bps: f64,
    pub(crate) network_down_bps: f64,
    pub(crate) network_up_bps: f64,
    pub(crate) paused: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct TimelineProcessSample {
    pub(crate) timestamp_ms: i64,
    pub(crate) pid: u32,
    pub(crate) start_time: u64,
    pub(crate) name: String,
    pub(crate) cpu_pct: f64,
    pub(crate) memory_bytes: u64,
    pub(crate) disk_read_bytes: u64,
    pub(crate) disk_write_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct IncidentContributor {
    pub(crate) name: String,
    pub(crate) pid: u32,
    pub(crate) start_time: u64,
    pub(crate) cpu_pct: f64,
    pub(crate) memory_bytes: u64,
    pub(crate) disk_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct IncidentAnalysis {
    pub(crate) timestamp_ms: i64,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) confidence: String,
    pub(crate) evidence: Vec<String>,
    pub(crate) contributors: Vec<IncidentContributor>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct TimelineWindow {
    pub(crate) query: TimelineQuery,
    pub(crate) metrics: Vec<TimelineMetricSample>,
    pub(crate) processes: Vec<TimelineProcessSample>,
    pub(crate) events: Vec<TimelineEvent>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TimelineStatus {
    pub(crate) enabled: bool,
    pub(crate) retention_days: u16,
    pub(crate) storage_bytes: u64,
    pub(crate) last_write_ms: Option<i64>,
    pub(crate) last_error: Option<String>,
}

pub(crate) struct TimelineUiState {
    pub(crate) range: TimelineRange,
    pub(crate) window: Option<TimelineWindow>,
    pub(crate) selected_timestamp_ms: Option<i64>,
    pub(crate) last_refresh: Option<Instant>,
    pub(crate) clear_confirmation: bool,
    pub(crate) message: Option<String>,
    pub(crate) active_alert_keys: std::collections::HashSet<String>,
    pub(crate) service_states: Option<HashMap<String, String>>,
    pub(crate) startup_states: Option<HashMap<String, bool>>,
}

impl Default for TimelineUiState {
    fn default() -> Self {
        Self {
            range: TimelineRange::OneHour,
            window: None,
            selected_timestamp_ms: None,
            last_refresh: None,
            clear_confirmation: false,
            message: None,
            active_alert_keys: std::collections::HashSet::new(),
            service_states: None,
            startup_states: None,
        }
    }
}

pub(crate) enum TimelineCommand {
    RecordSnapshot(Box<SystemSnapshot>),
    RecordEvent(TimelineEvent),
    SetPolicy {
        enabled: bool,
        retention_days: u16,
    },
    Query {
        query: TimelineQuery,
        reply: SyncSender<Result<TimelineWindow, String>>,
    },
    Export {
        query: TimelineQuery,
        destination: PathBuf,
        reply: SyncSender<Result<PathBuf, String>>,
    },
    Clear,
    Shutdown,
}

#[derive(Clone)]
pub(crate) struct TimelineHandle {
    sender: Sender<TimelineCommand>,
    status: Arc<Mutex<TimelineStatus>>,
    query_result: Arc<Mutex<Option<Result<TimelineWindow, String>>>>,
    query_in_flight: Arc<AtomicBool>,
    export_result: Arc<Mutex<Option<Result<PathBuf, String>>>>,
    export_in_flight: Arc<AtomicBool>,
    last_snapshot_queued: Arc<Mutex<Option<Instant>>>,
}

impl TimelineHandle {
    pub(crate) fn start(enabled: bool, retention_days: u16) -> Self {
        Self::start_at(timeline_db_path(), enabled, retention_days)
    }

    fn start_at(path: Option<PathBuf>, enabled: bool, retention_days: u16) -> Self {
        let retention_days = validate_retention(retention_days);
        let (sender, receiver) = mpsc::channel();
        let status = Arc::new(Mutex::new(TimelineStatus {
            enabled,
            retention_days,
            storage_bytes: path.as_deref().map(storage_bytes).unwrap_or(0),
            last_write_ms: None,
            last_error: path
                .is_none()
                .then(|| "Local application-data directory is unavailable".into()),
        }));
        let worker_status = status.clone();
        std::thread::Builder::new()
            .name("timeline_storage".into())
            .spawn(move || run_worker(path, enabled, retention_days, receiver, worker_status))
            .expect("failed to spawn timeline storage worker");

        Self {
            sender,
            status,
            query_result: Arc::new(Mutex::new(None)),
            query_in_flight: Arc::new(AtomicBool::new(false)),
            export_result: Arc::new(Mutex::new(None)),
            export_in_flight: Arc::new(AtomicBool::new(false)),
            last_snapshot_queued: Arc::new(Mutex::new(None)),
        }
    }

    pub(crate) fn status(&self) -> TimelineStatus {
        self.status.lock().clone()
    }

    pub(crate) fn record_snapshot(&self, snapshot: SystemSnapshot) {
        if !self.status.lock().enabled {
            return;
        }
        let now = Instant::now();
        let mut last = self.last_snapshot_queued.lock();
        if last.is_some_and(|previous| now.saturating_duration_since(previous) < SAMPLE_INTERVAL) {
            return;
        }
        *last = Some(now);
        let _ = self.sender.send(TimelineCommand::RecordSnapshot(Box::new(snapshot)));
    }

    pub(crate) fn record_event(&self, event: TimelineEvent) {
        if self.status.lock().enabled {
            let _ = self.sender.send(TimelineCommand::RecordEvent(event));
        }
    }

    pub(crate) fn set_policy(&self, enabled: bool, retention_days: u16) {
        let retention_days = validate_retention(retention_days);
        {
            let mut status = self.status.lock();
            status.enabled = enabled;
            status.retention_days = retention_days;
        }
        if !enabled {
            *self.last_snapshot_queued.lock() = None;
        }
        let _ = self.sender.send(TimelineCommand::SetPolicy {
            enabled,
            retention_days,
        });
    }

    pub(crate) fn request_window(&self, query: TimelineQuery) {
        if self.query_in_flight.swap(true, Ordering::AcqRel) {
            return;
        }
        let sender = self.sender.clone();
        let result_slot = self.query_result.clone();
        let in_flight = self.query_in_flight.clone();
        let spawn = std::thread::Builder::new()
            .name("timeline_query".into())
            .spawn(move || {
                let (reply, receiver) = mpsc::sync_channel(1);
                let result = sender
                    .send(TimelineCommand::Query {
                        query: query.validated(),
                        reply,
                    })
                    .map_err(|_| "Timeline worker is unavailable".to_string())
                    .and_then(|_| {
                        receiver
                            .recv()
                            .map_err(|_| "Timeline query was interrupted".to_string())
                    })
                    .and_then(|result| result);
                *result_slot.lock() = Some(result);
                in_flight.store(false, Ordering::Release);
            });
        if spawn.is_err() {
            self.query_in_flight.store(false, Ordering::Release);
            *self.query_result.lock() = Some(Err("Could not start timeline query".into()));
        }
    }

    pub(crate) fn query_in_flight(&self) -> bool {
        self.query_in_flight.load(Ordering::Acquire)
    }

    pub(crate) fn take_query_result(&self) -> Option<Result<TimelineWindow, String>> {
        self.query_result.lock().take()
    }

    pub(crate) fn request_export(&self, query: TimelineQuery, destination: PathBuf) {
        if self.export_in_flight.swap(true, Ordering::AcqRel) {
            return;
        }
        let sender = self.sender.clone();
        let result_slot = self.export_result.clone();
        let in_flight = self.export_in_flight.clone();
        let spawn = std::thread::Builder::new()
            .name("timeline_export".into())
            .spawn(move || {
                let (reply, receiver) = mpsc::sync_channel(1);
                let result = sender
                    .send(TimelineCommand::Export {
                        query: query.validated(),
                        destination,
                        reply,
                    })
                    .map_err(|_| "Timeline worker is unavailable".to_string())
                    .and_then(|_| {
                        receiver
                            .recv()
                            .map_err(|_| "Timeline export was interrupted".to_string())
                    })
                    .and_then(|result| result);
                *result_slot.lock() = Some(result);
                in_flight.store(false, Ordering::Release);
            });
        if spawn.is_err() {
            self.export_in_flight.store(false, Ordering::Release);
            *self.export_result.lock() = Some(Err("Could not start timeline export".into()));
        }
    }

    pub(crate) fn export_in_flight(&self) -> bool {
        self.export_in_flight.load(Ordering::Acquire)
    }

    pub(crate) fn take_export_result(&self) -> Option<Result<PathBuf, String>> {
        self.export_result.lock().take()
    }

    pub(crate) fn clear(&self) {
        let _ = self.sender.send(TimelineCommand::Clear);
    }

    pub(crate) fn shutdown(&self) {
        let _ = self.sender.send(TimelineCommand::Shutdown);
    }
}

fn run_worker(
    path: Option<PathBuf>,
    mut enabled: bool,
    mut retention_days: u16,
    receiver: Receiver<TimelineCommand>,
    status: Arc<Mutex<TimelineStatus>>,
) {
    let Some(path) = path else {
        while !matches!(receiver.recv(), Ok(TimelineCommand::Shutdown) | Err(_)) {}
        return;
    };
    let mut connection: Option<Connection> = None;
    let mut last_prune = Instant::now()
        .checked_sub(Duration::from_secs(86_400))
        .unwrap_or_else(Instant::now);
    let mut previous_providers: HashMap<String, bool> = HashMap::new();
    let mut previous_paused: Option<bool> = None;
    let mut previous_power: Option<String> = None;

    if enabled {
        if let Err(error) = ensure_connection(&mut connection, &path, true) {
            status.lock().last_error = Some(error);
        }
    }

    while let Ok(command) = receiver.recv() {
        let result = match command {
            TimelineCommand::RecordSnapshot(snapshot) if enabled => ensure_connection(&mut connection, &path, true)
                .and_then(|_| {
                    let conn = connection.as_mut().expect("connection initialized");
                    record_derived_events(
                        conn,
                        &snapshot,
                        &mut previous_providers,
                        &mut previous_paused,
                        &mut previous_power,
                    )?;
                    write_snapshot(conn, &snapshot)?;
                    if last_prune.elapsed() >= Duration::from_secs(86_400) || storage_bytes(&path) > MAX_DATABASE_BYTES
                    {
                        prune(conn, retention_days, &path)?;
                        last_prune = Instant::now();
                    }
                    let mut current = status.lock();
                    current.storage_bytes = storage_bytes(&path);
                    current.last_write_ms = Some(system_time_ms(snapshot.sampled_at));
                    current.last_error = None;
                    Ok(())
                }),
            TimelineCommand::RecordSnapshot(_) => Ok(()),
            TimelineCommand::RecordEvent(event) if enabled => ensure_connection(&mut connection, &path, true)
                .and_then(|_| insert_event(connection.as_ref().expect("connection initialized"), &event)),
            TimelineCommand::RecordEvent(_) => Ok(()),
            TimelineCommand::SetPolicy {
                enabled: new_enabled,
                retention_days: new_retention,
            } => {
                enabled = new_enabled;
                retention_days = validate_retention(new_retention);
                if enabled {
                    ensure_connection(&mut connection, &path, true).and_then(|_| {
                        prune(
                            connection.as_ref().expect("connection initialized"),
                            retention_days,
                            &path,
                        )
                    })
                } else {
                    Ok(())
                }
            }
            TimelineCommand::Query { query, reply } => {
                let response = if path.is_file() {
                    ensure_connection(&mut connection, &path, false)
                        .and_then(|_| query_window(connection.as_ref().expect("connection initialized"), query))
                } else {
                    Ok(TimelineWindow {
                        query,
                        ..Default::default()
                    })
                };
                let _ = reply.send(response);
                Ok(())
            }
            TimelineCommand::Export {
                query,
                destination,
                reply,
            } => {
                let response = if path.is_file() {
                    ensure_connection(&mut connection, &path, false)
                        .and_then(|_| query_window(connection.as_ref().expect("connection initialized"), query))
                        .and_then(|window| export_window(&window, &destination))
                } else {
                    Err("No timeline history is available to export".into())
                };
                let _ = reply.send(response);
                Ok(())
            }
            TimelineCommand::Clear => ensure_connection(&mut connection, &path, true).and_then(|_| {
                clear_history(connection.as_ref().expect("connection initialized"))?;
                let mut current = status.lock();
                current.storage_bytes = storage_bytes(&path);
                current.last_write_ms = None;
                Ok(())
            }),
            TimelineCommand::Shutdown => break,
        };

        if let Err(error) = result {
            status.lock().last_error = Some(error);
        }
    }
}

fn timeline_db_path() -> Option<PathBuf> {
    crate::app_paths::timeline_db_path()
}

fn ensure_connection(connection: &mut Option<Connection>, path: &Path, create: bool) -> Result<(), String> {
    if connection.is_some() {
        return Ok(());
    }
    if !create && !path.is_file() {
        return Err("Timeline history does not exist".into());
    }
    if create {
        let parent = path
            .parent()
            .ok_or_else(|| "Timeline path has no parent directory".to_string())?;
        std::fs::create_dir_all(parent).map_err(|error| format!("Could not create timeline directory: {error}"))?;
    }
    let conn = Connection::open(path).map_err(|error| format!("Could not open timeline database: {error}"))?;
    conn.busy_timeout(Duration::from_secs(2))
        .map_err(|error| format!("Could not configure timeline timeout: {error}"))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA foreign_keys=ON;
         PRAGMA auto_vacuum=INCREMENTAL;",
    )
    .map_err(|error| format!("Could not configure timeline database: {error}"))?;
    migrate(&conn)?;
    *connection = Some(conn);
    Ok(())
}

fn migrate(conn: &Connection) -> Result<(), String> {
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| format!("Could not read timeline schema version: {error}"))?;
    if version > SCHEMA_VERSION {
        return Err(format!(
            "Timeline database schema {version} is newer than supported schema {SCHEMA_VERSION}"
        ));
    }
    if version == 0 {
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE metric_samples (
               timestamp_ms INTEGER PRIMARY KEY,
               cpu_pct REAL NOT NULL,
               memory_pct REAL NOT NULL,
               gpu_pct REAL,
               cpu_temp_c REAL,
               gpu_temp_c REAL,
               disk_read_bps REAL NOT NULL,
               disk_write_bps REAL NOT NULL,
               network_down_bps REAL NOT NULL,
               network_up_bps REAL NOT NULL,
               paused INTEGER NOT NULL CHECK(paused IN (0, 1))
             );
             CREATE TABLE process_samples (
               timestamp_ms INTEGER NOT NULL,
               pid INTEGER NOT NULL,
               start_time INTEGER NOT NULL,
               name TEXT NOT NULL,
               cpu_pct REAL NOT NULL,
               memory_bytes INTEGER NOT NULL,
               disk_read_bytes INTEGER NOT NULL,
               disk_write_bytes INTEGER NOT NULL,
               PRIMARY KEY(timestamp_ms, pid, start_time),
               FOREIGN KEY(timestamp_ms) REFERENCES metric_samples(timestamp_ms) ON DELETE CASCADE
             );
             CREATE TABLE timeline_events (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               timestamp_ms INTEGER NOT NULL,
               kind TEXT NOT NULL,
               source TEXT NOT NULL,
               severity TEXT NOT NULL,
               summary TEXT NOT NULL,
               evidence TEXT NOT NULL
             );
             CREATE INDEX idx_process_time ON process_samples(timestamp_ms);
             CREATE INDEX idx_event_time ON timeline_events(timestamp_ms);
             PRAGMA user_version=1;
             COMMIT;",
        )
        .map_err(|error| format!("Could not create timeline schema: {error}"))?;
    }
    Ok(())
}

fn write_snapshot(conn: &mut Connection, snapshot: &SystemSnapshot) -> Result<(), String> {
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

fn select_process_union(snapshot: &SystemSnapshot, timestamp_ms: i64, limit: usize) -> Vec<TimelineProcessSample> {
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

fn record_derived_events(
    conn: &Connection,
    snapshot: &SystemSnapshot,
    previous_providers: &mut HashMap<String, bool>,
    previous_paused: &mut Option<bool>,
    previous_power: &mut Option<String>,
) -> Result<(), String> {
    for (name, provider) in &snapshot.provider_status {
        if let Some(previous) = previous_providers.insert(name.clone(), provider.available) {
            if previous != provider.available {
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
    }

    if let Some(previous) = previous_paused.replace(snapshot.paused) {
        if previous != snapshot.paused {
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
    }

    let power = snapshot.battery.as_ref().map(|battery| {
        format!(
            "{}:{}",
            battery.status,
            battery.discharge_state.as_deref().unwrap_or("unknown")
        )
    });
    if let (Some(previous), Some(current)) = (previous_power.as_ref(), power.as_ref()) {
        if previous != current {
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
    }
    if power.is_some() {
        *previous_power = power;
    }
    Ok(())
}

fn insert_event(conn: &Connection, event: &TimelineEvent) -> Result<(), String> {
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

fn query_window(conn: &Connection, query: TimelineQuery) -> Result<TimelineWindow, String> {
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

fn prune(conn: &Connection, retention_days: u16, path: &Path) -> Result<(), String> {
    let cutoff = now_ms().saturating_sub(i64::from(validate_retention(retention_days)) * DAY_MS);
    conn.execute("DELETE FROM metric_samples WHERE timestamp_ms < ?1", [cutoff])
        .map_err(|error| format!("Could not prune timeline metrics: {error}"))?;
    conn.execute("DELETE FROM timeline_events WHERE timestamp_ms < ?1", [cutoff])
        .map_err(|error| format!("Could not prune timeline events: {error}"))?;

    // Return WAL pages before measuring the hard ceiling; otherwise deleted
    // rows can remain charged to the sidecar until an unrelated checkpoint.
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|error| format!("Could not checkpoint timeline database: {error}"))?;

    let mut attempts = 0;
    while storage_bytes(path) > MAX_DATABASE_BYTES && attempts < 64 {
        let oldest: Option<i64> = conn
            .query_row("SELECT MIN(timestamp_ms) FROM metric_samples", [], |row| row.get(0))
            .optional()
            .map_err(|error| format!("Could not inspect timeline size: {error}"))?
            .flatten();
        let Some(oldest) = oldest else {
            break;
        };
        let chunk_end = oldest.saturating_add(DAY_MS);
        conn.execute("DELETE FROM metric_samples WHERE timestamp_ms <= ?1", [chunk_end])
            .map_err(|error| format!("Could not cap timeline metrics: {error}"))?;
        conn.execute("DELETE FROM timeline_events WHERE timestamp_ms <= ?1", [chunk_end])
            .map_err(|error| format!("Could not cap timeline events: {error}"))?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|error| format!("Could not checkpoint capped timeline database: {error}"))?;
        attempts += 1;
    }
    conn.execute_batch("PRAGMA incremental_vacuum(2000); PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|error| format!("Could not compact timeline database: {error}"))?;
    Ok(())
}

fn clear_history(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         DELETE FROM process_samples;
         DELETE FROM metric_samples;
         DELETE FROM timeline_events;
         COMMIT;
         PRAGMA wal_checkpoint(TRUNCATE);
         PRAGMA incremental_vacuum;",
    )
    .map_err(|error| format!("Could not clear timeline history: {error}"))
}

fn export_window(window: &TimelineWindow, destination: &Path) -> Result<PathBuf, String> {
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

fn timestamp_rfc3339(timestamp_ms: i64) -> String {
    chrono::DateTime::<Utc>::from_timestamp_millis(timestamp_ms)
        .unwrap_or(chrono::DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339()
}

fn sanitize_text(mut value: String, max_chars: usize) -> String {
    value.retain(|character| character != '\0' && !character.is_control() || matches!(character, '\n' | '\t'));
    value.chars().take(max_chars).collect()
}

fn validate_retention(days: u16) -> u16 {
    match days {
        1 | 7 | 30 => days,
        _ => 7,
    }
}

fn now_ms() -> i64 {
    system_time_ms(SystemTime::now())
}

fn system_time_ms(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

fn storage_bytes(path: &Path) -> u64 {
    let mut total = path.metadata().map_or(0, |metadata| metadata.len());
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", path.display()));
        total = total.saturating_add(sidecar.metadata().map_or(0, |metadata| metadata.len()));
    }
    total
}

fn to_sql_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

fn from_sql_i64(value: i64) -> u64 {
    value.max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::snapshot::ProcessSnapshot;

    fn temp_db(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sysmon-timeline-{name}-{}.sqlite3", now_ms()))
    }

    fn snapshot(timestamp_ms: i64) -> SystemSnapshot {
        SystemSnapshot {
            sampled_at: UNIX_EPOCH + Duration::from_millis(timestamp_ms as u64),
            cpu_usage: 60.0,
            memory_percentage: 70.0,
            processes: vec![ProcessSnapshot {
                pid: 42,
                start_time: 1234,
                name: "worker.exe".into(),
                cpu_usage: 50.0,
                memory: 100,
                status: "Run".into(),
                disk_read_bytes: 20,
                disk_written_bytes: 30,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn retention_values_are_allowlisted() {
        assert_eq!(validate_retention(1), 1);
        assert_eq!(validate_retention(7), 7);
        assert_eq!(validate_retention(30), 30);
        assert_eq!(validate_retention(365), 7);
    }

    #[test]
    fn process_identity_includes_start_time() {
        let mut sample = snapshot(1_000);
        sample.processes.push(ProcessSnapshot {
            start_time: 9999,
            ..sample.processes[0].clone()
        });
        let selected = select_process_union(&sample, 1_000, 10);
        assert_eq!(selected.len(), 2);
        assert_ne!(selected[0].start_time, selected[1].start_time);
    }

    #[test]
    fn database_round_trip_and_schema_exclude_sensitive_fields() {
        let path = temp_db("round-trip");
        let mut conn = None;
        ensure_connection(&mut conn, &path, true).unwrap();
        write_snapshot(conn.as_mut().unwrap(), &snapshot(10_000)).unwrap();
        let window = query_window(
            conn.as_ref().unwrap(),
            TimelineQuery {
                start_ms: 0,
                end_ms: 20_000,
            },
        )
        .unwrap();
        assert_eq!(window.metrics.len(), 1);
        assert_eq!(window.processes[0].name, "worker.exe");

        let columns: String = conn
            .as_ref()
            .unwrap()
            .prepare("SELECT name FROM pragma_table_info('process_samples') ORDER BY cid")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join(",");
        for forbidden in ["command", "path", "cwd", "username", "remote_ip"] {
            assert!(!columns.contains(forbidden));
        }
        drop(conn);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn analysis_refuses_to_guess_without_baseline() {
        let window = TimelineWindow {
            query: TimelineQuery {
                start_ms: 0,
                end_ms: 10,
            },
            metrics: vec![TimelineMetricSample {
                timestamp_ms: 5,
                cpu_pct: 99.0,
                ..Default::default()
            }],
            ..Default::default()
        };
        let analysis = analyze_window(&window, 5);
        assert_eq!(analysis.title, "Insufficient baseline");
        assert_eq!(analysis.confidence, "low");
    }

    #[test]
    fn sanitization_removes_controls_and_limits_length() {
        assert_eq!(sanitize_text("ab\0cd\r\nef".into(), 5), "abcd\n");
    }

    #[test]
    fn timeline_event_serialization_round_trips() {
        let event = TimelineEvent::new(
            TimelineEventKind::ProviderRecovered,
            "gpu",
            "info",
            "GPU telemetry recovered",
            "The provider returned a valid sample.",
        );
        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: TimelineEvent = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.kind, TimelineEventKind::ProviderRecovered);
        assert_eq!(decoded.source, "gpu");
    }

    #[test]
    fn retention_prunes_expired_metrics_and_processes() {
        let path = temp_db("retention");
        let mut conn = None;
        ensure_connection(&mut conn, &path, true).unwrap();
        let current = now_ms();
        write_snapshot(conn.as_mut().unwrap(), &snapshot(current - 2 * DAY_MS)).unwrap();
        write_snapshot(conn.as_mut().unwrap(), &snapshot(current)).unwrap();
        prune(conn.as_ref().unwrap(), 1, &path).unwrap();

        let metric_count: i64 = conn
            .as_ref()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM metric_samples", [], |row| row.get(0))
            .unwrap();
        let process_count: i64 = conn
            .as_ref()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM process_samples", [], |row| row.get(0))
            .unwrap();
        assert_eq!(metric_count, 1);
        assert_eq!(process_count, 1);
        drop(conn);
        remove_database_files(&path);
    }

    #[test]
    fn confirmed_clear_removes_all_history_rows() {
        let path = temp_db("clear");
        let mut conn = None;
        ensure_connection(&mut conn, &path, true).unwrap();
        write_snapshot(conn.as_mut().unwrap(), &snapshot(now_ms())).unwrap();
        insert_event(
            conn.as_ref().unwrap(),
            &TimelineEvent::new(
                TimelineEventKind::MonitoringPaused,
                "monitor",
                "info",
                "Paused",
                "User request",
            ),
        )
        .unwrap();
        clear_history(conn.as_ref().unwrap()).unwrap();
        for table in ["metric_samples", "process_samples", "timeline_events"] {
            let count: i64 = conn
                .as_ref()
                .unwrap()
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
                .unwrap();
            assert_eq!(count, 0, "{table} was not cleared");
        }
        drop(conn);
        remove_database_files(&path);
    }

    #[test]
    fn corrupt_database_is_reported_without_panicking() {
        let path = temp_db("corrupt");
        std::fs::write(&path, b"not a sqlite database").unwrap();
        let mut conn = None;
        assert!(ensure_connection(&mut conn, &path, false).is_err());
        remove_database_files(&path);
    }

    fn remove_database_files(path: &Path) {
        let _ = std::fs::remove_file(path);
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }
}
