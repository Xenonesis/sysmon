use crate::app::commands::UiIntent;
use crate::app::page_state::ServicePageState;
use crate::services::{ServiceControlAction, ServiceInfo};
use crate::ui::components::{card_frame, status_pill};
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint(
    ui: &mut egui::Ui,
    state: &mut ServicePageState,
    services: &[ServiceInfo],
    is_dark: bool,
    is_elevated: bool,
    intents: &mut Vec<UiIntent>,
) {
    let Some(selected_name) = state.selected_name.clone() else {
        return;
    };
    let Some(service) = services.iter().find(|service| service.name == selected_name) else {
        return;
    };

    let is_running = service.state.eq_ignore_ascii_case("running");
    let is_stopped = service.state.eq_ignore_ascii_case("stopped");
    let state_color = super::service_state_color(&service.state, is_dark);

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("SERVICE INSPECTOR")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::ACCENT_PRIMARY),
            );
            status_pill(ui, &service.state.to_uppercase(), state_color, is_dark);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕ Close").on_hover_text("Deselect service").clicked() {
                    state.selected_name = None;
                }
            });
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            paint_identity(ui, service, is_dark);
            ui.add_space(24.0);
            paint_copy_commands(ui, service, is_dark);
        });

        ui.add_space(8.0);
        paint_actions(ui, service, is_running, is_stopped, is_dark, is_elevated, intents);
    });
    ui.add_space(8.0);
}

fn paint_identity(ui: &mut egui::Ui, service: &ServiceInfo, is_dark: bool) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Display Name:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.label(
                egui::RichText::new(&service.display_name)
                    .strong()
                    .color(ThemePalette::text_primary(is_dark)),
            );
        });
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Service Identifier:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.label(
                egui::RichText::new(&service.name)
                    .monospace()
                    .color(ThemePalette::ACCENT_PRIMARY),
            );
        });
    });
}

fn paint_copy_commands(ui: &mut egui::Ui, service: &ServiceInfo, is_dark: bool) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("CLI & Scripting Snippets:")
                .size(10.5)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.horizontal(|ui| {
            if ui
                .small_button("📋 Copy Name")
                .on_hover_text("Copy service identifier to clipboard")
                .clicked()
            {
                ui.output_mut(|output| output.copied_text = service.name.clone());
            }
            if ui
                .small_button("📋 PowerShell: Get-Service")
                .on_hover_text("Copy 'Get-Service -Name <svc>' command")
                .clicked()
            {
                ui.output_mut(|output| output.copied_text = format!("Get-Service -Name \"{}\"", service.name));
            }
            if ui
                .small_button("📋 PowerShell: Restart-Service")
                .on_hover_text("Copy 'Restart-Service -Name <svc> -Force' command")
                .clicked()
            {
                ui.output_mut(|output| {
                    output.copied_text = format!("Restart-Service -Name \"{}\" -Force", service.name)
                });
            }
            if ui
                .small_button("📋 SC Query")
                .on_hover_text("Copy 'sc.exe query <svc>' command")
                .clicked()
            {
                ui.output_mut(|output| output.copied_text = format!("sc.exe query \"{}\"", service.name));
            }
        });
    });
}

fn paint_actions(
    ui: &mut egui::Ui,
    service: &ServiceInfo,
    is_running: bool,
    is_stopped: bool,
    is_dark: bool,
    is_elevated: bool,
    intents: &mut Vec<UiIntent>,
) {
    ui.horizontal(|ui| {
        let tooltip = if is_elevated {
            "Execute service control action"
        } else {
            "Administrator privileges required to control services"
        };
        let start = ui.add_enabled(
            is_elevated && !is_running,
            egui::Button::new(egui::RichText::new("▶ Start Service").strong()),
        );
        let stop =
            ui.add_enabled(
                is_elevated && !is_stopped,
                egui::Button::new(egui::RichText::new("⏹ Stop Service").strong().color(
                    if is_elevated && !is_stopped {
                        ThemePalette::STATUS_CRITICAL
                    } else {
                        ThemePalette::text_dimmed(is_dark)
                    },
                )),
            );
        let restart = ui.add_enabled(
            is_elevated && is_running,
            egui::Button::new(egui::RichText::new("🔄 Restart Service").strong()),
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

        if !is_elevated {
            ui.label(
                egui::RichText::new("⚠ Elevated permissions required for service control")
                    .size(11.0)
                    .color(ThemePalette::STATUS_WARNING),
            );
        }
    });
}

fn control_intent(service: &ServiceInfo, action: ServiceControlAction) -> UiIntent {
    UiIntent::ControlService {
        name: service.name.clone(),
        action,
    }
}
