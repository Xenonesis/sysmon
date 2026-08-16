use std::fs;
use std::path::{Path, PathBuf};

use crate::{app::models::AppTheme, monitoring::SystemSnapshot, AppSettings};
use serde::Serialize;

#[derive(Serialize)]
struct Diagnostics<'a> {
    app_version: &'static str,
    snapshot: &'a SystemSnapshot,
    settings: RedactedSettings,
}

#[derive(Serialize)]
struct RedactedSettings {
    refresh_interval: u64,
    theme: AppTheme,
    show_notifications: bool,
}

pub(crate) fn export(
    destination: &Path,
    snapshot: &SystemSnapshot,
    settings: &AppSettings,
) -> Result<PathBuf, std::io::Error> {
    fs::create_dir_all(destination)?;
    let output = destination.join("sysmon-diagnostics.json");
    let document = Diagnostics {
        app_version: env!("CARGO_PKG_VERSION"),
        snapshot,
        settings: RedactedSettings {
            refresh_interval: settings.refresh_interval,
            theme: settings.theme,
            show_notifications: settings.show_notifications,
        },
    };
    let bytes = serde_json::to_vec_pretty(&document).map_err(std::io::Error::other)?;
    fs::write(&output, bytes)?;
    Ok(output)
}
