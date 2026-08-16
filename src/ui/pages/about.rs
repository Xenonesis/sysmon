use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn show(app: &crate::SystemMonitorApp, ui: &mut egui::Ui, _data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "About System Monitor", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Hero Brand Card ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("../../../assets/icon.png"))
                        .max_width(42.0)
                        .max_height(42.0),
                );
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("SysMon")
                                .size(20.0)
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        status_pill(
                            ui,
                            &format!("v{} · COCKPIT DENSE", APP_VERSION),
                            ThemePalette::ACCENT_PRIMARY,
                            is_dark,
                        );
                    });
                    ui.label(
                        egui::RichText::new("Precision real-time telemetry and diagnostics console for Windows.")
                            .size(12.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                });
            });
        });

        ui.add_space(10.0);

        // ── 2. Features & Technical Specs ──
        ui.columns(2, |cols| {
            // Features Card
            card_frame(is_dark).show(&mut cols[0], |ui| {
                ui.label(
                    egui::RichText::new("CAPABILITIES & FEATURES")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.add_space(8.0);

                for item in &[
                    "Real-time CPU, Memory, GPU & Storage telemetry",
                    "Bounded in-memory historical statistical buffers",
                    "Per-core topology and thread frequency tracking",
                    "Background process manager & safe action plans",
                    "Windows services inspection & lifecycle control",
                    "Working-set RAM cleaner with automated policy",
                    "Startup applications manager & impact profiling",
                    "Local-first diagnostic session flight recorder",
                ] {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("›").strong().color(ThemePalette::ACCENT_PRIMARY));
                        ui.label(
                            egui::RichText::new(*item)
                                .size(12.0)
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                    });
                    ui.add_space(2.0);
                }
            });

            // Technical Architecture Card
            card_frame(is_dark).show(&mut cols[1], |ui| {
                ui.label(
                    egui::RichText::new("TECHNICAL ARCHITECTURE")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.add_space(8.0);

                let refresh_str = format!("{} s interval", app.settings.refresh_interval);
                let specs: Vec<(&str, &str)> = vec![
                    ("Core Language", "Rust 2021 Edition"),
                    ("UI Framework", "egui & eframe (immediate mode)"),
                    ("Telemetry Engine", "sysinfo + Win32 Native APIs"),
                    ("GPU Acceleration", "NVML (NVIDIA) + Generic DXGI"),
                    ("Refresh Cadence", &refresh_str),
                    ("Ring Buffer", "60 / 300 / 1800 / 3600 samples"),
                    ("Privilege Engine", "Win32 Token Elevation Checks"),
                    ("Open Source", "MIT License"),
                ];

                egui::Grid::new("about_technical_grid")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for (k, v) in &specs {
                            ui.label(
                                egui::RichText::new(*k)
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new(*v)
                                    .monospace()
                                    .size(11.5)
                                    .strong()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                            ui.end_row();
                        }
                    });
            });
        });

        ui.add_space(10.0);

        // ── 3. Diagnostic Threshold Legend ──
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("TELEMETRY THRESHOLD LEGEND")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Color indicators across the interface follow strict cockpit diagnostic ranges:")
                    .size(12.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                status_pill(ui, "OPTIMAL / HEALTHY (< 70%)", ThemePalette::STATUS_HEALTHY, is_dark);
                ui.add_space(8.0);
                status_pill(
                    ui,
                    "ELEVATED / WARNING (70% - 90%)",
                    ThemePalette::STATUS_WARNING,
                    is_dark,
                );
                ui.add_space(8.0);
                status_pill(
                    ui,
                    "CRITICAL SATURATION (> 90%)",
                    ThemePalette::STATUS_CRITICAL,
                    is_dark,
                );
            });
        });
    });
}
