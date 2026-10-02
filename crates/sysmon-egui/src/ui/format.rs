//! Unit conversions and human-readable string formatting.

use crate::ui::theme::ThemePalette;
use eframe::egui;

/// Convert raw byte counts to megabytes (MiB).
pub(crate) fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

/// Convert raw byte counts to gigabytes (GiB).
pub(crate) fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

/// Formats a byte count with appropriate binary unit suffix (B, KB, MB, GB, TB).
pub(crate) fn bytes_to_human(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Formats byte transfer rates with standard rate suffix (B/s, KB/s, MB/s, GB/s).
#[allow(dead_code)]
pub(crate) fn rates_to_human(bytes_per_sec: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes_per_sec >= GB {
        format!("{:.2} GB/s", bytes_per_sec / GB)
    } else if bytes_per_sec >= MB {
        format!("{:.1} MB/s", bytes_per_sec / MB)
    } else if bytes_per_sec >= KB {
        format!("{:.1} KB/s", bytes_per_sec / KB)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

/// Format seconds of system uptime into "Xd Xh Xm" format.
#[allow(dead_code)]
pub(crate) fn format_uptime(uptime_secs: u64) -> String {
    let d = uptime_secs / 86400;
    let h = (uptime_secs % 86400) / 3600;
    let m = (uptime_secs % 3600) / 60;
    format!("{}d {}h {}m", d, h, m)
}

/// Semantic threshold color mapping (<70% Emerald, 70-90% Amber, >90% Red).
pub(crate) fn get_usage_color(percentage: f32) -> egui::Color32 {
    if percentage < 70.0 {
        ThemePalette::STATUS_HEALTHY
    } else if percentage < 90.0 {
        ThemePalette::STATUS_WARNING
    } else {
        ThemePalette::STATUS_CRITICAL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_formatting() {
        assert_eq!(bytes_to_human(500), "500 B");
        assert_eq!(bytes_to_human(1024), "1.0 KB");
        assert_eq!(bytes_to_human(1024 * 1024 * 5), "5.0 MB");
        assert_eq!(bytes_to_human(1024 * 1024 * 1024 * 2), "2.00 GB");
    }

    #[test]
    fn test_rates_formatting() {
        assert_eq!(rates_to_human(500.0), "500 B/s");
        assert_eq!(rates_to_human(1024.0 * 250.0), "250.0 KB/s");
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(3665), "0d 1h 1m");
        assert_eq!(format_uptime(90000), "1d 1h 0m");
    }
}
