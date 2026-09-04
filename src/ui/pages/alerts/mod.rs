mod buttons;
mod control_hub;
mod health_board;
mod incidents_feed;
mod proximity;

use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "System Alerts & Incident Feed", is_dark);

    let mut remove_alert_idx: Option<usize> = None;
    let mut clear_all_alerts = false;
    let mut trigger_test_alert = false;
    let mut navigate_tab: Option<Tab> = None;
    let mut run_ram_clean = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        card_frame(is_dark).show(ui, |ui| {
            let avail_w = ui.available_width();
            let is_wide = avail_w >= 1050.0;

            if is_wide {
                // Wide single-row layout with defensive bounds preventing text collision
                ui.horizontal(|ui| {
                    let controls_w = if data.alerts.is_empty() { 420.0 } else { 540.0 };
                    let left_w = (avail_w - controls_w - 16.0).max(200.0);

                    ui.allocate_ui_with_layout(
                        egui::vec2(left_w, 0.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            control_hub::paint_status_headline(ui, data, is_dark);
                        },
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        control_hub::paint_control_hub_actions(
                            app,
                            ui,
                            data,
                            is_dark,
                            &mut trigger_test_alert,
                            &mut clear_all_alerts,
                        );
                    });
                });
            } else {
                // Responsive 2-tier layout: Status headline on top, unified toolbar below
                ui.horizontal(|ui| {
                    control_hub::paint_status_headline(ui, data, is_dark);
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    // Notification preferences
                    ui.label(
                        egui::RichText::new("Preferences:")
                            .size(11.5)
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.add_space(2.0);

                    let toast_on = app.settings.show_notifications;
                    let toast_label = if toast_on { "🔔 Toast: ON" } else { "🔕 Toast: OFF" };
                    if buttons::toggle_button(
                        ui,
                        toast_label,
                        toast_on,
                        "Toggle Windows desktop notification popups on/off",
                        is_dark,
                    )
                    .clicked()
                    {
                        app.settings.show_notifications = !app.settings.show_notifications;
                        let _ = app.settings.save();
                    }

                    let sound_on = app.settings.enable_alert_sound && app.settings.enable_sounds;
                    let sound_label = if sound_on { "🔊 Sound: ON" } else { "🔇 Sound: OFF" };
                    if buttons::toggle_button(
                        ui,
                        sound_label,
                        sound_on,
                        "Toggle alert notification audio chime on/off",
                        is_dark,
                    )
                    .clicked()
                    {
                        app.settings.enable_alert_sound = !app.settings.enable_alert_sound;
                        let _ = app.settings.save();
                    }

                    // Action controls right-aligned
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if buttons::secondary_button(
                            ui,
                            "⚙ Thresholds",
                            "Configure alert trigger thresholds in Settings (Ctrl+,)",
                            is_dark,
                        )
                        .clicked()
                        {
                            app.show_settings = true;
                        }

                        if buttons::accent_button(
                            ui,
                            "🧪 Test Alert",
                            "Simulate a test hardware alert to verify audio chimes & visual indicators",
                            is_dark,
                        )
                        .clicked()
                        {
                            trigger_test_alert = true;
                        }

                        if !data.alerts.is_empty() {
                            let clear_btn = egui::Button::new(
                                egui::RichText::new("Clear All Alerts")
                                    .strong()
                                    .size(11.5)
                                    .color(ThemePalette::STATUS_CRITICAL),
                            )
                            .fill(ThemePalette::STATUS_CRITICAL.gamma_multiply(if is_dark { 0.15 } else { 0.10 }))
                            .stroke(egui::Stroke::new(
                                1.0,
                                ThemePalette::STATUS_CRITICAL.gamma_multiply(0.45),
                            ))
                            .corner_radius(egui::CornerRadius::same(4));

                            if ui
                                .add(clear_btn)
                                .on_hover_text("Dismiss all active system alerts")
                                .clicked()
                            {
                                clear_all_alerts = true;
                            }
                        }
                    });
                });
            }
        });

        ui.add_space(10.0);

        let avail_w = ui.available_width();
        let is_wide = avail_w >= 840.0;

        if is_wide {
            let col_w = (avail_w - 12.0) / 2.0;

            ui.horizontal_top(|ui| {
                // LEFT COLUMN: Live Metric Proximity & Safety Headroom Matrix
                ui.allocate_ui_with_layout(egui::vec2(col_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    proximity::paint_proximity_matrix(app, ui, data, is_dark);
                });

                ui.add_space(12.0);

                // RIGHT COLUMN: Active Incidents Stream OR Health Board
                ui.allocate_ui_with_layout(egui::vec2(col_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    if data.alerts.is_empty() {
                        health_board::paint_nominal_health_board(ui, data, is_dark);
                    } else {
                        incidents_feed::paint_active_incidents_feed(
                            ui,
                            data,
                            is_dark,
                            &mut remove_alert_idx,
                            &mut navigate_tab,
                            &mut run_ram_clean,
                        );
                    }
                });
            });
        } else {
            // Narrow layout: Stacked vertically
            proximity::paint_proximity_matrix(app, ui, data, is_dark);
            ui.add_space(10.0);
            if data.alerts.is_empty() {
                health_board::paint_nominal_health_board(ui, data, is_dark);
            } else {
                incidents_feed::paint_active_incidents_feed(
                    ui,
                    data,
                    is_dark,
                    &mut remove_alert_idx,
                    &mut navigate_tab,
                    &mut run_ram_clean,
                );
            }
        }
    });

    if let Some(idx) = remove_alert_idx {
        let mut d = app.data.write();
        if idx < d.alerts.len() {
            d.alerts.remove(idx);
        }
    }

    if clear_all_alerts {
        app.data.write().alerts.clear();
    }

    if trigger_test_alert {
        play_alert_sound();
        if app.settings.show_notifications {
            let _ = notify_rust::Notification::new()
                .summary("SysMon Alert Simulation")
                .body("Diagnostic test alert triggered. Audio chime & notification verified.")
                .timeout(notify_rust::Timeout::Milliseconds(5000))
                .show();
        }
        let mut d = app.data.write();
        d.alerts.push(AlertInfo {
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            alert_type: AlertType::CpuHigh,
            source: AlertSource::Cpu,
            message: "Simulated Test Alert: CPU load threshold exceeded (Diagnostic Test)".to_string(),
            value: 95.0,
        });
    }

    if let Some(tab) = navigate_tab {
        app.selected_tab = tab;
    }

    if run_ram_clean {
        app.start_ram_clean(ui.ctx());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::engine::SystemMonitorApp;

    #[test]
    fn test_alerts_page_render_nominal_headless() {
        let mut app = SystemMonitorApp::test_app();
        let data = SystemData::default();

        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                show(&mut app, ui, &data);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_alerts_page_render_with_active_incidents() {
        let mut app = SystemMonitorApp::test_app();
        let data = SystemData {
            alerts: vec![
                AlertInfo {
                    timestamp: "2026-08-17 15:00:00".into(),
                    alert_type: AlertType::CpuHigh,
                    source: AlertSource::Cpu,
                    message: "CPU usage exceeded 90% threshold (94.2%)".into(),
                    value: 94.2,
                },
                AlertInfo {
                    timestamp: "2026-08-17 15:01:00".into(),
                    alert_type: AlertType::MemoryHigh,
                    source: AlertSource::Memory,
                    message: "RAM memory usage critical (92.5%)".into(),
                    value: 92.5,
                },
                AlertInfo {
                    timestamp: "2026-08-17 15:02:00".into(),
                    alert_type: AlertType::GpuTempHigh,
                    source: AlertSource::Gpu {
                        index: 0,
                        name: "Test GPU".into(),
                    },
                    message: "GPU temperature high (88 °C)".into(),
                    value: 88.0,
                },
                AlertInfo {
                    timestamp: "2026-08-17 15:03:00".into(),
                    alert_type: AlertType::DiskSpaceLow,
                    source: AlertSource::Disk {
                        mount_point: "C:\\".into(),
                        name: "C:\\".into(),
                    },
                    message: "C:\\ disk volume almost full (93.1%)".into(),
                    value: 93.1,
                },
                AlertInfo {
                    timestamp: "2026-08-17 15:04:00".into(),
                    alert_type: AlertType::StartupHighImpact,
                    source: AlertSource::Startup,
                    message: "High-impact startup apps detected".into(),
                    value: 3.0,
                },
            ],
            ..Default::default()
        };

        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                show(&mut app, ui, &data);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_alerts_page_render_compact_viewport() {
        let mut app = SystemMonitorApp::test_app();
        let data = SystemData::default();

        let ctx = egui::Context::default();
        // Simulate the 860px width from the screenshot
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(860.0, 700.0))),
            ..Default::default()
        };
        ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                show(&mut app, ui, &data);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_alerts_page_render_wide_viewport() {
        let mut app = SystemMonitorApp::test_app();
        let data = SystemData::default();

        let ctx = egui::Context::default();
        // Simulate 1200px wide viewport
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))),
            ..Default::default()
        };
        ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                show(&mut app, ui, &data);
            });
        })
        .textures_delta
        .clear();
    }
}
