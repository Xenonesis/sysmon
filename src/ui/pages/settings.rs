use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui) {
        paint_section_header(ui, "Application Settings");

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut changed = false;
            let mut theme_changed = false;

            // --- General Group ---
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.heading("General");
                });
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
                            .checkbox(&mut app.settings.show_widget, "Show desktop mini-widget")
                            .changed();
                        app.widget_open = app.settings.show_widget;
                    });

                    cols[1].vertical(|ui| {
                        changed |= ui
                            .checkbox(&mut app.settings.show_notifications, "Enable Desktop Notifications")
                            .changed();
                        changed |= ui
                            .checkbox(&mut app.settings.enable_sounds, "Enable System Event Sounds")
                            .changed();
                        if ui
                            .checkbox(&mut app.settings.theme_dark, "Dark Theme (Terminal Noir)")
                            .changed()
                        {
                            changed = true;
                            theme_changed = true;
                        }
                        changed |= ui
                            .checkbox(&mut app.settings.auto_clear_alerts, "Auto-clear resolved alerts")
                            .changed();
                    });
                });
            });
            ui.add_space(12.0);

            // --- Monitoring Group ---
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.heading("Monitoring");
                });
                ui.add_space(8.0);

                egui::Grid::new("monitoring_grid")
                    .num_columns(2)
                    .spacing([24.0, 12.0])
                    .min_col_width(200.0)
                    .show(ui, |ui| {
                        ui.label("Data refresh interval:");
                        changed |= ui
                            .add(egui::Slider::new(&mut app.settings.refresh_interval, 1..=10).suffix("s"))
                            .changed();
                        ui.end_row();

                        ui.label("Number of processes to show:");
                        changed |= ui
                            .add(egui::Slider::new(&mut app.settings.process_count, 5..=100))
                            .changed();
                        ui.end_row();
                    });
            });
            ui.add_space(12.0);

            // --- Alert Thresholds Group ---
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.heading("Alert Thresholds");
                });
                ui.add_space(8.0);

                egui::Grid::new("alert_thresholds_grid")
                    .num_columns(2)
                    .spacing([24.0, 12.0])
                    .min_col_width(200.0)
                    .show(ui, |ui| {
                        ui.label("CPU Usage % Alert:");
                        changed |= ui
                            .add(egui::Slider::new(&mut app.settings.notification_cpu_threshold, 50.0..=100.0).suffix("%"))
                            .changed();
                        ui.end_row();

                        ui.label("Memory Usage % Alert:");
                        changed |= ui
                            .add(egui::Slider::new(&mut app.settings.notification_memory_threshold, 50.0..=100.0).suffix("%"))
                            .changed();
                        ui.end_row();

                        ui.label("Temperature °C Alert:");
                        changed |= ui
                            .add(egui::Slider::new(&mut app.settings.notification_temp_threshold, 60..=105).suffix("°C"))
                            .changed();
                        ui.end_row();
                    });
            });
            ui.add_space(12.0);

            // --- Windows Integration Group ---
            #[cfg(target_os = "windows")]
            {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.heading("Windows Integration");
                    });
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
                });
                ui.add_space(12.0);
            }

            // --- Export Group ---
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.heading("Export Data");
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("📊 Export to CSV").clicked() {
                        app.show_export_csv = true;
                    }
                    if ui.button("📄 Export to JSON").clicked() {
                        app.show_export = true;
                    }
                });
                ui.label(egui::RichText::new("Save current system snapshot to a file")
                    .size(11.0).color(ThemePalette::TEXT_DIMMED));
            });
            ui.add_space(12.0);

            if changed {
                let _ = app.settings.save();
                let _ = app.app_channels.monitoring_sender.send(crate::app::commands::MonitoringCommand::SetSettings(app.settings.clone()));
                // Sync settings to the background thread
                {
                    let mut shared = app.shared_settings.lock();
                    *shared = app.settings.clone();
                }
            }

                if app.action_pending {
                    ui.spinner();
                    ui.label("Action in progress...");
                } else if let Some(status) = &app.action_status {
                    ui.label(egui::RichText::new(status).small().color(ThemePalette::TEXT_LABEL));
                }
                ui.add_space(12.0);
                if ui.button("Export Diagnostics").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        app.action_status = Some(match app.export_diagnostics(&folder) {
                            Ok(path) => format!("Diagnostics saved to {}", path.display()),
                            Err(error) => format!("Diagnostics export failed: {error}"),
                        });
                    }
                }

            // Apply theme change live
            if theme_changed {
                if app.settings.theme_dark {
                    let mut visuals = egui::Visuals::dark();
                    visuals.panel_fill = ThemePalette::BG_DEEP;
                    visuals.window_fill = ThemePalette::BG_SURFACE;
                    visuals.extreme_bg_color = ThemePalette::BG_DEEPEST;
                    visuals.selection.bg_fill = ThemePalette::ACCENT_PRIMARY;
                    visuals.selection.stroke = egui::Stroke::new(1.0, ThemePalette::ACCENT_ACTIVE);
                    visuals.hyperlink_color = ThemePalette::ACCENT_PRIMARY;
                    visuals.widgets.noninteractive.bg_fill = ThemePalette::BG_CARD;
                    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
                    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_PRIMARY);
                    visuals.widgets.inactive.bg_fill = ThemePalette::WIDGET_INACTIVE;
                    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER);
                    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SECONDARY);
                    visuals.widgets.hovered.bg_fill = ThemePalette::WIDGET_HOVERED;
                    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER_LIGHT);
                    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);
                    visuals.widgets.active.bg_fill = ThemePalette::ACCENT_ACTIVE;
                    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ThemePalette::ACCENT_PRIMARY);
                    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, ThemePalette::TEXT_SELECTED);
                    visuals.window_rounding = egui::Rounding::same(8.0);
                    visuals.menu_rounding = egui::Rounding::same(8.0);
                    visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
                    visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
                    visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
                    visuals.widgets.active.rounding = egui::Rounding::same(8.0);
                    visuals.window_stroke = egui::Stroke::new(1.0, ThemePalette::BORDER_LIGHT);
                    visuals.window_shadow = egui::epaint::Shadow {
                        offset: egui::vec2(0.0, 12.0),
                        blur: 32.0,
                        spread: -4.0,
                        color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 180),
                    };
                    visuals.popup_shadow = egui::epaint::Shadow {
                        offset: egui::vec2(0.0, 8.0),
                        blur: 24.0,
                        spread: -2.0,
                        color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 150),
                    };
                    ui.ctx().set_visuals(visuals);
                } else {
                    ui.ctx().set_visuals(egui::Visuals::light());
                }
            }
        });
    }
