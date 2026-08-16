use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Renders the Graphics Processing Unit (GPU) and Display Adapters card(s).
pub(crate) fn paint_gpu_display_card(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    if data.gpu_info.is_empty() {
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("GRAPHICS PROCESSING UNIT (GPU)")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("No dedicated GPU detected via NVML or generic graphics adapter.")
                    .color(ThemePalette::text_dimmed(is_dark)),
            );
        });
    } else {
        for (idx, gpu_info) in data.gpu_info.iter().enumerate() {
            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("GPU #{}: {}", idx, gpu_info.name))
                            .size(13.0)
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let gpu_color = get_usage_color(gpu_info.utilization);
                        ui.label(
                            egui::RichText::new(format!("{:.1}%", gpu_info.utilization))
                                .monospace()
                                .strong()
                                .color(gpu_color),
                        );
                        status_pill(
                            ui,
                            if gpu_info.utilization >= 90.0 {
                                "HIGH LOAD"
                            } else {
                                "ONLINE"
                            },
                            gpu_color,
                            is_dark,
                        );
                    });
                });

                ui.add_space(8.0);

                egui::Grid::new(format!("sysinfo_gpu_grid_{}", idx))
                    .num_columns(4)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        if let (Some(used), Some(total)) = (gpu_info.memory_used, gpu_info.memory_total) {
                            ui.label(
                                egui::RichText::new("VRAM Usage:")
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            let used_mb = bytes_to_mb(used);
                            let total_mb = bytes_to_mb(total);
                            let vram_str = if total_mb >= 1024.0 {
                                format!("{:.2} / {:.2} GB", used_mb / 1024.0, total_mb / 1024.0)
                            } else {
                                format!("{:.0} / {:.0} MB", used_mb, total_mb)
                            };
                            ui.label(
                                egui::RichText::new(vram_str)
                                    .monospace()
                                    .strong()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                        }

                        if let Some(temp) = gpu_info.temperature {
                            ui.label(
                                egui::RichText::new("GPU Temperature:")
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            let temp_color = if temp < 70 {
                                ThemePalette::STATUS_HEALTHY
                            } else if temp < 85 {
                                ThemePalette::STATUS_WARNING
                            } else {
                                ThemePalette::STATUS_CRITICAL
                            };
                            ui.label(
                                egui::RichText::new(format!("{} °C", temp))
                                    .monospace()
                                    .strong()
                                    .color(temp_color),
                            );
                        }
                        ui.end_row();

                        if let Some(drv) = &data.system_info.gpu_driver {
                            ui.label(
                                egui::RichText::new("Driver Version:")
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new(drv)
                                    .monospace()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                        }

                        if let Some(clock) = gpu_info.clock_mhz {
                            ui.label(
                                egui::RichText::new("Clock Speed:")
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new(format!("{} MHz", clock))
                                    .monospace()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                        }
                        ui.end_row();
                    });

                ui.add_space(6.0);
                paint_progress_bar(
                    ui,
                    gpu_info.utilization / 100.0,
                    get_usage_color(gpu_info.utilization),
                    6.0,
                    is_dark,
                );
            });
            ui.add_space(8.0);
        }
    }
}
