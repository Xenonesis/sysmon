use super::query::query_window;
use super::records::{insert_event, record_derived_events, write_snapshot};
use super::*;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

pub(super) fn run_worker(
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

    if enabled && let Err(error) = ensure_connection(&mut connection, &path, true) {
        status.lock().last_error = Some(error);
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

pub(super) fn timeline_db_path() -> Option<PathBuf> {
    crate::app_paths::timeline_db_path()
}

pub(super) fn ensure_connection(connection: &mut Option<Connection>, path: &Path, create: bool) -> Result<(), String> {
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

pub(super) fn prune(conn: &Connection, retention_days: u16, path: &Path) -> Result<(), String> {
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

pub(super) fn clear_history(conn: &Connection) -> Result<(), String> {
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
