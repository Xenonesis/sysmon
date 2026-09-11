use crate::SystemMonitorApp;
use crate::ui::components::{card_frame, status_pill};
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_bsod_history_card(app: &mut SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    app.crash_reports.poll();
    if app.crash_reports.outcome.is_none() {
        app.crash_reports.request();
    }
    let crashes = app
        .crash_reports
        .outcome
        .as_ref()
        .map(|outcome| outcome.reports.as_slice())
        .unwrap_or(&[]);
    let mut rescan_requested = false;

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("BSOD & CRASH MINIDUMP HISTORY")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("⟳ Scan Dumps").clicked() {
                    rescan_requested = true;
                }
                if crashes.is_empty() {
                    status_pill(ui, "HEALTHY (0 CRASHES)", ThemePalette::STATUS_HEALTHY, is_dark);
                } else {
                    status_pill(
                        ui,
                        &format!("{} INCIDENT(S)", crashes.len()),
                        ThemePalette::STATUS_CRITICAL,
                        is_dark,
                    );
                }
            });
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Kernel BSOD minidumps (%SystemRoot%\\Minidump) and application crash dumps (%LOCALAPPDATA%\\CrashDumps) decoded via offline diagnostic dictionary.",
            )
            .size(12.0)
            .color(ThemePalette::text_secondary(is_dark)),
        );

        ui.add_space(8.0);
        if crashes.is_empty() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    status_pill(ui, "HEALTHY", ThemePalette::STATUS_HEALTHY, is_dark);
                    ui.label(
                        egui::RichText::new(
                            "No BSOD or application minidump crash files detected. System crash logs are clear.",
                        )
                        .size(12.5)
                        .color(ThemePalette::text_primary(is_dark)),
                    );
                });
            });
        } else {
            for crash in crashes {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        status_pill(
                            ui,
                            &format!("0x{:08X}", crash.code),
                            ThemePalette::STATUS_CRITICAL,
                            is_dark,
                        );
                        ui.label(
                            egui::RichText::new(&crash.code_name)
                                .strong()
                                .size(13.5)
                                .color(ThemePalette::text_primary(is_dark)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(crash.timestamp.as_deref().unwrap_or("unknown time"))
                                    .monospace()
                                    .size(11.0)
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                        });
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Dump File: {}", crash.file_name))
                                .monospace()
                                .size(11.0)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        if let Some(module) = &crash.address_module {
                            status_pill(ui, &format!("Faulting: {module}"), ThemePalette::STATUS_WARNING, is_dark);
                        }
                    });

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("EXPLANATION")
                            .size(10.0)
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(&crash.explanation)
                            .size(12.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("ACTIONABLE RECOMMENDATION")
                            .size(10.0)
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(&crash.recommendation)
                            .strong()
                            .size(12.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                });
                ui.add_space(6.0);
            }
        }
    });

    if rescan_requested && app.crash_reports.outcome.is_some() {
        app.crash_reports.outcome = None;
        app.crash_reports.request();
    }
}
