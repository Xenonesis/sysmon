use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::app::actions::ActionAuditRecord;

const MAX_LOG_BYTES: u64 = 4 * 1024 * 1024;
const MAX_RECORD_BYTES: usize = 64 * 1024;
const MAX_RECENT_RECORDS: usize = 500;
static LOG_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

fn log_path() -> Option<PathBuf> {
    crate::app_paths::action_log_path()
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("jsonl.1")
}

pub(crate) fn append(record: &ActionAuditRecord) -> Result<(), std::io::Error> {
    let path = log_path().ok_or_else(|| std::io::Error::other("application data directory unavailable"))?;
    let _guard = LOG_LOCK.lock();
    append_at(&path, record, MAX_LOG_BYTES)
}

fn append_at(path: &Path, record: &ActionAuditRecord, cap: u64) -> Result<(), std::io::Error> {
    let mut bytes = serde_json::to_vec(record).map_err(std::io::Error::other)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_RECORD_BYTES || bytes.len() as u64 > cap {
        return Err(std::io::Error::other(
            "action audit record exceeds the bounded log record limit",
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => return Err(error),
    };
    if existing.saturating_add(bytes.len() as u64) > cap {
        let backup = backup_path(path);
        match fs::remove_file(&backup) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        // Legacy unbounded logs retain only their newest bounded tail, without a full read.
        if existing > cap {
            let mut source = fs::File::open(path)?;
            source.seek(SeekFrom::Start(existing - cap))?;
            let mut reader = BufReader::new(source);
            let mut partial = Vec::new();
            reader.read_until(b'\n', &mut partial)?;
            let mut target = OpenOptions::new().write(true).create_new(true).open(&backup)?;
            std::io::copy(&mut reader.take(cap), &mut target)?;
            target.sync_data()?;
            fs::remove_file(path)?;
        } else {
            fs::rename(path, backup)?;
        }
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(&bytes)?;
    file.sync_data()
}

pub(crate) fn load_recent(limit: usize) -> Vec<ActionAuditRecord> {
    let Some(path) = log_path() else {
        return Vec::new();
    };
    let _guard = LOG_LOCK.lock();
    load_recent_at(&path, limit.min(MAX_RECENT_RECORDS))
}

fn load_recent_at(path: &Path, limit: usize) -> Vec<ActionAuditRecord> {
    if limit == 0 {
        return Vec::new();
    }
    let mut records = VecDeque::with_capacity(limit);
    for path in [backup_path(path), path.to_path_buf()] {
        let Ok(mut file) = fs::File::open(path) else {
            continue;
        };
        let Ok(metadata) = file.metadata() else {
            continue;
        };
        let start = metadata.len().saturating_sub(MAX_LOG_BYTES);
        if file.seek(SeekFrom::Start(start)).is_err() {
            continue;
        }
        let mut reader = BufReader::new(file.take(MAX_LOG_BYTES));
        if start > 0 {
            let mut partial = Vec::new();
            if reader.read_until(b'\n', &mut partial).is_err() {
                continue;
            }
        }
        for line in reader.lines().map_while(Result::ok) {
            if line.len() > MAX_RECORD_BYTES {
                continue;
            }
            if let Ok(record) = serde_json::from_str(&line) {
                if records.len() == limit {
                    records.pop_front();
                }
                records.push_back(record);
            }
        }
    }
    records.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_preserves_latest_actions_and_bounds_both_files() {
        let root = std::env::temp_dir().join(format!(
            "sysmon-action-log-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let path = root.join("actions.jsonl");
        for index in 0..20 {
            append_at(
                &path,
                &ActionAuditRecord::automatic(format!("action-{index}"), "ok"),
                1024,
            )
            .unwrap();
        }
        assert!(fs::metadata(&path).unwrap().len() <= 1024);
        assert!(fs::metadata(backup_path(&path)).unwrap().len() <= 1024);
        let recent = load_recent_at(&path, 2);
        assert_eq!(
            recent.iter().map(|r| r.action.as_str()).collect::<Vec<_>>(),
            ["action-18", "action-19"]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oversized_records_do_not_destroy_existing_audit() {
        let root = std::env::temp_dir().join(format!(
            "sysmon-action-log-large-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let path = root.join("actions.jsonl");
        append_at(&path, &ActionAuditRecord::automatic("kept", "ok"), 1024).unwrap();
        assert!(
            append_at(
                &path,
                &ActionAuditRecord::automatic("too large", "x".repeat(MAX_RECORD_BYTES)),
                1024
            )
            .is_err()
        );
        assert_eq!(load_recent_at(&path, 1)[0].action, "kept");
        fs::remove_dir_all(root).unwrap();
    }
}
