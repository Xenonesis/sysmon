use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_manual_clean_card(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("MANUAL WORKING-SET CLEAN")
                    .size(13.0)
                    .strong()
                    .color(ThemePalette::text_primary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.ram_cleaner_state.is_cleaning {
                    status_pill(ui, "CLEANING IN PROGRESS...", ThemePalette::ACCENT_PRIMARY, is_dark);
                }
            });
        });
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Trims process working sets. Observed decreases are not guaranteed physical RAM recovered; \
                 paging active data back in can cause slowdowns.",
            )
            .size(12.0)
            .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(10.0);

        let is_cleaning = app.ram_cleaner_state.is_cleaning;
        ui.add_enabled_ui(!is_cleaning, |ui| {
            let btn = egui::Button::new(
                egui::RichText::new("Trim RAM Working Sets Now")
                    .size(13.5)
                    .strong()
                    .color(if is_cleaning {
                        ThemePalette::text_dimmed(is_dark)
                    } else {
                        ThemePalette::bg_deepest(is_dark)
                    }),
            )
            .fill(if is_cleaning {
                ThemePalette::bg_track(is_dark)
            } else {
                ThemePalette::ACCENT_PRIMARY
            })
            .corner_radius(egui::CornerRadius::same(4));

            if ui.add_sized([ui.available_width(), 34.0], btn).clicked() {
                app.start_ram_clean(ui.ctx());
            }
        });
    });
}
