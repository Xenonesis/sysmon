use crate::diagnostics::{self, Severity};
use crate::ui::components::{card_frame, paint_progress_bar, status_pill};
use crate::ui::theme::ThemePalette;
use crate::{SystemData, SystemMonitorApp, snapshot_from_data};
use eframe::egui;

pub(super) fn paint_findings(app: &mut SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
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

    ui.add_space(10.0);
}
