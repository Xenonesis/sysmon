use crate::processes::ProcessSortColumn;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(super) fn paint_process_table(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    filtered_processes: &[&crate::processes::ProcessInfo],
    data: &SystemData,
    is_dark: bool,
) {
    card_frame(is_dark).show(ui, |ui| {
        // Sticky Header with sortable columns
        let sort_col = app.process_sort_column;
        let sort_asc = app.process_sort_ascending;

        let header_button = |ui: &mut egui::Ui,
                             label: &str,
                             width: f32,
                             col: ProcessSortColumn,
                             current_col: ProcessSortColumn,
                             asc: bool|
         -> egui::Response {
            let text = super::sort_header_label(label, col, current_col, asc);
            let is_active = col == current_col;
            let text_color = if is_active {
                ThemePalette::ACCENT_PRIMARY
            } else {
                ThemePalette::text_primary(is_dark)
            };
            let btn = egui::Button::new(egui::RichText::new(text).strong().size(11.5).color(text_color))
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE);
            ui.add_sized([width, 22.0], btn)
        };

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            if header_button(ui, "PID", 55.0, ProcessSortColumn::Pid, sort_col, sort_asc).clicked() {
                if app.process_sort_column == ProcessSortColumn::Pid {
                    app.process_sort_ascending = !app.process_sort_ascending;
                } else {
                    app.process_sort_column = ProcessSortColumn::Pid;
                    app.process_sort_ascending = true;
                }
            }

            if header_button(ui, "Process Name", 180.0, ProcessSortColumn::Name, sort_col, sort_asc).clicked() {
                if app.process_sort_column == ProcessSortColumn::Name {
                    app.process_sort_ascending = !app.process_sort_ascending;
                } else {
                    app.process_sort_column = ProcessSortColumn::Name;
                    app.process_sort_ascending = true;
                }
            }

            if header_button(ui, "Memory", 80.0, ProcessSortColumn::Memory, sort_col, sort_asc).clicked() {
                if app.process_sort_column == ProcessSortColumn::Memory {
                    app.process_sort_ascending = !app.process_sort_ascending;
                } else {
                    app.process_sort_column = ProcessSortColumn::Memory;
                    app.process_sort_ascending = false;
                }
            }

            if header_button(ui, "CPU %", 65.0, ProcessSortColumn::Cpu, sort_col, sort_asc).clicked() {
                if app.process_sort_column == ProcessSortColumn::Cpu {
                    app.process_sort_ascending = !app.process_sort_ascending;
                } else {
                    app.process_sort_column = ProcessSortColumn::Cpu;
                    app.process_sort_ascending = false;
                }
            }

            if header_button(ui, "Disk Read", 80.0, ProcessSortColumn::Disk, sort_col, sort_asc).clicked() {
                if app.process_sort_column == ProcessSortColumn::Disk {
                    app.process_sort_ascending = !app.process_sort_ascending;
                } else {
                    app.process_sort_column = ProcessSortColumn::Disk;
                    app.process_sort_ascending = false;
                }
            }

            if header_button(ui, "Disk Write", 80.0, ProcessSortColumn::Disk, sort_col, sort_asc).clicked() {
                if app.process_sort_column == ProcessSortColumn::Disk {
                    app.process_sort_ascending = !app.process_sort_ascending;
                } else {
                    app.process_sort_column = ProcessSortColumn::Disk;
                    app.process_sort_ascending = false;
                }
            }

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

        // Virtualized high-performance rows (renders only the visible ~15-20 rows)
        let row_height = 26.0;
        let num_rows = filtered_processes.len();

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, num_rows, |ui, row_range| {
                ui.spacing_mut().item_spacing.y = 0.0;

                for idx in row_range {
                    let process = filtered_processes[idx];
                    let memory_mb = bytes_to_mb(process.memory);

                    let mut text_color = ThemePalette::text_primary(is_dark);
                    if memory_mb > 500.0 || process.cpu_usage > 20.0 {
                        text_color = ThemePalette::STATUS_CRITICAL;
                    } else if memory_mb > 200.0 || process.cpu_usage > 10.0 {
                        text_color = ThemePalette::STATUS_WARNING;
                    }

                    let selected = app.details_pid == Some(process.pid);
                    let is_even = idx % 2 == 0;

                    let (row_rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width().max(680.0), row_height),
                        egui::Sense::hover(),
                    );

                    // Row background styling
                    if selected {
                        let sel_fill = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(16, 185, 129, 25)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(16, 185, 129, 35)
                        };
                        ui.painter().rect_filled(row_rect, egui::Rounding::same(3.0), sel_fill);
                    } else if is_even {
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

                            // PID
                            ui.add_sized(
                                [55.0, row_height],
                                egui::Label::new(
                                    egui::RichText::new(process.pid.to_string())
                                        .monospace()
                                        .size(11.5)
                                        .color(text_color),
                                ),
                            );

                            // Process Name
                            let display_name = if process.name.chars().count() > 22 {
                                let truncated: String = process.name.chars().take(19).collect();
                                format!("{}...", truncated)
                            } else {
                                process.name.clone()
                            };

                            let name_btn = egui::Button::new(
                                egui::RichText::new(&display_name)
                                    .monospace()
                                    .size(11.5)
                                    .color(text_color),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE);

                            if ui
                                .add_sized([180.0, row_height], name_btn)
                                .on_hover_text(format!("Click to inspect {}\nPID: {}", process.name, process.pid))
                                .clicked()
                            {
                                if app.details_pid == Some(process.pid) {
                                    app.details_pid = None;
                                } else {
                                    app.details_pid = Some(process.pid);
                                }
                            }

                            // Memory
                            ui.add_sized(
                                [80.0, row_height],
                                egui::Label::new(
                                    egui::RichText::new(format!("{:.1} MB", memory_mb))
                                        .monospace()
                                        .size(11.5)
                                        .color(text_color),
                                ),
                            );

                            // CPU %
                            ui.add_sized(
                                [65.0, row_height],
                                egui::Label::new(
                                    egui::RichText::new(format!("{:.1}%", process.cpu_usage))
                                        .monospace()
                                        .size(11.5)
                                        .color(text_color),
                                ),
                            );

                            // Disk Read & Write
                            let refresh_interval = app.settings.refresh_interval.max(1);
                            let effective_elapsed = if refresh_interval > 0 {
                                refresh_interval as f64
                            } else {
                                1.0
                            };
                            let read_rate_mb = process.disk_read_bytes as f64 / effective_elapsed / 1024.0 / 1024.0;
                            let write_rate_mb = process.disk_written_bytes as f64 / effective_elapsed / 1024.0 / 1024.0;

                            ui.add_sized(
                                [80.0, row_height],
                                egui::Label::new(
                                    egui::RichText::new(format!("{:.2} MB/s", read_rate_mb))
                                        .monospace()
                                        .size(11.0)
                                        .color(text_color),
                                ),
                            );

                            ui.add_sized(
                                [80.0, row_height],
                                egui::Label::new(
                                    egui::RichText::new(format!("{:.2} MB/s", write_rate_mb))
                                        .monospace()
                                        .size(11.0)
                                        .color(text_color),
                                ),
                            );

                            // Action buttons: Kill, Tree, Suspend/Resume, Menu
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
                                            ui.close_menu();
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
                                });

                                ui.separator();

                                if ui.button("📋 Copy PID").clicked() {
                                    ui.output_mut(|o| o.copied_text = process.pid.to_string());
                                    ui.close_menu();
                                }

                                if ui.button("🔍 Inspect Details").clicked() {
                                    app.details_pid = Some(process.pid);
                                    ui.close_menu();
                                }
                            })
                            .response
                            .on_hover_text("More actions (Priority, Affinity, Copy PID, Details)");
                        });
                    });
                }
            });
    });

    // Details panel for selected process in a card frame
    if let Some((pid, details)) = &data.selected_process_details {
        if app.details_pid == Some(*pid) {
            ui.add_space(12.0);
            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new(format!("Process Details — PID {}", pid))
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("× Close").clicked() {
                            app.details_pid = None;
                        }
                    });
                });
                ui.separator();
                egui::Grid::new("process_details_grid")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        details_row(ui, "Executable", details.exe_path.as_deref().unwrap_or("N/A"), is_dark);
                        details_row(ui, "Command Line", &details.command_line, is_dark);
                        details_row(
                            ui,
                            "Working Directory",
                            details.cwd.as_deref().unwrap_or("N/A"),
                            is_dark,
                        );
                        details_row(ui, "Started", &format_started(details.start_time), is_dark);
                        details_row(
                            ui,
                            "Run Time",
                            &format!("{}m {}s", details.run_time / 60, details.run_time % 60),
                            is_dark,
                        );
                        details_row(
                            ui,
                            "Parent PID",
                            &details
                                .parent_pid
                                .map(|p| p.to_string())
                                .unwrap_or_else(|| "—".to_string()),
                            is_dark,
                        );
                        details_row(
                            ui,
                            "Parent Name",
                            details.parent_name.as_deref().unwrap_or("—"),
                            is_dark,
                        );
                    });
            });
        }
    }
}
