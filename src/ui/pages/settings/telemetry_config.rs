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
