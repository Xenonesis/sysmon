use crate::app::models::AppTheme;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Application Settings", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut changed = false;
        let mut theme_changed = false;

        // ── 1. General Preferences ──
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("GENERAL PREFERENCES")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    changed |= ui
                        .checkbox(&mut app.settings.show_graphs, "Show Performance Graphs")
                        .changed();
                    changed |= ui
                        .checkbox(&mut app.settings.show_gpu, "Show GPU Information")
                        .changed();
                    changed |= ui
                        .checkbox(&mut app.settings.show_processes, "Show Process List")
                        .changed();
                    changed |= ui
                        .checkbox(&mut app.settings.show_per_core_cpu, "Show Per-Core CPU in Overview")
                        .changed();
                    changed |= ui
                        .checkbox(&mut app.settings.show_cpu_cores, "Show CPU Cores Tab")
                        .changed();
                    changed |= ui
                        .checkbox(&mut app.settings.show_widget, "Show Desktop Mini-Widget")
                        .changed();
                    changed |= ui
                        .checkbox(&mut app.settings.sidebar_collapsed, "Collapse Navigation Sidebar")
                        .changed();
                    app.widget_open = app.settings.show_widget;
                });

                cols[1].vertical(|ui| {
                    changed |= ui
                        .checkbox(&mut app.settings.show_notifications, "Enable Desktop Notifications")
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut app.settings.enable_alert_sound,
                            "Enable Alert Sounds (Audio Chime)",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(&mut app.settings.enable_sounds, "Enable System Event Sounds")
                        .changed();
                    changed |= ui
                        .checkbox(&mut app.settings.auto_clear_alerts, "Auto-clear Resolved Alerts")
                        .changed();

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
                                .rounding(egui::Rounding::same(4.0))
                            } else {
                                egui::Button::new(
                                    egui::RichText::new(label)
                                        .size(11.0)
                                        .color(ThemePalette::text_secondary(is_dark)),
                                )
                                .fill(ThemePalette::bg_deepest(is_dark))
                                .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
                                .rounding(egui::Rounding::same(4.0))
                            };
                            if ui.add(btn).clicked() {
                                app.settings.theme = theme;
                                changed = true;
                                theme_changed = true;
                            }
                        }
                    });
                });
            });
        });

        ui.add_space(10.0);

        // ── 2. Monitoring & Intervals ──
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("MONITORING & INTERVALS")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(8.0);

            egui::Grid::new("monitoring_grid")
                .num_columns(2)
                .spacing([24.0, 10.0])
                .min_col_width(220.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Data refresh interval:").color(ThemePalette::text_secondary(is_dark)),
                    );
                    changed |= ui
                        .add(egui::Slider::new(&mut app.settings.refresh_interval, 1..=10).suffix(" s"))
                        .changed();
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Tracked processes limit:").color(ThemePalette::text_secondary(is_dark)),
                    );
                    changed |= ui
                        .add(egui::Slider::new(&mut app.settings.process_count, 5..=100))
                        .changed();
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // ── 3. Alert Thresholds ──
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("ALERT THRESHOLDS")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(8.0);

            egui::Grid::new("alert_thresholds_grid")
                .num_columns(2)
                .spacing([24.0, 10.0])
                .min_col_width(220.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("CPU usage alert threshold:").color(ThemePalette::text_secondary(is_dark)),
                    );
                    changed |= ui
                        .add(egui::Slider::new(&mut app.settings.notification_cpu_threshold, 50.0..=100.0).suffix(" %"))
                        .changed();
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Memory usage alert threshold:")
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut app.settings.notification_memory_threshold, 50.0..=100.0)
                                .suffix(" %"),
                        )
                        .changed();
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Temperature alert threshold:")
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    changed |= ui
                        .add(egui::Slider::new(&mut app.settings.notification_temp_threshold, 60..=105).suffix(" °C"))
                        .changed();
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Disk usage alert threshold:").color(ThemePalette::text_secondary(is_dark)),
                    );
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut app.settings.notification_disk_threshold, 50.0..=100.0).suffix(" %"),
                        )
                        .changed();
                    ui.end_row();
                });

            ui.add_space(8.0);
            changed |= ui
                .checkbox(
                    &mut app.settings.enable_alert_sound,
                    "Play audio chime when alert triggers",
                )
                .changed();
        });

        ui.add_space(10.0);

        // ── 4. Windows Integration ──
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
                    changed = true;
                    let _ = app.settings.set_auto_start(app.settings.auto_start);
                }
                changed |= ui
                    .checkbox(&mut app.settings.minimize_to_tray, "Minimize to system tray on close")
                    .changed();
                changed |= ui
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

        // ── 5. Data Export & Diagnostics ──
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

        // ── 6. Safety and Audit ──
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

        ui.add_space(12.0);

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
