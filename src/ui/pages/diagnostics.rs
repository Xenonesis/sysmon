use crate::diagnostics::{self, Severity};
use crate::ui::components::{card_frame, paint_progress_bar, paint_section_header, status_pill};
use crate::ui::theme::ThemePalette;
use crate::{snapshot_from_data, SystemData, SystemMonitorApp};
use eframe::egui;

pub(crate) fn show(app: &mut SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Diagnostics & Session Recorder", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Session Recorder Banner ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("SESSION RECORDER")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.session_recorder.is_recording() {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                        let pulse = ((ui.input(|i| i.time) * 4.0).sin().abs() as f32) * 0.4 + 0.6;
                        let rec_color = ThemePalette::STATUS_CRITICAL.gamma_multiply(pulse);
                        status_pill(ui, "RECORDING", rec_color, is_dark);
                    } else {
                        status_pill(ui, "IDLE", ThemePalette::text_dimmed(is_dark), is_dark);
                    }
                });
            });

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Captures 1 local JSONL snapshot per second for diagnostic reproduction. Zero external telemetry.",
                )
                .size(12.0)
                .color(ThemePalette::text_secondary(is_dark)),
            );

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if app.session_recorder.is_recording() {
                    let samples = app.session_recorder.sample_count();
                    let mins = samples / 60;
                    let secs = samples % 60;

                    if ui
                        .button(
                            egui::RichText::new("Stop Recording")
                                .strong()
                                .color(ThemePalette::STATUS_CRITICAL),
                        )
                        .clicked()
                    {
                        app.session_status = Some(match app.session_recorder.stop() {
                            Ok(Some(path)) => format!("Session saved to {}", path.display()),
                            Ok(None) => "Session stopped".into(),
                            Err(error) => format!("Could not stop recording: {error}"),
                        });
                    }

                    ui.label(
                        egui::RichText::new(format!("Elapsed: {:02}:{:02}  ·  {} samples", mins, secs, samples))
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                } else if ui
                    .button(
                        egui::RichText::new("Start Recording")
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    )
                    .clicked()
                {
                    app.session_status = Some(match app.session_recorder.start() {
                        Ok(path) => format!("Recording to {}", path.display()),
                        Err(error) => format!("Could not start recording: {error}"),
                    });
                }
            });

            if let Some(status) = &app.session_status {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(status)
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            } else if let Some(path) = app.session_recorder.path() {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("Last session: {}", path.display()))
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
            }
        });

        ui.add_space(12.0);

        // ── 2. Diagnostic Analysis Findings ──
        let snapshot = app.latest_snapshot.clone().unwrap_or_else(|| snapshot_from_data(data));
        let report = diagnostics::analyze(&snapshot, &data.telemetry_history_stats);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("ANOMALY FINDINGS & DIAGNOSTIC REPORT")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{} finding(s)", report.findings.len()))
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
            });
        });

        ui.add_space(6.0);

        if report.findings.is_empty() {
            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    status_pill(ui, "OPTIMAL", ThemePalette::STATUS_HEALTHY, is_dark);
                    ui.label(
                        egui::RichText::new("No diagnostic anomalies or hardware bottlenecks detected.")
                            .size(13.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                });
            });
        } else {
            for finding in report.findings {
                card_frame(is_dark).show(ui, |ui| {
                    let color = match finding.severity {
                        Severity::Healthy => ThemePalette::STATUS_HEALTHY,
                        Severity::Info => ThemePalette::ACCENT_PRIMARY,
                        Severity::Warning => ThemePalette::STATUS_WARNING,
                        Severity::Critical => ThemePalette::STATUS_CRITICAL,
                    };

                    // Header row: badge, title, confidence
                    ui.horizontal(|ui| {
                        status_pill(ui, finding.severity.label(), color, is_dark);
                        ui.label(
                            egui::RichText::new(&finding.title)
                                .strong()
                                .size(14.0)
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{}% CONFIDENCE", finding.confidence))
                                    .monospace()
                                    .size(11.0)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                        });
                    });

                    // Confidence meter
                    ui.add_space(4.0);
                    paint_progress_bar(ui, finding.confidence as f32 / 100.0, color, 4.0, is_dark);

                    // Evidence
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("EVIDENCE")
                            .size(10.0)
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(&finding.evidence)
                            .monospace()
                            .size(12.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    // Recommendation
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("ACTION RECOMMENDATION")
                            .size(10.0)
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(&finding.recommendation)
                            .strong()
                            .size(12.5)
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                });
                ui.add_space(8.0);
            }
        }
    });
}
