use crate::*;
use crate::ui::components::*;
use eframe::egui;

pub(crate) fn show(_app: &crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "Storage Devices");

        egui::ScrollArea::vertical().show(ui, |ui| {
            for disk in &data.disk_info {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(&disk.name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let color = get_usage_color(disk.usage_percentage);
                            ui.colored_label(color, format!("{:.1}%", disk.usage_percentage));

                            // Warning icon for high usage
                            if disk.usage_percentage > 90.0 {
                                ui.colored_label(egui::Color32::RED, "[!]");
                            }
                        });
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Mount Point:");
                        ui.strong(&disk.mount_point);
                    });

                    ui.horizontal(|ui| {
                        ui.label("File System:");
                        ui.strong(&disk.file_system);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Total Space:");
                        ui.strong(format!("{:.2} GB", bytes_to_gb(disk.total_space)));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Available:");
                        ui.strong(format!("{:.2} GB", bytes_to_gb(disk.available_space)));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Used:");
                        let used = disk.total_space.saturating_sub(disk.available_space);
                        ui.strong(format!("{:.2} GB", bytes_to_gb(used)));
                    });

                    let color = get_usage_color(disk.usage_percentage);
                    paint_progress_bar(ui, disk.usage_percentage / 100.0, color, 6.0);

                    // Show warning for low disk space
                    if disk.usage_percentage > 90.0 {
                        ui.add_space(5.0);
                        ui.colored_label(
                            egui::Color32::RED,
                            format!(
                                "Warning: Only {:.2} GB remaining!",
                                bytes_to_gb(disk.available_space)
                            ),
                        );
                    }
                });

                ui.add_space(10.0);
            }

            if data.disk_info.is_empty() {
                ui.label("No storage devices detected.");
            }
        });
    }
