use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Calculates row groupings of card indices based on available width.
/// Breakpoints:
/// - Desktop Wide (avail_w >= 1050.0): 5 cards across in 1 row [0, 1, 2, 3, 4]
/// - Standard (700.0 <= avail_w < 1050.0): Row 1 [0, 1, 2], Row 2 [3, 4]
/// - Compact (avail_w < 700.0): Row 1 [0, 1], Row 2 [2, 3], Row 3 [4]
pub(crate) fn calculate_metric_grid_rows(avail_w: f32) -> Vec<Vec<usize>> {
    if avail_w >= 1050.0 {
        vec![vec![0, 1, 2, 3, 4]]
    } else if avail_w >= 700.0 {
        vec![vec![0, 1, 2], vec![3, 4]]
    } else {
        vec![vec![0, 1], vec![2, 3], vec![4]]
    }
}

pub(crate) fn format_uptime(uptime_secs: u64) -> String {
    let d = uptime_secs / 86400;
    let h = (uptime_secs % 86400) / 3600;
    let m = (uptime_secs % 3600) / 60;
    format!("{}d {}h {}m", d, h, m)
}

struct MetricCard {
    title: &'static str,
    accent: egui::Color32,
    subtitle: String,
    fraction: f32,
    color: egui::Color32,
}

fn paint_metric_card(ui: &mut egui::Ui, cr: egui::Rect, card: &MetricCard, is_dark: bool) {
    let card_bg = ThemePalette::bg_card(is_dark);
    let card_border = egui::Stroke::new(1.0, ThemePalette::border(is_dark));
    let card_rnd = egui::Rounding::same(6.0);

    // Card background & 1px border
    ui.painter().rect_filled(cr, card_rnd, card_bg);
    ui.painter().rect_stroke(cr, card_rnd, card_border);

    // Accent indicator dot
    ui.painter()
        .circle_filled(cr.min + egui::vec2(14.0, 14.0), 3.0, card.accent);

    // Title (Monospace uppercase)
    ui.painter().text(
        cr.min + egui::vec2(22.0, 7.0),
        egui::Align2::LEFT_TOP,
        card.title,
        egui::FontId::monospace(10.5),
        ThemePalette::text_secondary(is_dark),
    );

    // Precision circular telemetry gauge
    let radius = 25.0;
    let center = cr.min + egui::vec2(cr.width() / 2.0, 62.0);
    paint_circular_gauge(ui, center, radius, card.fraction, card.color, is_dark);

    // Monospace secondary subtitle
    ui.painter().text(
        cr.min + egui::vec2(cr.width() / 2.0, cr.height() - 11.0),
        egui::Align2::CENTER_BOTTOM,
        &card.subtitle,
        egui::FontId::monospace(10.0),
        ThemePalette::text_dimmed(is_dark),
    );
}

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "System Overview", is_dark);

    // Show loading state until first telemetry data arrives
    if data.memory_total == 0 {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("Initializing telemetry engines...")
                    .size(14.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Waiting for system telemetry snapshot")
                    .size(11.0)
                    .monospace()
                    .color(ThemePalette::text_dimmed(is_dark)),
            );
        });
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        let avail_w = ui.available_width();

        // ── 1. Dynamic Breakpoint-Aware Metric Cards Grid ──
        let cpu_c = get_usage_color(data.cpu_usage);
        let mem_c = get_usage_color(data.memory_percentage);

        let net_total_rate = data
            .network_info
            .iter()
            .map(|n| n.received_rate + n.transmitted_rate)
            .sum::<f64>();
        let net_download_rate = data.network_info.iter().map(|n| n.received_rate).sum::<f64>();
        let net_upload_rate = data.network_info.iter().map(|n| n.transmitted_rate).sum::<f64>();
        let net_c = if net_total_rate > 25.0 {
            ThemePalette::STATUS_CRITICAL
        } else if net_total_rate > 5.0 {
            ThemePalette::STATUS_WARNING
        } else if net_total_rate > 0.05 {
            ThemePalette::STATUS_HEALTHY
        } else {
            ThemePalette::text_dimmed(is_dark)
        };

        let disk_total_rate = data.disk_read_rate + data.disk_write_rate;
        let disk_c = if disk_total_rate > 100.0 {
            ThemePalette::STATUS_CRITICAL
        } else if disk_total_rate > 20.0 {
            ThemePalette::STATUS_WARNING
        } else if disk_total_rate > 0.05 {
            ThemePalette::STATUS_HEALTHY
        } else {
            ThemePalette::text_dimmed(is_dark)
        };

        let (gpu_sub, gpu_frac, gpu_c) = if let Some(gpu) = data.gpu_info.first() {
            let c = get_usage_color(gpu.utilization);
            let sub = if let (Some(u), Some(t)) = (gpu.memory_used, gpu.memory_total) {
                format!("{:.0}/{:.0} MB", bytes_to_mb(u), bytes_to_mb(t))
            } else if let Some(mhz) = gpu.clock_mhz {
                format!("{} MHz", mhz)
            } else {
                let name = if gpu.name.chars().count() > 20 {
                    let truncated: String = gpu.name.chars().take(18).collect();
                    format!("{}…", truncated)
                } else {
                    gpu.name.clone()
                };
                name
            };
            (sub, (gpu.utilization / 100.0).clamp(0.0, 1.0), c)
        } else {
            ("Not detected".to_string(), 0.0, ThemePalette::text_dimmed(is_dark))
        };

        let cpu_sub = if let Some(temp) = data.cpu_temperature {
            format!("{} Cores · {:.0}°C", data.cpu_cores.len(), temp)
        } else {
            format!("{} Cores", data.cpu_cores.len())
        };

        let cards = [
            MetricCard {
                title: "CPU",
                accent: ThemePalette::ACCENT_PRIMARY,
                subtitle: cpu_sub,
                fraction: (data.cpu_usage / 100.0).clamp(0.0, 1.0),
                color: cpu_c,
            },
            MetricCard {
                title: "MEMORY",
                accent: ThemePalette::ACCENT_ACTIVE,
                subtitle: format!(
                    "{:.1} / {:.1} GB",
                    bytes_to_gb(data.memory_used),
                    bytes_to_gb(data.memory_total)
                ),
                fraction: (data.memory_percentage / 100.0).clamp(0.0, 1.0),
                color: mem_c,
            },
            MetricCard {
                title: "GPU",
                accent: ThemePalette::text_secondary(is_dark),
                subtitle: gpu_sub,
                fraction: gpu_frac,
                color: gpu_c,
            },
            MetricCard {
                title: "DISK I/O",
                accent: ThemePalette::text_secondary(is_dark),
                subtitle: format!(
                    "R: {} · W: {}",
                    format_rate(data.disk_read_rate),
                    format_rate(data.disk_write_rate)
                ),
                fraction: ((disk_total_rate / 200.0).clamp(0.0, 1.0) as f32),
                color: disk_c,
            },
            MetricCard {
                title: "NETWORK",
                accent: ThemePalette::text_secondary(is_dark),
                subtitle: format!(
                    "D: {} · U: {}",
                    format_rate(net_download_rate),
                    format_rate(net_upload_rate)
                ),
                fraction: ((net_total_rate / 10.0).clamp(0.0, 1.0) as f32),
                color: net_c,
            },
        ];

        let card_spacing = 10.0;
        let card_height = 126.0;
        let rows = calculate_metric_grid_rows(avail_w);

        for row_indices in rows {
            let count = row_indices.len() as f32;
            let card_w = if count == 1.0 && avail_w < 700.0 {
                (avail_w - card_spacing) / 2.0
            } else {
                (avail_w - card_spacing * (count - 1.0).max(0.0)) / count
            };

            let (row_rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, card_height), egui::Sense::hover());

            for (col_i, &card_i) in row_indices.iter().enumerate() {
                let x = row_rect.min.x + (card_w + card_spacing) * col_i as f32;
                let card_rect =
                    egui::Rect::from_min_size(egui::pos2(x, row_rect.min.y), egui::vec2(card_w, card_height));
                paint_metric_card(ui, card_rect, &cards[card_i], is_dark);
            }
            ui.add_space(card_spacing);
        }

        ui.add_space(4.0);

        // ── 2. Hardware Spec & Uptime Banner ──
        card_frame(is_dark).show(ui, |ui| {
            let width = ui.available_width();
            if width >= 850.0 {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("CPU")
                            .size(10.5)
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_dimmed(is_dark)),
                    );
                    let cpu_brand = if data.system_info.cpu_brand.is_empty() {
                        "Generic Processor".to_string()
                    } else {
                        data.system_info.cpu_brand.trim().to_string()
                    };
                    ui.label(
                        egui::RichText::new(cpu_brand)
                            .size(11.0)
                            .monospace()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    if let Some(temp) = data.cpu_temperature {
                        let tc = get_usage_color(temp);
                        ui.label(
                            egui::RichText::new(format!("{:.0}°C", temp))
                                .size(10.5)
                                .monospace()
                                .strong()
                                .color(tc),
                        );
                    }
                    ui.separator();

                    ui.label(
                        egui::RichText::new("GPU")
                            .size(10.5)
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_dimmed(is_dark)),
                    );
                    if let Some(gpu) = data.gpu_info.first() {
                        ui.label(
                            egui::RichText::new(&gpu.name)
                                .size(11.0)
                                .monospace()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        if let Some(temp) = gpu.temperature {
                            let tc = get_usage_color(temp as f32);
                            ui.label(
                                egui::RichText::new(format!("{}°C", temp))
                                    .size(10.5)
                                    .monospace()
                                    .strong()
                                    .color(tc),
                            );
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("Standard Graphics")
                                .size(11.0)
                                .monospace()
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(&data.last_update)
                                .size(10.5)
                                .monospace()
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                        ui.separator();

                        ui.label(
                            egui::RichText::new(format!("Uptime: {}", format_uptime(data.system_info.uptime)))
                                .size(11.0)
                                .monospace()
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.separator();

                        if data.cpu_usage > 90.0 || data.memory_percentage > 90.0 {
                            status_pill(ui, "● High Load", ThemePalette::STATUS_CRITICAL, is_dark);
                        } else if data.cpu_usage > 75.0 || data.memory_percentage > 80.0 {
                            status_pill(ui, "● Moderate Load", ThemePalette::STATUS_WARNING, is_dark);
                        } else {
                            status_pill(ui, "● System Healthy", ThemePalette::STATUS_HEALTHY, is_dark);
                        }
                    });
                });
            } else {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("CPU:")
                                .size(10.5)
                                .monospace()
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                        let cpu_brand = if data.system_info.cpu_brand.is_empty() {
                            "Generic Processor".to_string()
                        } else {
                            data.system_info.cpu_brand.trim().to_string()
                        };
                        ui.label(
                            egui::RichText::new(cpu_brand)
                                .size(10.5)
                                .monospace()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        if let Some(temp) = data.cpu_temperature {
                            let tc = get_usage_color(temp);
                            ui.label(
                                egui::RichText::new(format!("{:.0}°C", temp))
                                    .size(10.5)
                                    .monospace()
                                    .strong()
                                    .color(tc),
                            );
                        }
                    });

                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("GPU:")
                                .size(10.5)
                                .monospace()
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                        if let Some(gpu) = data.gpu_info.first() {
                            ui.label(
                                egui::RichText::new(&gpu.name)
                                    .size(10.5)
                                    .monospace()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                            if let Some(temp) = gpu.temperature {
                                let tc = get_usage_color(temp as f32);
                                ui.label(
                                    egui::RichText::new(format!("{}°C", temp))
                                        .size(10.5)
                                        .monospace()
                                        .strong()
                                        .color(tc),
                                );
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("Standard Graphics")
                                    .size(10.5)
                                    .monospace()
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                        }
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Uptime: {}", format_uptime(data.system_info.uptime)))
                                .size(10.5)
                                .monospace()
                                .color(ThemePalette::text_secondary(is_dark)),
                        );

                        if data.cpu_usage > 90.0 || data.memory_percentage > 90.0 {
                            status_pill(ui, "● High Load", ThemePalette::STATUS_CRITICAL, is_dark);
                        } else if data.cpu_usage > 75.0 || data.memory_percentage > 80.0 {
                            status_pill(ui, "● Moderate Load", ThemePalette::STATUS_WARNING, is_dark);
                        } else {
                            status_pill(ui, "● System Healthy", ThemePalette::STATUS_HEALTHY, is_dark);
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(&data.last_update)
                                    .size(10.0)
                                    .monospace()
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                        });
                    });
                });
            }
        });

        ui.add_space(10.0);

        // ── 3. Per-core CPU usage bars (if enabled) ──
        if app.settings.show_per_core_cpu && !data.cpu_cores.is_empty() {
            card_frame(is_dark).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("PER-CORE CPU USAGE")
                        .size(11.0)
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.add_space(6.0);
                let cols = data.cpu_cores.len().min(16);
                ui.columns(cols, |col_uis| {
                    for (i, core) in data.cpu_cores.iter().take(cols).enumerate() {
                        let frac = (core.usage / 100.0).clamp(0.0, 1.0);
                        let c = get_usage_color(core.usage);
                        col_uis[i].label(
                            egui::RichText::new(format!("C{:02}", core.core_id))
                                .size(9.0)
                                .monospace()
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                        paint_progress_bar(&mut col_uis[i], frac, c, 4.0, is_dark);
                        col_uis[i].label(
                            egui::RichText::new(format!("{:.0}%", core.usage))
                                .size(9.0)
                                .monospace()
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                    }
                });
            });
            ui.add_space(10.0);
        }

        // ── 4. Startup & Boot Health Quick Card ──
        {
            let high = startup::high_impact_count(&app.startup_items);
            let total = app.startup_items.len();

            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("STARTUP & BOOT TELEMETRY")
                            .size(11.0)
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(egui::RichText::new("Manage Startup Apps →").size(11.0))
                            .clicked()
                        {
                            app.selected_tab = Tab::StartupManager;
                        }
                    });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if let Some(boot_ms) = app.boot_diagnostics.as_ref().and_then(|bd| bd.boot_duration_ms) {
                        let sec = boot_ms as f64 / 1000.0;
                        let c = if boot_ms < 30000 {
                            ThemePalette::STATUS_HEALTHY
                        } else if boot_ms < 60000 {
                            ThemePalette::STATUS_WARNING
                        } else {
                            ThemePalette::STATUS_CRITICAL
                        };
                        ui.label(
                            egui::RichText::new(format!("Boot Duration: {:.1}s", sec))
                                .monospace()
                                .strong()
                                .color(c),
                        );
                        ui.separator();
                    }

                    if high > 0 {
                        status_pill(
                            ui,
                            &format!("{} High Impact", high),
                            ThemePalette::STATUS_WARNING,
                            is_dark,
                        );
                    } else {
                        status_pill(ui, "● 0 High Impact", ThemePalette::STATUS_HEALTHY, is_dark);
                    }

                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("{} Registered Items", total))
                            .monospace()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                });
            });
        }

        ui.add_space(10.0);

        // ── 5. Top Processes Table Preview ──
        if app.settings.show_processes {
            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("TOP PROCESSES BY RESOURCE USAGE")
                            .size(11.0)
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(egui::RichText::new("View All Processes →").size(11.0))
                            .clicked()
                        {
                            app.selected_tab = Tab::Processes;
                        }
                    });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                if data.top_processes.is_empty() {
                    ui.label(
                        egui::RichText::new("No active process telemetry available")
                            .monospace()
                            .color(ThemePalette::text_dimmed(is_dark)),
                    );
                } else {
                    let max_proc_mem = data.top_processes.iter().map(|p| p.memory).max().unwrap_or(1).max(1);

                    egui::Grid::new("overview_top_processes_grid")
                        .striped(false)
                        .spacing([16.0, 6.0])
                        .min_col_width(50.0)
                        .show(ui, |ui| {
                            // Header row
                            ui.label(
                                egui::RichText::new("PID")
                                    .size(10.0)
                                    .monospace()
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new("PROCESS NAME")
                                    .size(10.0)
                                    .monospace()
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new("MEMORY USAGE")
                                    .size(10.0)
                                    .monospace()
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new("CPU %")
                                    .size(10.0)
                                    .monospace()
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                            ui.end_row();

                            for process in data.top_processes.iter().take(8) {
                                // PID
                                ui.label(
                                    egui::RichText::new(format!("{:>5}", process.pid))
                                        .size(11.0)
                                        .monospace()
                                        .color(ThemePalette::text_dimmed(is_dark)),
                                );

                                // Process Name
                                let name = if process.name.chars().count() > 30 {
                                    let truncated: String = process.name.chars().take(28).collect();
                                    format!("{}…", truncated)
                                } else {
                                    process.name.clone()
                                };
                                ui.label(
                                    egui::RichText::new(name)
                                        .size(11.5)
                                        .monospace()
                                        .strong()
                                        .color(ThemePalette::text_primary(is_dark)),
                                );

                                // Memory with inline ratio bar
                                let mb = bytes_to_mb(process.memory);
                                let mc = if mb > 1000.0 {
                                    ThemePalette::STATUS_CRITICAL
                                } else if mb > 400.0 {
                                    ThemePalette::STATUS_WARNING
                                } else {
                                    ThemePalette::STATUS_HEALTHY
                                };
                                let mem_bar_frac = (process.memory as f32 / max_proc_mem as f32).clamp(0.0, 1.0);

                                ui.horizontal(|ui| {
                                    let bar_w = 48.0;
                                    let bar_h = 4.0;
                                    let (rect, _) =
                                        ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
                                    let rnd = egui::Rounding::same(2.0);
                                    ui.painter().rect_filled(rect, rnd, ThemePalette::bg_deepest(is_dark));
                                    let fill_w = (bar_w * mem_bar_frac).max(2.0);
                                    let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(fill_w, bar_h));
                                    ui.painter().rect_filled(fill_rect, rnd, mc);

                                    ui.label(
                                        egui::RichText::new(format!("{:>7.1} MB", mb))
                                            .size(11.0)
                                            .monospace()
                                            .color(ThemePalette::text_primary(is_dark)),
                                    );
                                });

                                // CPU %
                                let cpu_c = get_usage_color(process.cpu_usage);
                                ui.label(
                                    egui::RichText::new(format!("{:>5.1}%", process.cpu_usage))
                                        .size(11.0)
                                        .monospace()
                                        .color(cpu_c),
                                );

                                ui.end_row();
                            }
                        });
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_metric_grid_rows_breakpoints() {
        // Desktop Wide >= 1050
        let wide_rows = calculate_metric_grid_rows(1200.0);
        assert_eq!(wide_rows, vec![vec![0, 1, 2, 3, 4]]);

        let edge_wide = calculate_metric_grid_rows(1050.0);
        assert_eq!(edge_wide, vec![vec![0, 1, 2, 3, 4]]);

        // Standard 700..1050
        let std_rows = calculate_metric_grid_rows(900.0);
        assert_eq!(std_rows, vec![vec![0, 1, 2], vec![3, 4]]);

        let edge_std = calculate_metric_grid_rows(700.0);
        assert_eq!(edge_std, vec![vec![0, 1, 2], vec![3, 4]]);

        // Compact < 700
        let compact_rows = calculate_metric_grid_rows(650.0);
        assert_eq!(compact_rows, vec![vec![0, 1], vec![2, 3], vec![4]]);

        let very_compact = calculate_metric_grid_rows(400.0);
        assert_eq!(very_compact, vec![vec![0, 1], vec![2, 3], vec![4]]);
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(0), "0d 0h 0m");
        assert_eq!(format_uptime(59), "0d 0h 0m");
        assert_eq!(format_uptime(60), "0d 0h 1m");
        assert_eq!(format_uptime(3665), "0d 1h 1m");
        assert_eq!(format_uptime(86400 + 7200 + 180), "1d 2h 3m");
        assert_eq!(format_uptime(112500), "1d 7h 15m");
    }
}
