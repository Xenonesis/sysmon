use crate::timeline::analyze_window;
use crate::ui::components::card_frame;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_analysis_card(
    app: &crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    window: &crate::timeline::TimelineWindow,
    is_dark: bool,
) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("EVIDENCE-BASED ANALYSIS")
                .strong()
                .size(11.0)
                .color(ThemePalette::ACCENT_PRIMARY),
        );
        let selected = app.timeline_ui.selected_timestamp_ms.unwrap_or_else(|| {
            window
                .metrics
                .last()
                .map_or(window.query.end_ms, |sample| sample.timestamp_ms)
        });
        let analysis = analyze_window(window, selected);
        ui.heading(&analysis.title);
        ui.label(&analysis.summary);
        ui.label(
            egui::RichText::new(format!("Confidence: {}", analysis.confidence))
                .monospace()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.separator();
        for evidence in &analysis.evidence {
            ui.label(format!("• {evidence}"));
        }
        if !analysis.contributors.is_empty() {
            ui.separator();
            ui.strong("Observed contributors");
            for process in &analysis.contributors {
                ui.label(format!(
                    "{} (PID {}) · CPU {:.1}% · RAM {:.1} MB · Disk {:.1} MB",
                    process.name,
                    process.pid,
                    process.cpu_pct,
                    process.memory_bytes as f64 / 1_048_576.0,
                    process.disk_bytes as f64 / 1_048_576.0
                ));
            }
        }
    });
}
