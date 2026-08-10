use crate::*;
use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "Performance Graphs");

        egui::ScrollArea::vertical().show(ui, |ui| {
            if app.settings.show_graphs {
                ui.columns(2, |cols| {
                    // CPU Graph
                    cols[0].group(|ui| {
                        ui.label(
                            egui::RichText::new("CPU Usage History")
                                .size(15.0)
                                .strong()
                                .color(ThemePalette::ACCENT_PRIMARY),
                        );
                        let cpu_points: PlotPoints = data.cpu_history.iter().map(|p| [p.time, p.value]).collect();

                        let line = Line::new(cpu_points).color(ThemePalette::ACCENT_PRIMARY);

                        Plot::new("cpu_plot")
                            .height(200.0)
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
                    cols[1].group(|ui| {
                        ui.label(
                            egui::RichText::new("Memory Usage History")
                                .size(15.0)
                                .strong()
                                .color(ThemePalette::STATUS_HEALTHY),
                        );
                        let mem_points: PlotPoints = data.memory_history.iter().map(|p| [p.time, p.value]).collect();

                        let line = Line::new(mem_points).color(ThemePalette::STATUS_HEALTHY);

                        Plot::new("memory_plot")
                            .height(200.0)
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
                        cols[0].group(|ui| {
                            ui.label(
                                egui::RichText::new("GPU Usage History")
                                    .size(15.0)
                                    .strong()
                                    .color(ThemePalette::STATUS_WARNING),
                            );
                            let gpu_points: PlotPoints = data.gpu_history.iter().map(|p| [p.time, p.value]).collect();

                            let line = Line::new(gpu_points).color(ThemePalette::STATUS_WARNING);

                            Plot::new("gpu_plot")
                                .height(200.0)
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
                    cols[1].group(|ui| {
                        ui.label(
                            egui::RichText::new("Disk I/O History")
                                .size(15.0)
                                .strong()
                                .color(ThemePalette::TEXT_LABEL_SUB),
                        );
                        let read_points: PlotPoints = data.disk_read_history.iter().map(|p| [p.time, p.value]).collect();
                        let write_points: PlotPoints = data.disk_write_history.iter().map(|p| [p.time, p.value]).collect();

                        let line_r = Line::new(read_points).name("Read MB/s").color(ThemePalette::STATUS_HEALTHY);
                        let line_w = Line::new(write_points).name("Write MB/s").color(ThemePalette::STATUS_WARNING);

                        Plot::new("disk_plot")
                            .height(200.0)
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
                    cols[0].group(|ui| {
                        ui.label(
                            egui::RichText::new("Network Traffic History")
                                .size(15.0)
                                .strong()
                                .color(ThemePalette::ACCENT_CYAN),
                        );
                        let down_points: PlotPoints = data.network_download_history.iter().map(|p| [p.time, p.value]).collect();
                        let up_points: PlotPoints = data.network_upload_history.iter().map(|p| [p.time, p.value]).collect();

                        let line_down = Line::new(down_points).name("Download MB/s").color(ThemePalette::ACCENT_CYAN);
                        let line_up = Line::new(up_points).name("Upload MB/s").color(ThemePalette::STATUS_WARNING);

                        Plot::new("network_plot")
                            .height(200.0)
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
                    cols[1].group(|ui| {
                        ui.label(
                            egui::RichText::new("CPU Temperature History")
                                .size(15.0)
                                .strong()
                                .color(ThemePalette::STATUS_CRITICAL),
                        );
                        let temp_points: PlotPoints = data.cpu_temp_history.iter().map(|p| [p.time, p.value]).collect();

                        let line_temp = Line::new(temp_points).name("Temperature °C").color(ThemePalette::STATUS_CRITICAL);

                        Plot::new("cpu_temp_plot")
                            .height(200.0)
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
                ui.label("Performance graphs are disabled. Enable them in View menu.");
            }
        });
    }
