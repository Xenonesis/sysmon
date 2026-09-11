use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_telemetry_settings(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    changed: &mut bool,
    is_dark: bool,
    intents: &mut Vec<crate::app::commands::UiIntent>,
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

        *changed |= ui
            .checkbox(&mut app.settings.show_graphs, "Show Performance Graphs")
            .changed();
        *changed |= ui
            .checkbox(&mut app.settings.show_gpu, "Show GPU Information (display only)")
            .changed();
        *changed |= ui
            .checkbox(&mut app.settings.show_processes, "Show Process List")
            .changed();
        *changed |= ui
            .checkbox(&mut app.settings.show_per_core_cpu, "Show Per-Core CPU in Overview")
            .changed();
        *changed |= ui
            .checkbox(&mut app.settings.show_cpu_cores, "Show CPU Cores Tab")
            .changed();
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

        ui.horizontal_wrapped(|ui| {
            if ui.button("Export to CSV").clicked() {
                app.show_export_csv = true;
            }
            if ui.button("Export to JSON").clicked() {
                app.show_export = true;
            }
            if ui.button("Export Diagnostics Package").clicked()
                && let Some(folder) = rfd::FileDialog::new().pick_folder()
            {
                app.action_status = Some(match app.export_diagnostics(&folder) {
                    Ok(path) => format!("Diagnostics saved to {}", path.display()),
                    Err(error) => format!("Diagnostics export failed: {error}"),
                });
            }
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Diagnostics exports numeric telemetry and redacted settings, not raw logs. Review all exports before sharing.")
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
            .on_hover_text("Stores five-second metrics, process executable names and allowlisted event metadata locally. Raw action/provider errors, command lines, paths and remote endpoints are excluded.")
            .changed();

        ui.add_enabled_ui(app.settings.timeline_enabled, |ui| {
            ui.horizontal_wrapped(|ui| {
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
                "Local storage: {:.1} MB · 512 MB live-data cap; incremental file reclamation",
                status.storage_bytes as f64 / 1_048_576.0
            ))
            .size(11.0)
            .color(ThemePalette::text_dimmed(is_dark)),
        );
        if let Some(error) = status.last_error {
            ui.colored_label(ThemePalette::STATUS_CRITICAL, error);
        }

        ui.horizontal_wrapped(|ui| {
            if !app.timeline_ui.clear_confirmation {
                if ui.button("Clear Timeline History").clicked() {
                    app.timeline_ui.clear_confirmation = true;
                }
            } else {
                ui.label("Delete all recorded timeline data?");
                if ui.add_enabled(!app.timeline.clear_in_flight(), egui::Button::new("Delete")).clicked() {
                    app.timeline.clear();
                    app.timeline_ui.clear_confirmation = false;
                    app.timeline_ui.message = Some("Clearing timeline; waiting for storage acknowledgement…".into());
                }
                if ui.button("Cancel").clicked() {
                    app.timeline_ui.clear_confirmation = false;
                }
            }
        });
        if app.timeline.clear_in_flight() {
            ui.horizontal(|ui| { ui.spinner(); ui.label("Clear pending…"); });
        }
        if let Some(message) = &app.timeline_ui.message { ui.label(message); }
        ui.label("Timeline retention also expires disabled history. Recorded sessions and detailed action logs have separate retention.");
        ui.label("Sessions are not automatically deleted; remove them explicitly. Detailed action logs may contain paths and account information and rotate at 4 MB (one backup, 8 MB total).");
    });

    ui.add_space(10.0);
    card_frame(is_dark).show(ui, |ui| {
        if ui.button("Check for updates").clicked() {
            intents.push(crate::app::commands::UiIntent::CheckUpdates);
        }
    });

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
