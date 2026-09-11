use std::fs;
use std::io::Write;
use std::path::Path;

use crate::AppSettings;

pub(crate) const REFRESH_INTERVAL: std::ops::RangeInclusive<u64> = 1..=10;
pub(crate) const PROCESS_COUNT: std::ops::RangeInclusive<usize> = 5..=100;
pub(crate) const ALERT_PERCENT: std::ops::RangeInclusive<f32> = 50.0..=100.0;
pub(crate) const ALERT_TEMPERATURE: std::ops::RangeInclusive<u32> = 60..=105;
pub(crate) const CLEAN_THRESHOLD: std::ops::RangeInclusive<f32> = 50.0..=100.0;
pub(crate) const CLEAN_TARGET: std::ops::RangeInclusive<f32> = 30.0..=95.0;
pub(crate) const CLEAN_BUDGET_MB: std::ops::RangeInclusive<u64> = 0..=4096;
pub(crate) const CLEAN_INTERVAL: std::ops::RangeInclusive<u64> = 30..=7200;

fn finite_percent(value: f32, range: std::ops::RangeInclusive<f32>, default: f32) -> f32 {
    if value.is_finite() {
        value.clamp(*range.start(), *range.end())
    } else {
        default
    }
}

#[derive(Debug)]
pub(crate) enum SettingsError {
    Io(std::io::Error),
    Json(serde_json::Error),
}
impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Json(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for SettingsError {}
impl From<std::io::Error> for SettingsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for SettingsError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

pub(crate) fn validated(mut settings: AppSettings) -> AppSettings {
    settings.refresh_interval = settings
        .refresh_interval
        .clamp(*REFRESH_INTERVAL.start(), *REFRESH_INTERVAL.end());
    settings.process_count = settings
        .process_count
        .clamp(*PROCESS_COUNT.start(), *PROCESS_COUNT.end());
    settings.notification_cpu_threshold = finite_percent(settings.notification_cpu_threshold, ALERT_PERCENT, 90.0);
    settings.notification_memory_threshold =
        finite_percent(settings.notification_memory_threshold, ALERT_PERCENT, 90.0);
    settings.notification_temp_threshold = settings
        .notification_temp_threshold
        .clamp(*ALERT_TEMPERATURE.start(), *ALERT_TEMPERATURE.end());
    settings.ram_clean_threshold = finite_percent(settings.ram_clean_threshold, CLEAN_THRESHOLD, 85.0);
    settings.auto_clean_target = finite_percent(settings.auto_clean_target, CLEAN_TARGET, 70.0);
    settings.auto_clean_max_mb = settings
        .auto_clean_max_mb
        .clamp(*CLEAN_BUDGET_MB.start(), *CLEAN_BUDGET_MB.end());
    settings.auto_clean_interval = settings
        .auto_clean_interval
        .clamp(*CLEAN_INTERVAL.start(), *CLEAN_INTERVAL.end());
    settings.notification_disk_threshold = finite_percent(settings.notification_disk_threshold, ALERT_PERCENT, 90.0);
    if !matches!(settings.timeline_retention_days, 1 | 7 | 30) {
        settings.timeline_retention_days = 7;
    }
    settings
}

pub(crate) fn load(path: &Path) -> Result<AppSettings, SettingsError> {
    Ok(validated(serde_json::from_str(&fs::read_to_string(path)?)?))
}

pub(crate) fn save(path: &Path, settings: &AppSettings) -> Result<(), SettingsError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(settings)?;
    let mut file = fs::File::create(&tmp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonfinite_and_unbounded_policy_is_normalized_before_use() {
        let settings = validated(AppSettings {
            notification_cpu_threshold: f32::NAN,
            notification_memory_threshold: f32::INFINITY,
            notification_disk_threshold: f32::NEG_INFINITY,
            ram_clean_threshold: f32::NAN,
            auto_clean_target: f32::INFINITY,
            auto_clean_interval: u64::MAX,
            ..AppSettings::default()
        });
        assert_eq!(settings.notification_cpu_threshold, 90.0);
        assert_eq!(settings.notification_memory_threshold, 90.0);
        assert_eq!(settings.notification_disk_threshold, 90.0);
        assert_eq!(settings.ram_clean_threshold, 85.0);
        assert_eq!(settings.auto_clean_target, 70.0);
        assert_eq!(settings.auto_clean_interval, *CLEAN_INTERVAL.end());
        assert!(serde_json::to_string(&settings).is_ok());
    }
}
