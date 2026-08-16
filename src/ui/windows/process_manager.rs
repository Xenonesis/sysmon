use crate::processes::ProcessSortColumn;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ctx: &egui::Context, data: &SystemData) {
    let mut show = app.show_process_manager;

    egui::Window::new("Process Manager")
        .open(&mut show)
        .resizable(true)
        .default_width(800.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            ui.heading("Running Processes");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label(format!("Total processes: {}", data.top_processes.len()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("🔄 Refresh")
                        .on_hover_text("Data updates automatically from the monitoring thread")
                        .clicked()
                    {
                        ui.ctx().request_repaint();
                    }
                });
            });
            // Toolbar: Search box & Tree View Mode Toggle
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.process_search)
                        .hint_text("Filter by name or PID…")
                        .desired_width(200.0),
                );
                if ui.button("x").clicked() {
                    app.process_search.clear();
                }

                ui.add_space(10.0);

                // Tree vs List Toggle
                let is_tree = app.process_tree_view;
                let list_btn = egui::Button::new(egui::RichText::new("☰ Flat List").size(11.0).strong().color(
                    if !is_tree {
                        ThemePalette::ACCENT_PRIMARY
                    } else {
                        ThemePalette::TEXT_LABEL
                    },
                ));
                let tree_btn = egui::Button::new(egui::RichText::new("🌲 Process Tree").size(11.0).strong().color(
                    if is_tree {
                        ThemePalette::ACCENT_PRIMARY
                    } else {
                        ThemePalette::TEXT_LABEL
                    },
                ));

                if ui.add(list_btn).on_hover_text("View sorted flat list").clicked() {
                    app.process_tree_view = false;
                }
                if ui
                    .add(tree_btn)
                    .on_hover_text("View parent-child hierarchy tree")
                    .clicked()
                {
                    app.process_tree_view = true;
                }
            });

            ui.add_space(5.0);

            if !app.process_tree_view {
                // Filter & Sort processes
                let mut filtered_processes = processes::filter_processes(&data.top_processes, &app.process_search);
                processes::sort_processes(
                    &mut filtered_processes,
                    app.process_sort_column,
                    app.process_sort_ascending,
                );

                ui.label(format!(
                    "Showing {} of {} processes",
                    filtered_processes.len(),
                    data.top_processes.len()
                ));
                ui.add_space(5.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("process_manager_grid")
                        .striped(true)
                        .spacing([10.0, 4.0])
                        .min_col_width(60.0)
                        .show(ui, |ui| {
                            let sort_arrow =
                                |col: ProcessSortColumn, current: ProcessSortColumn, asc: bool| -> &'static str {
                                    if col == current {
                                        if asc {
                                            " ^"
                                        } else {
                                            " v"
                                        }
                                    } else {
                                        ""
                                    }
                                };
                            let sort_col = app.process_sort_column;
                            let sort_asc = app.process_sort_ascending;

                            if ui
                                .button(format!("PID{}", sort_arrow(ProcessSortColumn::Pid, sort_col, sort_asc)))
                                .clicked()
                            {
                                if app.process_sort_column == ProcessSortColumn::Pid {
                                    app.process_sort_ascending = !app.process_sort_ascending;
                                } else {
                                    app.process_sort_column = ProcessSortColumn::Pid;
                                    app.process_sort_ascending = true;
                                }
                            }
                            if ui
                                .button(format!(
                                    "Process Name{}",
                                    sort_arrow(ProcessSortColumn::Name, sort_col, sort_asc)
                                ))
                                .clicked()
                            {
                                if app.process_sort_column == ProcessSortColumn::Name {
                                    app.process_sort_ascending = !app.process_sort_ascending;
                                } else {
                                    app.process_sort_column = ProcessSortColumn::Name;
                                    app.process_sort_ascending = true;
                                }
                            }
                            if ui
                                .button(format!(
                                    "Memory{}",
                                    sort_arrow(ProcessSortColumn::Memory, sort_col, sort_asc)
                                ))
                                .clicked()
                            {
                                if app.process_sort_column == ProcessSortColumn::Memory {
                                    app.process_sort_ascending = !app.process_sort_ascending;
                                } else {
                                    app.process_sort_column = ProcessSortColumn::Memory;
                                    app.process_sort_ascending = false;
                                }
                            }
                            if ui
                                .button(format!(
                                    "CPU %{}",
                                    sort_arrow(ProcessSortColumn::Cpu, sort_col, sort_asc)
                                ))
                                .clicked()
                            {
                                if app.process_sort_column == ProcessSortColumn::Cpu {
                                    app.process_sort_ascending = !app.process_sort_ascending;
                                } else {
                                    app.process_sort_column = ProcessSortColumn::Cpu;
                                    app.process_sort_ascending = false;
                                }
                            }
                            if ui
                                .button(format!(
                                    "Disk I/O{}",
                                    sort_arrow(ProcessSortColumn::Disk, sort_col, sort_asc)
                                ))
                                .clicked()
                            {
                                if app.process_sort_column == ProcessSortColumn::Disk {
                                    app.process_sort_ascending = !app.process_sort_ascending;
                                } else {
                                    app.process_sort_column = ProcessSortColumn::Disk;
                                    app.process_sort_ascending = false;
                                }
                            }
                            ui.strong("Actions");
                            ui.end_row();

                            // Processes
                            for process in &filtered_processes {
                                let memory_mb = bytes_to_mb(process.memory);
                                let memory_color = if memory_mb > 500.0 {
                                    ThemePalette::STATUS_CRITICAL
                                } else if memory_mb > 200.0 {
                                    ThemePalette::STATUS_WARNING
                                } else {
                                    ThemePalette::STATUS_HEALTHY
                                };

                                ui.label(process.pid.to_string());

                                // Safe truncation using char boundaries
                                let display_name = if process.name.chars().count() > 25 {
                                    let truncated: String = process.name.chars().take(22).collect();
                                    format!("{}...", truncated)
                                } else {
                                    process.name.clone()
                                };
                                ui.label(display_name);

                                ui.colored_label(memory_color, format!("{:.2} MB", memory_mb));
                                ui.label(format!("{:.1}%", process.cpu_usage));
                                ui.label(&process.status);
                                let disk_total = process.disk_read_bytes.saturating_add(process.disk_written_bytes);
                                let disk_str = if disk_total > 0 {
                                    format!(
                                        "R:{} W:{}",
                                        bytes_to_human(process.disk_read_bytes),
                                        bytes_to_human(process.disk_written_bytes)
                                    )
                                } else {
                                    "—".to_string()
                                };
                                ui.label(egui::RichText::new(disk_str).monospace().size(11.0));
                                ui.horizontal(|ui| {
                                    if ui.small_button("Kill").on_hover_text("Kill Process").clicked() {
                                        app.selected_process_pid = Some(process.pid);
                                    }
                                    let is_suspended = app.suspended_pids.contains(&process.pid);
                                    if is_suspended {
                                        if ui.small_button("Resume").on_hover_text("Resume Process").clicked() {
                                            app.resume_process_pid = Some(process.pid);
                                        }
                                    } else if ui.small_button("Suspend").on_hover_text("Suspend Process").clicked() {
                                        app.suspend_process_pid = Some(process.pid);
                                    }
                                    // Priority menu
                                    ui.menu_button("Priority", |ui| {
                                        ui.label("Set Priority:");
                                        for priority in &["High", "AboveNormal", "Normal", "BelowNormal", "Idle"] {
                                            if ui.button(*priority).clicked() {
                                                app.priority_change = Some((process.pid, priority.to_string()));
                                                ui.close_menu();
                                            }
                                        }
                                    })
                                    .response
                                    .on_hover_text("Set Priority");
                                    // Affinity menu
                                    ui.menu_button("Affinity", |ui| {
                                        ui.label("CPU Core Affinity:");
                                        let num_cores = data.cpu_cores.len().max(1);
                                        let all_mask = if num_cores >= 64 {
                                            usize::MAX
                                        } else {
                                            (1usize << num_cores) - 1
                                        };
                                        if ui.button("All Cores (Default)").clicked() {
                                            app.affinity_change = Some((process.pid, all_mask));
                                            ui.close_menu();
                                        }
                                        if num_cores > 1 {
                                            if ui.button("Core 0 Only (0x1)").clicked() {
                                                app.affinity_change = Some((process.pid, 1));
                                                ui.close_menu();
                                            }
                                            if ui.button("Core 1 Only (0x2)").clicked() {
                                                app.affinity_change = Some((process.pid, 2));
                                                ui.close_menu();
                                            }
                                            if num_cores >= 4 {
                                                let half_mask = (1usize << (num_cores / 2)) - 1;
                                                if ui.button(format!("First {} Cores", num_cores / 2)).clicked() {
                                                    app.affinity_change = Some((process.pid, half_mask));
                                                    ui.close_menu();
                                                }
                                            }
                                        }
                                    })
                                    .response
                                    .on_hover_text("Set CPU Core Affinity");
                                });

                                ui.end_row();
                            }
                        });
                });
            } else {
                // ── Tree View Mode ──
                let parent_map: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
                let tree = processes::build_tree(&parent_map);
                let tree_rows = processes::build_tree_rows(&data.top_processes, &tree, &app.process_search);

                ui.label(format!("Showing {} hierarchical processes in tree", tree_rows.len()));
                ui.add_space(5.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("process_manager_tree_grid")
                        .striped(true)
                        .spacing([10.0, 4.0])
                        .min_col_width(60.0)
                        .show(ui, |ui| {
                            ui.strong("PID");
                            ui.strong("Process Hierarchy");
                            ui.strong("Memory");
                            ui.strong("CPU %");
                            ui.strong("Status");
                            ui.strong("Actions");
                            ui.end_row();

                            for r in &tree_rows {
                                let memory_mb = bytes_to_mb(r.process.memory);
                                let memory_color = if memory_mb > 500.0 {
                                    ThemePalette::STATUS_CRITICAL
                                } else if memory_mb > 200.0 {
                                    ThemePalette::STATUS_WARNING
                                } else {
                                    ThemePalette::STATUS_HEALTHY
                                };

                                ui.label(r.process.pid.to_string());
                                let tree_label = format!("{}{}", r.prefix, r.process.name);
                                ui.label(egui::RichText::new(tree_label).monospace());
                                ui.colored_label(memory_color, format!("{:.2} MB", memory_mb));
                                ui.label(format!("{:.1}%", r.process.cpu_usage));
                                ui.label(&r.process.status);

                                ui.horizontal(|ui| {
                                    if ui.small_button("Kill").on_hover_text("Kill Process").clicked() {
                                        app.selected_process_pid = Some(r.process.pid);
                                    }
                                    let is_suspended = app.suspended_pids.contains(&r.process.pid);
                                    if is_suspended {
                                        if ui.small_button("Resume").on_hover_text("Resume Process").clicked() {
                                            app.resume_process_pid = Some(r.process.pid);
                                        }
                                    } else if ui.small_button("Suspend").on_hover_text("Suspend Process").clicked() {
                                        app.suspend_process_pid = Some(r.process.pid);
                                    }
                                    // Priority menu
                                    ui.menu_button("Priority", |ui| {
                                        ui.label("Set Priority:");
                                        for priority in &["High", "AboveNormal", "Normal", "BelowNormal", "Idle"] {
                                            if ui.button(*priority).clicked() {
                                                app.priority_change = Some((r.process.pid, priority.to_string()));
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                    // Affinity menu
                                    ui.menu_button("Affinity", |ui| {
                                        ui.label("CPU Core Affinity:");
                                        let num_cores = data.cpu_cores.len().max(1);
                                        let all_mask = if num_cores >= 64 {
                                            usize::MAX
                                        } else {
                                            (1usize << num_cores) - 1
                                        };
                                        if ui.button("All Cores (Default)").clicked() {
                                            app.affinity_change = Some((r.process.pid, all_mask));
                                            ui.close_menu();
                                        }
                                        if num_cores > 1 {
                                            if ui.button("Core 0 Only (0x1)").clicked() {
                                                app.affinity_change = Some((r.process.pid, 1));
                                                ui.close_menu();
                                            }
                                            if ui.button("Core 1 Only (0x2)").clicked() {
                                                app.affinity_change = Some((r.process.pid, 2));
                                                ui.close_menu();
                                            }
                                            if num_cores >= 4 {
                                                let half_mask = (1usize << (num_cores / 2)) - 1;
                                                if ui.button(format!("First {} Cores", num_cores / 2)).clicked() {
                                                    app.affinity_change = Some((r.process.pid, half_mask));
                                                    ui.close_menu();
                                                }
                                            }
                                        }
                                    });
                                });
                                ui.end_row();
                            }
                        });
                });
            }

            if let Some(status) = &app.action_status {
                ui.label(egui::RichText::new(status).small().color(ThemePalette::TEXT_LABEL));
            }
            ui.separator();
            ui.colored_label(
                egui::Color32::YELLOW,
                "Warning: Killing/suspending processes may cause system instability!",
            );
            if !app.suspended_pids.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 165, 0),
                    format!("{} process(s) suspended", app.suspended_pids.len()),
                );
            }
        });

    app.show_process_manager = show;
}
