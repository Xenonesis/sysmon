use crate::ui::components::*;
use crate::*;
use eframe::egui;

pub(crate) fn show(_app: &crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    paint_section_header(ui, "CPU Cores Monitoring");

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(format!(
            "Total Cores: {} ({} logical processors)",
            data.system_info.cpu_count,
            data.cpu_cores.len()
        ));
        ui.add_space(10.0);

        // Grid layout for cores
        let cores_per_row = 4;
        let mut core_index = 0;

        while core_index < data.cpu_cores.len() {
            ui.horizontal(|ui| {
                for _ in 0..cores_per_row {
                    if core_index >= data.cpu_cores.len() {
                        break;
                    }

                    let core = &data.cpu_cores[core_index];
                    ui.group(|ui| {
                        ui.set_min_width(180.0);

                        ui.horizontal(|ui| {
                            ui.strong(format!("Core {}", core.core_id));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let color = get_usage_color(core.usage);
                                ui.colored_label(color, format!("{:.1}%", core.usage));
                            });
                        });

                        let color = get_usage_color(core.usage);
                        paint_progress_bar(ui, core.usage / 100.0, color, 5.0);
                    });

                    core_index += 1;
                }
            });
            ui.add_space(5.0);
        }

        ui.add_space(10.0);

        // Summary statistics
        ui.group(|ui| {
            ui.heading("Core Statistics");
            ui.separator();

            let avg_usage: f32 = data.cpu_cores.iter().map(|c| c.usage).sum::<f32>() / data.cpu_cores.len() as f32;
            let max_usage = data.cpu_cores.iter().map(|c| c.usage).fold(0.0f32, f32::max);
            let min_usage = data.cpu_cores.iter().map(|c| c.usage).fold(100.0f32, f32::min);

            ui.horizontal(|ui| {
                ui.label("Average Usage:");
                let color = get_usage_color(avg_usage);
                ui.colored_label(color, format!("{:.1}%", avg_usage));
            });

            ui.horizontal(|ui| {
                ui.label("Maximum Core:");
                let color = get_usage_color(max_usage);
                ui.colored_label(color, format!("{:.1}%", max_usage));
            });

            ui.horizontal(|ui| {
                ui.label("Minimum Core:");
                ui.label(format!("{:.1}%", min_usage));
            });

            ui.horizontal(|ui| {
                ui.label("Cores Above 50%:");
                let high_cores = data.cpu_cores.iter().filter(|c| c.usage > 50.0).count();
                ui.label(format!("{} / {}", high_cores, data.cpu_cores.len()));
            });
        });
    });
}
