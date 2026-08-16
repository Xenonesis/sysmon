use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(super) fn paint_health_deck(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    data: &SystemData,
    is_dark: bool,
) {
    ui.columns(2, |cols| {
        // Column 1: Storage Volumes Quick View
        card_frame(is_dark).show(&mut cols[0], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("STORAGE VOLUMES")
                        .size(11.0)
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Storage Hub →").size(10.5)).clicked() {
                        app.selected_tab = Tab::Storage;
                    }
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            if data.disk_info.is_empty() {
                ui.label(
                    egui::RichText::new("No storage devices detected")
                        .size(11.0)
                        .monospace()
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
            } else {
                for disk in data.disk_info.iter().take(3) {
                    let frac = (disk.usage_percentage / 100.0).clamp(0.0, 1.0);
                    let dc = get_usage_color(disk.usage_percentage);
                    let used_gb = bytes_to_gb(disk.total_space.saturating_sub(disk.available_space));
                    let total_gb = bytes_to_gb(disk.total_space);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&disk.name)
                                .size(11.5)
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        if !disk.file_system.is_empty() {
                            ui.label(
                                egui::RichText::new(&disk.file_system)
                                    .size(10.0)
                                    .monospace()
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.0}%", disk.usage_percentage))
                                    .size(11.0)
                                    .monospace()
                                    .strong()
                                    .color(dc),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.0}/{:.0} GB", used_gb, total_gb))
                                    .size(10.5)
                                    .monospace()
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                        });
                    });

                    paint_progress_bar(ui, frac, dc, 4.0, is_dark);
                    ui.add_space(3.0);
                }
            }
        });

        // Column 2: Startup & System Health Overview
        card_frame(is_dark).show(&mut cols[1], |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("BOOT & SYSTEM HEALTH")
                        .size(11.0)
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Startup Apps →").size(10.5)).clicked() {
                        app.selected_tab = Tab::StartupManager;
                    }
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            let high = startup::high_impact_count(&app.startup_items);
            let total = app.startup_items.len();

            ui.horizontal(|ui| {
                if let Some(boot_ms) = app.boot_diagnostics.as_ref().and_then(|bd| bd.boot_duration_ms) {
                    let sec = boot_ms as f64 / 1000.0;
                    let c = if boot_ms < 30000 {
                        ThemePalette::STATUS_HEALTHY
                    } else if boot_ms < 60000 {
                        ThemePalette::STATUS_WARNING
                    } else {
                        ThemePalette::STATUS_CRITICAL
                    };
                    ui.label(
                        egui::RichText::new(format!("Boot: {:.1}s", sec))
                            .monospace()
                            .strong()
                            .size(11.5)
                            .color(c),
                    );
                    ui.separator();
                }

                if high > 0 {
                    status_pill(
                        ui,
                        &format!("{} High Impact", high),
                        ThemePalette::STATUS_WARNING,
                        is_dark,
                    );
                } else {
                    status_pill(ui, "0 High Impact", ThemePalette::STATUS_HEALTHY, is_dark);
                }

                ui.label(
                    egui::RichText::new(format!("{} Items", total))
                        .monospace()
                        .size(10.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if !data.alerts.is_empty() {
                    status_pill(
                        ui,
                        &format!("⚠ {} Active Alerts", data.alerts.len()),
                        ThemePalette::STATUS_WARNING,
                        is_dark,
                    );
                } else {
                    status_pill(ui, "✓ No Alerts Active", ThemePalette::STATUS_HEALTHY, is_dark);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Diagnostics →").size(10.5)).clicked() {
                        app.selected_tab = Tab::Diagnostics;
                    }
                });
            });
        });
    });
}
