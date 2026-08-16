use crate::processes::ProcessSortColumn;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn sort_header_label(
    label: &str,
    col: ProcessSortColumn,
    current_col: ProcessSortColumn,
    asc: bool,
) -> String {
    if col == current_col {
        let arrow = if asc { " ▲" } else { " ▼" };
        format!("{}{}", label, arrow)
    } else {
        label.to_string()
    }
}

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Process Monitor", is_dark);

    // Filter and Sort processes upfront
    let mut filtered_processes = processes::filter_processes(&data.top_processes, &app.process_search);
    let ascending = app.process_sort_ascending;
    processes::sort_processes(&mut filtered_processes, app.process_sort_column, ascending);

    // ── Integrated Toolbar Container ──
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            // Search Input with integrated Clear button
            ui.label(
                egui::RichText::new("Search:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.process_search)
                    .hint_text("Filter by name or PID...")
                    .desired_width(220.0),
            );
            if !app.process_search.is_empty() && ui.small_button("×").on_hover_text("Clear search filter").clicked() {
                app.process_search.clear();
            }

            ui.add_space(8.0);

            // Process count badge
            let count_label = format!("Showing {} / {}", filtered_processes.len(), data.top_processes.len());
            status_pill(ui, &count_label, ThemePalette::ACCENT_PRIMARY, is_dark);

            // Right-aligned management actions
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui::RichText::new("Export JSON").strong())
                    .on_hover_text("Export current process list to JSON")
                    .clicked()
                {
                    app.show_export = true;
                }
                if ui
                    .button(egui::RichText::new("Export CSV").strong())
                    .on_hover_text("Export current process list to CSV")
                    .clicked()
                {
                    app.show_export_csv = true;
                }
                if ui
                    .button(egui::RichText::new("Full Process Manager").strong())
                    .on_hover_text("Open advanced window with Kill, Suspend & Priority controls")
                    .clicked()
                {
                    app.show_process_manager = true;
                }
            });
        });
    });

    ui.add_space(8.0);

    // ── Responsive Process Table ──
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        egui::Grid::new("full_process_grid")
            .striped(false)
            .spacing([12.0, 6.0])
            .min_col_width(70.0)
            .show(ui, |ui| {
                // Clickable sort headers with directional indicators
                let sort_col = app.process_sort_column;
                let sort_asc = app.process_sort_ascending;

                let header_button = |ui: &mut egui::Ui,
                                     label: &str,
                                     col: ProcessSortColumn,
                                     current_col: ProcessSortColumn,
                                     asc: bool|
                 -> egui::Response {
                    let text = sort_header_label(label, col, current_col, asc);
                    let is_active = col == current_col;
                    let text_color = if is_active {
                        ThemePalette::ACCENT_PRIMARY
                    } else {
                        ThemePalette::text_primary(is_dark)
                    };
                    ui.button(egui::RichText::new(text).strong().color(text_color))
                };

                if header_button(ui, "PID", ProcessSortColumn::Pid, sort_col, sort_asc).clicked() {
                    if app.process_sort_column == ProcessSortColumn::Pid {
                        app.process_sort_ascending = !app.process_sort_ascending;
                    } else {
                        app.process_sort_column = ProcessSortColumn::Pid;
                        app.process_sort_ascending = true;
                    }
                }
                if header_button(ui, "Process Name", ProcessSortColumn::Name, sort_col, sort_asc).clicked() {
                    if app.process_sort_column == ProcessSortColumn::Name {
                        app.process_sort_ascending = !app.process_sort_ascending;
                    } else {
                        app.process_sort_column = ProcessSortColumn::Name;
                        app.process_sort_ascending = true;
                    }
                }
                if header_button(ui, "Memory", ProcessSortColumn::Memory, sort_col, sort_asc).clicked() {
                    if app.process_sort_column == ProcessSortColumn::Memory {
                        app.process_sort_ascending = !app.process_sort_ascending;
                    } else {
                        app.process_sort_column = ProcessSortColumn::Memory;
                        app.process_sort_ascending = false; // default descending for memory
                    }
                }
                if header_button(ui, "CPU %", ProcessSortColumn::Cpu, sort_col, sort_asc).clicked() {
                    if app.process_sort_column == ProcessSortColumn::Cpu {
                        app.process_sort_ascending = !app.process_sort_ascending;
                    } else {
                        app.process_sort_column = ProcessSortColumn::Cpu;
                        app.process_sort_ascending = false; // default descending for CPU
                    }
                }
                if header_button(ui, "Disk Read", ProcessSortColumn::Disk, sort_col, sort_asc).clicked() {
                    if app.process_sort_column == ProcessSortColumn::Disk {
                        app.process_sort_ascending = !app.process_sort_ascending;
                    } else {
                        app.process_sort_column = ProcessSortColumn::Disk;
                        app.process_sort_ascending = false;
                    }
                }
                if header_button(ui, "Disk Write", ProcessSortColumn::Disk, sort_col, sort_asc).clicked() {
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
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.end_row();

                // Process data rows
                for process in &filtered_processes {
                    let memory_mb = bytes_to_mb(process.memory);

                    let mut text_color = ThemePalette::text_primary(is_dark);
                    if memory_mb > 500.0 || process.cpu_usage > 20.0 {
                        text_color = ThemePalette::STATUS_CRITICAL;
                    } else if memory_mb > 200.0 || process.cpu_usage > 10.0 {
                        text_color = ThemePalette::STATUS_WARNING;
                    }

                    // Monospace PID
                    ui.label(
                        egui::RichText::new(process.pid.to_string())
                            .monospace()
                            .color(text_color),
                    );

                    // Process name
                    let display_name = if process.name.chars().count() > 36 {
                        let truncated: String = process.name.chars().take(33).collect();
                        format!("{}...", truncated)
                    } else {
                        process.name.clone()
                    };
                    let selected = app.details_pid == Some(process.pid);
                    if ui
                        .selectable_label(
                            selected,
                            egui::RichText::new(display_name).monospace().color(text_color),
                        )
                        .on_hover_text("Click to toggle process details inspection")
                        .clicked()
                    {
                        if app.details_pid == Some(process.pid) {
                            app.details_pid = None;
                        } else {
                            app.details_pid = Some(process.pid);
                        }
                    }

                    // Monospace Memory & CPU
                    ui.label(
                        egui::RichText::new(format!("{:.2} MB", memory_mb))
                            .monospace()
                            .color(text_color),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.1}%", process.cpu_usage))
                            .monospace()
                            .color(text_color),
                    );

                    // Monospace Disk I/O Rates
                    let refresh_interval = app.settings.refresh_interval.max(1);
                    let effective_elapsed = if refresh_interval > 0 {
                        refresh_interval as f64
                    } else {
                        1.0
                    };
                    let read_rate_mb = process.disk_read_bytes as f64 / effective_elapsed / 1024.0 / 1024.0;
                    let write_rate_mb = process.disk_written_bytes as f64 / effective_elapsed / 1024.0 / 1024.0;
                    ui.label(
                        egui::RichText::new(format!("{:.2} MB/s", read_rate_mb))
                            .monospace()
                            .color(text_color),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.2} MB/s", write_rate_mb))
                            .monospace()
                            .color(text_color),
                    );

                    // Action buttons: Tree, Kill, Suspend/Resume, Priority, Copy PID
                    ui.horizontal(|ui| {
                        if ui
                            .small_button("Tree")
                            .on_hover_text("Kill this process and all its children (deepest first)")
                            .clicked()
                        {
                            app.kill_tree_pid = Some(process.pid);
                        }
                        if ui
                            .small_button(egui::RichText::new("Kill").color(ThemePalette::STATUS_CRITICAL))
                            .on_hover_text("Terminate this process (requires Admin for system processes)")
                            .clicked()
                        {
                            app.selected_process_pid = Some(process.pid);
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
                        .on_hover_text("Set process scheduling priority");
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
                        .on_hover_text("Set process CPU core affinity mask");
                        if ui
                            .small_button("Copy PID")
                            .on_hover_text("Copy PID to clipboard")
                            .clicked()
                        {
                            ui.output_mut(|o| o.copied_text = process.pid.to_string());
                        }
                    });

                    ui.end_row();
                }
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
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_header_label_indicators() {
        assert_eq!(
            sort_header_label("PID", ProcessSortColumn::Pid, ProcessSortColumn::Pid, true),
            "PID ▲"
        );
        assert_eq!(
            sort_header_label("PID", ProcessSortColumn::Pid, ProcessSortColumn::Pid, false),
            "PID ▼"
        );
        assert_eq!(
            sort_header_label("CPU %", ProcessSortColumn::Cpu, ProcessSortColumn::Memory, false),
            "CPU %"
        );
        assert_eq!(
            sort_header_label("Memory", ProcessSortColumn::Memory, ProcessSortColumn::Memory, false),
            "Memory ▼"
        );
    }
}
