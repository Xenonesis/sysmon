use crate::app::models::AppTheme;
use crate::persistence::settings::{PROCESS_COUNT, REFRESH_INTERVAL};
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_general_settings(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    changed: &mut bool,
    theme_changed: &mut bool,
    is_dark: bool,
    intents: &mut Vec<crate::app::commands::UiIntent>,
) {
    // ── 1. General Preferences & Theme ──
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("GENERAL PREFERENCES & THEME")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        ui.vertical(|ui| {
            ui.vertical(|ui| {
                *changed |= ui
                    .checkbox(&mut app.settings.show_widget, "Show Desktop Mini-Widget")
                    .changed();
                *changed |= ui
                    .checkbox(&mut app.settings.sidebar_collapsed, "Collapse Navigation Sidebar")
                    .changed();
                app.widget_open = app.settings.show_widget;

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Theme Selection")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.add_space(2.0);

                // 3-way theme selector with clean brutalist tiles
                ui.horizontal_wrapped(|ui| {
                    for (theme, label) in [
                        (AppTheme::Dark, "Dark (Noir)"),
                        (AppTheme::Light, "Light (Slate)"),
                        (AppTheme::System, "System"),
                    ] {
                        let is_selected = app.settings.theme == theme;
                        let btn = if is_selected {
                            egui::Button::new(
                                egui::RichText::new(label)
                                    .size(11.0)
                                    .strong()
                                    .color(ThemePalette::TEXT_SELECTED),
                            )
                            .fill(ThemePalette::ACCENT_PRIMARY)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(egui::CornerRadius::same(4))
                        } else {
                            egui::Button::new(
                                egui::RichText::new(label)
                                    .size(11.0)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            )
                            .fill(ThemePalette::bg_deepest(is_dark))
                            .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
                            .corner_radius(egui::CornerRadius::same(4))
                        };
                        if ui.add(btn.selected(is_selected)).clicked() {
                            app.settings.theme = theme;
                            *changed = true;
                            *theme_changed = true;
                        }
                    }
                });
            });

            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Polling & Refresh Rates")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.add_space(2.0);

                egui::Grid::new("general_intervals_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Refresh Interval:").color(ThemePalette::text_secondary(is_dark)));
                        *changed |= ui
                            .add(
                                egui::Slider::new(&mut app.settings.refresh_interval, REFRESH_INTERVAL)
                                    .suffix(" s")
                                    .text("Refresh interval"),
                            )
                            .changed();
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Tracked Processes:").color(ThemePalette::text_secondary(is_dark)),
                        );
                        *changed |= ui
                            .add(
                                egui::Slider::new(&mut app.settings.process_count, PROCESS_COUNT)
                                    .text("Tracked processes"),
                            )
                            .changed();
                        ui.end_row();
                    });
            });
        });
    });

    ui.add_space(10.0);

    // ── 2. Windows Integration ──
    #[cfg(target_os = "windows")]
    {
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("WINDOWS INTEGRATION")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(8.0);

            let mut requested_auto_start = app.settings.auto_start;
            if ui.checkbox(&mut requested_auto_start, "Start with Windows").changed() {
                match app.settings.set_auto_start(requested_auto_start) {
                    Ok(()) => {
                        app.settings.auto_start = requested_auto_start;
                        app.settings_integration_error = None;
                        *changed = true;
                    }
                    Err(error) => {
                        app.settings_integration_error = Some(format!(
                            "Windows auto-start was not changed: {error}. Try the checkbox again to retry."
                        ));
                    }
                }
            }
            *changed |= ui
                .checkbox(&mut app.settings.minimize_to_tray, "Minimize to system tray on close")
                .changed();
            *changed |= ui
                .checkbox(&mut app.settings.start_minimized, "Start minimized on launch")
                .changed();

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let elevated = crate::privilege::is_app_elevated();
                if elevated {
                    status_pill(ui, "ADMINISTRATOR (ELEVATED)", ThemePalette::STATUS_HEALTHY, is_dark);
                } else {
                    status_pill(ui, "STANDARD USER", ThemePalette::text_dimmed(is_dark), is_dark);
                    ui.add_space(8.0);
                    if ui
                        .button(
                            egui::RichText::new("Relaunch as Administrator")
                                .strong()
                                .color(ThemePalette::STATUS_WARNING),
                        )
                        .clicked()
                    {
                        intents.push(crate::app::commands::UiIntent::RelaunchAsAdmin);
                    }
                }
            });
        });
        ui.add_space(10.0);
    }
}
