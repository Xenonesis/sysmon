use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Renders the Processor (CPU) specifications and architecture card.
pub(crate) fn paint_cpu_arch_card(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("PROCESSOR (CPU)")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        egui::Grid::new("sysinfo_cpu_grid")
            .num_columns(4)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("CPU Model:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(&data.system_info.cpu_brand)
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );

                ui.label(
                    egui::RichText::new("Core Count:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} logical cores ({} physical)",
                        data.cpu_cores.len(),
                        data.system_info.cpu_count
                    ))
                    .monospace()
                    .strong()
                    .color(ThemePalette::text_primary(is_dark)),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Global Utilization:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                let cpu_color = get_usage_color(data.cpu_usage);
                ui.label(
                    egui::RichText::new(format!("{:.1}%", data.cpu_usage))
                        .monospace()
                        .strong()
                        .color(cpu_color),
                );

                ui.label(
                    egui::RichText::new("CPU Temperature:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                if let Some(temp) = data.cpu_temperature {
                    let temp_color = if temp < 70.0 {
                        ThemePalette::STATUS_HEALTHY
                    } else if temp < 85.0 {
                        ThemePalette::STATUS_WARNING
                    } else {
                        ThemePalette::STATUS_CRITICAL
                    };
                    ui.label(
                        egui::RichText::new(format!("{:.1} °C", temp))
                            .monospace()
                            .strong()
                            .color(temp_color),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("N/A")
                            .monospace()
                            .color(ThemePalette::text_dimmed(is_dark)),
                    );
                }
                ui.end_row();
            });

        ui.add_space(6.0);
        paint_progress_bar(
            ui,
            data.cpu_usage / 100.0,
            get_usage_color(data.cpu_usage),
            6.0,
            is_dark,
        );
    });
}
