
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
        // ── 1. Top Status & Control Hub ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                if data.alerts.is_empty() {
                    status_pill(ui, "ALL SYSTEMS NOMINAL", ThemePalette::STATUS_HEALTHY, is_dark);
                    ui.label(
                        egui::RichText::new("Zero active threshold violations · Real-time telemetry operating within safety boundaries.")
                            .size(12.5)
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                } else {
                    let count = data.alerts.len();
                    status_pill(ui, &format!("⚠️ {} ACTIVE INCIDENT{}", count, if count > 1 { "S" } else { "" }), ThemePalette::STATUS_WARNING, is_dark);
                    ui.label(
                        egui::RichText::new("Metric threshold violations detected · Immediate review and mitigation recommended.")
                            .size(12.5)
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Settings shortcut
                    if ui.button("⚙ Thresholds").on_hover_text("Configure alert trigger thresholds in Settings (Ctrl+,)").clicked() {
                        app.show_settings = true;
                    }

                    // Test Alert Button
                    let test_btn = egui::Button::new(
                        egui::RichText::new("🧪 Test Alert")
                            .size(11.5)
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    )
                    .fill(ThemePalette::ACCENT_PRIMARY.gamma_multiply(if is_dark { 0.15 } else { 0.10 }))
                    .stroke(egui::Stroke::new(1.0, ThemePalette::ACCENT_PRIMARY.gamma_multiply(0.45)))
                    .rounding(egui::Rounding::same(4.0));

                    if ui.add(test_btn).on_hover_text("Simulate a test hardware alert to verify audio chimes & visual indicators").clicked() {
                        trigger_test_alert = true;
                    }

                    // Sound Toggle Button
                    let sound_on = app.settings.enable_alert_sound && app.settings.enable_sounds;
                    let sound_label = if sound_on { "🔔 Sound: ON" } else { "🔕 Sound: OFF" };
                    let sound_btn = egui::Button::new(
                        egui::RichText::new(sound_label).size(11.5).strong().color(
                            if sound_on {
                                ThemePalette::STATUS_HEALTHY
                            } else {
                                ThemePalette::text_dimmed(is_dark)
                            },
                        ),
                    )
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

                    if ui.add(sound_btn).on_hover_text("Toggle alert notification audio chime on/off").clicked() {
                        app.settings.enable_alert_sound = !app.settings.enable_alert_sound;
                        let _ = app.settings.save();
                    }

                    // Desktop Toast Notifications Toggle Button
                    let toast_on = app.settings.show_notifications;
                    let toast_label = if toast_on { "🖥️ Desktop Toast: ON" } else { "🖥️ Desktop Toast: OFF" };
                    let toast_btn = egui::Button::new(
                        egui::RichText::new(toast_label).size(11.5).strong().color(
                            if toast_on {
                                ThemePalette::STATUS_HEALTHY
                            } else {
                                ThemePalette::text_dimmed(is_dark)
                            },
                        ),
                    )
                    .fill(if toast_on {
                        ThemePalette::STATUS_HEALTHY.gamma_multiply(if is_dark { 0.15 } else { 0.10 })
                    } else {
                        ThemePalette::bg_track(is_dark)
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if toast_on {
                            ThemePalette::STATUS_HEALTHY.gamma_multiply(0.4)
                        } else {
                            ThemePalette::border(is_dark)
                        },
                    ))
                    .rounding(egui::Rounding::same(4.0));

                    if ui.add(toast_btn).on_hover_text("Toggle Windows desktop notification popups on/off").clicked() {
                        app.settings.show_notifications = !app.settings.show_notifications;
                        let _ = app.settings.save();
                    }

                    // Clear All Alerts (when alerts exist)
                    if !data.alerts.is_empty() {
                        let clear_btn = egui::Button::new(
                            egui::RichText::new("Clear All Alerts")
                                .strong()
                                .size(11.5)
                                .color(ThemePalette::STATUS_CRITICAL),
                        )
                        .fill(ThemePalette::STATUS_CRITICAL.gamma_multiply(if is_dark { 0.15 } else { 0.10 }))
                        .stroke(egui::Stroke::new(1.0, ThemePalette::STATUS_CRITICAL.gamma_multiply(0.45)))
                        .rounding(egui::Rounding::same(4.0));

                        if ui.add(clear_btn).on_hover_text("Dismiss all active system alerts").clicked() {
                            clear_all_alerts = true;
                        }
                    }
                });
            });
        });

        ui.add_space(10.0);

        // ── 2. Responsive 2-Column Incident & Proximity Grid ──
        let avail_w = ui.available_width();
        let is_wide = avail_w >= 840.0;

        if is_wide {
            let col_w = (avail_w - 12.0) / 2.0;

            ui.horizontal_top(|ui| {
                // LEFT COLUMN: Live Metric Proximity & Safety Headroom Matrix
                ui.allocate_ui_with_layout(
                    egui::vec2(col_w, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        paint_proximity_matrix(app, ui, data, is_dark);
                    },
                );

                ui.add_space(12.0);

                // RIGHT COLUMN: Active Incidents Stream OR Health Board
                ui.allocate_ui_with_layout(
                    egui::vec2(col_w, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        if data.alerts.is_empty() {
                            paint_nominal_health_board(ui, data, is_dark);
                        } else {
                            paint_active_incidents_feed(
                                ui,
                                data,
                                is_dark,
                                &mut remove_alert_idx,
                                &mut navigate_tab,
                                &mut run_ram_clean,
                            );
                        }
                    },
                );
            });
        } else {
            // Narrow layout: Stacked vertically
            paint_proximity_matrix(app, ui, data, is_dark);
            ui.add_space(10.0);
            if data.alerts.is_empty() {
                paint_nominal_health_board(ui, data, is_dark);
            } else {
                paint_active_incidents_feed(
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

    // ── Execute Actions after UI rendering to avoid state borrow conflicts ──
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

/// Renders the Live Metric Proximity & Headroom Matrix card.
fn paint_proximity_matrix(
    app: &crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    data: &SystemData,
    is_dark: bool,
) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("LIVE THRESHOLD PROXIMITY & HEADROOM")
                    .size(11.5)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_pill(ui, "REAL-TIME GAUGES", ThemePalette::ACCENT_PRIMARY, is_dark);
            });
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Live sensor telemetry evaluated against automated guard thresholds:")
                .size(12.0)
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(10.0);

        // 1. CPU Saturation Metric
        let cpu_curr = data.cpu_usage;
        let cpu_thresh = app.settings.notification_cpu_threshold;
        let cpu_headroom = (cpu_thresh - cpu_curr).max(0.0);
        let cpu_color = if cpu_curr >= cpu_thresh {
            ThemePalette::STATUS_CRITICAL
        } else if cpu_curr >= cpu_thresh * 0.85 {
            ThemePalette::STATUS_WARNING
        } else {
            ThemePalette::STATUS_HEALTHY
        };

        paint_proximity_row(
            ui,
            "CPU Saturation Limit",
            &format!("{:.1}%", cpu_curr),
            &format!("> {:.0}%", cpu_thresh),
            &format!("+{:.1}% Headroom", cpu_headroom),
            cpu_curr / 100.0,
            cpu_color,
            is_dark,
        );

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // 2. Memory Exhaustion Metric
        let mem_curr = data.memory_percentage;
        let mem_thresh = app.settings.notification_memory_threshold;
        let mem_headroom = (mem_thresh - mem_curr).max(0.0);
        let mem_color = if mem_curr >= mem_thresh {
            ThemePalette::STATUS_CRITICAL
        } else if mem_curr >= mem_thresh * 0.85 {
            ThemePalette::STATUS_WARNING
        } else {
            ThemePalette::STATUS_HEALTHY
        };

        paint_proximity_row(
            ui,
            "Memory Exhaustion Limit",
            &format!("{:.1}%", mem_curr),
            &format!("> {:.0}%", mem_thresh),
            &format!("+{:.1}% Headroom", mem_headroom),
            mem_curr / 100.0,
            mem_color,
            is_dark,
        );

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // 3. GPU Thermal Boundary
        let gpu_temp_opt = data.gpu_info.first().and_then(|g| g.temperature);
        let gpu_thresh = app.settings.notification_temp_threshold as f32;
        let (gpu_val_str, gpu_headroom_str, gpu_frac, gpu_color) = if let Some(temp) = gpu_temp_opt {
            let temp_f = temp as f32;
            let headroom = (gpu_thresh - temp_f).max(0.0);
            let color = if temp_f >= gpu_thresh {
                ThemePalette::STATUS_CRITICAL
            } else if temp_f >= gpu_thresh * 0.85 {
                ThemePalette::STATUS_WARNING
            } else {
                ThemePalette::STATUS_HEALTHY
            };
            (format!("{:.0} °C", temp_f), format!("+{:.0} °C Margin", headroom), (temp_f / 100.0).clamp(0.0, 1.0), color)
        } else {
            ("N/A".to_string(), "Thermal sensor offline".to_string(), 0.0, ThemePalette::text_dimmed(is_dark))
        };

        paint_proximity_row(
            ui,
            "GPU Thermal Boundary",
            &gpu_val_str,
            &format!("> {:.0} °C", gpu_thresh),
            &gpu_headroom_str,
            gpu_frac,
            gpu_color,
            is_dark,
        );

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // 4. Disk Space Warning Metric (Highest capacity disk)
        let max_disk_usage = data
            .disk_info
            .iter()
            .map(|d| d.usage_percentage)
            .fold(0.0f32, |acc, val| acc.max(val));
        let disk_thresh = app.settings.notification_disk_threshold;
        let disk_headroom = (disk_thresh - max_disk_usage).max(0.0);
        let disk_color = if max_disk_usage >= disk_thresh {
            ThemePalette::STATUS_CRITICAL
        } else if max_disk_usage >= disk_thresh * 0.85 {
            ThemePalette::STATUS_WARNING
        } else {
            ThemePalette::STATUS_HEALTHY
        };

        paint_proximity_row(
            ui,
            "Disk Capacity Warning",
            &format!("{:.1}%", max_disk_usage),
            &format!("> {:.0}%", disk_thresh),
            &format!("+{:.1}% Free Space Margin", disk_headroom),
            max_disk_usage / 100.0,
            disk_color,
            is_dark,
        );
    });
}

/// Helper to render an individual metric threshold proximity row with progress bar and headroom badge.
fn paint_proximity_row(
    ui: &mut egui::Ui,
    title: &str,
    current_val: &str,
    threshold_val: &str,
    headroom_text: &str,
    fraction: f32,
    color: egui::Color32,
    is_dark: bool,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title)
                .strong()
                .size(12.0)
                .color(ThemePalette::text_primary(is_dark)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(headroom_text)
                    .monospace()
                    .size(11.0)
                    .strong()
                    .color(color),
            );
        });
    });

    ui.add_space(3.0);
    paint_progress_bar(ui, fraction, color, 6.0, is_dark);
    ui.add_space(3.0);

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("Current Load: {}", current_val))
                .monospace()
                .size(11.0)
                .color(ThemePalette::text_primary(is_dark)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("Trigger Limit: {}", threshold_val))
                    .monospace()
                    .size(11.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
        });
    });
}

/// Renders the Nominal System Health & Reliability Board when zero active alerts are present.
fn paint_nominal_health_board(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("SYSTEM HEALTH & GUARD STATUS")
                    .size(11.5)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_pill(ui, "ACTIVE MONITORING", ThemePalette::STATUS_HEALTHY, is_dark);
            });
        });

        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Automated diagnostic telemetry engines are actively policing hardware parameters:")
                .size(12.0)
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(10.0);

        // Guard Status Items
        let guards = [
            ("✓ CPU Thermal & Saturation Guard", "Continuous sampling of global usage & per-core throttling thresholds."),
            ("✓ Memory Working Set Watchdog", "Proactively tracks RAM exhaustion with automated one-click working set cleanup."),
            ("✓ Storage Volume Exhaustion Sentinel", "Monitors NTFS & ReFS partition limits to avoid critical disk-write lockouts."),
            ("✓ Windows Startup Degradation Scanner", "Evaluates Boot Diagnostics event logs for rogue startup performance impacts."),
        ];

        for (name, desc) in guards {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(name)
                        .strong()
                        .size(12.0)
                        .color(ThemePalette::STATUS_HEALTHY),
                );
            });
            ui.label(
                egui::RichText::new(desc)
                    .size(11.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(6.0);
        }

        ui.separator();
        ui.add_space(6.0);

        // Live Health Telemetry Summary Grid
        ui.label(
            egui::RichText::new("LIVE DIAGNOSTIC TELEMETRY SCOPE")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(6.0);

        egui::Grid::new("health_board_summary_grid")
            .num_columns(2)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("System Uptime:").size(11.5).color(ThemePalette::text_secondary(is_dark)));
                ui.label(egui::RichText::new(format_uptime(data.system_info.uptime)).monospace().strong().color(ThemePalette::text_primary(is_dark)));
                ui.end_row();

                ui.label(egui::RichText::new("Monitored Processes:").size(11.5).color(ThemePalette::text_secondary(is_dark)));
                ui.label(egui::RichText::new(format!("{} active processes", data.top_processes.len())).monospace().strong().color(ThemePalette::text_primary(is_dark)));
                ui.end_row();

                ui.label(egui::RichText::new("Network Interfaces:").size(11.5).color(ThemePalette::text_secondary(is_dark)));
                ui.label(egui::RichText::new(format!("{} adapters polled", data.network_info.len())).monospace().strong().color(ThemePalette::text_primary(is_dark)));
                ui.end_row();

                ui.label(egui::RichText::new("Storage Volumes:").size(11.5).color(ThemePalette::text_secondary(is_dark)));
                ui.label(egui::RichText::new(format!("{} physical/logical volumes", data.disk_info.len())).monospace().strong().color(ThemePalette::text_primary(is_dark)));
                ui.end_row();
            });
    });
}

/// Renders the stream of active incidents with contextual remediation actions.
fn paint_active_incidents_feed(
    ui: &mut egui::Ui,
    data: &SystemData,
    is_dark: bool,
    remove_alert_idx: &mut Option<usize>,
    navigate_tab: &mut Option<Tab>,
    run_ram_clean: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("ACTIVE INCIDENTS FEED")
                .size(11.5)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.label(
            egui::RichText::new(format!("({} active)", data.alerts.len()))
                .monospace()
                .size(11.0)
                .color(ThemePalette::STATUS_WARNING),
        );
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
                        .size(12.5)
                        .color(ThemePalette::text_primary(is_dark)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("× Dismiss").on_hover_text("Dismiss this incident").clicked() {
                        *remove_alert_idx = Some(i);
                    }

                    ui.label(
                        egui::RichText::new(format!("Peak: {:.1}", alert.value))
                            .monospace()
                            .strong()
                            .size(11.5)
                            .color(color),
                    );
                });
            });

            ui.add_space(6.0);

            // Sub-row: Timestamp & Contextual Remediation Actions
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Triggered: {}", alert.timestamp))
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::text_dimmed(is_dark)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    match alert.alert_type {
                        AlertType::MemoryHigh => {
                            if ui
                                .button(egui::RichText::new("🧹 Clean RAM Now").strong().size(11.0).color(ThemePalette::ACCENT_PRIMARY))
                                .on_hover_text("Execute working set optimization to free up RAM")
                                .clicked()
                            {
                                *run_ram_clean = true;
                            }
                        }
                        AlertType::CpuHigh => {
                            if ui
                                .button(egui::RichText::new("📋 Inspect Processes").strong().size(11.0).color(ThemePalette::ACCENT_PRIMARY))
                                .on_hover_text("Open Process Monitor to inspect high CPU consumers")
                                .clicked()
                            {
                                *navigate_tab = Some(Tab::Processes);
                            }
                        }
                        AlertType::DiskSpaceLow => {
                            if ui
                                .button(egui::RichText::new("💾 Open Storage Manager").strong().size(11.0).color(ThemePalette::ACCENT_PRIMARY))
                                .on_hover_text("Inspect disk usage and partition breakdown")
                                .clicked()
                            {
                                *navigate_tab = Some(Tab::Storage);
                            }
                        }
                        AlertType::GpuTempHigh => {
                            if ui
                                .button(egui::RichText::new("🩺 GPU Diagnostics").strong().size(11.0).color(ThemePalette::ACCENT_PRIMARY))
                                .on_hover_text("Inspect GPU clock rates, fan speeds, and memory usage")
                                .clicked()
                            {
                                *navigate_tab = Some(Tab::Performance);
                            }
                        }
                        AlertType::StartupHighImpact => {
                            if ui
                                .button(egui::RichText::new("🚀 Manage Startup Apps").strong().size(11.0).color(ThemePalette::ACCENT_PRIMARY))
                                .on_hover_text("Open Startup Manager to disable heavy startup programs")
                                .clicked()
                            {
                                *navigate_tab = Some(Tab::StartupManager);
                            }
                        }
                    }
                });
            });
        });

        if i < data.alerts.len() - 1 {
            ui.add_space(6.0);
        }
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
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }

    #[test]
    fn test_alerts_page_render_with_active_incidents() {
        let mut app = SystemMonitorApp::test_app();
        let mut data = SystemData::default();

        data.alerts = vec![
            AlertInfo {
                timestamp: "2026-08-17 15:00:00".into(),
                alert_type: AlertType::CpuHigh,
                message: "CPU usage exceeded 90% threshold (94.2%)".into(),
                value: 94.2,
            },
            AlertInfo {
                timestamp: "2026-08-17 15:01:00".into(),
                alert_type: AlertType::MemoryHigh,
                message: "RAM memory usage critical (92.5%)".into(),
                value: 92.5,
            },
            AlertInfo {
                timestamp: "2026-08-17 15:02:00".into(),
                alert_type: AlertType::GpuTempHigh,
                message: "GPU temperature high (88 °C)".into(),
                value: 88.0,
            },
            AlertInfo {
                timestamp: "2026-08-17 15:03:00".into(),
                alert_type: AlertType::DiskSpaceLow,
                message: "C:\\ disk volume almost full (93.1%)".into(),
                value: 93.1,
            },
            AlertInfo {
                timestamp: "2026-08-17 15:04:00".into(),
                alert_type: AlertType::StartupHighImpact,
                message: "High-impact startup apps detected".into(),
                value: 3.0,
            },
        ];

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }
}
