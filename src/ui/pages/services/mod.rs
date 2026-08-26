use crate::SystemData;
use crate::app::commands::UiIntent;
use crate::ui::components::card_frame;
use crate::ui::theme::ThemePalette;
use eframe::egui;

mod header;
mod inspector;
mod model;
mod row;
mod summary;
mod table;
mod toolbar;

pub(crate) use model::{service_state_color, sort_header_label};

pub(crate) fn show(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    data: &SystemData,
    is_elevated: bool,
) -> Vec<UiIntent> {
    let is_dark = ui.visuals().dark_mode;
    let mut intents = Vec::new();

    header::paint(ui, is_dark, &mut intents);
    if data.services.is_empty() {
        paint_loading(ui, is_dark);
        return intents;
    }

    let counts = summary::ServiceCounts::from_services(&data.services);
    summary::paint(ui, counts, is_dark, is_elevated, &mut intents);
    ui.add_space(8.0);

    let visible = app.service_page.visible_services(&data.services);
    toolbar::paint(ui, &mut app.service_page, counts, visible.len(), is_dark);
    ui.add_space(8.0);

    inspector::paint(
        ui,
        &mut app.service_page,
        &data.services,
        is_dark,
        is_elevated,
        &mut intents,
    );
    table::paint(ui, &mut app.service_page, &visible, is_dark, is_elevated, &mut intents);

    intents
}

fn paint_loading(ui: &mut egui::Ui, is_dark: bool) {
    ui.add_space(16.0);
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Loading Windows services telemetry...")
                    .color(ThemePalette::text_secondary(is_dark)),
            );
        });
    });
}

#[cfg(test)]
mod tests;
