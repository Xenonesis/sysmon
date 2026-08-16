use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Performance Graphs", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        show_history_summary(ui, data, is_dark);
        ui.add_space(10.0);

        if app.settings.show_graphs {
            ui.columns(2, |cols| {
                // CPU Graph
                card_frame(is_dark).show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("CPU USAGE HISTORY")
                                .size(11.0)
                                .strong()
                                .color(ThemePalette::ACCENT_PRIMARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", data.cpu_usage))
                                    .monospace()
                                    .strong()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                        });
                    });

                    ui.add_space(4.0);
                    let cpu_points: PlotPoints = data.cpu_history.iter().map(|p| [p.time, p.value]).collect();
                    let line = Line::new(cpu_points).color(ThemePalette::ACCENT_PRIMARY).width(1.5);

                    Plot::new("cpu_plot")
                        .height(180.0)
                        .allow_zoom(false)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .include_y(0.0)
                        .include_y(100.0)
                        .y_axis_label("CPU %")
                        .show(ui, |plot_ui| {
                            plot_ui.line(line);
                        });
                });

                // Memory Graph
                card_frame(is_dark).show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("MEMORY USAGE HISTORY")
                                .size(11.0)
                                .strong()
                                .color(ThemePalette::STATUS_HEALTHY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", data.memory_percentage))
                                    .monospace()
                                    .strong()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                        });
                    });

                    ui.add_space(4.0);
                    let mem_points: PlotPoints = data.memory_history.iter().map(|p| [p.time, p.value]).collect();
                    let line = Line::new(mem_points).color(ThemePalette::STATUS_HEALTHY).width(1.5);

                    Plot::new("memory_plot")
                        .height(180.0)
                        .allow_zoom(false)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .include_y(0.0)
                        .include_y(100.0)
                        .y_axis_label("Memory %")
                        .show(ui, |plot_ui| {
                            plot_ui.line(line);
                        });
                });
            });

            ui.add_space(10.0);

            ui.columns(2, |cols| {
                // GPU Graph
                if !data.gpu_history.is_empty() {
                    card_frame(is_dark).show(&mut cols[0], |ui| {
                        let gpu_usage = data.gpu_info.first().map(|g| g.utilization).unwrap_or(0.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("GPU USAGE HISTORY")
                                    .size(11.0)
                                    .strong()
                                    .color(ThemePalette::STATUS_WARNING),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(format!("{:.1}%", gpu_usage))
                                        .monospace()
                                        .strong()
                                        .color(ThemePalette::text_primary(is_dark)),
                                );
                            });
                        });

                        ui.add_space(4.0);
                        let gpu_points: PlotPoints = data.gpu_history.iter().map(|p| [p.time, p.value]).collect();
                        let line = Line::new(gpu_points).color(ThemePalette::STATUS_WARNING).width(1.5);

                        Plot::new("gpu_plot")
                            .height(180.0)
                            .allow_zoom(false)
                            .allow_drag(false)
                            .allow_scroll(false)
                            .include_y(0.0)
                            .include_y(100.0)
                            .y_axis_label("GPU %")
                            .show(ui, |plot_ui| {
                                plot_ui.line(line);
                            });
                    });
                }

                // Disk I/O Graph
                card_frame(is_dark).show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("DISK I/O HISTORY")
                                .size(11.0)
                                .strong()
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "R: {}  W: {}",
                                    format_rate(data.disk_read_rate),
                                    format_rate(data.disk_write_rate)
                                ))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                            );
                        });
                    });

                    ui.add_space(4.0);
                    let read_points: PlotPoints = data.disk_read_history.iter().map(|p| [p.time, p.value]).collect();
                    let write_points: PlotPoints = data.disk_write_history.iter().map(|p| [p.time, p.value]).collect();

                    let line_r = Line::new(read_points)
                        .name("Read MB/s")
                        .color(ThemePalette::STATUS_HEALTHY)
                        .width(1.5);
                    let line_w = Line::new(write_points)
                        .name("Write MB/s")
                        .color(ThemePalette::STATUS_WARNING)
                        .width(1.5);

                    Plot::new("disk_plot")
                        .height(180.0)
                        .allow_zoom(false)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .legend(egui_plot::Legend::default())
                        .y_axis_label("MB/s")
                        .show(ui, |plot_ui| {
                            plot_ui.line(line_r);
                            plot_ui.line(line_w);
                        });
                });
            });

            ui.add_space(10.0);

            ui.columns(2, |cols| {
                // Network Graph
                card_frame(is_dark).show(&mut cols[0], |ui| {
                    let total_rx_rate: f64 = data.network_info.iter().map(|n| n.received_rate).sum();
                    let total_tx_rate: f64 = data.network_info.iter().map(|n| n.transmitted_rate).sum();

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("NETWORK TRAFFIC HISTORY")
                                .size(11.0)
                                .strong()
                                .color(ThemePalette::ACCENT_PRIMARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "↓ {}  ↑ {}",
                                    format_rate(total_rx_rate),
                                    format_rate(total_tx_rate)
                                ))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                            );
                        });
                    });

                    ui.add_space(4.0);
                    let down_points: PlotPoints = data
                        .network_download_history
                        .iter()
                        .map(|p| [p.time, p.value])
                        .collect();
                    let up_points: PlotPoints = data.network_upload_history.iter().map(|p| [p.time, p.value]).collect();

                    let line_down = Line::new(down_points)
                        .name("Download MB/s")
                        .color(ThemePalette::ACCENT_PRIMARY)
                        .width(1.5);
                    let line_up = Line::new(up_points)
                        .name("Upload MB/s")
                        .color(ThemePalette::STATUS_WARNING)
                        .width(1.5);

                    Plot::new("network_plot")
                        .height(180.0)
                        .allow_zoom(false)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .legend(egui_plot::Legend::default())
                        .y_axis_label("MB/s")
                        .show(ui, |plot_ui| {
                            plot_ui.line(line_down);
                            plot_ui.line(line_up);
                        });
                });

                // CPU Temp Graph
                card_frame(is_dark).show(&mut cols[1], |ui| {
                    let temp_str = data
                        .cpu_temperature
                        .map(|t| format!("{:.1} °C", t))
                        .unwrap_or_else(|| "N/A".to_string());

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("CPU TEMPERATURE HISTORY")
                                .size(11.0)
                                .strong()
                                .color(ThemePalette::STATUS_CRITICAL),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(temp_str)
                                    .monospace()
                                    .strong()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                        });
                    });

                    ui.add_space(4.0);
                    let temp_points: PlotPoints = data.cpu_temp_history.iter().map(|p| [p.time, p.value]).collect();
                    let line_temp = Line::new(temp_points)
                        .name("Temperature °C")
                        .color(ThemePalette::STATUS_CRITICAL)
                        .width(1.5);

                    Plot::new("cpu_temp_plot")
                        .height(180.0)
                        .allow_zoom(false)
                        .allow_drag(false)
                        .allow_scroll(false)
                        .include_y(0.0)
                        .y_axis_label("°C")
                        .show(ui, |plot_ui| {
                            plot_ui.line(line_temp);
                        });
                });
            });
        } else {
            card_frame(is_dark).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Performance graphs are currently disabled. Enable them in Settings.")
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        }
    });
}

fn show_history_summary(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("TELEMETRY WINDOW STATISTICAL SUMMARY")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(
                "Rolling window statistics (Min / Avg / Max) computed in-memory across sample intervals.",
            )
            .size(11.5)
            .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        egui::Grid::new("telemetry_window_summary")
            .striped(false)
            .min_col_width(110.0)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("METRIC")
                        .strong()
                        .size(11.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                for label in ["60 SECONDS", "5 MINUTES", "30 MINUTES", "1 HOUR"] {
                    ui.label(
                        egui::RichText::new(label)
                            .strong()
                            .size(11.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                }
                ui.end_row();

                show_metric_stats(ui, data, "CPU Usage", "cpu.global_usage", "%", 1.0, is_dark);
                show_metric_stats(
                    ui,
                    data,
                    "Memory Used",
                    "memory.used",
                    " GiB",
                    1.0 / 1_073_741_824.0,
                    is_dark,
                );

                let gpu_key = data
                    .telemetry_history_stats
                    .keys()
                    .find(|key| key.starts_with("gpu.") && key.ends_with(".utilization"));
                if let Some(key) = gpu_key {
                    show_metric_stats(ui, data, "GPU Util", key, "%", 1.0, is_dark);
                }
            });
    });
}

fn show_metric_stats(
    ui: &mut egui::Ui,
    data: &SystemData,
    label: &str,
    key: &str,
    unit: &str,
    scale: f64,
    is_dark: bool,
) {
    let Some(history) = data.telemetry_history_stats.get(key) else {
        return;
    };
    ui.label(
        egui::RichText::new(label)
            .strong()
            .size(12.0)
            .color(ThemePalette::text_primary(is_dark)),
    );
    for stats in [
        &history.sixty_seconds,
        &history.five_minutes,
        &history.thirty_minutes,
        &history.one_hour,
    ] {
        ui.label(
            egui::RichText::new(format!(
                "min {:.1} · avg {:.1} · max {:.1}{unit}",
                stats.min * scale,
                stats.avg * scale,
                stats.max * scale
            ))
            .monospace()
            .size(11.5)
            .color(ThemePalette::text_primary(is_dark)),
        );
    }
    ui.end_row();
}
