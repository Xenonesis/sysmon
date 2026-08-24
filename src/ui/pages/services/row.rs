use crate::app::commands::UiIntent;
use crate::app::page_state::ServicePageState;
use crate::services::{ServiceControlAction, ServiceInfo};
use crate::ui::components::status_pill;
use crate::ui::theme::ThemePalette;
use eframe::egui;

use super::table::ColumnWidths;

#[allow(clippy::too_many_arguments)]
pub(super) fn paint(
    ui: &mut egui::Ui,
    state: &mut ServicePageState,
    service: &ServiceInfo,
    index: usize,
    row_height: f32,
    widths: ColumnWidths,
    is_dark: bool,
    is_elevated: bool,
    intents: &mut Vec<UiIntent>,
) {
    let selected = state.selected_name.as_deref() == Some(service.name.as_str());
    let (row_rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(widths.total), row_height),
        egui::Sense::click(),
    );
    if response.clicked() {
        state.toggle_selected(&service.name);
    }
    paint_background(ui, row_rect, &response, index, selected, is_dark);

    ui.allocate_ui_at_rect(row_rect, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = widths.spacing;
            paint_display_name(ui, service, selected, widths.display, row_height, is_dark);
            paint_identifier(ui, service, widths.identifier, row_height, is_dark);
            paint_state(ui, service, widths.state, row_height, is_dark);
            paint_actions(ui, service, widths.actions, row_height, is_dark, is_elevated, intents);
        });
    });
}

fn paint_background(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
    index: usize,
    selected: bool,
    is_dark: bool,
) {
    if selected {
        let fill = if is_dark {
            egui::Color32::from_rgba_unmultiplied(16, 185, 129, 30)
        } else {
            egui::Color32::from_rgba_unmultiplied(16, 185, 129, 20)
        };
        ui.painter().rect_filled(rect, egui::Rounding::same(3.0), fill);
        let indicator = egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height()));
        ui.painter()
            .rect_filled(indicator, egui::Rounding::same(1.5), ThemePalette::ACCENT_PRIMARY);
    } else if response.hovered() {
        let fill = if is_dark {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10)
        } else {
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 8)
        };
        ui.painter().rect_filled(rect, egui::Rounding::same(3.0), fill);
    } else if index % 2 == 0 {
        let fill = if is_dark {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 3)
        } else {
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 3)
        };
        ui.painter().rect_filled(rect, egui::Rounding::same(3.0), fill);
    }
}

fn paint_display_name(
    ui: &mut egui::Ui,
    service: &ServiceInfo,
    selected: bool,
    width: f32,
    height: f32,
    is_dark: bool,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(width);
            let color = if selected {
                ThemePalette::ACCENT_PRIMARY
            } else {
                ThemePalette::text_primary(is_dark)
            };
            ui.add(egui::Label::new(egui::RichText::new(&service.display_name).strong().color(color)).truncate())
                .on_hover_text(format!(
                    "{}\nClick row to inspect / copy commands",
                    service.display_name
                ));
        },
    );
}

fn paint_identifier(ui: &mut egui::Ui, service: &ServiceInfo, width: f32, height: f32, is_dark: bool) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(width);
            let response = ui
                .add(
                    egui::Label::new(
                        egui::RichText::new(&service.name)
                            .monospace()
                            .color(ThemePalette::text_secondary(is_dark)),
                    )
                    .truncate()
                    .sense(egui::Sense::click()),
                )
                .on_hover_text(format!("Click to copy '{}'", service.name));
            if response.clicked() {
                ui.output_mut(|output| output.copied_text = service.name.clone());
            }
        },
    );
}

fn paint_state(ui: &mut egui::Ui, service: &ServiceInfo, width: f32, height: f32, is_dark: bool) {
    let color = super::service_state_color(&service.state, is_dark);
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(width);
            status_pill(ui, &service.state.to_uppercase(), color, is_dark);
        },
    );
}

fn paint_actions(
    ui: &mut egui::Ui,
    service: &ServiceInfo,
    width: f32,
    height: f32,
    is_dark: bool,
    is_elevated: bool,
    intents: &mut Vec<UiIntent>,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(width);
            let is_running = service.state.eq_ignore_ascii_case("running");
            let is_stopped = service.state.eq_ignore_ascii_case("stopped");
            let tooltip = if is_elevated {
                "Send service control command"
            } else {
                "Administrator privileges required to control services"
            };
            let start = ui.add_enabled(
                is_elevated && !is_running,
                egui::Button::new(egui::RichText::new("Start").small()),
            );
            let stop = ui.add_enabled(
                is_elevated && !is_stopped,
                egui::Button::new(
                    egui::RichText::new("Stop")
                        .small()
                        .color(if is_elevated && !is_stopped {
                            ThemePalette::STATUS_CRITICAL
                        } else {
                            ThemePalette::text_dimmed(is_dark)
                        }),
                ),
            );
            let restart = ui.add_enabled(
                is_elevated && is_running,
                egui::Button::new(egui::RichText::new("Restart").small()),
            );
            if start.on_hover_text(tooltip).clicked() {
                intents.push(control_intent(service, ServiceControlAction::Start));
            }
            if stop.on_hover_text(tooltip).clicked() {
                intents.push(control_intent(service, ServiceControlAction::Stop));
            }
            if restart.on_hover_text(tooltip).clicked() {
                intents.push(control_intent(service, ServiceControlAction::Restart));
            }
        },
    );
}

fn control_intent(service: &ServiceInfo, action: ServiceControlAction) -> UiIntent {
    UiIntent::ControlService {
        name: service.name.clone(),
        action,
    }
}
