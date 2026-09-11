use crate::SystemData;
use crate::processes::{AffinityPreset, ProcessInfo};
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_row_actions(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    process: &ProcessInfo,
    _data: &SystemData,
    _is_dark: bool,
    row_height: f32,
    action_w: f32,
) {
    // Action buttons container: Kill, Tree, Suspend/Resume, Menu
    ui.allocate_ui_with_layout(
        egui::vec2(action_w, row_height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if process.identity.is_none() {
                ui.label("Actions unavailable")
                    .on_hover_text("Native process creation time could not be queried; PID-only actions are unsafe.");
                return;
            }
            let identity = process.identity.expect("identity checked above");

            if ui
                .small_button(egui::RichText::new("Kill").color(ThemePalette::STATUS_CRITICAL))
                .on_hover_text("Terminate this process")
                .clicked()
            {
                app.selected_process_pid = Some(identity);
            }

            if ui
                .small_button("Tree")
                .on_hover_text("Kill this process and all its children (deepest first)")
                .clicked()
            {
                app.kill_tree_pid = Some(identity);
            }

            let is_suspended = app.suspended_pids.contains(&identity);
            if is_suspended {
                if ui
                    .small_button(egui::RichText::new("Resume").color(ThemePalette::STATUS_HEALTHY))
                    .on_hover_text("Resume suspended process")
                    .clicked()
                {
                    app.resume_process_pid = Some(identity);
                }
            } else if ui
                .small_button("Suspend")
                .on_hover_text("Freeze process execution (Windows only)")
                .clicked()
            {
                app.suspend_process_pid = Some(identity);
            }

            // Unified lightweight Menu Button
            ui.menu_button("⚙", |ui| {
                ui.set_min_width(160.0);
                ui.label(egui::RichText::new(format!("PID {} Actions", process.pid)).strong());
                ui.separator();

                ui.menu_button("Set Priority ▸", |ui| {
                    for priority in &["High", "AboveNormal", "Normal", "BelowNormal", "Idle"] {
                        if ui.button(*priority).clicked() {
                            app.priority_change = Some((identity, priority.to_string()));
                            ui.close();
                        }
                    }
                });

                ui.menu_button("Set CPU Affinity", |ui| {
                    for (label, preset) in [
                        ("All currently allowed cores", AffinityPreset::All),
                        ("First allowed core", AffinityPreset::First),
                        ("Second allowed core", AffinityPreset::Second),
                        ("First half of allowed cores", AffinityPreset::FirstHalf),
                    ] {
                        if ui.button(label).clicked() {
                            app.affinity_change = Some((identity, preset));
                            ui.close();
                        }
                    }
                    ui.label("Validated against the native mask. Multi-group processes are unsupported.");
                });

                ui.separator();

                if ui.button("📋 Copy PID").clicked() {
                    ui.ctx().copy_text(process.pid.to_string());
                    ui.close();
                }

                if ui.button("🔍 Inspect Details").clicked() {
                    app.details_pid = process.identity;
                    ui.close();
                }
            })
            .response
            .on_hover_text("More actions (Priority, Affinity, Copy PID, Details)");
        },
    );
}
