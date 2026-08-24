use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::app::actions::ActionAuditRecord;

fn log_path() -> Option<PathBuf> {
    crate::app_paths::action_log_path()
}

pub(crate) fn append(record: &ActionAuditRecord) -> Result<(), std::io::Error> {
    let path = log_path().ok_or_else(|| std::io::Error::other("application data directory unavailable"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, record).map_err(std::io::Error::other)?;
    file.write_all(b"\n")?;
    file.sync_data()
}

pub(crate) fn load_recent(limit: usize) -> Vec<ActionAuditRecord> {
    let Some(path) = log_path() else {
        return Vec::new();
    };
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    let mut records: Vec<_> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect();
    if records.len() > limit {
        records.drain(0..records.len() - limit);
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_log_round_trip_is_hermetic_and_bounded() {
        let root = std::env::temp_dir().join(format!("sysmon-action-log-test-{}", std::process::id()));
        crate::app_paths::with_test_data_local_dir(root.clone(), || {
            append(&ActionAuditRecord::automatic("first", "ok")).unwrap();
            append(&ActionAuditRecord::automatic("second", "ok")).unwrap();
            let recent = load_recent(1);
            assert_eq!(recent.len(), 1);
            assert_eq!(recent[0].action, "second");
            assert_eq!(recent[0].initiator, "automatic policy");
        });
        std::fs::remove_dir_all(root).unwrap();
    }
}
