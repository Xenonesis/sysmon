use crate::app::models::AppTheme;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_general_settings(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    changed: &mut bool,
    theme_changed: &mut bool,
    is_dark: bool,
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

        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
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
                ui.horizontal(|ui| {
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
                        if ui.add(btn).clicked() {
                            app.settings.theme = theme;
                            *changed = true;
                            *theme_changed = true;
                        }
                    }
                });
            });

            cols[1].vertical(|ui| {
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
                            .add(egui::Slider::new(&mut app.settings.refresh_interval, 1..=10).suffix(" s"))
                            .changed();
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Tracked Processes:").color(ThemePalette::text_secondary(is_dark)),
                        );
                        *changed |= ui
                            .add(egui::Slider::new(&mut app.settings.process_count, 5..=100))
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

            if ui
                .checkbox(&mut app.settings.auto_start, "Start with Windows")
                .changed()
            {
                *changed = true;
                let _ = app.settings.set_auto_start(app.settings.auto_start);
            }
            *changed |= ui
                .checkbox(&mut app.settings.minimize_to_tray, "Minimize to system tray on close")
                .changed();
            *changed |= ui
                .checkbox(&mut app.settings.start_minimized, "Start minimized on launch")
                .changed();

            ui.add_space(8.0);
            ui.horizontal(|ui| {
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
                        && crate::privilege::relaunch_as_admin()
                    {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            });
        });
        ui.add_space(10.0);
    }
}
