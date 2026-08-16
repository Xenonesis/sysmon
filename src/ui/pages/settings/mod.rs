mod alerts_config;
mod general;
mod ram_cleaner_config;
mod telemetry_config;

use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Application Settings", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut changed = false;
        let mut theme_changed = false;

        // ── 1. General Preferences & Theme ──
        general::paint_general_settings(app, ui, &mut changed, &mut theme_changed, is_dark);
        ui.add_space(4.0);

        // ── 2. Telemetry & View Preferences ──
        telemetry_config::paint_telemetry_settings(app, ui, &mut changed, is_dark);
        ui.add_space(4.0);

        // ── 3. Alert Thresholds & Notifications ──
        alerts_config::paint_alerts_settings(app, ui, &mut changed, is_dark);
        ui.add_space(4.0);

        // ── 4. Automated RAM Cleaner Configuration ──
        ram_cleaner_config::paint_ram_cleaner_settings(app, ui, &mut changed, is_dark);
        ui.add_space(8.0);

        if changed {
            let _ = app.settings.save();
            let _ = app
                .app_channels
                .monitoring_sender
                .send(crate::app::commands::MonitoringCommand::SetSettings(Box::new(
                    app.settings.clone(),
                )));
            // Sync settings to the background thread
            {
                let mut shared = app.shared_settings.lock();
                *shared = app.settings.clone();
            }
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
                visuals.window_rounding = egui::Rounding::same(4.0);
                visuals.menu_rounding = egui::Rounding::same(4.0);
                visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
                visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
                visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
                visuals.widgets.active.rounding = egui::Rounding::same(4.0);
                visuals.window_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
                visuals.window_shadow = egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 4.0),
                    blur: 16.0,
                    spread: 0.0,
                    color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 40),
                };
                visuals.popup_shadow = egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 4.0),
                    blur: 16.0,
                    spread: 0.0,
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
                visuals.window_rounding = egui::Rounding::same(8.0);
                visuals.menu_rounding = egui::Rounding::same(8.0);
                ui.ctx().set_visuals(visuals);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_page_render_headless() {
        let mut app = crate::SystemMonitorApp::test_app();

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui);
            });
        });

        // Test with action pending spinner
        app.action_pending = true;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui);
            });
        });

        // Test with action status message
        app.action_pending = false;
        app.action_status = Some("Export completed successfully".to_string());
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui);
            });
        });
    }

    #[test]
    fn test_settings_subcomponents_direct() {
        let mut app = crate::SystemMonitorApp::test_app();
        let mut changed = false;
        let mut theme_changed = false;

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                general::paint_general_settings(&mut app, ui, &mut changed, &mut theme_changed, true);
                telemetry_config::paint_telemetry_settings(&mut app, ui, &mut changed, true);
                alerts_config::paint_alerts_settings(&mut app, ui, &mut changed, true);
                ram_cleaner_config::paint_ram_cleaner_settings(&mut app, ui, &mut changed, true);
            });
        });
    }

    #[test]
    fn test_settings_ram_cleaner_and_theme_options() {
        let mut app = crate::SystemMonitorApp::test_app();
        app.settings.auto_ram_clean = true;
        app.settings.theme = crate::app::models::AppTheme::Light;
        app.settings.auto_clean_exclusions = vec!["custom_game.exe".to_string(), "editor.exe".to_string()];

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui);
            });
        });
    }
}
