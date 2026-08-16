use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(super) fn paint_top_processes_table(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    data: &SystemData,
    is_dark: bool,
) {
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

        ui.add_space(4.0);
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
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
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

                        // CPU % with status color
                        let cc = get_usage_color(process.cpu_usage);
                        ui.label(
                            egui::RichText::new(format!("{:>5.1}%", process.cpu_usage))
                                .size(11.0)
                                .monospace()
                                .strong()
                                .color(cc),
                        );
                        ui.end_row();
                    }
                });
        }
    });
}
