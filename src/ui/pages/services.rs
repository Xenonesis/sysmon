use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    paint_section_header(ui, "Windows Services");

    if data.services.is_empty() {
        ui.add_space(16.0);
        ui.label(egui::RichText::new("Loading services…").color(ThemePalette::TEXT_SECONDARY));
        return;
    }

    // Search + state filter bar
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.add(
            egui::TextEdit::singleline(&mut app.service_search)
                .hint_text("Filter by name…")
                .desired_width(200.0),
        );
        ui.add_space(16.0);
        ui.label("State:");
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
        if ui.small_button("✕ Clear").clicked() {
            app.service_search.clear();
            app.service_state_filter = None;
        }
    });
    ui.add_space(8.0);

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

    ui.label(
        egui::RichText::new(format!("{} / {} services", filtered.len(), data.services.len()))
            .size(11.0)
            .color(ThemePalette::TEXT_DIMMED),
    );
    ui.add_space(4.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("services_grid")
            .striped(true)
            .min_col_width(200.0)
            .show(ui, |ui| {
                ui.strong("Display Name");
                ui.strong("Service Name");
                ui.strong("State");
                ui.strong("Actions");
                ui.end_row();

                for svc in &filtered {
                    ui.label(&svc.display_name);
                    ui.label(&svc.name);
                    let color = if svc.state == "Running" {
                        egui::Color32::from_rgb(0, 200, 100)
                    } else {
                        ThemePalette::TEXT_SECONDARY
                    };
                    ui.colored_label(color, &svc.state);

                    ui.horizontal(|ui| {
                        let start_btn = ui.small_button("Start");
                        let stop_btn = ui.small_button("Stop");
                        let restart_btn = ui.small_button("Restart");
                        if start_btn.clicked() {
                            app.pending_service_action = Some(services::ServiceAction {
                                name: svc.name.clone(),
                                action: services::ServiceControlAction::Start,
                            });
                        }
                        if stop_btn.clicked() {
                            app.pending_service_action = Some(services::ServiceAction {
                                name: svc.name.clone(),
                                action: services::ServiceControlAction::Stop,
                            });
                        }
                        if restart_btn.clicked() {
                            app.pending_service_action = Some(services::ServiceAction {
                                name: svc.name.clone(),
                                action: services::ServiceControlAction::Restart,
                            });
                        }
                    });
                    ui.end_row();
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
