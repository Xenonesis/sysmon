use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(super) fn paint_active_incidents_feed(
    ui: &mut egui::Ui,
    data: &SystemData,
    is_dark: bool,
    remove_alert_idx: &mut Option<usize>,
    navigate_tab: &mut Option<Tab>,
    run_ram_clean: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("ACTIVE INCIDENTS FEED")
                .size(11.5)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.label(
            egui::RichText::new(format!("({} active)", data.alerts.len()))
                .monospace()
                .size(11.0)
                .color(ThemePalette::STATUS_WARNING),
        );
    });

    ui.add_space(6.0);

    for (i, alert) in data.alerts.iter().enumerate() {
        card_frame(is_dark).show(ui, |ui| {
            let (cat_label, color, severity_label) = match alert.alert_type {
                AlertType::CpuHigh => ("CPU", ThemePalette::STATUS_WARNING, "WARNING"),
                AlertType::MemoryHigh => ("RAM", ThemePalette::STATUS_WARNING, "WARNING"),
                AlertType::GpuTempHigh => ("GPU", ThemePalette::STATUS_CRITICAL, "CRITICAL"),
                AlertType::DiskSpaceLow => ("DISK", ThemePalette::STATUS_CRITICAL, "CRITICAL"),
                AlertType::StartupHighImpact => ("STARTUP", ThemePalette::ACCENT_PRIMARY, "INFO"),
            };

            // Header row: Status badges and Peak value on left, Dismiss button on right
            ui.horizontal(|ui| {
                status_pill(ui, severity_label, color, is_dark);
                status_pill(ui, cat_label, ThemePalette::text_secondary(is_dark), is_dark);
                ui.label(
                    egui::RichText::new(format!("Peak: {:.1}", alert.value))
                        .monospace()
                        .strong()
                        .size(11.5)
                        .color(color),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("× Dismiss")
                        .on_hover_text("Dismiss this incident")
                        .clicked()
                    {
                        *remove_alert_idx = Some(i);
                    }
                });
            });

            ui.add_space(4.0);

            // Body: Full-width wrapping incident message (immune to button collision)
            ui.label(
                egui::RichText::new(&alert.message)
                    .strong()
                    .size(12.0)
                    .color(ThemePalette::text_primary(is_dark)),
            );

            ui.add_space(4.0);

            // Sub-row: Timestamp on left & Contextual Remediation Actions on right
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Triggered: {}", alert.timestamp))
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::text_dimmed(is_dark)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    match alert.alert_type {
                        AlertType::MemoryHigh => {
                            if ui
                                .button(
                                    egui::RichText::new("🧹 Clean RAM Now")
                                        .strong()
                                        .size(11.0)
                                        .color(ThemePalette::ACCENT_PRIMARY),
                                )
                                .on_hover_text("Execute working set optimization to free up RAM")
                                .clicked()
                            {
                                *run_ram_clean = true;
                            }
                        }
                        AlertType::CpuHigh => {
                            if ui
                                .button(
                                    egui::RichText::new("📋 Inspect Processes")
                                        .strong()
                                        .size(11.0)
                                        .color(ThemePalette::ACCENT_PRIMARY),
                                )
                                .on_hover_text("Open Process Monitor to inspect high CPU consumers")
                                .clicked()
                            {
                                *navigate_tab = Some(Tab::Processes);
                            }
                        }
                        AlertType::DiskSpaceLow => {
                            if ui
                                .button(
                                    egui::RichText::new("💾 Open Storage Manager")
                                        .strong()
                                        .size(11.0)
                                        .color(ThemePalette::ACCENT_PRIMARY),
                                )
                                .on_hover_text("Inspect disk usage and partition breakdown")
                                .clicked()
                            {
                                *navigate_tab = Some(Tab::Storage);
                            }
                        }
                        AlertType::GpuTempHigh => {
                            if ui
                                .button(
                                    egui::RichText::new("🩺 GPU Diagnostics")
                                        .strong()
                                        .size(11.0)
                                        .color(ThemePalette::ACCENT_PRIMARY),
                                )
                                .on_hover_text("Inspect GPU clock rates, fan speeds, and memory usage")
                                .clicked()
                            {
                                *navigate_tab = Some(Tab::Performance);
                            }
                        }
                        AlertType::StartupHighImpact => {
                            if ui
                                .button(
                                    egui::RichText::new("🚀 Manage Startup Apps")
                                        .strong()
                                        .size(11.0)
                                        .color(ThemePalette::ACCENT_PRIMARY),
                                )
                                .on_hover_text("Open Startup Manager to disable heavy startup programs")
                                .clicked()
                            {
                                *navigate_tab = Some(Tab::StartupManager);
                            }
                        }
                    }
                });
            });
        });

        if i < data.alerts.len() - 1 {
            ui.add_space(4.0);
        }
    }
}
