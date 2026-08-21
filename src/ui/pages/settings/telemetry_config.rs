use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_telemetry_settings(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    changed: &mut bool,
    is_dark: bool,
) {
    // ── 1. Telemetry & View Preferences ──
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("TELEMETRY & VIEW PREFERENCES")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
                *changed |= ui
                    .checkbox(&mut app.settings.show_graphs, "Show Performance Graphs")
                    .changed();
                *changed |= ui
                    .checkbox(&mut app.settings.show_gpu, "Show GPU Information")
                    .changed();
                *changed |= ui
                    .checkbox(&mut app.settings.show_processes, "Show Process List")
                    .changed();
            });

            cols[1].vertical(|ui| {
                *changed |= ui
                    .checkbox(&mut app.settings.show_per_core_cpu, "Show Per-Core CPU in Overview")
                    .changed();
                *changed |= ui
                    .checkbox(&mut app.settings.show_cpu_cores, "Show CPU Cores Tab")
                    .changed();
            });
        });
    });

    ui.add_space(10.0);

    // ── 2. Data Export & Diagnostics ──
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("DATA EXPORT & DIAGNOSTICS")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("Export to CSV").clicked() {
                app.show_export_csv = true;
            }
            if ui.button("Export to JSON").clicked() {
                app.show_export = true;
            }
            if ui.button("Export Diagnostics Package").clicked() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    app.action_status = Some(match app.export_diagnostics(&folder) {
                        Ok(path) => format!("Diagnostics saved to {}", path.display()),
                        Err(error) => format!("Diagnostics export failed: {error}"),
                    });
                }
            }
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Save comprehensive system telemetry, hardware metrics, and logs.")
                .size(11.0)
                .color(ThemePalette::text_dimmed(is_dark)),
        );
    });

    ui.add_space(10.0);

    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("LOCAL DIAGNOSTIC TIMELINE")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(6.0);
        *changed |= ui
            .checkbox(
                &mut app.settings.timeline_enabled,
                "Record a private rolling timeline on this PC",
            )
            .on_hover_text("Stores five-second metric samples and process names locally. No command lines, paths, usernames, or remote IPs are retained.")
            .changed();

        ui.add_enabled_ui(app.settings.timeline_enabled, |ui| {
            ui.horizontal(|ui| {
                ui.label("Retention:");
                for days in [1_u16, 7, 30] {
                    *changed |= ui
                        .selectable_value(
                            &mut app.settings.timeline_retention_days,
                            days,
                            format!("{days} day{}", if days == 1 { "" } else { "s" }),
                        )
                        .changed();
                }
            });
        });

        let status = app.timeline.status();
        ui.label(
            egui::RichText::new(format!(
                "Local storage: {:.1} MB · hard limit 512 MB",
                status.storage_bytes as f64 / 1_048_576.0
            ))
            .size(11.0)
            .color(ThemePalette::text_dimmed(is_dark)),
        );
        if let Some(error) = status.last_error {
            ui.colored_label(ThemePalette::STATUS_CRITICAL, error);
        }

        ui.horizontal(|ui| {
            if !app.timeline_ui.clear_confirmation {
                if ui.button("Clear Timeline History").clicked() {
                    app.timeline_ui.clear_confirmation = true;
                }
            } else {
                ui.label("Delete all recorded timeline data?");
                if ui.button("Delete").clicked() {
                    app.timeline.clear();
                    app.timeline_ui.window = None;
                    app.timeline_ui.clear_confirmation = false;
                    app.timeline_ui.message = Some("Timeline history cleared.".into());
                }
                if ui.button("Cancel").clicked() {
                    app.timeline_ui.clear_confirmation = false;
                }
            }
        });
    });

    ui.add_space(10.0);

    // ── 3. Safety & Audit ──
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("SAFETY & AUDIT")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("System-changing actions require a risk preview and are logged locally.")
                .size(12.0)
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);
        if ui.button("View System Action History").clicked() {
            app.show_action_history = true;
        }
    });
}
