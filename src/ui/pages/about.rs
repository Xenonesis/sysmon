use crate::*;
use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &crate::SystemMonitorApp, ui: &mut egui::Ui, _data: &SystemData) {
        paint_section_header(ui, "About");

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);

            // Hero brand
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Image::new(egui::include_image!("../../../assets/icon.png"))
                            .max_width(40.0)
                            .max_height(40.0),
                    );
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("System Monitor")
                                .size(22.0)
                                .strong()
                                .color(ThemePalette::TEXT_PRIMARY),
                        );
                        ui.label(
                            egui::RichText::new(format!("v{} · Terminal Noir", APP_VERSION))
                                .size(12.0)
                                .color(ThemePalette::TEXT_TERTIARY),
                        );
                    });
                });
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Professional system intelligence for Windows — built with Rust and egui.")
                        .size(13.0)
                        .color(ThemePalette::TEXT_SUBTITLE),
                );
            });

            ui.add_space(10.0);

            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.label(
                        egui::RichText::new("FEATURES")
                            .size(10.0)
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.add_space(6.0);
                    for item in &[
                        "Real-time CPU, Memory & GPU",
                        "Historical performance graphs",
                        "Process monitoring & management",
                        "Color-coded usage indicators",
                        "Per-core CPU breakdown",
                        "Smart alerts system",
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("›").color(ThemePalette::ACCENT_PRIMARY));
                            ui.label(egui::RichText::new(*item).size(12.5).color(ThemePalette::TEXT_FEATURE));
                        });
                    }
                });

                cols[1].group(|ui| {
                    ui.label(
                        egui::RichText::new("TECHNICAL")
                            .size(10.0)
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.add_space(6.0);
                    let refresh_str = format!("{} s interval", app.settings.refresh_interval);
                    let specs: Vec<(&str, &str)> = vec![
                        ("Framework", "egui + eframe"),
                        ("System", "sysinfo crate"),
                        ("GPU", "NVML (NVIDIA)"),
                        ("Refresh", &refresh_str),
                        ("History", "60 data points"),
                        ("License", "MIT — open source"),
                    ];
                    for (k, v) in &specs {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(*k).size(11.5).color(ThemePalette::TEXT_TERTIARY));
                            ui.label(
                                egui::RichText::new(*v)
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(185, 195, 215)),
                            );
                        });
                    }
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("COLOR LEGEND")
                        .size(10.0)
                        .color(ThemePalette::ACCENT_PRIMARY),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.colored_label(ThemePalette::STATUS_HEALTHY, "●  Healthy < 50%");
                    ui.add_space(16.0);
                    ui.colored_label(ThemePalette::STATUS_WARNING, "●  Moderate 50-75%");
                    ui.add_space(16.0);
                    ui.colored_label(ThemePalette::STATUS_CRITICAL, "●  Critical > 75%");
                });
            });
        });
    }
