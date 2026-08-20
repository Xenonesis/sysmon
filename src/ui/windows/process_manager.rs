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

            let is_dark = ui.visuals().dark_mode;
            let row_height = 26.0;

            if !app.process_tree_view {
                // Filter & Sort processes
                let mut filtered_processes = processes::filter_processes(&data.top_processes, &app.process_search);
                processes::sort_processes_refs(
                    &mut filtered_processes,
                    app.process_sort_column,
                    app.process_sort_ascending,
                );

                ui.label(format!(
                    "Showing {} of {} processes",
                    filtered_processes.len(),
                    data.top_processes.len()
                ));
                ui.add_space(4.0);

                // Sticky Header with sortable columns
                let sort_col = app.process_sort_column;
                let sort_asc = app.process_sort_ascending;

                let header_btn =
                    |ui: &mut egui::Ui, label: &str, width: f32, col: ProcessSortColumn| -> egui::Response {
                        let arrow = if col == sort_col {
                            if sort_asc {
                                " ▲"
                            } else {
                                " ▼"
                            }
                        } else {
                            ""
                        };
                        let text = format!("{}{}", label, arrow);
                        let color = if col == sort_col {
                            ThemePalette::ACCENT_PRIMARY
                        } else {
                            ThemePalette::text_primary(is_dark)
                        };
                        let btn = egui::Button::new(egui::RichText::new(text).strong().size(11.5).color(color))
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE);
                        ui.add_sized([width, 22.0], btn)
                    };

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if header_btn(ui, "PID", 55.0, ProcessSortColumn::Pid).clicked() {
                        if app.process_sort_column == ProcessSortColumn::Pid {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Pid;
                            app.process_sort_ascending = true;
                        }
                    }
                    if header_btn(ui, "Process Name", 180.0, ProcessSortColumn::Name).clicked() {
                        if app.process_sort_column == ProcessSortColumn::Name {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Name;
                            app.process_sort_ascending = true;
                        }
                    }
                    if header_btn(ui, "Memory", 80.0, ProcessSortColumn::Memory).clicked() {
                        if app.process_sort_column == ProcessSortColumn::Memory {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Memory;
                            app.process_sort_ascending = false;
                        }
                    }
                    if header_btn(ui, "CPU %", 65.0, ProcessSortColumn::Cpu).clicked() {
                        if app.process_sort_column == ProcessSortColumn::Cpu {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Cpu;
                            app.process_sort_ascending = false;
                        }
                    }
                    if header_btn(ui, "Disk I/O", 90.0, ProcessSortColumn::Disk).clicked() {
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

                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);

                let num_rows = filtered_processes.len();
                egui::ScrollArea::both().auto_shrink([false, false]).show_rows(
                    ui,
                    row_height,
                    num_rows,
                    |ui, row_range| {
                        ui.spacing_mut().item_spacing.y = 0.0;

                        for idx in row_range {
                            let process = filtered_processes[idx];
                            let memory_mb = bytes_to_mb(process.memory);
                            let is_even = idx % 2 == 0;

                            let mut text_color = ThemePalette::text_primary(is_dark);
                            if memory_mb > 500.0 || process.cpu_usage > 20.0 {
                                text_color = ThemePalette::STATUS_CRITICAL;
                            } else if memory_mb > 200.0 || process.cpu_usage > 10.0 {
                                text_color = ThemePalette::STATUS_WARNING;
                            }

                            let (row_rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width().max(660.0), row_height),
                                egui::Sense::hover(),
                            );

                            if is_even {
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
                                    ui.add_sized(
                                        [180.0, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(&display_name)
                                                .monospace()
                                                .size(11.5)
                                                .color(text_color),
                                        ),
                                    );

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

                                    // Disk I/O
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
                                    ui.add_sized(
                                        [90.0, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(disk_str).monospace().size(11.0).color(text_color),
                                        ),
                                    );

                                    // Actions
                                    if ui
                                        .small_button(egui::RichText::new("Kill").color(ThemePalette::STATUS_CRITICAL))
                                        .on_hover_text("Kill Process")
                                        .clicked()
                                    {
                                        app.selected_process_pid = Some(process.pid);
                                    }

                                    let is_suspended = app.suspended_pids.contains(&process.pid);
                                    if is_suspended {
                                        if ui
                                            .small_button(
                                                egui::RichText::new("Resume").color(ThemePalette::STATUS_HEALTHY),
                                            )
                                            .on_hover_text("Resume Process")
                                            .clicked()
                                        {
                                            app.resume_process_pid = Some(process.pid);
                                        }
                                    } else if ui.small_button("Suspend").on_hover_text("Suspend Process").clicked() {
                                        app.suspend_process_pid = Some(process.pid);
                                    }

                                    // Options menu
                                    ui.menu_button("⚙", |ui| {
                                        ui.set_min_width(160.0);
                                        ui.label(egui::RichText::new(format!("PID {} Options", process.pid)).strong());
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
                                            }
                                        });
                                    })
                                    .response
                                    .on_hover_text("Set Priority or CPU Affinity");
                                });
                            });
                        }
                    },
                );
            } else {
                // ── Tree View Mode ──
                let parent_map: std::collections::HashMap<u32, u32> = data
                    .top_processes
                    .iter()
                    .filter_map(|process| process.parent_pid.map(|parent| (process.pid, parent)))
                    .collect();
                let tree = processes::build_tree(&parent_map);
                let tree_rows = processes::build_tree_rows(&data.top_processes, &tree, &app.process_search);

                ui.label(format!("Showing {} hierarchical processes in tree", tree_rows.len()));
                ui.add_space(4.0);

                // Header
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.add_sized(
                        [55.0, 22.0],
                        egui::Label::new(egui::RichText::new("PID").strong().size(11.5)),
                    );
                    ui.add_sized(
                        [220.0, 22.0],
                        egui::Label::new(egui::RichText::new("Process Hierarchy").strong().size(11.5)),
                    );
                    ui.add_sized(
                        [80.0, 22.0],
                        egui::Label::new(egui::RichText::new("Memory").strong().size(11.5)),
                    );
                    ui.add_sized(
                        [65.0, 22.0],
                        egui::Label::new(egui::RichText::new("CPU %").strong().size(11.5)),
                    );
                    ui.label(
                        egui::RichText::new("Actions")
                            .strong()
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                });

                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);

                let num_tree_rows = tree_rows.len();
                egui::ScrollArea::both().auto_shrink([false, false]).show_rows(
                    ui,
                    row_height,
                    num_tree_rows,
                    |ui, row_range| {
                        ui.spacing_mut().item_spacing.y = 0.0;

                        for idx in row_range {
                            let r = &tree_rows[idx];
                            let memory_mb = bytes_to_mb(r.process.memory);
                            let is_even = idx % 2 == 0;

                            let mut text_color = ThemePalette::text_primary(is_dark);
                            if memory_mb > 500.0 || r.process.cpu_usage > 20.0 {
                                text_color = ThemePalette::STATUS_CRITICAL;
                            } else if memory_mb > 200.0 || r.process.cpu_usage > 10.0 {
                                text_color = ThemePalette::STATUS_WARNING;
                            }

                            let (row_rect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width().max(660.0), row_height),
                                egui::Sense::hover(),
                            );

                            if is_even {
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

                                    ui.add_sized(
                                        [55.0, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(r.process.pid.to_string())
                                                .monospace()
                                                .size(11.5)
                                                .color(text_color),
                                        ),
                                    );

                                    let tree_label = format!("{}{}", r.prefix, r.process.name);
                                    let display_tree = if tree_label.chars().count() > 28 {
                                        let trunc: String = tree_label.chars().take(25).collect();
                                        format!("{}...", trunc)
                                    } else {
                                        tree_label
                                    };

                                    ui.add_sized(
                                        [220.0, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(display_tree)
                                                .monospace()
                                                .size(11.5)
                                                .color(text_color),
                                        ),
                                    );

                                    ui.add_sized(
                                        [80.0, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(format!("{:.1} MB", memory_mb))
                                                .monospace()
                                                .size(11.5)
                                                .color(text_color),
                                        ),
                                    );

                                    ui.add_sized(
                                        [65.0, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(format!("{:.1}%", r.process.cpu_usage))
                                                .monospace()
                                                .size(11.5)
                                                .color(text_color),
                                        ),
                                    );

                                    if ui
                                        .small_button(egui::RichText::new("Kill").color(ThemePalette::STATUS_CRITICAL))
                                        .on_hover_text("Kill Process")
                                        .clicked()
                                    {
                                        app.selected_process_pid = Some(r.process.pid);
                                    }

                                    let is_suspended = app.suspended_pids.contains(&r.process.pid);
                                    if is_suspended {
                                        if ui
                                            .small_button(
                                                egui::RichText::new("Resume").color(ThemePalette::STATUS_HEALTHY),
                                            )
                                            .on_hover_text("Resume Process")
                                            .clicked()
                                        {
                                            app.resume_process_pid = Some(r.process.pid);
                                        }
                                    } else if ui.small_button("Suspend").on_hover_text("Suspend Process").clicked() {
                                        app.suspend_process_pid = Some(r.process.pid);
                                    }

                                    ui.menu_button("⚙", |ui| {
                                        ui.set_min_width(160.0);
                                        ui.label(
                                            egui::RichText::new(format!("PID {} Options", r.process.pid)).strong(),
                                        );
                                        ui.separator();
                                        ui.menu_button("Set Priority ▸", |ui| {
                                            for priority in &["High", "AboveNormal", "Normal", "BelowNormal", "Idle"] {
                                                if ui.button(*priority).clicked() {
                                                    app.priority_change = Some((r.process.pid, priority.to_string()));
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
                                            }
                                        });
                                    })
                                    .response
                                    .on_hover_text("Set Priority or CPU Affinity");
                                });
                            });
                        }
                    },
                );
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
