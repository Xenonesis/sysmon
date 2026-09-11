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
                        .on_hover_text("Acquire a fresh, complete process inventory")
                        .clicked()
                    {
                        request_refresh(app);
                    }
                });
            });
            let age = data
                .metric_status
                .get("processes")
                .and_then(|status| status.observed_at)
                .and_then(|time| time.elapsed().ok());
            let fresh = !data.monitoring_paused
                && data
                    .metric_status
                    .get("processes")
                    .is_some_and(|status| status.is_fresh())
                && age.is_some_and(|age| age.as_secs() <= app.settings.refresh_interval.saturating_mul(3).max(15));
            if !fresh {
                ui.colored_label(
                    ThemePalette::STATUS_WARNING,
                    if data.monitoring_paused {
                        "Paused — displaying the last process inventory. Refresh acquires a new inventory."
                    } else {
                        "Process inventory is stale or unavailable. Refresh to acquire current processes."
                    },
                );
            }
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
                            if sort_asc { " ▲" } else { " ▼" }
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
                    if header_btn(ui, "VRAM", 70.0, ProcessSortColumn::Vram).clicked() {
                        if app.process_sort_column == ProcessSortColumn::Vram {
                            app.process_sort_ascending = !app.process_sort_ascending;
                        } else {
                            app.process_sort_column = ProcessSortColumn::Vram;
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
                                egui::vec2(ui.available_width().max(730.0), row_height),
                                egui::Sense::hover(),
                            );

                            if is_even {
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

                                    // VRAM
                                    let vram_str = match process.vram_bytes {
                                        Some(bytes) => format!("{:.1} MB", bytes_to_mb(bytes)),
                                        None => "—".to_string(),
                                    };
                                    ui.add_sized(
                                        [70.0, row_height],
                                        egui::Label::new(
                                            egui::RichText::new(vram_str).monospace().size(11.5).color(text_color),
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

                                    paint_actions(app, ui, process);
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
                                    .rect_filled(row_rect, egui::CornerRadius::same(2), stripe_fill);
                            }

                            ui.new_child(egui::UiBuilder::new().max_rect(row_rect)).scope(|ui| {
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

                                    paint_actions(app, ui, &r.process);
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

fn request_refresh(app: &mut SystemMonitorApp) {
    if let Err(error) = app
        .app_channels
        .monitoring_sender
        .send(crate::app::commands::MonitoringCommand::RefreshNow)
    {
        app.action_status = Some(format!("Process refresh could not be requested: {error}"));
    }
}

fn paint_actions(app: &mut SystemMonitorApp, ui: &mut egui::Ui, process: &processes::ProcessInfo) {
    let Some(identity) = process.identity else {
        ui.add_enabled(false, egui::Button::new("Unavailable"))
            .on_hover_text("Process creation identity could not be verified; mutations are disabled");
        return;
    };
    ui.push_id(identity, |ui| {
        if ui
            .small_button(egui::RichText::new("Kill").color(ThemePalette::STATUS_CRITICAL))
            .on_hover_text("Kill Process")
            .clicked()
        {
            app.selected_process_pid = Some(identity);
        }
        if app.suspended_pids.contains(&identity) {
            if ui
                .small_button(egui::RichText::new("Resume").color(ThemePalette::STATUS_HEALTHY))
                .on_hover_text("Resume Process")
                .clicked()
            {
                app.resume_process_pid = Some(identity);
            }
        } else if ui.small_button("Suspend").on_hover_text("Suspend Process").clicked() {
            app.suspend_process_pid = Some(identity);
        }
        ui.menu_button("Options", |ui| {
            ui.set_min_width(160.0);
            ui.label(egui::RichText::new(format!("PID {} Options", identity.pid)).strong());
            ui.separator();
            ui.menu_button("Set Priority", |ui| {
                for priority in ["High", "AboveNormal", "Normal", "BelowNormal", "Idle"] {
                    if ui.button(priority).clicked() {
                        app.priority_change = Some((identity, priority.to_string()));
                        ui.close();
                    }
                }
            });
            ui.menu_button("Set CPU Affinity", |ui| {
                use crate::processes::AffinityPreset;
                for (label, preset) in [
                    ("All available cores", AffinityPreset::All),
                    ("First available core", AffinityPreset::First),
                    ("Second available core", AffinityPreset::Second),
                    ("First half of available cores", AffinityPreset::FirstHalf),
                ] {
                    if ui.button(label).clicked() {
                        app.affinity_change = Some((identity, preset));
                        ui.close();
                    }
                }
            });
        })
        .response
        .on_hover_text("Set Priority or CPU Affinity");
    });
}

const PICKER_CAPTURE_KEY: &str = "sysmon.window_picker.capture";

#[cfg(target_os = "windows")]
struct PickerCapture(usize);

#[cfg(target_os = "windows")]
impl Drop for PickerCapture {
    fn drop(&mut self) {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture};
        // Do not release capture belonging to another control/window.
        unsafe {
            if GetCapture() as usize == self.0 {
                ReleaseCapture();
            }
        }
    }
}

/// Call only from a primary-button drag_started response with Sense::drag().
pub(crate) fn begin_window_picker(app: &mut SystemMonitorApp, ctx: &egui::Context) {
    cancel_window_picker(app, ctx);
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            GetActiveWindow, GetAsyncKeyState, GetCapture, SetCapture, VK_LBUTTON,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
        unsafe {
            let hwnd = GetActiveWindow();
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            // A headless call must never capture another application's window.
            if hwnd.is_null() || pid != std::process::id() || GetAsyncKeyState(VK_LBUTTON as i32) >= 0 {
                app.action_status = Some("Hold the primary mouse button and drag Target from SysMon.".into());
                return;
            }
            SetCapture(hwnd);
            if GetCapture() != hwnd {
                app.action_status = Some("Windows could not capture the pointer for target picking.".into());
                return;
            }
            ctx.data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(PICKER_CAPTURE_KEY),
                    std::sync::Arc::new(PickerCapture(hwnd as usize)),
                );
            });
            app.window_picker_active = true;
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            ctx.request_repaint();
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        app.action_status = Some("Desktop window targeting is available on Windows only.".into());
    }
}

/// Release our capture on Escape, lost capture, shutdown, or explicit cancellation.
pub(crate) fn cancel_window_picker(app: &mut SystemMonitorApp, ctx: &egui::Context) {
    #[cfg(target_os = "windows")]
    ctx.data_mut(|data| {
        data.remove::<std::sync::Arc<PickerCapture>>(egui::Id::new(PICKER_CAPTURE_KEY));
    });
    app.window_picker_active = false;
    ctx.set_cursor_icon(egui::CursorIcon::Default);
}

/// Call every root logic tick, including when the toolbar/root window is not painted.
/// Native polling observes release and Escape even beyond the SysMon client area.
pub(crate) fn update_window_picker(app: &mut SystemMonitorApp, ctx: &egui::Context) {
    if !app.window_picker_active {
        return;
    }
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, GetCapture, VK_ESCAPE, VK_LBUTTON};
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let capture =
            ctx.data(|data| data.get_temp::<std::sync::Arc<PickerCapture>>(egui::Id::new(PICKER_CAPTURE_KEY)));
        let Some(capture) = capture else {
            cancel_window_picker(app, ctx);
            return;
        };
        let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape))
            || unsafe { GetAsyncKeyState(VK_ESCAPE as i32) < 0 };
        let pressed = unsafe { GetAsyncKeyState(VK_LBUTTON as i32) < 0 };
        if escape || (pressed && unsafe { GetCapture() as usize != capture.0 }) {
            // Drop the local Arc before removing the final owner.
            drop(capture);
            cancel_window_picker(app, ctx);
            app.action_status = Some("Window targeting cancelled.".into());
            return;
        }
        if pressed {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            return;
        }
        let mut point = POINT { x: 0, y: 0 };
        let cursor_ok = unsafe { GetCursorPos(&mut point) != 0 };
        drop(capture);
        cancel_window_picker(app, ctx);
        let identity = if cursor_ok {
            processes::get_process_id_from_screen_point(point.x, point.y)
                .ok_or_else(|| "No window process found at the pointer.".to_string())
                .and_then(processes::process_identity)
        } else {
            Err("Windows could not read the pointer position.".into())
        };
        match identity {
            Ok(identity) => {
                app.selected_tab = Tab::Processes;
                app.details_pid = Some(identity);
                app.process_search = identity.pid.to_string();
                app.action_status = Some(format!(
                    "Targeted window at ({}, {}): PID {} (creation {})",
                    point.x, point.y, identity.pid, identity.creation_time,
                ));
                request_refresh(app);
            }
            Err(error) => app.action_status = Some(format!("Window targeting failed: {error}")),
        }
    }
    #[cfg(not(target_os = "windows"))]
    cancel_window_picker(app, ctx);
}
