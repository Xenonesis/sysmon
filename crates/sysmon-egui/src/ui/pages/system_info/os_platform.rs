use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Renders the Operating System & Platform specifications card.
pub(crate) fn paint_os_platform_card(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("OPERATING SYSTEM & PLATFORM")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        egui::Grid::new("sysinfo_os_grid")
            .num_columns(4)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("OS Name:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(&data.system_info.os_name)
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );

                ui.label(
                    egui::RichText::new("OS Version:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(&data.system_info.os_version)
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Kernel Version:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(&data.system_info.kernel_version)
                        .monospace()
                        .color(ThemePalette::text_primary(is_dark)),
                );

                ui.label(
                    egui::RichText::new("Hostname:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(&data.system_info.hostname)
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.end_row();

                let days = data.system_info.uptime / 86400;
                let hours = (data.system_info.uptime % 86400) / 3600;
                let minutes = (data.system_info.uptime % 3600) / 60;

                ui.label(
                    egui::RichText::new("System Uptime:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format!("{}d {}h {}m", days, hours, minutes))
                        .monospace()
                        .strong()
                        .color(ThemePalette::STATUS_HEALTHY),
                );

                if let Some(build) = &data.system_info.os_build {
                    ui.label(
                        egui::RichText::new("OS Build:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(build)
                            .monospace()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                } else {
                    ui.label("");
                    ui.label("");
                }
                ui.end_row();

                if let Some(mb) = &data.system_info.motherboard {
                    ui.label(
                        egui::RichText::new("Motherboard:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(mb)
                            .monospace()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                }
                if let Some(bios) = &data.system_info.bios_version {
                    ui.label(
                        egui::RichText::new("BIOS Version:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(bios)
                            .monospace()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.end_row();
                } else if data.system_info.motherboard.is_some() {
                    ui.end_row();
                }
            });
    });
}
