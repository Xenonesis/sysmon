use crate::startup::{self, StartupOptimizationEntry};
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(crate) fn handle_startup_action(app: &mut crate::SystemMonitorApp, name: &str, source: &str, cmd: &str, act: &str) {
    let success = match act {
        "disable" => startup::disable_startup_item(name, source, cmd),
        "enable" => startup::reenable_startup_item(name, source),
        "remove" => startup::remove_startup_item(name, source),
        _ => false,
    };

    if success {
        if let Some(pos) = app
            .startup_items
            .iter()
            .position(|it| it.name == name && it.source == source)
        {
            let tier_before = app.startup_items[pos].impact_tier.label().to_string();
            let high_before = startup::high_impact_count(&app.startup_items);

            if act == "disable" {
                app.startup_items[pos].enabled = false;
            } else if act == "enable" {
                app.startup_items[pos].enabled = true;
            } else if act == "remove" {
                app.startup_items.remove(pos);
            }

            let high_after = startup::high_impact_count(&app.startup_items);
            app.data.write().high_impact_startup_count = high_after;

            app.settings
                .startup_optimization_history
                .push(StartupOptimizationEntry {
                    timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                    action: act.to_string(),
                    item_name: name.to_string(),
                    item_source: source.to_string(),
                    impact_tier_before: tier_before,
                    high_impact_count_before: high_before,
                    high_impact_count_after: high_after,
                });
            let _ = app.settings.save();
        }
    }
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
