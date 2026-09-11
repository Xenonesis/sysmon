use crate::monitoring::SystemSnapshot;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod export;
mod query;
mod records;
mod worker;

pub(crate) use export::export_window;
pub(crate) use query::analyze_window;
use worker::{run_worker, timeline_db_path};

const SCHEMA_VERSION: i64 = 2;
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
    ThirtyDays,
}

impl TimelineRange {
    pub(crate) const ALL: [Self; 6] = [
        Self::FifteenMinutes,
        Self::OneHour,
        Self::SixHours,
        Self::OneDay,
        Self::SevenDays,
        Self::ThirtyDays,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::FifteenMinutes => "15m",
            Self::OneHour => "1h",
            Self::SixHours => "6h",
            Self::OneDay => "24h",
            Self::SevenDays => "7d",
            Self::ThirtyDays => "30d",
        }
    }

    pub(crate) fn duration_ms(self) -> i64 {
        match self {
            Self::FifteenMinutes => 15 * 60_000,
            Self::OneHour => 60 * 60_000,
            Self::SixHours => 6 * 60 * 60_000,
            Self::OneDay => DAY_MS,
            Self::SevenDays => 7 * DAY_MS,
            Self::ThirtyDays => 30 * DAY_MS,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TimelineQuery {
    pub(crate) start_ms: i64,
    pub(crate) end_ms: i64,
}

impl TimelineQuery {
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
            source: source.into(),
            severity: severity.into(),
            summary: summary.into(),
            evidence: evidence.into(),
        }
        .privacy_safe()
    }

    pub(crate) fn from_audit(record: &crate::app::actions::ActionAuditRecord) -> Self {
        let timestamp_ms = chrono::DateTime::parse_from_rfc3339(&record.timestamp)
            .map(|timestamp| timestamp.timestamp_millis())
            .unwrap_or_else(|_| now_ms());
        let mut event = Self::new(
            if record.succeeded {
                TimelineEventKind::ActionSucceeded
            } else {
                TimelineEventKind::ActionFailed
            },
            "guarded_action",
            record.risk.label().to_ascii_lowercase(),
            "",
            "",
        );
        event.timestamp_ms = timestamp_ms;
        event.evidence = serde_json::json!({
            "version": 1,
            "initiator": if record.initiator == "automatic policy" { "automatic policy" } else { "user" }
        })
        .to_string();
        event
    }

    /// Never retain arbitrary producer text, including errors and legacy payloads.
    pub(crate) fn privacy_safe(&self) -> Self {
        let (source, summary) = match self.kind {
            TimelineEventKind::AlertTriggered => ("alerts", "Resource alert triggered"),
            TimelineEventKind::AlertResolved => ("alerts", "Resource alert resolved"),
            TimelineEventKind::ActionSucceeded => ("guarded_action", "Guarded action succeeded"),
            TimelineEventKind::ActionFailed => ("guarded_action", "Guarded action failed"),
            TimelineEventKind::ProviderUnavailable => ("telemetry", "Telemetry provider unavailable"),
            TimelineEventKind::ProviderRecovered => ("telemetry", "Telemetry provider recovered"),
            TimelineEventKind::MonitoringPaused => ("monitoring", "Monitoring paused"),
            TimelineEventKind::MonitoringResumed => ("monitoring", "Monitoring resumed"),
            TimelineEventKind::PowerChanged => ("power", "Power state changed"),
            TimelineEventKind::ServiceChanged => ("services", "Service inventory changed"),
            TimelineEventKind::StartupChanged => ("startup", "Startup inventory changed"),
            TimelineEventKind::System => ("system", "System event"),
        };
        let payload = serde_json::from_str::<serde_json::Value>(&self.evidence).ok();
        let mut evidence = serde_json::json!({"version": 1});
        if let Some(payload) = payload.filter(|p| p.get("version").and_then(|v| v.as_u64()) == Some(1)) {
            if matches!(
                self.kind,
                TimelineEventKind::ActionSucceeded | TimelineEventKind::ActionFailed
            ) && let Some(initiator @ ("user" | "automatic policy")) =
                payload.get("initiator").and_then(|v| v.as_str())
            {
                evidence["initiator"] = initiator.into();
            }
            if matches!(
                self.kind,
                TimelineEventKind::ServiceChanged | TimelineEventKind::StartupChanged
            ) {
                for key in ["added", "removed", "changed"] {
                    if let Some(count) = payload.get(key).and_then(|v| v.as_u64()) {
                        evidence[key] = count.into();
                    }
                }
            }
        }
        Self {
            id: self.id,
            timestamp_ms: self.timestamp_ms,
            kind: self.kind,
            source: source.into(),
            severity: match self.severity.as_str() {
                "info" | "low" => "info",
                "warning" | "medium" => "warning",
                "critical" | "high" => "critical",
                _ => "info",
            }
            .into(),
            summary: summary.into(),
            evidence: evidence.to_string(),
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
    pub(crate) total_events: u64,
    pub(crate) events_offset: u64,
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
    pub(crate) window: Option<Arc<TimelineWindow>>,
    pub(crate) selected_timestamp_ms: Option<i64>,
    pub(crate) selected_event_id: Option<i64>,
    pub(crate) range_end_ms: Option<i64>,
    pub(crate) events_offset: u64,
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
            selected_event_id: None,
            range_end_ms: None,
            events_offset: 0,
            last_refresh: None,
            clear_confirmation: false,
            message: None,
            active_alert_keys: std::collections::HashSet::new(),
            service_states: None,
            startup_states: None,
        }
    }
}

impl TimelineUiState {
    pub(crate) fn query(&self) -> TimelineQuery {
        let end_ms = self.range_end_ms.unwrap_or_else(now_ms);
        TimelineQuery {
            start_ms: end_ms.saturating_sub(self.range.duration_ms()),
            end_ms,
        }
    }

    pub(crate) fn service_inventory_event(
        &mut self,
        current: Option<HashMap<String, String>>,
    ) -> Option<TimelineEvent> {
        inventory_event(&mut self.service_states, current?, TimelineEventKind::ServiceChanged)
    }

    pub(crate) fn startup_inventory_event(&mut self, current: Option<HashMap<String, bool>>) -> Option<TimelineEvent> {
        inventory_event(&mut self.startup_states, current?, TimelineEventKind::StartupChanged)
    }
}

fn inventory_event<T: PartialEq>(
    previous: &mut Option<HashMap<String, T>>,
    current: HashMap<String, T>,
    kind: TimelineEventKind,
) -> Option<TimelineEvent> {
    let old = previous.replace(current)?;
    let current = previous.as_ref()?;
    let added = current.keys().filter(|key| !old.contains_key(*key)).count();
    let removed = old.keys().filter(|key| !current.contains_key(*key)).count();
    let changed = current
        .iter()
        .filter(|(key, value)| old.get(*key).is_some_and(|old| old != *value))
        .count();
    if added + removed + changed == 0 {
        return None;
    }
    let mut event = TimelineEvent::new(kind, "", "info", "", "");
    event.evidence =
        serde_json::json!({"version": 1, "added": added, "removed": removed, "changed": changed}).to_string();
    Some(event)
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct IncidentSelection {
    pub(crate) query: TimelineQuery,
    pub(crate) timestamp_ms: i64,
    pub(crate) event_id: Option<i64>,
    pub(crate) events_offset: u64,
}

#[derive(Default)]
struct QueryState {
    generation: u64,
    active: Option<(u64, Receiver<Result<TimelineWindow, String>>)>,
    desired: Option<(TimelineQuery, u64)>,
    result: Option<Result<TimelineWindow, String>>,
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
        events_offset: u64,
        reply: SyncSender<Result<TimelineWindow, String>>,
    },
    Export {
        selection: IncidentSelection,
        destination: PathBuf,
        reply: SyncSender<Result<PathBuf, String>>,
    },
    Clear {
        reply: SyncSender<Result<(), String>>,
    },
    Shutdown,
}

#[derive(Clone)]
pub(crate) struct TimelineHandle {
    sender: Sender<TimelineCommand>,
    status: Arc<Mutex<TimelineStatus>>,
    query: Arc<Mutex<QueryState>>,
    clear_result: ClearResultSlot,
    monitoring_paused: Arc<Mutex<bool>>,
    export_result: Arc<Mutex<Option<Result<PathBuf, String>>>>,
    export_in_flight: Arc<AtomicBool>,
    last_snapshot_queued: Arc<Mutex<Option<Instant>>>,
}

type ClearResultSlot = Arc<Mutex<Option<Receiver<Result<(), String>>>>>;
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
            query: Arc::new(Mutex::new(QueryState::default())),
            clear_result: Arc::new(Mutex::new(None)),
            monitoring_paused: Arc::new(Mutex::new(false)),
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

    pub(crate) fn record_monitoring_transition(&self, paused: bool) {
        let mut previous = self.monitoring_paused.lock();
        if *previous != paused {
            *previous = paused;
            self.record_event(TimelineEvent::new(
                if paused {
                    TimelineEventKind::MonitoringPaused
                } else {
                    TimelineEventKind::MonitoringResumed
                },
                "monitoring",
                "info",
                "",
                "",
            ));
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
        self.request_window_page(query, 0);
    }

    pub(crate) fn request_window_page(&self, query: TimelineQuery, offset: u64) {
        let mut state = self.query.lock();
        state.generation = state.generation.wrapping_add(1);
        state.result = None;
        state.desired = Some((query.validated(), offset));
        if !self.clear_in_flight() {
            self.dispatch_query(&mut state);
        }
    }

    fn dispatch_query(&self, state: &mut QueryState) {
        if state.active.is_some() {
            return;
        }
        if let Some((query, events_offset)) = state.desired.take() {
            let (reply, receiver) = mpsc::sync_channel(1);
            if self
                .sender
                .send(TimelineCommand::Query {
                    query,
                    events_offset,
                    reply,
                })
                .is_err()
            {
                state.result = Some(Err("Timeline worker is unavailable".into()));
            } else {
                state.active = Some((state.generation, receiver));
            }
        }
    }

    pub(crate) fn query_in_flight(&self) -> bool {
        let state = self.query.lock();
        state.active.is_some() || state.desired.is_some()
    }

    pub(crate) fn take_query_result(&self) -> Option<Result<TimelineWindow, String>> {
        let mut state = self.query.lock();
        if let Some((generation, receiver)) = &state.active {
            let result = match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Disconnected) => Some(Err("Timeline query was interrupted".into())),
                Err(TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                if *generation == state.generation {
                    state.result = Some(result);
                }
                state.active = None;
            }
        }
        if !self.clear_in_flight() {
            self.dispatch_query(&mut state);
        }
        state.result.take()
    }

    pub(crate) fn request_export(&self, selection: IncidentSelection, destination: PathBuf) {
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
                        selection,
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
        // Lock order matches query polling. Reject every result begun before this request.
        let mut state = self.query.lock();
        let mut pending = self.clear_result.lock();
        if pending.is_some() {
            return;
        }
        state.generation = state.generation.wrapping_add(1);
        state.result = None;
        state.desired = None;
        let (reply, receiver) = mpsc::sync_channel(1);
        if self
            .sender
            .send(TimelineCommand::Clear { reply: reply.clone() })
            .is_err()
        {
            let _ = reply.send(Err("Timeline worker is unavailable".into()));
        }
        *pending = Some(receiver);
    }

    pub(crate) fn clear_in_flight(&self) -> bool {
        self.clear_result.lock().is_some()
    }

    pub(crate) fn take_clear_result(&self) -> Option<Result<(), String>> {
        let mut pending = self.clear_result.lock();
        let receiver = pending.as_ref()?;
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("Timeline clear was interrupted".into()),
        };
        *pending = None;
        Some(result)
    }

    pub(crate) fn shutdown(&self) {
        let _ = self.sender.send(TimelineCommand::Shutdown);
    }
}

fn sanitize_text(mut value: String, max_chars: usize) -> String {
    value.retain(|character| character != '\0' && !character.is_control() || matches!(character, '\n' | '\t'));
    value.chars().take(max_chars).collect()
}

pub(super) fn validate_retention(days: u16) -> u16 {
    match days {
        1 | 7 | 30 => days,
        _ => 7,
    }
}

pub(super) fn now_ms() -> i64 {
    system_time_ms(SystemTime::now())
}

pub(super) fn system_time_ms(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

pub(super) fn storage_bytes(path: &Path) -> u64 {
    let mut total = path.metadata().map_or(0, |metadata| metadata.len());
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", path.display()));
        total = total.saturating_add(sidecar.metadata().map_or(0, |metadata| metadata.len()));
    }
    total
}

pub(super) fn to_sql_i64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

pub(super) fn from_sql_i64(value: i64) -> u64 {
    value.max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::query::query_window;
    use super::records::{insert_event, select_process_union, write_snapshot};
    use super::worker::ensure_connection;
    use super::worker::{clear_history, prune};
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
                identity: None,
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
        let safe = event.privacy_safe();
        let encoded = serde_json::to_string(&safe).unwrap();
        let decoded: TimelineEvent = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.kind, TimelineEventKind::ProviderRecovered);
        assert_eq!(decoded.source, "telemetry");
        assert_eq!(decoded.summary, "Telemetry provider recovered");
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
