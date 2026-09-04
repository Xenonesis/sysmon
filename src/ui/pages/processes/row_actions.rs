use crate::SystemData;
use crate::processes::ProcessInfo;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_row_actions(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    process: &ProcessInfo,
    data: &SystemData,
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

            if ui
                .small_button(egui::RichText::new("Kill").color(ThemePalette::STATUS_CRITICAL))
                .on_hover_text("Terminate this process")
                .clicked()
            {
                app.selected_process_pid = Some(process.pid);
            }

            if ui
                .small_button("Tree")
                .on_hover_text("Kill this process and all its children (deepest first)")
                .clicked()
            {
                app.kill_tree_pid = Some(process.pid);
            }

            let is_suspended = app.suspended_pids.contains(&process.pid);
            if is_suspended {
                if ui
                    .small_button(egui::RichText::new("Resume").color(ThemePalette::STATUS_HEALTHY))
                    .on_hover_text("Resume suspended process")
                    .clicked()
                {
                    app.resume_process_pid = Some(process.pid);
                }
            } else if ui
                .small_button("Suspend")
                .on_hover_text("Freeze process execution (Windows only)")
                .clicked()
            {
                app.suspend_process_pid = Some(process.pid);
            }

            // Unified lightweight Menu Button
            ui.menu_button("⚙", |ui| {
                ui.set_min_width(160.0);
                ui.label(egui::RichText::new(format!("PID {} Actions", process.pid)).strong());
                ui.separator();

                ui.menu_button("Set Priority ▸", |ui| {
                    for priority in &["High", "AboveNormal", "Normal", "BelowNormal", "Idle"] {
                        if ui.button(*priority).clicked() {
                            app.priority_change = Some((process.pid, priority.to_string()));
                            ui.close();
                        }
                    }
                });

                ui.menu_button("Set CPU Affinity ▸", |ui| {
                    let num_cores = data.cpu_cores.len().max(1);
                    let all_mask = if num_cores >= 64 {
                        usize::MAX
                    } else {
                        (1usize << num_cores) - 1
                    };
                    if ui.button("All Cores (Default)").clicked() {
                        app.affinity_change = Some((process.pid, all_mask));
                        ui.close();
                    }
                    if num_cores > 1 {
                        if ui.button("Core 0 Only (0x1)").clicked() {
                            app.affinity_change = Some((process.pid, 1));
                            ui.close();
                        }
                        if ui.button("Core 1 Only (0x2)").clicked() {
                            app.affinity_change = Some((process.pid, 2));
                            ui.close();
                        }
                        if num_cores >= 4 {
                            let half_mask = (1usize << (num_cores / 2)) - 1;
                            if ui.button(format!("First {} Cores", num_cores / 2)).clicked() {
                                app.affinity_change = Some((process.pid, half_mask));
                                ui.close();
                            }
                        }
                    }
                });

                ui.separator();

                if ui.button("📋 Copy PID").clicked() {
                    ui.ctx().copy_text(process.pid.to_string());
                    ui.close();
                }

                if ui.button("🔍 Inspect Details").clicked() {
                    app.details_pid = Some(process.pid);
                    ui.close();
                }
            })
            .response
            .on_hover_text("More actions (Priority, Affinity, Copy PID, Details)");
        },
    );
}
