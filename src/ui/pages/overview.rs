use crate::*;
use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "System Overview");

        // Show loading state until first data arrives
        if data.memory_total == 0 {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Collecting system data...")
                        .size(18.0)
                        .color(ThemePalette::TEXT_SUBTITLE),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Please wait a moment.")
                        .size(13.0)
                        .color(ThemePalette::TEXT_DIMMED),
                );
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Metric cards row ──
            let card_bg = ThemePalette::BG_CARD;
            let card_border = egui::Stroke::new(1.0, ThemePalette::BORDER);
            let card_rnd = egui::Rounding::same(12.0); // Premium smooth rounding

            let full_avail = ui.available_width();
            let card_spacing = 16.0;
            let card_h = 120.0;
            let (row_rect, _) = ui.allocate_exact_size(egui::vec2(full_avail, card_h), egui::Sense::hover());

            // Account for HiDPI: at ppp>1, available_width can exceed visible area
            let ppp = ui.ctx().pixels_per_point();
            let visible_w = if ppp > 1.01 {
                let screen_w = ui.ctx().screen_rect().width();
                (screen_w / ppp - row_rect.min.x).max(200.0)
            } else {
                full_avail
            };
            let card_w = ((visible_w - card_spacing * 4.0) / 5.0).max(80.0);

            // Prepare card data
            let cpu_c = get_usage_color(data.cpu_usage);
            let mem_c = get_usage_color(data.memory_percentage);

            let net_total_rate = data.network_info.iter().map(|n| n.received_rate + n.transmitted_rate).sum::<f64>();
            let net_download_rate = data.network_info.iter().map(|n| n.received_rate).sum::<f64>();
            let net_upload_rate = data.network_info.iter().map(|n| n.transmitted_rate).sum::<f64>();
            let net_c = if net_total_rate > 5_000_000.0 {
                ThemePalette::STATUS_WARNING
            } else if net_total_rate > 1_000_000.0 {
                ThemePalette::STATUS_HEALTHY
            } else {
                ThemePalette::TEXT_LABEL_SUB
            };

            let (gpu_val, gpu_sub, gpu_frac, gpu_c) = if let Some(gpu) = data.gpu_info.first() {
                let c = get_usage_color(gpu.utilization);
                let mut sub = if let (Some(u), Some(t)) = (gpu.memory_used, gpu.memory_total) {
                    format!("{:.0}/{:.0} MB", bytes_to_mb(u), bytes_to_mb(t))
                } else {
                    gpu.name.clone()
                };
                sub = match gpu.clock_mhz {
                    Some(mhz) => format!("{} · {} MHz", sub, mhz),
                    None => sub,
                };
                (format!("{:.1}%", gpu.utilization), sub, gpu.utilization / 100.0, c)
            } else {
                (
                    "N/A".to_string(),
                    "Not detected".to_string(),
                    0.0,
                    ThemePalette::GPU_UNAVAILABLE,
                )
            };

            let cards = [
                (
                    ThemePalette::ACCENT_PRIMARY,
                    "CPU",
                    format!("{:.1}%", data.cpu_usage),
                    if let Some(temp) = data.cpu_temperature {
                        format!("{} cores • {}°C", data.cpu_cores.len(), temp)
                    } else {
                        format!("{} cores", data.cpu_cores.len())
                    },
                    data.cpu_usage / 100.0,
                    cpu_c,
                ),
                (
                    ThemePalette::ACCENT_ACTIVE,
                    "MEMORY",
                    format!("{:.1}%", data.memory_percentage),
                    format!(
                        "{:.1} / {:.1} GB",
                        bytes_to_gb(data.memory_used),
                        bytes_to_gb(data.memory_total)
                    ),
                    data.memory_percentage / 100.0,
                    mem_c,
                ),
                (ThemePalette::ACCENT_PURPLE, "GPU", gpu_val, gpu_sub, gpu_frac, gpu_c),
                (
                    ThemePalette::TEXT_LABEL_SUB,
                    "DISK I/O",
                    format_rate(data.disk_read_rate + data.disk_write_rate),
                    format!("R: {}  W: {}", format_rate(data.disk_read_rate), format_rate(data.disk_write_rate)),
                    ((data.disk_read_rate + data.disk_write_rate) / 200.0).clamp(0.0, 1.0) as f32,
                    ThemePalette::TEXT_LABEL_SUB,
                ),
                (
                    ThemePalette::ACCENT_CYAN,
                    "NETWORK",
                    format_rate(net_total_rate),
                    format!("D: {}  U: {}", format_rate(net_download_rate), format_rate(net_upload_rate)),
                    (net_total_rate / 10_000_000.0).clamp(0.0, 1.0) as f32,
                    net_c,
                ),
            ];

            for (i, (accent, label, value, sub, frac, color)) in cards.iter().enumerate() {
                let x = row_rect.min.x + (card_w + card_spacing) * i as f32;
                let cr = egui::Rect::from_min_size(egui::pos2(x, row_rect.min.y), egui::vec2(card_w, card_h));

                // Deep card background with subtle inner border
                ui.painter().rect_filled(cr, card_rnd, ThemePalette::BG_DEEPEST);
                ui.painter().rect_filled(cr.shrink(1.0), card_rnd, card_bg);
                ui.painter().rect_stroke(cr, card_rnd, card_border);

                // Accent dot
                ui.painter().circle_filled(
                    cr.min + egui::vec2(16.0, 18.0),
                    3.0,
                    *accent,
                );

                // Title
                ui.painter().text(
                    cr.min + egui::vec2(26.0, 12.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::FontId::proportional(12.0),
                    ThemePalette::TEXT_LABEL_SUB,
                );

                // Circular Gauge
                let radius = 28.0;
                let center = cr.min + egui::vec2(card_w / 2.0, card_h / 2.0 - 4.0);
                paint_circular_gauge(ui, center, radius, *frac, *color, "");

                // Value Text inside gauge
                ui.painter().text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    value,
                    egui::FontId::new(14.0, egui::FontFamily::Monospace),
                    ThemePalette::TEXT_PRIMARY,
                );

                // Subtitle
                ui.painter().text(
                    cr.min + egui::vec2(card_w / 2.0, card_h - 18.0),
                    egui::Align2::CENTER_BOTTOM,
                    sub,
                    egui::FontId::proportional(11.0),
                    ThemePalette::TEXT_DIMMED,
                );
            }

            ui.add_space(16.0);

            // ── Detail strip ──
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if let Some(gpu) = data.gpu_info.first() {
                        if let Some(temp) = gpu.temperature {
                            let tc = if temp < 70 {
                                ThemePalette::STATUS_HEALTHY
                            } else if temp < 85 {
                                ThemePalette::STATUS_WARNING
                            } else {
                                ThemePalette::STATUS_CRITICAL
                            };
                            ui.label(egui::RichText::new(format!("{}°C", temp)).strong().color(tc));
                            ui.separator();
                        }
                        if gpu.clock_mhz.is_some() || gpu.power_watts.is_some() || gpu.fan_percent.is_some() {
                            let mut parts = Vec::new();
                            if let Some(mhz) = gpu.clock_mhz {
                                parts.push(format!("{} MHz", mhz));
                            }
                            if let Some(w) = gpu.power_watts {
                                parts.push(format!("{:.0} W", w));
                            }
                            if let Some(f) = gpu.fan_percent {
                                parts.push(format!("{}% fan", f));
                            }
                            ui.label(parts.join(" · "));
                            ui.separator();
                        }
                        ui.label(
                            egui::RichText::new(&gpu.name)
                                .size(11.5)
                                .color(ThemePalette::TEXT_LABEL_SUB),
                        );
                        ui.separator();
                    }
                    let d = data.system_info.uptime / 86400;
                    let h = (data.system_info.uptime % 86400) / 3600;
                    let m = (data.system_info.uptime % 3600) / 60;
                    ui.label(
                        egui::RichText::new(format!("Uptime {}d {}h {}m", d, h, m))
                            .size(11.5)
                            .color(ThemePalette::TEXT_LABEL_SUB),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(&data.last_update)
                                .size(11.0)
                                .color(ThemePalette::TEXT_DIMMED),
                        );
                    });
                });
            });

            ui.add_space(12.0);

            // ── Startup Health ──
            {
                let high = startup::high_impact_count(&app.startup_items);
                let total = app.startup_items.len();
                let boot_text = app.boot_diagnostics.as_ref()
                    .and_then(|bd| bd.boot_duration_ms)
                    .map(|ms| format!("{:.1}s", ms as f64 / 1000.0));

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("STARTUP HEALTH")
                            .size(10.0).color(ThemePalette::TEXT_DIMMED));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("View All >").clicked() {
                                app.selected_tab = Tab::StartupManager;
                            }
                        });
                    });

                    ui.horizontal(|ui| {
                        if let Some(ref bt) = boot_text {
                            let boot_ms = app.boot_diagnostics.as_ref().and_then(|b| b.boot_duration_ms).unwrap_or(0);
                            let c = if boot_ms < 30000 { ThemePalette::STATUS_HEALTHY }
                                    else if boot_ms < 60000 { ThemePalette::STATUS_WARNING }
                                    else { ThemePalette::STATUS_CRITICAL };
                            ui.colored_label(c, egui::RichText::new(format!("Boot: {}", bt)).strong());
                            ui.separator();
                        }
                        if high > 0 {
                            ui.colored_label(ThemePalette::STATUS_CRITICAL,
                                format!("{} high-impact", high));
                        } else {
                            ui.colored_label(ThemePalette::STATUS_HEALTHY, "Healthy");
                        }
                        ui.separator();
                        ui.label(egui::RichText::new(format!("{} startup items", total))
                            .color(ThemePalette::TEXT_LABEL_SUB));
                    });
                });
            }

            ui.add_space(12.0);

            // ── Top processes ──
            if app.settings.show_processes {
                paint_section_header(ui, "Top Processes");

                egui::Grid::new("overview_process_grid")
                    .striped(true)
                    .spacing([14.0, 5.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("PROCESS")
                                .size(10.0)
                                .color(ThemePalette::TEXT_DIMMED),
                        );
                        ui.label(
                            egui::RichText::new("MEMORY")
                                .size(10.0)
                                .color(ThemePalette::TEXT_DIMMED),
                        );
                        ui.label(egui::RichText::new("CPU").size(10.0).color(ThemePalette::TEXT_DIMMED));
                        ui.end_row();

                        for process in data.top_processes.iter().take(8) {
                            let mb = bytes_to_mb(process.memory);
                            let mc = if mb > 500.0 {
                                ThemePalette::STATUS_CRITICAL
                            } else if mb > 200.0 {
                                ThemePalette::STATUS_WARNING
                            } else {
                                ThemePalette::STATUS_HEALTHY
                            };
                            let name = if process.name.chars().count() > 32 {
                                let truncated: String = process.name.chars().take(30).collect();
                                format!("{}…", truncated)
                            } else {
                                process.name.clone()
                            };
                            ui.label(egui::RichText::new(name).size(12.5));
                            ui.colored_label(mc, format!("{:.1} MB", mb));
                            ui.label(format!("{:.1}%", process.cpu_usage));
                            ui.end_row();
                        }
                    });
            }
        });
    }
