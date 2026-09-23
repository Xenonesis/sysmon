use crate::ui::components::*;
use crate::ui::format::bytes_to_mb;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_stats_card(app: &crate::SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("SESSION STATISTICS")
                .size(13.0)
                .strong()
                .color(ThemePalette::text_primary(is_dark)),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Clean Passes:").color(ThemePalette::text_secondary(is_dark)));
            ui.label(
                egui::RichText::new(format!("{}", app.ram_cleaner_state.clean_count))
                    .monospace()
                    .strong()
                    .color(ThemePalette::text_primary(is_dark)),
            );

            ui.add_space(24.0);
            ui.label(
                egui::RichText::new("Observed Working-Set Decrease:").color(ThemePalette::text_secondary(is_dark)),
            );
            ui.label(
                egui::RichText::new(format!("{:.2} MB", bytes_to_mb(app.ram_cleaner_state.bytes_freed)))
                    .monospace()
                    .strong()
                    .color(ThemePalette::STATUS_HEALTHY),
            );

            ui.add_space(24.0);
            ui.label(egui::RichText::new("Last Clean:").color(ThemePalette::text_secondary(is_dark)));
            if app.ram_cleaner_state.last_cleaned.is_some() {
                ui.label(
                    egui::RichText::new(&app.ram_cleaner_state.last_cleaned_display)
                        .monospace()
                        .color(ThemePalette::text_primary(is_dark)),
                );
            } else {
                ui.label(egui::RichText::new("Never").color(ThemePalette::text_dimmed(is_dark)));
            }
        });
    });
}
