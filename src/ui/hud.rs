//! Desktop floating mini-widget telemetry HUD.

use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::{SystemData, SystemMonitorApp};
use eframe::egui;

pub(crate) fn render_hud(app: &mut SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    ui.spacing_mut().item_spacing = egui::vec2(6.0, 5.0);
    ui.set_width(240.0);

    // Header
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("◰ HUD TELEMETRY")
                .size(11.0)
                .monospace()
                .strong()
                .color(ThemePalette::ACCENT_PRIMARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("✕").on_hover_text("Close HUD (Ctrl+M)").clicked() {
                app.widget_open = false;
                app.settings.show_widget = false;
                let _ = app.settings.save();
                {
                    let mut shared = app.shared_settings.lock();
                    *shared = app.settings.clone();
                }
            }
        });
    });

    ui.add_space(2.0);

    // CPU Metric
    let cpu_color = get_usage_color(data.cpu_usage);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("CPU")
                .size(11.0)
                .monospace()
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(temp) = data.cpu_temperature {
                ui.label(
                    egui::RichText::new(format!("{temp:.0}°C"))
                        .size(10.5)
                        .monospace()
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
                ui.add_space(4.0);
            }
            ui.label(
                egui::RichText::new(format!("{:.1}%", data.cpu_usage))
                    .size(11.5)
                    .monospace()
                    .strong()
                    .color(cpu_color),
            );
        });
    });
    paint_progress_bar(ui, data.cpu_usage / 100.0, cpu_color, 4.0, is_dark);

    // RAM Metric
    let mem_color = get_usage_color(data.memory_percentage);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("RAM")
                .size(11.0)
                .monospace()
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let used_gb = data.memory_used as f64 / 1024.0 / 1024.0 / 1024.0;
            let total_gb = data.memory_total as f64 / 1024.0 / 1024.0 / 1024.0;
            ui.label(
                egui::RichText::new(format!("{used_gb:.1}/{total_gb:.1}G"))
                    .size(10.5)
                    .monospace()
                    .color(ThemePalette::text_dimmed(is_dark)),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("{:.1}%", data.memory_percentage))
                    .size(11.5)
                    .monospace()
                    .strong()
                    .color(mem_color),
            );
        });
    });
    paint_progress_bar(ui, data.memory_percentage / 100.0, mem_color, 4.0, is_dark);

    // GPU Metric (if available)
    if let Some(gpu) = data.gpu_info.first() {
        let gpu_color = get_usage_color(gpu.utilization);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("GPU")
                    .size(11.0)
                    .monospace()
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(temp) = gpu.temperature {
                    ui.label(
                        egui::RichText::new(format!("{temp:.0}°C"))
                            .size(10.5)
                            .monospace()
                            .color(ThemePalette::text_dimmed(is_dark)),
                    );
                    ui.add_space(4.0);
                }
                ui.label(
                    egui::RichText::new(format!("{:.1}%", gpu.utilization))
                        .size(11.5)
                        .monospace()
                        .strong()
                        .color(gpu_color),
                );
            });
        });
        paint_progress_bar(ui, gpu.utilization / 100.0, gpu_color, 4.0, is_dark);
    }

    // Network I/O
    let dl: f64 = data.network_info.iter().map(|n| n.received_rate).sum();
    let ul: f64 = data.network_info.iter().map(|n| n.transmitted_rate).sum();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("↓ {:.0} KB/s", dl))
                .size(10.5)
                .monospace()
                .color(ThemePalette::ACCENT_PRIMARY),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!("↑ {:.0} KB/s", ul))
                .size(10.5)
                .monospace()
                .color(ThemePalette::ACCENT_ACTIVE),
        );
    });

    ui.separator();

    // Footer Actions
    ui.horizontal(|ui| {
        let is_cleaning = app.ram_cleaner_state.is_cleaning;
        let clean_label = if is_cleaning { "..." } else { "⚡ Clean RAM" };
        if ui
            .add_enabled(
                !is_cleaning,
                egui::Button::new(egui::RichText::new(clean_label).size(10.5)),
            )
            .clicked()
        {
            app.queue_action(crate::app::commands::ActionCommand::CleanRam);
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
}
