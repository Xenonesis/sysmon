use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(super) fn paint_hardware_banner(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
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
                        egui::RichText::new(format!("Uptime: {}", super::format_uptime(data.system_info.uptime)))
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
                        egui::RichText::new(format!("Uptime: {}", super::format_uptime(data.system_info.uptime)))
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
}
