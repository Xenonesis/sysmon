use crate::diagnostics::{self, Severity};
use crate::ui::components::paint_section_header;
use crate::ui::theme::ThemePalette;
use crate::{snapshot_from_data, SystemData, SystemMonitorApp};
use eframe::egui;

pub(crate) fn show(app: &mut SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    paint_section_header(ui, "Diagnostics and Session Recorder");

    ui.group(|ui| {
        ui.heading("Reproduce and record");
        ui.label("A recording stores one local JSON snapshot per second and never uploads data.");
        ui.horizontal(|ui| {
            if app.session_recorder.is_recording() {
                if ui.button("Stop recording").clicked() {
                    app.session_status = Some(match app.session_recorder.stop() {
                        Ok(Some(path)) => format!("Session saved to {}", path.display()),
                        Ok(None) => "Session stopped".into(),
                        Err(error) => format!("Could not stop recording: {error}"),
                    });
                }
                ui.colored_label(
                    ThemePalette::STATUS_CRITICAL,
                    format!("Recording - {} samples", app.session_recorder.sample_count()),
                );
            } else if ui.button("Start recording").clicked() {
                app.session_status = Some(match app.session_recorder.start() {
                    Ok(path) => format!("Recording to {}", path.display()),
                    Err(error) => format!("Could not start recording: {error}"),
                });
            }
        });
        if let Some(status) = &app.session_status {
            ui.small(status);
        } else if let Some(path) = app.session_recorder.path() {
            ui.small(format!("Last session: {}", path.display()));
        }
    });

    ui.add_space(10.0);
    let snapshot = app.latest_snapshot.clone().unwrap_or_else(|| snapshot_from_data(data));
    let report = diagnostics::analyze(&snapshot, &data.telemetry_history_stats);

    egui::ScrollArea::vertical().show(ui, |ui| {
        for finding in report.findings {
            ui.group(|ui| {
                let color = match finding.severity {
                    Severity::Healthy => ThemePalette::STATUS_HEALTHY,
                    Severity::Info => ThemePalette::ACCENT_PRIMARY,
                    Severity::Warning => ThemePalette::STATUS_WARNING,
                    Severity::Critical => ThemePalette::STATUS_CRITICAL,
                };
                ui.horizontal(|ui| {
                    ui.colored_label(color, finding.severity.label());
                    ui.heading(&finding.title);
                    ui.weak(format!("{}% confidence", finding.confidence));
                });
                ui.label(&finding.evidence);
                ui.label(egui::RichText::new(&finding.recommendation).strong());
            });
            ui.add_space(6.0);
        }
    });
}
