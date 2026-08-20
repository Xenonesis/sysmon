use crate::ui::components::*;
use crate::ui::pages::startup_manager::item_card::StartupActionRequest;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(crate) fn handle_startup_action(app: &mut crate::SystemMonitorApp, request: StartupActionRequest) {
    app.queue_action(request.command);
}

pub(crate) fn paint_action_history(app: &crate::SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    if !app.settings.startup_optimization_history.is_empty() {
        ui.add_space(16.0);
        card_frame(is_dark).show(ui, |ui| {
            ui.heading(egui::RichText::new("Optimization History").color(ThemePalette::text_primary(is_dark)));
            ui.separator();

            let history = &app.settings.startup_optimization_history;
            let show_count = history.len().min(10);
            for entry in history.iter().rev().take(show_count) {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&entry.timestamp)
                            .monospace()
                            .size(11.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{} \"{}\"", entry.action, entry.item_name))
                            .size(11.5)
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    let delta = entry.high_impact_count_before as i32 - entry.high_impact_count_after as i32;
                    if delta > 0 {
                        status_pill(ui, &format!("-{} HIGH", delta), ThemePalette::STATUS_HEALTHY, is_dark);
                    }
                });
            }
        });
    }
}
