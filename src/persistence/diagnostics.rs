use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use crate::{monitoring::SystemSnapshot, AppSettings};

#[derive(Serialize)]
struct Diagnostics<'a> {
    app_version: &'static str,
    snapshot: &'a SystemSnapshot,
    settings: RedactedSettings,
}

#[derive(Serialize)]
struct RedactedSettings { refresh_interval: u64, theme_dark: bool, show_notifications: bool }

pub(crate) fn export(destination: &Path, snapshot: &SystemSnapshot, settings: &AppSettings) -> Result<PathBuf, std::io::Error> {
    fs::create_dir_all(destination)?;
    let output = destination.join("sysmon-diagnostics.json");
    let document = Diagnostics {
        app_version: env!("CARGO_PKG_VERSION"),
        snapshot,
        settings: RedactedSettings {
            refresh_interval: settings.refresh_interval,
            theme_dark: settings.theme_dark,
            show_notifications: settings.show_notifications,
        },
    };
    let bytes = serde_json::to_vec_pretty(&document).map_err(std::io::Error::other)?;
    fs::write(&output, bytes)?;
    Ok(output)
}
