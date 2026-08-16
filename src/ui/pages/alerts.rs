use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "System Alerts & Incident Feed", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Notification Warning Banner ──
        if !app.settings.show_notifications {
            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    status_pill(ui, "NOTIFICATIONS DISABLED", ThemePalette::STATUS_WARNING, is_dark);
                    ui.label(
                        egui::RichText::new("Desktop notifications are off; alerts are tracked in-app only.")
                            .size(12.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Enable in Settings").clicked() {
                            app.show_settings = true;
                        }
                    });
                });
            });
            ui.add_space(8.0);
        }

        // ── 2. Alerts Feed or Empty State ──
        if data.alerts.is_empty() {
            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    status_pill(ui, "ALL SYSTEMS NORMAL", ThemePalette::STATUS_HEALTHY, is_dark);
                    ui.label(
                        egui::RichText::new("Zero active hardware bottlenecks or metric threshold violations.")
                            .size(13.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let sound_on = app.settings.enable_alert_sound && app.settings.enable_sounds;
                        let sound_label = if sound_on { "🔔 Sound: ON" } else { "🔕 Sound: OFF" };
                        let sound_btn = egui::Button::new(egui::RichText::new(sound_label).size(11.5).strong().color(
                            if sound_on {
                                ThemePalette::STATUS_HEALTHY
                            } else {
                                ThemePalette::text_dimmed(is_dark)
                            },
                        ))
                        .fill(if sound_on {
                            ThemePalette::STATUS_HEALTHY.gamma_multiply(if is_dark { 0.15 } else { 0.10 })
                        } else {
                            ThemePalette::bg_track(is_dark)
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if sound_on {
                                ThemePalette::STATUS_HEALTHY.gamma_multiply(0.4)
                            } else {
                                ThemePalette::border(is_dark)
                            },
                        ))
                        .rounding(egui::Rounding::same(4.0));

                        if ui
                            .add(sound_btn)
                            .on_hover_text("Toggle alert notification audio chime on/off")
                            .clicked()
                        {
                            app.settings.enable_alert_sound = !app.settings.enable_alert_sound;
                            let _ = app.settings.save();
                        }
                    });
                });
            });

            ui.add_space(10.0);

            card_frame(is_dark).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("CURRENT ALERT THRESHOLDS")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Alerts trigger automatically when telemetry exceeds configured limits:")
                        .size(12.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.add_space(8.0);

                egui::Grid::new("alerts_threshold_info_grid")
                    .num_columns(2)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("CPU Saturation:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("> {:.0}%", app.settings.notification_cpu_threshold))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Memory Exhaustion:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("> {:.0}%", app.settings.notification_memory_threshold))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("GPU Temperature:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("> {} °C", app.settings.notification_temp_threshold))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Disk Space Warning:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("> {:.0}%", app.settings.notification_disk_threshold))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();
                    });

                ui.add_space(10.0);
                if ui.button("Configure Alert Thresholds in Settings").clicked() {
                    app.show_settings = true;
                }
            });
        } else {
            // Control Header
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("ACTIVE INCIDENTS & ALERTS")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format!("({} active)", data.alerts.len()))
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::STATUS_WARNING),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(
                            egui::RichText::new("Clear All Alerts")
                                .strong()
                                .color(ThemePalette::STATUS_CRITICAL),
                        )
                        .clicked()
                    {
                        let mut app_data = app.data.write();
                        app_data.alerts.clear();
                    }

                    let sound_on = app.settings.enable_alert_sound && app.settings.enable_sounds;
                    let sound_label = if sound_on { "🔔 Sound: ON" } else { "🔕 Sound: OFF" };
                    let sound_btn =
                        egui::Button::new(egui::RichText::new(sound_label).size(11.5).strong().color(if sound_on {
                            ThemePalette::STATUS_HEALTHY
                        } else {
                            ThemePalette::text_dimmed(is_dark)
                        }))
                        .fill(if sound_on {
                            ThemePalette::STATUS_HEALTHY.gamma_multiply(if is_dark { 0.15 } else { 0.10 })
                        } else {
                            ThemePalette::bg_track(is_dark)
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if sound_on {
                                ThemePalette::STATUS_HEALTHY.gamma_multiply(0.4)
                            } else {
                                ThemePalette::border(is_dark)
                            },
                        ))
                        .rounding(egui::Rounding::same(4.0));

                    if ui
                        .add(sound_btn)
                        .on_hover_text("Toggle alert notification audio chime on/off")
                        .clicked()
                    {
                        app.settings.enable_alert_sound = !app.settings.enable_alert_sound;
                        let _ = app.settings.save();
                    }

                    if ui.button("Configure Thresholds").clicked() {
                        app.show_settings = true;
                    }
                });
            });

            ui.add_space(6.0);

            for (i, alert) in data.alerts.iter().enumerate() {
                card_frame(is_dark).show(ui, |ui| {
                    let (cat_label, color, severity_label) = match alert.alert_type {
                        AlertType::CpuHigh => ("CPU", ThemePalette::STATUS_WARNING, "WARNING"),
                        AlertType::MemoryHigh => ("RAM", ThemePalette::STATUS_WARNING, "WARNING"),
                        AlertType::GpuTempHigh => ("GPU", ThemePalette::STATUS_CRITICAL, "CRITICAL"),
                        AlertType::DiskSpaceLow => ("DISK", ThemePalette::STATUS_CRITICAL, "CRITICAL"),
                        AlertType::StartupHighImpact => ("STARTUP", ThemePalette::ACCENT_PRIMARY, "INFO"),
                    };

                    ui.horizontal(|ui| {
                        status_pill(ui, severity_label, color, is_dark);
                        status_pill(ui, cat_label, ThemePalette::text_secondary(is_dark), is_dark);
                        ui.label(
                            egui::RichText::new(&alert.message)
                                .strong()
                                .size(13.5)
                                .color(ThemePalette::text_primary(is_dark)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("Trigger Value: {:.1}", alert.value))
                                    .monospace()
                                    .strong()
                                    .size(12.0)
                                    .color(color),
                            );
                        });
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Timestamp:")
                                .size(11.0)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(&alert.timestamp)
                                .monospace()
                                .size(11.0)
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                    });
                });

                if i < data.alerts.len() - 1 {
                    ui.add_space(6.0);
                }
            }
        }
    });
}
