use super::query::{query_selected_window, query_window_page};
use super::records::{insert_event, record_derived_events, write_snapshot};
use super::*;
use parking_lot::Mutex;
use rusqlite::Connection;
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
        while let Ok(command) = receiver.recv() {
            let error = "Local application-data directory is unavailable".to_string();
            match command {
                TimelineCommand::Query { reply, .. } => {
                    let _ = reply.send(Err(error));
                }
                TimelineCommand::Export { reply, .. } => {
                    let _ = reply.send(Err(error));
                }
                TimelineCommand::Clear { reply } => {
                    let _ = reply.send(Err(error));
                }
                TimelineCommand::Shutdown => break,
                _ => {}
            }
        }
        return;
    };
    let mut connection: Option<Connection> = None;
    let mut last_prune = Instant::now()
        .checked_sub(Duration::from_secs(86_400))
        .unwrap_or_else(Instant::now);
    let mut previous_providers: HashMap<String, bool> = HashMap::new();
    let mut previous_power: Option<String> = None;

    if enabled && let Err(error) = ensure_connection(&mut connection, &path, true) {
        status.lock().last_error = Some(error);
    }

    loop {
        let command = match receiver.recv_timeout(Duration::from_secs(60)) {
            Ok(command) => command,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if path.is_file() {
                    let result = ensure_connection(&mut connection, &path, false).and_then(|_| {
                        prune(
                            connection.as_ref().expect("connection initialized"),
                            retention_days,
                            &path,
                        )
                    });
                    let mut current = status.lock();
                    current.storage_bytes = storage_bytes(&path);
                    if let Err(error) = result {
                        current.last_error = Some(error);
                    }
                }
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let result = match command {
            TimelineCommand::RecordSnapshot(snapshot) if enabled => ensure_connection(&mut connection, &path, true)
                .and_then(|_| {
                    let conn = connection.as_mut().expect("connection initialized");
                    record_derived_events(conn, &snapshot, &mut previous_providers, &mut previous_power)?;
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
            TimelineCommand::RecordEvent(event) if enabled => {
                ensure_connection(&mut connection, &path, true).and_then(|_| {
                    let conn = connection.as_ref().expect("connection initialized");
                    insert_event(conn, &event)?;
                    if last_prune.elapsed() >= Duration::from_secs(60) || storage_bytes(&path) > MAX_DATABASE_BYTES {
                        prune(conn, retention_days, &path)?;
                        last_prune = Instant::now();
                    }
                    status.lock().storage_bytes = storage_bytes(&path);
                    Ok(())
                })
            }
            TimelineCommand::RecordEvent(_) => Ok(()),
            TimelineCommand::SetPolicy {
                enabled: new_enabled,
                retention_days: new_retention,
            } => {
                enabled = new_enabled;
                retention_days = validate_retention(new_retention);
                if enabled || path.is_file() {
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
            TimelineCommand::Query {
                query,
                events_offset,
                reply,
            } => {
                let response = if path.is_file() {
                    ensure_connection(&mut connection, &path, false).and_then(|_| {
                        query_window_page(
                            connection.as_ref().expect("connection initialized"),
                            query,
                            events_offset,
                        )
                    })
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
                selection,
                destination,
                reply,
            } => {
                let response = if path.is_file() {
                    ensure_connection(&mut connection, &path, false)
                        .and_then(|_| {
                            query_selected_window(connection.as_ref().expect("connection initialized"), selection)
                        })
                        .and_then(|window| export_window(&window, selection, &destination))
                } else {
                    Err("No timeline history is available to export".into())
                };
                let _ = reply.send(response);
                Ok(())
            }
            TimelineCommand::Clear { reply } => {
                let response = ensure_connection(&mut connection, &path, true).and_then(|_| {
                    clear_history(connection.as_ref().expect("connection initialized"))?;
                    let mut current = status.lock();
                    current.storage_bytes = storage_bytes(&path);
                    current.last_write_ms = None;
                    current.last_error = None;
                    Ok(())
                });
                if let Err(error) = &response {
                    status.lock().last_error = Some(error.clone());
                }
                let _ = reply.send(response);
                Ok(())
            }
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
         PRAGMA auto_vacuum=INCREMENTAL;
         PRAGMA secure_delete=ON;
         PRAGMA wal_autocheckpoint=256;
         PRAGMA journal_size_limit=1048576;",
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
    if version < 2 {
        // Legacy event fields were unrestricted diagnostic text. Do not reinterpret them as safe.
        let transaction = conn
            .unchecked_transaction()
            .map_err(|e| format!("Could not migrate privacy policy: {e}"))?;
        transaction
            .execute(
                "UPDATE timeline_events SET source='system', severity='info',
             summary='Legacy event (details removed for privacy)', evidence='{}'",
                [],
            )
            .map_err(|e| format!("Could not redact legacy timeline events: {e}"))?;
        transaction
            .execute("UPDATE process_samples SET name='[redacted legacy process]'", [])
            .map_err(|e| format!("Could not redact legacy process data: {e}"))?;
        transaction
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|e| format!("Could not set timeline schema version: {e}"))?;
        transaction
            .commit()
            .map_err(|e| format!("Could not commit timeline migration: {e}"))?;
        // VACUUM is required once when upgrading databases that predate auto_vacuum.
        conn.execute_batch("PRAGMA auto_vacuum=INCREMENTAL; VACUUM;")
            .map_err(|e| format!("Could not reclaim legacy timeline payloads: {e}"))?;
        checkpoint(conn)?;
    }
    Ok(())
}

pub(super) fn prune(conn: &Connection, retention_days: u16, path: &Path) -> Result<(), String> {
    prune_with_cap(conn, retention_days, path, MAX_DATABASE_BYTES)
}

fn checkpoint(conn: &Connection) -> Result<(), String> {
    let busy: i64 = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
        .map_err(|e| format!("Could not checkpoint timeline: {e}"))?;
    if busy != 0 {
        return Err("Timeline checkpoint is busy; retention will retry".into());
    }
    Ok(())
}

fn live_bytes(conn: &Connection) -> Result<u64, String> {
    let read = |pragma| {
        conn.query_row(pragma, [], |row| row.get::<_, i64>(0))
            .map(|v| v as u64)
            .map_err(|e| e.to_string())
    };
    Ok(read("PRAGMA page_count")?.saturating_sub(read("PRAGMA freelist_count")?) * read("PRAGMA page_size")?)
}

fn reclaim(conn: &Connection) -> Result<(), String> {
    // SQLite can return after one reclaimed page; keep stepping in bounded batches.
    for _ in 0..128 {
        let free: i64 = conn
            .query_row("PRAGMA freelist_count", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if free == 0 {
            break;
        }
        conn.execute_batch("PRAGMA incremental_vacuum(256)")
            .map_err(|e| e.to_string())?;
        let after: i64 = conn
            .query_row("PRAGMA freelist_count", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if after >= free {
            break;
        }
    }
    checkpoint(conn)
}

pub(super) fn prune_with_cap(conn: &Connection, retention_days: u16, path: &Path, cap: u64) -> Result<(), String> {
    let cutoff = now_ms().saturating_sub(i64::from(validate_retention(retention_days)) * DAY_MS);
    // Each transaction deletes at most 256 metric samples (and their bounded process union)
    // plus 256 events. Recheck live pages, never charge freed pages to younger history.
    for _ in 0..4096 {
        let transaction = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        let metrics = transaction
            .execute(
                "DELETE FROM metric_samples WHERE timestamp_ms IN
             (SELECT timestamp_ms FROM metric_samples WHERE timestamp_ms < ?1 ORDER BY timestamp_ms LIMIT 256)",
                [cutoff],
            )
            .map_err(|e| e.to_string())?;
        let events = transaction
            .execute(
                "DELETE FROM timeline_events WHERE id IN
             (SELECT id FROM timeline_events WHERE timestamp_ms < ?1 ORDER BY timestamp_ms,id LIMIT 256)",
                [cutoff],
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())?;
        if metrics + events == 0 {
            break;
        }
        checkpoint(conn)?;
    }
    reclaim(conn)?;
    for _ in 0..4096 {
        if live_bytes(conn)? <= cap {
            break;
        }
        let transaction = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        // Delete oldest records across both tables; event-only histories also make progress.
        let oldest_metric: Option<i64> = transaction
            .query_row("SELECT MIN(timestamp_ms) FROM metric_samples", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let oldest_event: Option<i64> = transaction
            .query_row("SELECT MIN(timestamp_ms) FROM timeline_events", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let deleted = match (oldest_metric, oldest_event) {
            (None, None) => 0,
            (Some(metric), event) if event.is_none_or(|event| metric <= event) => transaction.execute(
                "DELETE FROM metric_samples WHERE timestamp_ms=(SELECT MIN(timestamp_ms) FROM metric_samples)", [],
            ).map_err(|e| e.to_string())?,
            _ => transaction.execute(
                "DELETE FROM timeline_events WHERE id IN (SELECT id FROM timeline_events ORDER BY timestamp_ms,id LIMIT 1)", [],
            ).map_err(|e| e.to_string())?,
        };
        transaction.commit().map_err(|e| e.to_string())?;
        if deleted == 0 {
            break;
        }
        reclaim(conn)?;
    }
    reclaim(conn)?;
    if live_bytes(conn)? > cap {
        return Err("Timeline live storage exceeds retention cap; maintenance will retry".into());
    }
    if storage_bytes(path) > cap.saturating_add(1024 * 1024) {
        return Err("Timeline has reclaimable storage; incremental maintenance will continue".into());
    }
    Ok(())
}

pub(super) fn clear_history(conn: &Connection) -> Result<(), String> {
    let transaction = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    transaction
        .execute_batch("DELETE FROM process_samples; DELETE FROM metric_samples; DELETE FROM timeline_events;")
        .map_err(|e| format!("Could not clear timeline history: {e}"))?;
    transaction
        .commit()
        .map_err(|e| format!("Could not commit timeline clear: {e}"))?;
    reclaim(conn)
}
