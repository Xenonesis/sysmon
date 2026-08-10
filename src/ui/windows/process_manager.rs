use crate::*;
use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
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
                        if ui.button("🔄 Refresh").on_hover_text("Data updates automatically from the monitoring thread").clicked() {
                            ui.ctx().request_repaint();
                        }
                    });
                });
                // Search box
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.add(egui::TextEdit::singleline(&mut app.process_search)
                        .hint_text("Filter by name or PID…")
                        .desired_width(200.0));
                    if ui.button("x").clicked() {
                        app.process_search.clear();
                    }
                });

                ui.add_space(5.0);

                // Filter & Sort processes
                let mut filtered_processes = processes::filter_processes(&data.top_processes, &app.process_search);
                processes::sort_processes(&mut filtered_processes, app.process_sort_column, app.process_sort_ascending);

                ui.label(format!("Showing {} of {} processes", filtered_processes.len(), data.top_processes.len()));
                ui.add_space(5.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("process_manager_grid")
                        .striped(true)
                        .spacing([10.0, 4.0])
                        .min_col_width(60.0)
                        .show(ui, |ui| {
                            let sort_arrow = |col: ProcessSortColumn, current: ProcessSortColumn, asc: bool| -> &'static str {
                                if col == current {
                                    if asc { " ^" } else { " v" }
                                } else {
                                    ""
                                }
                            };
                            let sort_col = app.process_sort_column;
                            let sort_asc = app.process_sort_ascending;

                            if ui.button(format!("PID{}", sort_arrow(ProcessSortColumn::Pid, sort_col, sort_asc))).clicked() {
                                if app.process_sort_column == ProcessSortColumn::Pid { app.process_sort_ascending = !app.process_sort_ascending; } else { app.process_sort_column = ProcessSortColumn::Pid; app.process_sort_ascending = true; }
                            }
                            if ui.button(format!("Process Name{}", sort_arrow(ProcessSortColumn::Name, sort_col, sort_asc))).clicked() {
                                if app.process_sort_column == ProcessSortColumn::Name { app.process_sort_ascending = !app.process_sort_ascending; } else { app.process_sort_column = ProcessSortColumn::Name; app.process_sort_ascending = true; }
                            }
                            if ui.button(format!("Memory{}", sort_arrow(ProcessSortColumn::Memory, sort_col, sort_asc))).clicked() {
                                if app.process_sort_column == ProcessSortColumn::Memory { app.process_sort_ascending = !app.process_sort_ascending; } else { app.process_sort_column = ProcessSortColumn::Memory; app.process_sort_ascending = false; }
                            }
                            if ui.button(format!("CPU %{}", sort_arrow(ProcessSortColumn::Cpu, sort_col, sort_asc))).clicked() {
                                if app.process_sort_column == ProcessSortColumn::Cpu { app.process_sort_ascending = !app.process_sort_ascending; } else { app.process_sort_column = ProcessSortColumn::Cpu; app.process_sort_ascending = false; }
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

                                ui.horizontal(|ui| {
                                    if ui.small_button("Kill").on_hover_text("Kill Process").clicked() {
                                        app.selected_process_pid = Some(process.pid);
                                    }
                                    let is_suspended = app.suspended_pids.contains(&process.pid);
                                    if is_suspended {
                                        if ui.small_button("Resume").on_hover_text("Resume Process").clicked() {
                                            app.resume_process_pid = Some(process.pid);
                                        }
                                    } else {
                                        if ui.small_button("Suspend").on_hover_text("Suspend Process").clicked() {
                                            app.suspend_process_pid = Some(process.pid);
                                        }
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
                                });

                                ui.end_row();
                            }
                        });
                });

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
