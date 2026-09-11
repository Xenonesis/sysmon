mod alerts_config;
mod general;
mod ram_cleaner_config;
mod telemetry_config;

use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

/// Commit one normalized policy to live consumers, retaining persistence failures for retry.
pub(crate) fn commit_settings(app: &mut crate::SystemMonitorApp) -> bool {
    app.settings = crate::persistence::settings::validated(app.settings.clone());
    app.settings_save_error = app.settings.save().err().map(|error| error.to_string());
    *app.shared_settings.lock() = app.settings.clone();
    app.timeline
        .set_policy(app.settings.timeline_enabled, app.settings.timeline_retention_days);
    if let Err(error) = app
        .app_channels
        .monitoring_sender
        .send(crate::app::commands::MonitoringCommand::SetSettings(Box::new(
            app.settings.clone(),
        )))
    {
        app.settings_integration_error = Some(format!("Monitoring settings could not be delivered: {error}"));
    }
    app.widget_open = app.settings.show_widget;
    app.ram_cleaner_state.auto_clean_enabled = app.settings.auto_ram_clean;
    app.ram_cleaner_state.auto_clean_threshold = app.settings.ram_clean_threshold;
    app.ram_cleaner_state.auto_clean_target = app.settings.auto_clean_target;
    app.ram_cleaner_state.auto_clean_interval = app.settings.auto_clean_interval;
    app.ram_cleaner_state.auto_clean_max_mb = app.settings.auto_clean_max_mb;
    app.ram_cleaner_state.auto_clean_idle_only = app.settings.auto_clean_idle_only;
    app.ram_cleaner_state.auto_clean_smart_only = app.settings.auto_clean_smart_only;
    app.ram_cleaner_state.auto_clean_notify = app.settings.auto_clean_notify;
    app.ram_cleaner_state
        .auto_clean_exclusions
        .clone_from(&app.settings.auto_clean_exclusions);
    app.settings_save_error.is_none()
}

pub(crate) fn paint_save_status(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui) {
    if let Some(error) = &app.settings_save_error {
        ui.colored_label(
            ThemePalette::STATUS_WARNING,
            format!("Unsaved settings — active for this session only: {error}"),
        );
        if ui.button("Retry saving settings").clicked() {
            commit_settings(app);
        }
    }
    if let Some(error) = &app.settings_integration_error {
        ui.colored_label(ThemePalette::STATUS_WARNING, error);
    }
}

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui) -> Vec<crate::app::commands::UiIntent> {
    let mut intents = Vec::new();
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Application Settings", is_dark);
    paint_save_status(app, ui);

    egui::ScrollArea::both().show(ui, |ui| {
        let mut changed = false;
        let mut theme_changed = false;

        // ── 1. General Preferences & Theme ──
        general::paint_general_settings(app, ui, &mut changed, &mut theme_changed, is_dark, &mut intents);
        ui.add_space(4.0);

        // ── 2. Telemetry & View Preferences ──
        telemetry_config::paint_telemetry_settings(app, ui, &mut changed, is_dark, &mut intents);
        ui.add_space(4.0);

        // ── 3. Alert Thresholds & Notifications ──
        alerts_config::paint_alerts_settings(app, ui, &mut changed, is_dark);
        ui.add_space(4.0);

        // ── 4. Automated RAM Cleaner Configuration ──
        ram_cleaner_config::paint_ram_cleaner_settings(app, ui, &mut changed, is_dark);
        ui.add_space(8.0);

        if changed {
            commit_settings(app);
        }

        if app.action_pending {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    egui::RichText::new("Action in progress...")
                        .monospace()
                        .size(12.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        } else if let Some(status) = &app.action_status {
            ui.label(
                egui::RichText::new(status)
                    .monospace()
                    .size(11.0)
                    .color(ThemePalette::TEXT_LABEL),
            );
        }

        // Apply theme change live
        if theme_changed {
            let is_dark = ThemePalette::is_dark_mode(app.settings.theme);
            if is_dark {
                let mut visuals = egui::Visuals::dark();
                visuals.panel_fill = ThemePalette::BG_DEEP;
                visuals.window_fill = ThemePalette::BG_SURFACE;
                visuals.extreme_bg_color = ThemePalette::BG_DEEPEST;
                visuals.selection.bg_fill = ThemePalette::ACCENT_PRIMARY;
                visuals.selection.stroke = egui::Stroke::NONE;
                visuals.hyperlink_color = ThemePalette::ACCENT_PRIMARY;
                visuals.widgets.noninteractive.bg_fill = ThemePalette::BG_CARD;
                visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
                visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_PRIMARY);
                visuals.widgets.inactive.bg_fill = ThemePalette::WIDGET_INACTIVE;
                visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SECONDARY);
                visuals.widgets.hovered.bg_fill = ThemePalette::WIDGET_HOVERED;
                visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER_LIGHT);
                visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);
                visuals.widgets.active.bg_fill = ThemePalette::ACCENT_ACTIVE;
                visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);
                visuals.window_corner_radius = egui::CornerRadius::same(4);
                visuals.menu_corner_radius = egui::CornerRadius::same(4);
                visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4);
                visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
                visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
                visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);
                visuals.window_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
                visuals.window_shadow = egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 40),
                };
                visuals.popup_shadow = egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 40),
                };
                ui.ctx().set_visuals(visuals);
            } else {
                let mut visuals = egui::Visuals::light();
                visuals.panel_fill = egui::Color32::from_rgb(245, 245, 247);
                visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
                visuals.extreme_bg_color = egui::Color32::from_rgb(235, 235, 240);
                visuals.selection.bg_fill = ThemePalette::ACCENT_PRIMARY;
                visuals.selection.stroke = egui::Stroke::NONE;
                visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(250, 250, 250);
                visuals.widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 220, 225));
                visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 45));
                visuals.window_corner_radius = egui::CornerRadius::same(8);
                visuals.menu_corner_radius = egui::CornerRadius::same(8);
                ui.ctx().set_visuals(visuals);
            }
        }
    });
    intents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_commit_normalizes_live_policy_and_retry_persists_same_policy() {
        let root = std::env::temp_dir().join(format!(
            "sysmon-settings-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir(&root).unwrap();
        crate::app_paths::with_test_data_local_dir(root.clone(), || {
            let mut app = crate::SystemMonitorApp::test_app();
            let config_path = root.join("config");
            std::fs::write(&config_path, b"not a directory").unwrap();
            app.settings.ram_clean_threshold = 1.0;
            app.settings.auto_clean_target = 99.0;
            app.settings.auto_clean_interval = 10;
            app.settings.auto_clean_max_mb = 16384;
            assert!(!commit_settings(&mut app));
            assert!(app.settings_save_error.is_some());
            assert_eq!(app.settings.ram_clean_threshold, 50.0);
            assert_eq!(app.settings.auto_clean_target, 95.0);
            assert_eq!(app.ram_cleaner_state.auto_clean_interval, 30);
            assert_eq!(app.ram_cleaner_state.auto_clean_max_mb, 4096);
            let live = serde_json::to_value(&app.settings).unwrap();
            assert_eq!(live, serde_json::to_value(&*app.shared_settings.lock()).unwrap());

            std::fs::remove_file(&config_path).unwrap();
            assert!(commit_settings(&mut app));
            assert!(app.settings_save_error.is_none());
            let reloaded = crate::persistence::settings::load(&config_path.join("settings.json")).unwrap();
            assert_eq!(live, serde_json::to_value(reloaded).unwrap());

            app.settings.refresh_interval = 10;
            assert!(commit_settings(&mut app));
            assert_eq!(
                crate::persistence::settings::load(&config_path.join("settings.json"))
                    .unwrap()
                    .refresh_interval,
                10
            );
        });
        std::fs::remove_dir_all(root).unwrap();
    }
}
