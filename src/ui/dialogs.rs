//! Global modal dialogs: action confirmations, action history, keyboard shortcuts.

use crate::ui::theme::ThemePalette;
use crate::{app, SystemMonitorApp};
use eframe::egui;

pub(crate) fn render_action_confirmation(app: &mut SystemMonitorApp, ctx: &egui::Context) {
    let Some(plan) = app.pending_action_plan.clone() else {
        return;
    };
    let mut confirm = false;
    let mut cancel = false;

    egui::Window::new("Confirm system action")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.heading(&plan.title);
            ui.label(&plan.summary);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.strong("Risk:");
                let color = match plan.risk {
                    app::actions::RiskLevel::Low => ThemePalette::STATUS_HEALTHY,
                    app::actions::RiskLevel::Medium => ThemePalette::STATUS_WARNING,
                    app::actions::RiskLevel::High | app::actions::RiskLevel::Critical => ThemePalette::STATUS_CRITICAL,
                };
                ui.colored_label(color, plan.risk.label());
            });
            ui.label(format!(
                "Administrator privileges: {}",
                if plan.requires_admin {
                    "usually required"
                } else {
                    "not required"
                }
            ));
            ui.label(format!(
                "Undo available: {}",
                if plan.reversible { "yes" } else { "no" }
            ));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                if ui.button(egui::RichText::new("Confirm and run").strong()).clicked() {
                    confirm = true;
                }
            });
        });

    if cancel {
        app.pending_action_plan = None;
    } else if confirm {
        app.pending_action_plan = None;
        if matches!(plan.command, app::commands::ActionCommand::CleanRam) {
            app.ram_cleaner_state.is_cleaning = true;
        }
        match app.app_channels.action_sender.send(plan.command) {
            Ok(()) => app.action_pending = true,
            Err(error) => app.action_status = Some(format!("Could not queue action: {error}")),
        }
    }
}

pub(crate) fn render_action_history(app: &mut SystemMonitorApp, ctx: &egui::Context) {
    if !app.show_action_history {
        return;
    }
    let mut open = app.show_action_history;
    let mut undo = None;
    egui::Window::new("System Action History")
        .open(&mut open)
        .default_width(620.0)
        .show(ctx, |ui| {
            ui.label(
                "Persistent audit records are stored locally. Undo is offered only when the original state is known.",
            );
            ui.separator();
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                for entry in app.action_history.iter().rev().take(100) {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong(&entry.record.action);
                            ui.label(entry.record.risk.label());
                            if entry.record.succeeded {
                                ui.colored_label(ThemePalette::STATUS_HEALTHY, "Succeeded");
                            } else {
                                ui.colored_label(ThemePalette::STATUS_CRITICAL, "Failed");
                            }
                        });
                        ui.small(&entry.record.timestamp);
                        ui.small(format!("Initiated by {}", entry.record.initiator));
                        ui.label(&entry.record.message);
                        if let Some(command) = &entry.undo {
                            if ui.button("Undo this action").clicked() {
                                undo = Some(command.clone());
                            }
                        }
                    });
                }
            });
        });
    app.show_action_history = open;
    if let Some(command) = undo {
        app.queue_action(command);
    }
}

pub(crate) fn render_shortcuts_dialog(app: &mut SystemMonitorApp, ctx: &egui::Context, is_dark: bool) {
    let mut show_shortcuts = app.show_shortcuts;
    egui::Window::new("Keyboard Shortcuts")
        .open(&mut show_shortcuts)
        .resizable(false)
        .default_width(400.0)
        .show(ctx, |ui| {
            ui.heading("Available Shortcuts");
            ui.separator();
            egui::Grid::new("shortcuts_grid").spacing([20.0, 6.0]).show(ui, |ui| {
                let shortcuts = [
                    ("F5", "Refresh / Reset statistics"),
                    ("Ctrl + E", "Export data to JSON"),
                    ("Ctrl + B", "Toggle Sidebar (Collapse/Expand)"),
                    ("Ctrl + M", "Toggle Floating Desktop HUD"),
                    ("Ctrl + ,", "Open Settings"),
                    ("Ctrl + U", "Check for updates"),
                ];
                for (key, desc) in &shortcuts {
                    ui.label(
                        egui::RichText::new(*key)
                            .monospace()
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.label(egui::RichText::new(*desc).color(ThemePalette::text_secondary(is_dark)));
                    ui.end_row();
                }
            });
        });
    app.show_shortcuts = show_shortcuts;
}
