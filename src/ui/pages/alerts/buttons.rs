use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn toggle_button(
    ui: &mut egui::Ui,
    label: &str,
    is_on: bool,
    tooltip: &str,
    is_dark: bool,
) -> egui::Response {
    let text_color = if is_on {
        ThemePalette::STATUS_HEALTHY
    } else {
        ThemePalette::text_secondary(is_dark)
    };
    let fill = if is_on {
        ThemePalette::STATUS_HEALTHY.gamma_multiply(if is_dark { 0.14 } else { 0.10 })
    } else {
        ThemePalette::bg_track(is_dark)
    };
    let stroke_color = if is_on {
        ThemePalette::STATUS_HEALTHY.gamma_multiply(0.40)
    } else {
        ThemePalette::border(is_dark)
    };

    let btn = egui::Button::new(egui::RichText::new(label).size(11.5).strong().color(text_color))
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke_color))
        .corner_radius(egui::CornerRadius::same(4));

    ui.add(btn).on_hover_text(tooltip)
}

pub(super) fn secondary_button(ui: &mut egui::Ui, label: &str, tooltip: &str, is_dark: bool) -> egui::Response {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .size(11.5)
            .strong()
            .color(ThemePalette::text_primary(is_dark)),
    )
    .fill(ThemePalette::bg_track(is_dark))
    .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
    .corner_radius(egui::CornerRadius::same(4));

    ui.add(btn).on_hover_text(tooltip)
}

pub(super) fn accent_button(ui: &mut egui::Ui, label: &str, tooltip: &str, is_dark: bool) -> egui::Response {
    let btn = egui::Button::new(
        egui::RichText::new(label)
            .size(11.5)
            .strong()
            .color(ThemePalette::ACCENT_PRIMARY),
    )
    .fill(ThemePalette::ACCENT_PRIMARY.gamma_multiply(if is_dark { 0.15 } else { 0.10 }))
    .stroke(egui::Stroke::new(
        1.0,
        ThemePalette::ACCENT_PRIMARY.gamma_multiply(0.45),
    ))
    .corner_radius(egui::CornerRadius::same(4));

    ui.add(btn).on_hover_text(tooltip)
}
