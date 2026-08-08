use crate::*;
use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "System Alerts");

        // Warn when desktop notifications are off — alerts tracked in-app only
        if !app.settings.show_notifications {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(ThemePalette::STATUS_WARNING,
                        "⚠  Desktop notifications are disabled.");
                    ui.label("Alerts are tracked inside the app only.");
                    if ui.small_button("Enable in Settings").clicked() {
                        app.show_settings = true;
                    }
                });
            });
            ui.add_space(8.0);
        }

        if data.alerts.is_empty() {
            ui.group(|ui| {
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.colored_label(egui::Color32::GREEN, "OK");
                    ui.heading("All Systems Normal");
                });
                ui.add_space(10.0);
                ui.label("No alerts detected. Your system is running smoothly.");
                ui.add_space(20.0);
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("Alert Configuration");
                ui.separator();
                ui.label("Alerts are triggered when:");
                ui.label(format!(
                    "  • CPU usage > {:.0}%",
                    app.settings.notification_cpu_threshold
                ));
                ui.label(format!(
                    "  • Memory usage > {:.0}%",
                    app.settings.notification_memory_threshold
                ));
                ui.label(format!(
                    "  • GPU temperature > {}°C",
                    app.settings.notification_temp_threshold
                ));
                ui.label("  • Disk usage > 90%");
                ui.add_space(5.0);
                if ui.button("Configure Alert Thresholds").clicked() {
                    app.show_settings = true;
                }
            });
        } else {
            ui.label(format!("{} active alert(s)", data.alerts.len()));
            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, alert) in data.alerts.iter().enumerate() {
                    ui.group(|ui| {
                        let (icon, color, severity) = match alert.alert_type {
                            AlertType::CpuHigh => ("CPU", egui::Color32::YELLOW, "WARNING"),
                            AlertType::MemoryHigh => ("RAM", egui::Color32::YELLOW, "WARNING"),
                            AlertType::GpuTempHigh => ("GPU", egui::Color32::RED, "CRITICAL"),
                            AlertType::DiskSpaceLow => ("DISK", egui::Color32::RED, "CRITICAL"),
                            AlertType::StartupHighImpact => ("STARTUP", egui::Color32::YELLOW, "INFO"),
                        };

                        ui.horizontal(|ui| {
                            ui.colored_label(color, icon);
                            ui.colored_label(color, severity);
                            ui.separator();
                            ui.strong(&alert.message);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Time:");
                            ui.label(&alert.timestamp);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("Value: {:.1}", alert.value));
                            });
                        });
                    });

                    if i < data.alerts.len() - 1 {
                        ui.add_space(5.0);
                    }
                }
            });

            ui.add_space(10.0);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Clear All Alerts").clicked() {
                    {
                        let mut data = app.data.lock();
                        data.alerts.clear();
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Tip: Configure alert thresholds in Settings");
                });
            });
        }
    }
