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
        let root = directories::ProjectDirs::from("com", "Xenonesis", "SystemMonitor")
            .ok_or_else(|| std::io::Error::other("application data directory unavailable"))?
            .data_local_dir()
            .join("sessions");
        fs::create_dir_all(&root)?;
        let name = format!("session-{}.jsonl", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
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
