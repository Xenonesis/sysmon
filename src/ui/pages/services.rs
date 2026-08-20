use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Resolves semantic color for Windows service states.
pub(crate) fn service_state_color(state: &str, is_dark: bool) -> egui::Color32 {
    match state.to_lowercase().as_str() {
        "running" => ThemePalette::STATUS_HEALTHY,
        "stopped" => ThemePalette::text_dimmed(is_dark),
        "paused" | "start pending" | "stop pending" | "continue pending" | "pause pending" => {
            ThemePalette::STATUS_WARNING
        }
        _ => ThemePalette::text_secondary(is_dark),
    }
}

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    let is_elevated = privilege::is_app_elevated();
    paint_section_header(ui, "Windows Services", is_dark);

    if data.services.is_empty() {
        ui.add_space(16.0);
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Loading services telemetry...").color(ThemePalette::text_secondary(is_dark)),
                );
            });
        });
        return;
    }

    // Apply filters
    let query = app.service_search.to_lowercase();
    let filtered: Vec<_> = data
        .services
        .iter()
        .filter(|svc| {
            let name_match = query.is_empty()
                || svc.name.to_lowercase().contains(&query)
                || svc.display_name.to_lowercase().contains(&query);
            let state_match = app.service_state_filter.as_deref().is_none_or(|s| svc.state == s);
            name_match && state_match
        })
        .collect();

    // ── Search & State Filter Toolbar ──
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            // Search Input with integrated Clear button
            ui.label(
                egui::RichText::new("Search:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.service_search)
                    .hint_text("Filter by name or display...")
                    .desired_width(220.0),
            );
            if !app.service_search.is_empty() && ui.small_button("×").on_hover_text("Clear search filter").clicked() {
                app.service_search.clear();
            }

            ui.add_space(12.0);

            // State filter ComboBox
            ui.label(
                egui::RichText::new("State:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            egui::ComboBox::from_id_source("svc_state_filter")
                .selected_text(app.service_state_filter.as_deref().unwrap_or("All"))
                .show_ui(ui, |ui| {
                    if ui.selectable_label(app.service_state_filter.is_none(), "All").clicked() {
                        app.service_state_filter = None;
                    }
                    for state in &["Running", "Stopped", "Start Pending", "Stop Pending", "Paused"] {
                        let selected = app.service_state_filter.as_deref() == Some(state);
                        if ui.selectable_label(selected, *state).clicked() {
                            app.service_state_filter = Some(state.to_string());
                        }
                    }
                });

            let has_active_filter = !app.service_search.is_empty() || app.service_state_filter.is_some();
            if has_active_filter
                && ui
                    .small_button("Reset")
                    .on_hover_text("Reset search and state filters")
                    .clicked()
            {
                app.service_search.clear();
                app.service_state_filter = None;
            }

            ui.add_space(8.0);

            // Count badge
            let count_label = format!("Showing {} / {}", filtered.len(), data.services.len());
            status_pill(ui, &count_label, ThemePalette::ACCENT_PRIMARY, is_dark);

            // Elevation Status Banner
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if is_elevated {
                    status_pill(ui, "ADMIN ELEVATED", ThemePalette::STATUS_HEALTHY, is_dark);
                } else {
                    status_pill(ui, "ADMIN REQUIRED FOR CONTROL", ThemePalette::STATUS_WARNING, is_dark);
                }
            });
        });
    });

    ui.add_space(8.0);

    // ── Responsive Virtualized Services Table ──
    card_frame(is_dark).show(ui, |ui| {
        // Sticky Header
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(220.0, 22.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new("Display Name")
                            .strong()
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(160.0, 22.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new("Service Identifier")
                            .strong()
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(100.0, 22.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new("State")
                            .strong()
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                },
            );
            ui.label(
                egui::RichText::new("Actions")
                    .strong()
                    .size(11.5)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        let row_height = 28.0;
        let num_rows = filtered.len();

        ui.spacing_mut().item_spacing.y = 0.0;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_height(540.0)
            .show_rows(ui, row_height, num_rows, |ui, row_range| {
                for idx in row_range {
                    let svc = filtered[idx];
                    let is_even = idx % 2 == 0;

                    let (row_rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width().max(640.0), row_height),
                        egui::Sense::hover(),
                    );

                    if is_even {
                        let stripe_fill = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 3)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 3)
                        };
                        ui.painter()
                            .rect_filled(row_rect, egui::Rounding::same(2.0), stripe_fill);
                    }

                    ui.allocate_ui_at_rect(row_rect, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;

                            // Display Name
                            let display = if svc.display_name.chars().count() > 30 {
                                let trunc: String = svc.display_name.chars().take(27).collect();
                                format!("{}...", trunc)
                            } else {
                                svc.display_name.clone()
                            };
                            ui.add_sized(
                                [220.0, row_height],
                                egui::Label::new(
                                    egui::RichText::new(display)
                                        .strong()
                                        .color(ThemePalette::text_primary(is_dark)),
                                ),
                            )
                            .on_hover_text(&svc.display_name);

                            // Service Identifier (Monospace)
                            ui.add_sized(
                                [160.0, row_height],
                                egui::Label::new(
                                    egui::RichText::new(&svc.name)
                                        .monospace()
                                        .color(ThemePalette::text_secondary(is_dark)),
                                ),
                            )
                            .on_hover_text(&svc.name);

                            // State Pill
                            let state_c = service_state_color(&svc.state, is_dark);
                            ui.allocate_ui_with_layout(
                                egui::vec2(100.0, row_height),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    status_pill(ui, &svc.state.to_uppercase(), state_c, is_dark);
                                },
                            );

                            // Action buttons with Elevation Protection
                            ui.horizontal(|ui| {
                                ui.add_enabled_ui(is_elevated, |ui| {
                                    let is_running = svc.state.eq_ignore_ascii_case("running");
                                    let is_stopped = svc.state.eq_ignore_ascii_case("stopped");

                                    let start_btn = ui.add_enabled(
                                        !is_running,
                                        egui::Button::new(egui::RichText::new("Start").small()),
                                    );
                                    let stop_btn = ui.add_enabled(
                                        !is_stopped,
                                        egui::Button::new(
                                            egui::RichText::new("Stop").small().color(ThemePalette::STATUS_CRITICAL),
                                        ),
                                    );
                                    let restart_btn = ui.add_enabled(
                                        is_running,
                                        egui::Button::new(egui::RichText::new("Restart").small()),
                                    );

                                    let tooltip = if !is_elevated {
                                        "Requires Administrator privileges"
                                    } else {
                                        "Send service control command"
                                    };

                                    if start_btn.on_hover_text(tooltip).clicked() {
                                        app.pending_service_action = Some(services::ServiceAction {
                                            name: svc.name.clone(),
                                            action: services::ServiceControlAction::Start,
                                        });
                                    }
                                    if stop_btn.on_hover_text(tooltip).clicked() {
                                        app.pending_service_action = Some(services::ServiceAction {
                                            name: svc.name.clone(),
                                            action: services::ServiceControlAction::Stop,
                                        });
                                    }
                                    if restart_btn.on_hover_text(tooltip).clicked() {
                                        app.pending_service_action = Some(services::ServiceAction {
                                            name: svc.name.clone(),
                                            action: services::ServiceControlAction::Restart,
                                        });
                                    }
                                });
                            });
                        });
                    });
                }
            });
    });

    if let Some(action) = app.pending_service_action.take() {
        app.queue_action(crate::app::commands::ActionCommand::ControlService {
            name: action.name,
            action: action.action,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_state_color_mapping() {
        assert_eq!(service_state_color("Running", true), ThemePalette::STATUS_HEALTHY);
        assert_eq!(service_state_color("RUNNING", false), ThemePalette::STATUS_HEALTHY);
        assert_eq!(service_state_color("Stopped", true), ThemePalette::text_dimmed(true));
        assert_eq!(service_state_color("Paused", true), ThemePalette::STATUS_WARNING);
        assert_eq!(service_state_color("Start Pending", true), ThemePalette::STATUS_WARNING);
        assert_eq!(service_state_color("Stop Pending", false), ThemePalette::STATUS_WARNING);
    }
}
