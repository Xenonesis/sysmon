use super::details_panel::paint_process_details_card;
use super::table_header::{TableLayout, paint_table_header};
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
        let layout = TableLayout::compute(ui.available_width(), app.settings.show_gpu);
        egui::ScrollArea::horizontal()
            .id_salt("process_columns")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(layout.total_w);

                // Sticky Header with sortable columns
                paint_table_header(app, ui, &layout, is_dark);

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
                                ui.allocate_exact_size(egui::vec2(layout.total_w, row_height), egui::Sense::hover());

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
                                    ui.spacing_mut().item_spacing.x = layout.spacing;

                                    // PID
                                    ui.add_sized(
                                        [layout.pid_w, row_height],
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
                                            ui.add_sized([layout.name_w, row_height], name_btn)
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
                                        [layout.mem_w, row_height],
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
                                    if layout.show_gpu {
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
                                            [layout.vram_w, row_height],
                                            egui::Label::new(
                                                egui::RichText::new(vram_label)
                                                    .monospace()
                                                    .size(11.5)
                                                    .color(vram_color),
                                            ),
                                        );
                                    }

                                    ui.add_sized(
                                        [layout.cpu_w, row_height],
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
                                        [layout.disk_read_w, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(read_label).monospace().size(11.0).color(read_color),
                                        ),
                                    );

                                    let (write_label, write_color) =
                                        rate_display(process.disk_written_bytes_per_second);

                                    ui.add_sized(
                                        [layout.disk_write_w, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(write_label)
                                                .monospace()
                                                .size(11.0)
                                                .color(write_color),
                                        ),
                                    );

                                    super::row_actions::paint_row_actions(
                                        app,
                                        ui,
                                        process,
                                        data,
                                        is_dark,
                                        row_height,
                                        layout.action_w,
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
        paint_process_details_card(app, ui, pid, details, is_dark);
    }
}
