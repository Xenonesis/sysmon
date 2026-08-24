use crate::services::ServiceSortColumn;
use crate::ui::theme::ThemePalette;
use eframe::egui;

/// Resolves semantic color for Windows service states.
pub(crate) fn service_state_color(state: &str, is_dark: bool) -> egui::Color32 {
    match state.to_lowercase().as_str() {
        "running" => ThemePalette::STATUS_HEALTHY,
        "stopped" => ThemePalette::text_dimmed(is_dark),
        "paused" | "start pending" | "stop pending" | "continue pending" | "pause pending" => {
            ThemePalette::STATUS_WARNING
        }
        _ => ThemePalette::text_secondary(is_dark),
    }
}

pub(crate) fn sort_header_label(
    label: &str,
    col: ServiceSortColumn,
    current_col: ServiceSortColumn,
    ascending: bool,
) -> String {
    if col == current_col {
        let arrow = if ascending { " ▲" } else { " ▼" };
        format!("{label}{arrow}")
    } else {
        label.to_string()
    }
}
