use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::SystemData;
use eframe::egui;

/// Renders the statistical summary card covering sample intervals (60s, 5m, 30m, 1h).
pub(crate) fn paint_history_summary(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("TELEMETRY WINDOW STATISTICAL SUMMARY")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(
                "Rolling window statistics (Min / Avg / Max) computed in-memory across sample intervals.",
            )
            .size(11.5)
            .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        egui::Grid::new("telemetry_window_summary")
            .striped(false)
            .min_col_width(110.0)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("METRIC")
                        .strong()
                        .size(11.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                for label in ["60 SECONDS", "5 MINUTES", "30 MINUTES", "1 HOUR"] {
                    ui.label(
                        egui::RichText::new(label)
                            .strong()
                            .size(11.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                }
                ui.end_row();

                show_metric_stats(ui, data, "CPU Usage", "cpu.global_usage", "%", 1.0, is_dark);
                show_metric_stats(
                    ui,
                    data,
                    "Memory Used",
                    "memory.used",
                    " GiB",
                    1.0 / 1_073_741_824.0,
                    is_dark,
                );

                let gpu_key = data
                    .telemetry_history_stats
                    .keys()
                    .find(|key| key.starts_with("gpu.") && key.ends_with(".utilization"));
                if let Some(key) = gpu_key {
                    show_metric_stats(ui, data, "GPU Util", key, "%", 1.0, is_dark);
                }
            });
    });
}

pub(crate) fn show_metric_stats(
    ui: &mut egui::Ui,
    data: &SystemData,
    label: &str,
    key: &str,
    unit: &str,
    scale: f64,
    is_dark: bool,
) {
    let Some(history) = data.telemetry_history_stats.get(key) else {
        return;
    };
    ui.label(
        egui::RichText::new(label)
            .strong()
            .size(12.0)
            .color(ThemePalette::text_primary(is_dark)),
    );
    for stats in [
        &history.sixty_seconds,
        &history.five_minutes,
        &history.thirty_minutes,
        &history.one_hour,
    ] {
        ui.label(
            egui::RichText::new(format!(
                "min {:.1} · avg {:.1} · max {:.1}{unit}",
                stats.min * scale,
                stats.avg * scale,
                stats.max * scale
            ))
            .monospace()
            .size(11.5)
            .color(ThemePalette::text_primary(is_dark)),
        );
    }
    ui.end_row();
}
