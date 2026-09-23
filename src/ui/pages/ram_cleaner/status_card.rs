use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(super) fn paint_status_card(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("MEMORY STATUS")
                    .size(13.0)
                    .strong()
                    .color(ThemePalette::text_primary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let total_gb = bytes_to_gb(data.memory_total);
                let used_gb = bytes_to_gb(data.memory_used);
                let free_gb = bytes_to_gb(data.memory_total.saturating_sub(data.memory_used));
                ui.label(
                    egui::RichText::new(format!(
                        "{:.2} GB / {:.2} GB · {:.2} GB Free",
                        used_gb, total_gb, free_gb
                    ))
                    .size(12.0)
                    .monospace()
                    .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        });

        ui.add_space(8.0);
        let color = get_usage_color(data.memory_percentage);
        paint_progress_bar(ui, data.memory_percentage / 100.0, color, 8.0, is_dark);

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{:.1}% Used", data.memory_percentage))
                    .size(14.0)
                    .strong()
                    .monospace()
                    .color(color),
            );
            ui.add_space(16.0);
            if privilege::is_app_elevated() {
                status_pill(
                    ui,
                    "ELEVATED (PROTECTED PROCESSES STILL RESTRICTED)",
                    ThemePalette::STATUS_HEALTHY,
                    is_dark,
                );
            } else {
                status_pill(
                    ui,
                    "ACCESS DEPENDS ON PROCESS PERMISSIONS",
                    ThemePalette::STATUS_WARNING,
                    is_dark,
                );
            }
        });
    });
}
