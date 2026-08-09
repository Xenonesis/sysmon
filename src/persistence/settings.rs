use std::fs;
use std::io::Write;
use std::path::Path;

use crate::AppSettings;

#[derive(Debug)]
pub(crate) enum SettingsError { Io(std::io::Error), Json(serde_json::Error) }
impl std::fmt::Display for SettingsError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { Self::Io(e) => e.fmt(f), Self::Json(e) => e.fmt(f) } } }
impl std::error::Error for SettingsError {}
impl From<std::io::Error> for SettingsError { fn from(e: std::io::Error) -> Self { Self::Io(e) } }
impl From<serde_json::Error> for SettingsError { fn from(e: serde_json::Error) -> Self { Self::Json(e) } }

pub(crate) fn validated(mut settings: AppSettings) -> AppSettings {
    settings.refresh_interval = settings.refresh_interval.clamp(1, 10);
    settings.process_count = settings.process_count.clamp(5, 100);
    settings.notification_cpu_threshold = settings.notification_cpu_threshold.clamp(50.0, 100.0);
    settings.notification_memory_threshold = settings.notification_memory_threshold.clamp(50.0, 100.0);
    settings.notification_temp_threshold = settings.notification_temp_threshold.clamp(60, 105);
    settings.ram_clean_threshold = settings.ram_clean_threshold.clamp(50.0, 100.0);
    settings.auto_clean_target = settings.auto_clean_target.clamp(30.0, 95.0);
    settings.auto_clean_max_mb = settings.auto_clean_max_mb.min(4096);
    settings.auto_clean_interval = settings.auto_clean_interval.max(30);
    settings
}

pub(crate) fn load(path: &Path) -> Result<AppSettings, SettingsError> {
    Ok(validated(serde_json::from_str(&fs::read_to_string(path)?)?))
}

pub(crate) fn save(path: &Path, settings: &AppSettings) -> Result<(), SettingsError> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(&validated(settings.clone()))?;
    let mut file = fs::File::create(&tmp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    fs::rename(tmp, path)?;
    Ok(())
}
