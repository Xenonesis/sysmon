use crate::app::commands::UiIntent;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint(ui: &mut egui::Ui, is_dark: bool, intents: &mut Vec<UiIntent>) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Windows Services")
                .size(15.5)
                .strong()
                .color(ThemePalette::text_primary(is_dark)),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new("LIVE")
                    .size(9.5)
                    .monospace()
                    .strong()
                    .color(ThemePalette::ACCENT_PRIMARY),
            );
            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(dot_rect.center(), 2.5, ThemePalette::ACCENT_PRIMARY);

            ui.add_space(8.0);
            if ui
                .button(egui::RichText::new("⚙ services.msc").size(11.0).strong())
                .on_hover_text("Open Windows Services Management Console (services.msc)")
                .clicked()
            {
                intents.push(UiIntent::OpenServicesConsole);
            }
        });
    });

    ui.add_space(3.0);
    let full_w = ui.available_width();
    let (line_rect, _) = ui.allocate_exact_size(egui::vec2(full_w, 1.0), egui::Sense::hover());
    let accent_w = 48.0f32.min(full_w);
    let accent_rect = egui::Rect::from_min_size(line_rect.min, egui::vec2(accent_w, 1.0));
    let remainder_rect = egui::Rect::from_min_size(
        egui::pos2(line_rect.min.x + accent_w, line_rect.min.y),
        egui::vec2((full_w - accent_w).max(0.0), 1.0),
    );
    ui.painter()
        .rect_filled(accent_rect, egui::Rounding::ZERO, ThemePalette::ACCENT_PRIMARY);
    ui.painter()
        .rect_filled(remainder_rect, egui::Rounding::ZERO, ThemePalette::border(is_dark));
    ui.add_space(10.0);
}
