use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Renders the System Memory (RAM) and Virtual Memory (Page File) specifications card.
pub(crate) fn paint_memory_specs_card(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("SYSTEM MEMORY & PAGE FILE")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        egui::Grid::new("sysinfo_mem_grid")
            .num_columns(4)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Total Physical RAM:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2} GB", bytes_to_gb(data.memory_total)))
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );

                ui.label(
                    egui::RichText::new("Used RAM:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format!("{:.2} GB", bytes_to_gb(data.memory_used)))
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Available RAM:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2} GB",
                        bytes_to_gb(data.memory_total.saturating_sub(data.memory_used))
                    ))
                    .monospace()
                    .strong()
                    .color(ThemePalette::STATUS_HEALTHY),
                );

                ui.label(
                    egui::RichText::new("RAM Utilization:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                let mem_color = get_usage_color(data.memory_percentage);
                ui.label(
                    egui::RichText::new(format!("{:.1}%", data.memory_percentage))
                        .monospace()
                        .strong()
                        .color(mem_color),
                );
                ui.end_row();
            });

        ui.add_space(6.0);
        paint_progress_bar(
            ui,
            data.memory_percentage / 100.0,
            get_usage_color(data.memory_percentage),
            6.0,
            is_dark,
        );

        if data.swap_info.total > 0 {
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            egui::Grid::new("sysinfo_swap_grid")
                .num_columns(4)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Total Page File:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.2} GB", bytes_to_gb(data.swap_info.total)))
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.label(
                        egui::RichText::new("Used Page File:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.2} GB", bytes_to_gb(data.swap_info.used)))
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Page File Load:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    let swap_color = get_usage_color(data.swap_info.percentage);
                    ui.label(
                        egui::RichText::new(format!("{:.1}%", data.swap_info.percentage))
                            .monospace()
                            .strong()
                            .color(swap_color),
                    );
                    ui.end_row();
                });

            ui.add_space(4.0);
            paint_progress_bar(
                ui,
                data.swap_info.percentage / 100.0,
                get_usage_color(data.swap_info.percentage),
                4.0,
                is_dark,
            );
        }
    });
}
