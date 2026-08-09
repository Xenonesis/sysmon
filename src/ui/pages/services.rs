use crate::*;
use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "Windows Services");

        if data.services.is_empty() {
            ui.add_space(16.0);
            ui.label(egui::RichText::new("Loading services…").color(ThemePalette::TEXT_SECONDARY));
            return;
        }

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

                    for svc in &data.services {
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
            let _ = app.app_channels.action_sender.send(crate::app::commands::ActionCommand::ControlService {
                name: action.name,
                action: action.action,
            });
        }
    }
