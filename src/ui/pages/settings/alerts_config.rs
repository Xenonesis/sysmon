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
            .min_col_width(220.0)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("CPU usage alert threshold:").color(ThemePalette::text_secondary(is_dark)),
                );
                *changed |= ui
                    .add(egui::Slider::new(&mut app.settings.notification_cpu_threshold, 50.0..=100.0).suffix(" %"))
                    .changed();
                ui.end_row();

                ui.label(
                    egui::RichText::new("Memory usage alert threshold:").color(ThemePalette::text_secondary(is_dark)),
                );
                *changed |= ui
                    .add(egui::Slider::new(&mut app.settings.notification_memory_threshold, 50.0..=100.0).suffix(" %"))
                    .changed();
                ui.end_row();

                ui.label(
                    egui::RichText::new("Temperature alert threshold:").color(ThemePalette::text_secondary(is_dark)),
                );
                *changed |= ui
                    .add(egui::Slider::new(&mut app.settings.notification_temp_threshold, 60..=105).suffix(" °C"))
                    .changed();
                ui.end_row();

                ui.label(
                    egui::RichText::new("Disk usage alert threshold:").color(ThemePalette::text_secondary(is_dark)),
                );
                *changed |= ui
                    .add(egui::Slider::new(&mut app.settings.notification_disk_threshold, 50.0..=100.0).suffix(" %"))
                    .changed();
                ui.end_row();
            });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
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

            cols[1].vertical(|ui| {
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
