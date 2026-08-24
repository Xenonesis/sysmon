use crate::app::page_state::ServicePageState;
use crate::ui::components::card_frame;
use crate::ui::theme::ThemePalette;
use eframe::egui;

use super::summary::ServiceCounts;

pub(super) fn paint(
    ui: &mut egui::Ui,
    state: &mut ServicePageState,
    counts: ServiceCounts,
    visible_count: usize,
    is_dark: bool,
) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(
                egui::RichText::new("🔍 Search:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut state.search)
                    .hint_text("Filter by name or identifier...")
                    .desired_width(240.0),
            );
            if !state.search.is_empty() && ui.small_button("×").on_hover_text("Clear search filter").clicked() {
                state.search.clear();
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("State:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );

            let is_all = state.state_filter.is_none();
            if ui.selectable_label(is_all, format!("All ({})", counts.total)).clicked() {
                state.state_filter = None;
            }

            let is_running = state.state_filter.as_deref() == Some("Running");
            if ui
                .selectable_label(is_running, format!("Running ({})", counts.running))
                .clicked()
            {
                state.state_filter = if is_running { None } else { Some("Running".to_string()) };
            }

            let is_stopped = state.state_filter.as_deref() == Some("Stopped");
            if ui
                .selectable_label(is_stopped, format!("Stopped ({})", counts.stopped))
                .clicked()
            {
                state.state_filter = if is_stopped { None } else { Some("Stopped".to_string()) };
            }

            let has_active_filter = !state.search.is_empty() || state.state_filter.is_some();
            if has_active_filter
                && ui
                    .small_button("Reset")
                    .on_hover_text("Reset search and state filters")
                    .clicked()
            {
                state.reset_filters();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("Showing {visible_count} of {}", counts.total))
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        });
    });
}
