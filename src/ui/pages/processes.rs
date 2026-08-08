use crate::*;
use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "Process Monitor");

        // Header action bar
        ui.horizontal(|ui| {
            if ui.button("⚙ Full Process Manager").on_hover_text("Open advanced window with Kill, Suspend & Priority controls").clicked() {
                app.show_process_manager = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📊 Export CSV").clicked() { app.show_export_csv = true; }
                if ui.button("📄 Export JSON").clicked() { app.show_export = true; }
            });
        });
        ui.add_space(4.0);

        // Search box
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.add(egui::TextEdit::singleline(&mut app.process_search).hint_text("Filter by name or PID (all processes)").desired_width(200.0));
            if ui.button("x").clicked() {
                app.process_search.clear();
            }
        });

        ui.add_space(5.0);

        // Filter processes
        let mut filtered_processes = processes::filter_processes(&data.top_processes, &app.process_search);

        // Sort processes
        let ascending = app.process_sort_ascending;
        processes::sort_processes(&mut filtered_processes, app.process_sort_column, ascending);

        ui.label(format!(
            "Showing {} of {} processes",
            filtered_processes.len(),
            data.top_processes.len()
        ));
        ui.add_space(5.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("full_process_grid")
                .striped(true)
                .spacing([10.0, 4.0])
                .min_col_width(80.0)
                .show(ui, |ui| {
                    // Clickable sort headers
                    let sort_arrow = |col: ProcessSortColumn, current: ProcessSortColumn, asc: bool| -> &'static str {
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
                            app.process_sort_ascending = false; // default descending for memory
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
                            app.process_sort_ascending = false; // default descending for CPU
                        }
                    }
                    ui.strong("Disk Read");
                    ui.strong("Disk Write");
                    ui.strong("Actions");
                    ui.end_row();

                    // Processes
                    for process in &filtered_processes {
                        let memory_mb = bytes_to_mb(process.memory);

                        let mut text_color = ui.visuals().text_color();
                        if memory_mb > 500.0 || process.cpu_usage > 20.0 {
                            text_color = ThemePalette::STATUS_CRITICAL;
                        } else if memory_mb > 200.0 || process.cpu_usage > 10.0 {
                            text_color = ThemePalette::STATUS_WARNING;
                        }

                        ui.label(egui::RichText::new(process.pid.to_string()).color(text_color));

                        let display_name = if process.name.chars().count() > 40 {
                            let truncated: String = process.name.chars().take(37).collect();
                            format!("{}...", truncated)
                        } else {
                            process.name.clone()
                        };
                        let selected = app.details_pid == Some(process.pid);
                        if ui
                            .selectable_label(selected, egui::RichText::new(display_name).color(text_color))
                            .on_hover_text("Click to view process details")
                            .clicked()
                        {
                            app.details_pid = Some(process.pid);
                        }

                        ui.label(egui::RichText::new(format!("{:.2} MB", memory_mb)).color(text_color));
                        ui.label(egui::RichText::new(format!("{:.1}%", process.cpu_usage)).color(text_color));

                        let refresh_interval = app.settings.refresh_interval.max(1);
                        let effective_elapsed = if refresh_interval > 0 { refresh_interval as f64 } else { 1.0 };
                        let read_rate_mb = process.disk_read_bytes as f64 / effective_elapsed / 1024.0 / 1024.0;
                        let write_rate_mb = process.disk_written_bytes as f64 / effective_elapsed / 1024.0 / 1024.0;
                        ui.label(egui::RichText::new(format!("{:.2} MB/s", read_rate_mb)).color(text_color));
                        ui.label(egui::RichText::new(format!("{:.2} MB/s", write_rate_mb)).color(text_color));

                        // Action buttons: Tree, Kill, Suspend/Resume, Priority, Copy PID
                        ui.horizontal(|ui| {
                            if ui.small_button("Tree").on_hover_text("Kill this process and all its children (deepest first)").clicked() {
                                app.kill_tree_pid = Some(process.pid);
                            }
                            if ui.small_button("Kill").on_hover_text("Terminate this process (requires Admin for system processes)").clicked() {
                                app.selected_process_pid = Some(process.pid);
                            }
                            let is_suspended = app.suspended_pids.contains(&process.pid);
                            if is_suspended {
                                if ui.small_button("Resume").on_hover_text("Resume suspended process").clicked() {
                                    app.resume_process_pid = Some(process.pid);
                                }
                            } else {
                                if ui.small_button("Suspend").on_hover_text("Freeze process execution (Windows only)").clicked() {
                                    app.suspend_process_pid = Some(process.pid);
                                }
                            }
                            ui.menu_button("Priority", |ui| {
                                ui.label("Set Priority:");
                                for priority in &["High", "AboveNormal", "Normal", "BelowNormal", "Idle"] {
                                    if ui.button(*priority).clicked() {
                                        app.priority_change = Some((process.pid, priority.to_string()));
                                        ui.close_menu();
                                    }
                                }
                            }).response.on_hover_text("Set process scheduling priority");
                            if ui.small_button("Copy PID").on_hover_text("Copy PID to clipboard").clicked() {
                                ui.output_mut(|o| o.copied_text = process.pid.to_string());
                            }
                        });

                        ui.end_row();
                    }
                });

                // Details panel for selected process
                if let Some((pid, details)) = &data.selected_process_details {
                    if app.details_pid == Some(*pid) {
                        ui.add_space(8.0);
                        ui.separator();
                        paint_section_header(ui, &format!("Process Details — PID {}", pid));
                        egui::Grid::new("process_details_grid")
                            .num_columns(2)
                            .spacing([16.0, 4.0])
                            .show(ui, |ui| {
                                details_row(ui, "Executable", details.exe_path.as_deref().unwrap_or("N/A"));
                                details_row(ui, "Command Line", &details.command_line);
                                details_row(ui, "Working Directory", details.cwd.as_deref().unwrap_or("N/A"));
                                details_row(ui, "Started", &format_started(details.start_time));
                                details_row(ui, "Run Time", &format!("{}m {}s", details.run_time / 60, details.run_time % 60));
                                details_row(ui, "Parent PID", &details.parent_pid.map(|p| p.to_string()).unwrap_or_else(|| "—".to_string()));
                                details_row(ui, "Parent Name", details.parent_name.as_deref().unwrap_or("—"));
                            });
                    }
                }
            });
    }
