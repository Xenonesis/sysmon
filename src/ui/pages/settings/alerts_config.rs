use crate::persistence::settings::{ALERT_PERCENT, ALERT_TEMPERATURE};
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_alerts_settings(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    changed: &mut bool,
    is_dark: bool,
) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("ALERT THRESHOLDS & NOTIFICATIONS")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        egui::Grid::new("alert_thresholds_grid")
            .num_columns(2)
            .spacing([24.0, 10.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("CPU usage alert threshold:").color(ThemePalette::text_secondary(is_dark)),
                );
                *changed |= ui
                    .add(
                        egui::Slider::new(&mut app.settings.notification_cpu_threshold, ALERT_PERCENT)
                            .suffix(" %")
                            .text("CPU threshold"),
                    )
                    .changed();
                ui.end_row();

                ui.label(
                    egui::RichText::new("Memory usage alert threshold:").color(ThemePalette::text_secondary(is_dark)),
                );
                *changed |= ui
                    .add(
                        egui::Slider::new(&mut app.settings.notification_memory_threshold, ALERT_PERCENT)
                            .suffix(" %")
                            .text("Memory threshold"),
                    )
                    .changed();
                ui.end_row();

                ui.label(
                    egui::RichText::new("Temperature alert threshold:").color(ThemePalette::text_secondary(is_dark)),
                );
                *changed |= ui
                    .add(
                        egui::Slider::new(&mut app.settings.notification_temp_threshold, ALERT_TEMPERATURE)
                            .suffix(" °C")
                            .text("Temperature threshold"),
                    )
                    .changed();
                ui.end_row();

                ui.label(
                    egui::RichText::new("Disk usage alert threshold:").color(ThemePalette::text_secondary(is_dark)),
                );
                *changed |= ui
                    .add(
                        egui::Slider::new(&mut app.settings.notification_disk_threshold, ALERT_PERCENT)
                            .suffix(" %")
                            .text("Disk threshold"),
                    )
                    .changed();
                ui.end_row();
            });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        ui.vertical(|ui| {
            ui.vertical(|ui| {
                *changed |= ui
                    .checkbox(&mut app.settings.show_notifications, "Enable Desktop Notifications")
                    .changed();
                *changed |= ui
                    .checkbox(
                        &mut app.settings.enable_alert_sound,
                        "Play audio chime when alert triggers",
                    )
                    .changed();
            });

            ui.vertical(|ui| {
                *changed |= ui
                    .checkbox(&mut app.settings.enable_sounds, "Enable System Event Sounds")
                    .changed();
                *changed |= ui
                    .checkbox(&mut app.settings.auto_clear_alerts, "Auto-clear Resolved Alerts")
                    .changed();
            });
        });
    });
}
