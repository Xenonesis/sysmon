use crate::app::commands::UiIntent;
use crate::app::page_state::ServicePageState;
use crate::services::{ServiceInfo, ServiceSortColumn};
use crate::ui::components::card_frame;
use crate::ui::theme::ThemePalette;
use eframe::egui;

#[derive(Clone, Copy)]
pub(super) struct ColumnWidths {
    pub(super) total: f32,
    pub(super) spacing: f32,
    pub(super) display: f32,
    pub(super) identifier: f32,
    pub(super) state: f32,
    pub(super) actions: f32,
}

impl ColumnWidths {
    fn for_width(available: f32) -> Self {
        let total = available.max(680.0);
        let spacing = 8.0;
        let actions = 175.0;
        let state = 110.0;
        let identifier = 180.0f32.min(total * 0.26).max(140.0);
        let display = (total - actions - state - identifier - (3.0 * spacing)).max(220.0);
        Self {
            total,
            spacing,
            display,
            identifier,
            state,
            actions,
        }
    }
}

pub(super) fn paint(
    ui: &mut egui::Ui,
    state: &mut ServicePageState,
    services: &[&ServiceInfo],
    is_dark: bool,
    is_elevated: bool,
    intents: &mut Vec<UiIntent>,
) {
    card_frame(is_dark).show(ui, |ui| {
        let widths = ColumnWidths::for_width(ui.available_width());
        paint_header(ui, state, widths, is_dark);
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        if services.is_empty() {
            paint_empty(ui, state, is_dark);
            return;
        }

        let row_height = 28.0;
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_height(520.0)
            .show_rows(ui, row_height, services.len(), |ui, row_range| {
                for index in row_range {
                    super::row::paint(
                        ui,
                        state,
                        services[index],
                        index,
                        row_height,
                        widths,
                        is_dark,
                        is_elevated,
                        intents,
                    );
                }
            });
    });
}

fn paint_header(ui: &mut egui::Ui, state: &mut ServicePageState, widths: ColumnWidths, is_dark: bool) {
    let current = state.sort_column;
    let ascending = state.sort_ascending;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = widths.spacing;
        if header_button(
            ui,
            "Display Name",
            widths.display,
            ServiceSortColumn::DisplayName,
            current,
            ascending,
            is_dark,
        )
        .clicked()
        {
            state.select_sort(ServiceSortColumn::DisplayName);
        }
        if header_button(
            ui,
            "Service Identifier",
            widths.identifier,
            ServiceSortColumn::Name,
            current,
            ascending,
            is_dark,
        )
        .clicked()
        {
            state.select_sort(ServiceSortColumn::Name);
        }
        if header_button(
            ui,
            "State",
            widths.state,
            ServiceSortColumn::State,
            current,
            ascending,
            is_dark,
        )
        .clicked()
        {
            state.select_sort(ServiceSortColumn::State);
        }
        ui.allocate_ui_with_layout(
            egui::vec2(widths.actions, 22.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_width(widths.actions);
                ui.label(
                    egui::RichText::new("Actions")
                        .strong()
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            },
        );
    });
}

#[allow(clippy::too_many_arguments)]
fn header_button(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    column: ServiceSortColumn,
    current: ServiceSortColumn,
    ascending: bool,
    is_dark: bool,
) -> egui::Response {
    let text = super::sort_header_label(label, column, current, ascending);
    let color = if column == current {
        ThemePalette::ACCENT_PRIMARY
    } else {
        ThemePalette::text_primary(is_dark)
    };
    let mut response = None;
    ui.allocate_ui_with_layout(
        egui::vec2(width, 22.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(width);
            response = Some(
                ui.add(
                    egui::Button::new(egui::RichText::new(text).strong().size(11.5).color(color))
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE),
                ),
            );
        },
    );
    response.expect("header response must be created")
}

fn paint_empty(ui: &mut egui::Ui, state: &mut ServicePageState, is_dark: bool) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new("🔍 No services match the current filter criteria.")
                .size(13.0)
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);
        if ui.button("Reset Filters").clicked() {
            state.reset_filters();
        }
    });
    ui.add_space(24.0);
}
