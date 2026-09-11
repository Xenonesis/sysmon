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
        let show_gpu = app.settings.show_gpu;
        let minimum_width = if show_gpu { 976.0 } else { 893.0 };
        let total_w = ui.available_width().max(minimum_width);
        egui::ScrollArea::horizontal()
            .id_salt("process_columns")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(total_w);
                let spacing = 8.0;
                let pid_w = 60.0;
                let mem_w = 85.0;
                let vram_w = if show_gpu { 75.0 } else { 0.0 };
                let cpu_w = 70.0;
                let disk_read_w = 85.0;
                let disk_write_w = 85.0;
                let action_w = 175.0;

                let fixed_w = pid_w
                    + mem_w
                    + vram_w
                    + cpu_w
                    + disk_read_w
                    + disk_write_w
                    + action_w
                    + (if show_gpu { 7.0 } else { 6.0 } * spacing);
                let name_w = (total_w - fixed_w).max(180.0);

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
                        .selected(is_active)
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE);
                    ui.add_sized([width, 22.0], btn)
                };

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = spacing;

                    if header_button(ui, "PID", pid_w, ProcessSortColumn::Pid, sort_col, sort_asc).clicked() {
                        if app.process_sort_column == ProcessSortColumn::Pid {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Pid;
                            app.process_sort_ascending = true;
                        }
                    }

                    if header_button(ui, "Process Name", name_w, ProcessSortColumn::Name, sort_col, sort_asc).clicked()
                    {
                        if app.process_sort_column == ProcessSortColumn::Name {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Name;
                            app.process_sort_ascending = true;
                        }
                    }

                    if header_button(ui, "Memory", mem_w, ProcessSortColumn::Memory, sort_col, sort_asc).clicked() {
                        if app.process_sort_column == ProcessSortColumn::Memory {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Memory;
                            app.process_sort_ascending = false;
                        }
                    }

                    if show_gpu
                        && header_button(ui, "VRAM", vram_w, ProcessSortColumn::Vram, sort_col, sort_asc).clicked()
                    {
                        if app.process_sort_column == ProcessSortColumn::Vram {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Vram;
                            app.process_sort_ascending = false;
                        }
                    }
                    if header_button(ui, "CPU %", cpu_w, ProcessSortColumn::Cpu, sort_col, sort_asc).clicked() {
                        if app.process_sort_column == ProcessSortColumn::Cpu {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Cpu;
                            app.process_sort_ascending = false;
                        }
                    }

                    if header_button(
                        ui,
                        "Disk Read",
                        disk_read_w,
                        ProcessSortColumn::Disk,
                        sort_col,
                        sort_asc,
                    )
                    .clicked()
                    {
                        if app.process_sort_column == ProcessSortColumn::Disk {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Disk;
                            app.process_sort_ascending = false;
                        }
                    }

                    if header_button(
                        ui,
                        "Disk Write",
                        disk_write_w,
                        ProcessSortColumn::Disk,
                        sort_col,
                        sort_asc,
                    )
                    .clicked()
                    {
                        if app.process_sort_column == ProcessSortColumn::Disk {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Disk;
                            app.process_sort_ascending = false;
                        }
                    }

                    ui.allocate_ui_with_layout(
                        egui::vec2(action_w, 22.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new("Actions")
                                    .strong()
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                        },
                    );
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                // Virtualized high-performance rows (renders only the visible ~15-20 rows)
                let row_height = 26.0;
                let num_rows = filtered_processes.len();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(460.0)
                    .show_rows(ui, row_height, num_rows, |ui, row_range| {
                        ui.spacing_mut().item_spacing.y = 0.0;

                        for idx in row_range {
                            let process = filtered_processes[idx];
                            let memory_mb = bytes_to_mb(process.memory);

                            let selected = process.identity.is_some() && app.details_pid == process.identity;
                            let is_even = idx % 2 == 0;

                            let (row_rect, _) =
                                ui.allocate_exact_size(egui::vec2(total_w, row_height), egui::Sense::hover());

                            // Row background styling
                            if selected {
                                let sel_fill = if is_dark {
                                    egui::Color32::from_rgba_unmultiplied(16, 185, 129, 25)
                                } else {
                                    egui::Color32::from_rgba_unmultiplied(16, 185, 129, 35)
                                };
                                ui.painter()
                                    .rect_filled(row_rect, egui::CornerRadius::same(3), sel_fill);
                            } else if is_even {
                                let stripe_fill = if is_dark {
                                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 3)
                                } else {
                                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 3)
                                };
                                ui.painter()
                                    .rect_filled(row_rect, egui::CornerRadius::same(2), stripe_fill);
                            }

                            ui.new_child(egui::UiBuilder::new().max_rect(row_rect)).scope(|ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = spacing;

                                    // PID
                                    ui.add_sized(
                                        [pid_w, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(process.pid.to_string())
                                                .monospace()
                                                .size(11.5)
                                                .color(ThemePalette::text_primary(is_dark)),
                                        ),
                                    );

                                    // Process Name (Dynamic width with responsive truncation)

                                    let name_btn = egui::Button::new(
                                        egui::RichText::new(&process.name)
                                            .monospace()
                                            .size(11.5)
                                            .color(if selected {
                                                ThemePalette::ACCENT_PRIMARY
                                            } else {
                                                ThemePalette::text_primary(is_dark)
                                            }),
                                    )
                                    .selected(selected)
                                    .truncate()
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(egui::Stroke::NONE);

                                    if ui
                                        .add_enabled_ui(process.identity.is_some(), |ui| {
                                            ui.add_sized([name_w, row_height], name_btn)
                                        })
                                        .inner
                                        .on_hover_text(format!(
                                            "Click to inspect {}\nPID: {}\nStatus: {}",
                                            process.name, process.pid, process.status
                                        ))
                                        .clicked()
                                    {
                                        if app.details_pid == process.identity {
                                            app.details_pid = None;
                                        } else {
                                            app.details_pid = process.identity;
                                        }
                                    }

                                    // Memory (Semantic highlighting only on the value)
                                    let mem_color = if memory_mb > 1024.0 {
                                        ThemePalette::STATUS_CRITICAL
                                    } else if memory_mb > 400.0 {
                                        ThemePalette::STATUS_WARNING
                                    } else {
                                        ThemePalette::text_primary(is_dark)
                                    };

                                    ui.add_sized(
                                        [mem_w, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(format!("{:.1} MB", memory_mb))
                                                .monospace()
                                                .size(11.5)
                                                .color(mem_color),
                                        ),
                                    );

                                    // CPU % (Semantic highlighting only on the value)
                                    let cpu_color = if process.cpu_usage > 50.0 {
                                        ThemePalette::STATUS_CRITICAL
                                    } else if process.cpu_usage > 15.0 {
                                        ThemePalette::STATUS_WARNING
                                    } else if process.cpu_usage > 0.05 {
                                        ThemePalette::text_primary(is_dark)
                                    } else {
                                        ThemePalette::text_dimmed(is_dark)
                                    };

                                    // VRAM
                                    if show_gpu {
                                        let (vram_label, vram_color) = match process.vram_bytes {
                                            Some(bytes) => {
                                                let vram_mb = bytes_to_mb(bytes);
                                                let color = if vram_mb > 2048.0 {
                                                    ThemePalette::STATUS_CRITICAL
                                                } else if vram_mb > 512.0 {
                                                    ThemePalette::STATUS_WARNING
                                                } else if vram_mb > 0.0 {
                                                    ThemePalette::text_primary(is_dark)
                                                } else {
                                                    ThemePalette::text_dimmed(is_dark)
                                                };
                                                (format!("{:.1} MB", vram_mb), color)
                                            }
                                            None => ("-".to_string(), ThemePalette::text_dimmed(is_dark)),
                                        };

                                        ui.add_sized(
                                            [vram_w, row_height],
                                            egui::Label::new(
                                                egui::RichText::new(vram_label)
                                                    .monospace()
                                                    .size(11.5)
                                                    .color(vram_color),
                                            ),
                                        );
                                    }

                                    ui.add_sized(
                                        [cpu_w, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(format!("{:.1}%", process.cpu_usage))
                                                .monospace()
                                                .size(11.5)
                                                .color(cpu_color),
                                        ),
                                    );

                                    // Disk Read & Write
                                    let rate_display = |rate: Option<f64>| match rate {
                                        Some(bytes_per_second)
                                            if bytes_per_second.is_finite() && bytes_per_second >= 0.0 =>
                                        {
                                            (
                                                crate::ui::components::format_rate(bytes_per_second),
                                                if bytes_per_second > 10.0 * 1024.0 * 1024.0 {
                                                    ThemePalette::STATUS_CRITICAL
                                                } else {
                                                    ThemePalette::text_primary(is_dark)
                                                },
                                            )
                                        }
                                        _ => ("Unavailable".to_string(), ThemePalette::text_dimmed(is_dark)),
                                    };
                                    let (read_label, read_color) = rate_display(process.disk_read_bytes_per_second);

                                    ui.add_sized(
                                        [disk_read_w, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(read_label).monospace().size(11.0).color(read_color),
                                        ),
                                    );

                                    let (write_label, write_color) =
                                        rate_display(process.disk_written_bytes_per_second);

                                    ui.add_sized(
                                        [disk_write_w, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(write_label)
                                                .monospace()
                                                .size(11.0)
                                                .color(write_color),
                                        ),
                                    );

                                    super::row_actions::paint_row_actions(
                                        app, ui, process, data, is_dark, row_height, action_w,
                                    );
                                });
                            });
                        }
                    });
            });
    });

    // Details panel for selected process in a card frame
    if let Some((pid, details)) = &data.selected_process_details
        && app.details_pid == Some(*pid)
    {
        ui.add_space(12.0);
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading(
                    egui::RichText::new(format!("Process Details — PID {}", pid.pid))
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
