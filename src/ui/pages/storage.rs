use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn show(_app: &crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Storage Devices & Partitions", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Global Disk I/O Banner ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("DISK I/O TELEMETRY")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{} volume(s) detected", data.disk_info.len()))
                            .monospace()
                            .size(11.0)
                            .color(ThemePalette::text_dimmed(is_dark)),
                    );
                });
            });

            ui.add_space(8.0);
            ui.columns(2, |cols| {
                cols[0].horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Total Read Rate:")
                            .size(12.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format_rate(data.disk_read_rate))
                            .monospace()
                            .strong()
                            .size(13.0)
                            .color(ThemePalette::STATUS_HEALTHY),
                    );
                });

                cols[1].horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Total Write Rate:")
                            .size(12.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format_rate(data.disk_write_rate))
                            .monospace()
                            .strong()
                            .size(13.0)
                            .color(ThemePalette::STATUS_WARNING),
                    );
                });
            });
        });

        ui.add_space(10.0);

        // ── 2. Storage Volume Cards ──
        for disk in &data.disk_info {
            let color = get_usage_color(disk.usage_percentage);
            let used_bytes = disk.total_space.saturating_sub(disk.available_space);

            card_frame(is_dark).show(ui, |ui| {
                // Header: Volume Name + FS Pill + Usage Pill + Monospace Percentage
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&disk.name)
                            .strong()
                            .size(14.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    if !disk.file_system.is_empty() {
                        status_pill(ui, &disk.file_system, ThemePalette::ACCENT_PRIMARY, is_dark);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{:.1}%", disk.usage_percentage))
                                .monospace()
                                .strong()
                                .size(13.0)
                                .color(color),
                        );
                        if disk.usage_percentage >= 90.0 {
                            status_pill(ui, "CRITICAL", ThemePalette::STATUS_CRITICAL, is_dark);
                        } else if disk.usage_percentage >= 70.0 {
                            status_pill(ui, "ELEVATED", ThemePalette::STATUS_WARNING, is_dark);
                        } else {
                            status_pill(ui, "HEALTHY", ThemePalette::STATUS_HEALTHY, is_dark);
                        }
                    });
                });

                ui.add_space(8.0);
                paint_progress_bar(ui, disk.usage_percentage / 100.0, color, 8.0, is_dark);
                ui.add_space(10.0);

                // Monospace Metrics Grid
                egui::Grid::new(format!("disk_grid_{}", disk.mount_point))
                    .num_columns(4)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Mount Point:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(&disk.mount_point)
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );

                        ui.label(
                            egui::RichText::new("Used Space:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2} GB", bytes_to_gb(used_bytes)))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("File System:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(&disk.file_system)
                                .monospace()
                                .color(ThemePalette::text_primary(is_dark)),
                        );

                        ui.label(
                            egui::RichText::new("Available:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2} GB", bytes_to_gb(disk.available_space)))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Total Capacity:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2} GB", bytes_to_gb(disk.total_space)))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();
                    });

                // High usage warning
                if disk.usage_percentage >= 90.0 {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        status_pill(ui, "LOW STORAGE WARNING", ThemePalette::STATUS_CRITICAL, is_dark);
                        ui.label(
                            egui::RichText::new(format!(
                                "Only {:.2} GB remaining on this volume. Clean temporary files or expand capacity.",
                                bytes_to_gb(disk.available_space)
                            ))
                            .size(11.5)
                            .color(ThemePalette::STATUS_CRITICAL),
                        );
                    });
                }
            });

            ui.add_space(8.0);
        }

        if data.disk_info.is_empty() {
            card_frame(is_dark).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("No storage devices or mounted partitions detected.")
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        }
    });
}
