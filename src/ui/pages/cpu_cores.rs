use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn show(_app: &crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "CPU Topology & Core Telemetry", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        let total_cores = data.cpu_cores.len();
        let (avg_usage, max_usage, min_usage, high_cores) = if total_cores > 0 {
            let avg = data.cpu_cores.iter().map(|c| c.usage).sum::<f32>() / total_cores as f32;
            let max = data.cpu_cores.iter().map(|c| c.usage).fold(0.0f32, f32::max);
            let min = data.cpu_cores.iter().map(|c| c.usage).fold(100.0f32, f32::min);
            let high = data.cpu_cores.iter().filter(|c| c.usage > 50.0).count();
            (avg, max, min, high)
        } else {
            (0.0, 0.0, 0.0, 0)
        };

        // ── 1. Topology & Statistical Summary Card ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("TOPOLOGY & CORE TELEMETRY SUMMARY")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} logical processors  ·  {} physical cores",
                            total_cores, data.system_info.cpu_count
                        ))
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::text_dimmed(is_dark)),
                    );
                });
            });

            ui.add_space(8.0);

            egui::Grid::new("cpu_cores_summary_grid")
                .num_columns(4)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Average Core Load:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.1}%", avg_usage))
                            .monospace()
                            .strong()
                            .color(get_usage_color(avg_usage)),
                    );

                    ui.label(
                        egui::RichText::new("Peak Core Load:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.1}%", max_usage))
                            .monospace()
                            .strong()
                            .color(get_usage_color(max_usage)),
                    );
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Minimum Core Load:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.1}%", min_usage))
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.label(
                        egui::RichText::new("Cores Under Load (>50%):")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{} / {}", high_cores, total_cores))
                            .monospace()
                            .strong()
                            .color(if high_cores > 0 {
                                ThemePalette::STATUS_WARNING
                            } else {
                                ThemePalette::STATUS_HEALTHY
                            }),
                    );
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // ── 2. Per-Core Topology Grid ──
        ui.label(
            egui::RichText::new("LOGICAL CORE LOAD METERS")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(6.0);

        let cores_per_row = 4;
        let mut core_index = 0;

        while core_index < data.cpu_cores.len() {
            ui.columns(cores_per_row, |cols| {
                for col in cols.iter_mut() {
                    if core_index >= data.cpu_cores.len() {
                        break;
                    }

                    let core = &data.cpu_cores[core_index];
                    let color = get_usage_color(core.usage);

                    card_frame(is_dark)
                        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                        .show(col, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Core {:02}", core.core_id))
                                        .monospace()
                                        .strong()
                                        .size(12.0)
                                        .color(ThemePalette::text_primary(is_dark)),
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{:.1}%", core.usage))
                                            .monospace()
                                            .strong()
                                            .size(12.0)
                                            .color(color),
                                    );
                                });
                            });

                            ui.add_space(4.0);
                            paint_progress_bar(ui, core.usage / 100.0, color, 6.0, is_dark);
                        });

                    core_index += 1;
                }
            });
            ui.add_space(6.0);
        }

        if data.cpu_cores.is_empty() {
            card_frame(is_dark).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("No CPU core telemetry available.")
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        }
    });
}
