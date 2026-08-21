use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Resolves semantic color for Windows service states.
pub(crate) fn service_state_color(state: &str, is_dark: bool) -> egui::Color32 {
    match state.to_lowercase().as_str() {
        "running" => ThemePalette::STATUS_HEALTHY,
        "stopped" => ThemePalette::text_dimmed(is_dark),
        "paused" | "start pending" | "stop pending" | "continue pending" | "pause pending" => {
            ThemePalette::STATUS_WARNING
        }
        _ => ThemePalette::text_secondary(is_dark),
    }
}

pub(crate) fn sort_header_label(
    label: &str,
    col: services::ServiceSortColumn,
    current_col: services::ServiceSortColumn,
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
    let is_elevated = privilege::is_app_elevated();

    // ── Section Header with Quick System Utilities ──
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Windows Services")
                .size(15.5)
                .strong()
                .color(ThemePalette::text_primary(is_dark)),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Live indicator
            ui.label(
                egui::RichText::new("LIVE")
                    .size(9.5)
                    .monospace()
                    .strong()
                    .color(ThemePalette::ACCENT_PRIMARY),
            );
            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(dot_rect.center(), 2.5, ThemePalette::ACCENT_PRIMARY);

            ui.add_space(8.0);

            // Open Windows Services MMC Console
            if ui
                .button(egui::RichText::new("⚙ services.msc").size(11.0).strong())
                .on_hover_text("Open Windows Services Management Console (services.msc)")
                .clicked()
            {
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("cmd")
                    .args(["/c", "start", "services.msc"])
                    .spawn();
            }
        });
    });

    ui.add_space(3.0);
    let full_w = ui.available_width();
    let (line_rect, _) = ui.allocate_exact_size(egui::vec2(full_w, 1.0), egui::Sense::hover());
    let accent_w = 48.0f32.min(full_w);
    let accent_rect = egui::Rect::from_min_size(line_rect.min, egui::vec2(accent_w, 1.0));
    let remainder_rect = egui::Rect::from_min_size(
        egui::pos2(line_rect.min.x + accent_w, line_rect.min.y),
        egui::vec2((full_w - accent_w).max(0.0), 1.0),
    );
    ui.painter()
        .rect_filled(accent_rect, egui::Rounding::ZERO, ThemePalette::ACCENT_PRIMARY);
    ui.painter()
        .rect_filled(remainder_rect, egui::Rounding::ZERO, ThemePalette::border(is_dark));
    ui.add_space(10.0);

    if data.services.is_empty() {
        ui.add_space(16.0);
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Loading Windows services telemetry...")
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        });
        return;
    }

    let total_services = data.services.len();
    let running_count = data
        .services
        .iter()
        .filter(|s| s.state.eq_ignore_ascii_case("running"))
        .count();
    let stopped_count = data
        .services
        .iter()
        .filter(|s| s.state.eq_ignore_ascii_case("stopped"))
        .count();
    let other_count = total_services.saturating_sub(running_count + stopped_count);
    let running_pct = if total_services > 0 {
        (running_count as f32 / total_services as f32) * 100.0
    } else {
        0.0
    };

    // ── KPI Telemetry Hero Deck ──
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;

            // Total Services Metric Pill
            status_pill(
                ui,
                &format!("{} TOTAL SERVICES", total_services),
                ThemePalette::ACCENT_PRIMARY,
                is_dark,
            );

            // Running Services Metric Pill
            status_pill(
                ui,
                &format!("{} RUNNING ({:.0}%)", running_count, running_pct),
                ThemePalette::STATUS_HEALTHY,
                is_dark,
            );

            // Stopped Services Metric Pill
            status_pill(
                ui,
                &format!("{} STOPPED", stopped_count),
                ThemePalette::text_dimmed(is_dark),
                is_dark,
            );

            if other_count > 0 {
                status_pill(
                    ui,
                    &format!("{} PENDING / PAUSED", other_count),
                    ThemePalette::STATUS_WARNING,
                    is_dark,
                );
            }

            // Elevation Status & Relaunch Action
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if is_elevated {
                    status_pill(ui, "ADMIN ELEVATED", ThemePalette::STATUS_HEALTHY, is_dark);
                } else {
                    #[cfg(target_os = "windows")]
                    if ui
                        .button(
                            egui::RichText::new("🛡 Elevate to Admin")
                                .size(11.0)
                                .strong()
                                .color(ThemePalette::STATUS_WARNING),
                        )
                        .on_hover_text("Relaunch System Monitor with Administrator privileges to enable service start/stop controls")
                        .clicked()
                    {
                        crate::privilege::relaunch_as_admin();
                    }

                    status_pill(
                        ui,
                        "ADMIN REQUIRED FOR CONTROL",
                        ThemePalette::STATUS_WARNING,
                        is_dark,
                    );
                }
            });
        });

        ui.add_space(8.0);

        // Segmented Status Distribution Ratio Bar
        let bar_h = 4.0;
        let bar_w = ui.available_width();
        let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
        ui.painter()
            .rect_filled(bar_rect, egui::Rounding::same(2.0), ThemePalette::bg_track(is_dark));

        if total_services > 0 {
            let run_w = (bar_w * (running_count as f32 / total_services as f32)).max(2.0);
            let run_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(run_w, bar_h));
            ui.painter().rect_filled(
                run_rect,
                egui::Rounding::same(2.0),
                ThemePalette::STATUS_HEALTHY,
            );

            if other_count > 0 {
                let other_w = bar_w * (other_count as f32 / total_services as f32);
                let other_rect = egui::Rect::from_min_size(
                    egui::pos2(bar_rect.min.x + run_w, bar_rect.min.y),
                    egui::vec2(other_w, bar_h),
                );
                ui.painter().rect_filled(
                    other_rect,
                    egui::Rounding::ZERO,
                    ThemePalette::STATUS_WARNING,
                );
            }
        }
    });

    ui.add_space(8.0);

    // ── Search & Filter Command Toolbar ──
    let query = app.service_search.to_lowercase();
    let mut filtered: Vec<&services::ServiceInfo> = data
        .services
        .iter()
        .filter(|svc| {
            let name_match = query.is_empty()
                || svc.name.to_lowercase().contains(&query)
                || svc.display_name.to_lowercase().contains(&query);
            let state_match = app
                .service_state_filter
                .as_deref()
                .is_none_or(|s| svc.state.eq_ignore_ascii_case(s));
            name_match && state_match
        })
        .collect();

    services::sort_services_refs(
        &mut filtered,
        app.service_sort_column,
        app.service_sort_ascending,
    );

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            // Search input
            ui.label(
                egui::RichText::new("🔍 Search:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.service_search)
                    .hint_text("Filter by name or identifier...")
                    .desired_width(240.0),
            );
            if !app.service_search.is_empty()
                && ui
                    .small_button("×")
                    .on_hover_text("Clear search filter")
                    .clicked()
            {
                app.service_search.clear();
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Filter Tabs
            ui.label(
                egui::RichText::new("State:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );

            let is_all = app.service_state_filter.is_none();
            if ui.selectable_label(is_all, format!("All ({total_services})")).clicked() {
                app.service_state_filter = None;
            }

            let is_run = app.service_state_filter.as_deref() == Some("Running");
            if ui.selectable_label(is_run, format!("Running ({running_count})")).clicked() {
                app.service_state_filter = if is_run { None } else { Some("Running".to_string()) };
            }

            let is_stop = app.service_state_filter.as_deref() == Some("Stopped");
            if ui.selectable_label(is_stop, format!("Stopped ({stopped_count})")).clicked() {
                app.service_state_filter = if is_stop { None } else { Some("Stopped".to_string()) };
            }

            let has_active_filter =
                !app.service_search.is_empty() || app.service_state_filter.is_some();
            if has_active_filter
                && ui
                    .small_button("Reset")
                    .on_hover_text("Reset search and state filters")
                    .clicked()
            {
                app.service_search.clear();
                app.service_state_filter = None;
            }

            // Showing count on right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let count_label = format!("Showing {} of {}", filtered.len(), total_services);
                ui.label(
                    egui::RichText::new(count_label)
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        });
    });

    ui.add_space(8.0);

    // ── Selected Service Inspector Drawer (If a service is selected) ──
    if let Some(ref sel_name) = app.selected_service_name.clone() {
        if let Some(svc) = data.services.iter().find(|s| &s.name == sel_name) {
            let is_running = svc.state.eq_ignore_ascii_case("running");
            let is_stopped = svc.state.eq_ignore_ascii_case("stopped");
            let state_c = service_state_color(&svc.state, is_dark);

            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("SERVICE INSPECTOR")
                            .size(11.0)
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );

                    status_pill(ui, &svc.state.to_uppercase(), state_c, is_dark);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("✕ Close")
                            .on_hover_text("Deselect service")
                            .clicked()
                        {
                            app.selected_service_name = None;
                        }
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(6.0);

                // Service Details Grid
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Display Name:")
                                    .strong()
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new(&svc.display_name)
                                    .strong()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Service Identifier:")
                                    .strong()
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new(&svc.name)
                                    .monospace()
                                    .color(ThemePalette::ACCENT_PRIMARY),
                            );
                        });
                    });

                    ui.add_space(24.0);

                    // Quick CLI Automation Snippets
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("CLI & Scripting Snippets:")
                                .size(10.5)
                                .strong()
                                .color(ThemePalette::text_secondary(is_dark)),
                        );

                        ui.horizontal(|ui| {
                            if ui
                                .small_button("📋 Copy Name")
                                .on_hover_text("Copy service identifier to clipboard")
                                .clicked()
                            {
                                ui.output_mut(|o| o.copied_text = svc.name.clone());
                            }

                            if ui
                                .small_button("📋 PowerShell: Get-Service")
                                .on_hover_text("Copy 'Get-Service -Name <svc>' command")
                                .clicked()
                            {
                                ui.output_mut(|o| {
                                    o.copied_text = format!("Get-Service -Name \"{}\"", svc.name)
                                });
                            }

                            if ui
                                .small_button("📋 PowerShell: Restart-Service")
                                .on_hover_text("Copy 'Restart-Service -Name <svc> -Force' command")
                                .clicked()
                            {
                                ui.output_mut(|o| {
                                    o.copied_text =
                                        format!("Restart-Service -Name \"{}\" -Force", svc.name)
                                });
                            }

                            if ui
                                .small_button("📋 SC Query")
                                .on_hover_text("Copy 'sc.exe query <svc>' command")
                                .clicked()
                            {
                                ui.output_mut(|o| {
                                    o.copied_text = format!("sc.exe query \"{}\"", svc.name)
                                });
                            }
                        });
                    });
                });

                ui.add_space(8.0);

                // Inspector Action Controls
                ui.horizontal(|ui| {
                    let tooltip = if !is_elevated {
                        "Administrator privileges required to control services"
                    } else {
                        "Execute service control action"
                    };

                    let start_btn = ui.add_enabled(
                        is_elevated && !is_running,
                        egui::Button::new(egui::RichText::new("▶ Start Service").strong()),
                    );
                    let stop_btn = ui.add_enabled(
                        is_elevated && !is_stopped,
                        egui::Button::new(
                            egui::RichText::new("⏹ Stop Service")
                                .strong()
                                .color(if is_elevated && !is_stopped {
                                    ThemePalette::STATUS_CRITICAL
                                } else {
                                    ThemePalette::text_dimmed(is_dark)
                                }),
                        ),
                    );
                    let restart_btn = ui.add_enabled(
                        is_elevated && is_running,
                        egui::Button::new(egui::RichText::new("🔄 Restart Service").strong()),
                    );

                    if start_btn.on_hover_text(tooltip).clicked() {
                        app.pending_service_action = Some(services::ServiceAction {
                            name: svc.name.clone(),
                            action: services::ServiceControlAction::Start,
                        });
                    }
                    if stop_btn.on_hover_text(tooltip).clicked() {
                        app.pending_service_action = Some(services::ServiceAction {
                            name: svc.name.clone(),
                            action: services::ServiceControlAction::Stop,
                        });
                    }
                    if restart_btn.on_hover_text(tooltip).clicked() {
                        app.pending_service_action = Some(services::ServiceAction {
                            name: svc.name.clone(),
                            action: services::ServiceControlAction::Restart,
                        });
                    }

                    if !is_elevated {
                        ui.label(
                            egui::RichText::new("⚠ Elevated permissions required for service control")
                                .size(11.0)
                                .color(ThemePalette::STATUS_WARNING),
                        );
                    }
                });
            });

            ui.add_space(8.0);
        }
    }

    // ── Responsive Virtualized Services Table ──
    card_frame(is_dark).show(ui, |ui| {
        let total_w = ui.available_width().max(680.0);
        let spacing = 8.0;
        let action_w = 175.0;
        let state_w = 110.0;
        let id_w = 180.0f32.min(total_w * 0.26).max(140.0);
        let display_w = (total_w - action_w - state_w - id_w - (3.0 * spacing)).max(220.0);

        let sort_col = app.service_sort_column;
        let sort_asc = app.service_sort_ascending;

        let header_button = |ui: &mut egui::Ui,
                             label: &str,
                             width: f32,
                             col: services::ServiceSortColumn,
                             current_col: services::ServiceSortColumn,
                             asc: bool|
         -> egui::Response {
            let text = sort_header_label(label, col, current_col, asc);
            let is_active = col == current_col;
            let text_color = if is_active {
                ThemePalette::ACCENT_PRIMARY
            } else {
                ThemePalette::text_primary(is_dark)
            };
            let mut response = None;
            ui.allocate_ui_with_layout(
                egui::vec2(width, 22.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_width(width);
                    let btn = egui::Button::new(
                        egui::RichText::new(text)
                            .strong()
                            .size(11.5)
                            .color(text_color),
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(egui::Stroke::NONE);
                    response = Some(ui.add(btn));
                }
            );
            response.unwrap()
        };

        // Sticky Header with interactive sorting
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing;

            if header_button(
                ui,
                "Display Name",
                display_w,
                services::ServiceSortColumn::DisplayName,
                sort_col,
                sort_asc,
            )
            .clicked()
            {
                if app.service_sort_column == services::ServiceSortColumn::DisplayName {
                    app.service_sort_ascending = !app.service_sort_ascending;
                } else {
                    app.service_sort_column = services::ServiceSortColumn::DisplayName;
                    app.service_sort_ascending = true;
                }
            }

            if header_button(
                ui,
                "Service Identifier",
                id_w,
                services::ServiceSortColumn::Name,
                sort_col,
                sort_asc,
            )
            .clicked()
            {
                if app.service_sort_column == services::ServiceSortColumn::Name {
                    app.service_sort_ascending = !app.service_sort_ascending;
                } else {
                    app.service_sort_column = services::ServiceSortColumn::Name;
                    app.service_sort_ascending = true;
                }
            }

            if header_button(
                ui,
                "State",
                state_w,
                services::ServiceSortColumn::State,
                sort_col,
                sort_asc,
            )
            .clicked()
            {
                if app.service_sort_column == services::ServiceSortColumn::State {
                    app.service_sort_ascending = !app.service_sort_ascending;
                } else {
                    app.service_sort_column = services::ServiceSortColumn::State;
                    app.service_sort_ascending = true;
                }
            }

            ui.allocate_ui_with_layout(
                egui::vec2(action_w, 22.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_width(action_w);
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

        if filtered.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("🔍 No services match the current filter criteria.")
                        .size(13.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.add_space(8.0);
                if ui.button("Reset Filters").clicked() {
                    app.service_search.clear();
                    app.service_state_filter = None;
                }
            });
            ui.add_space(24.0);
            return;
        }

        let row_height = 28.0;
        let num_rows = filtered.len();

        ui.spacing_mut().item_spacing.y = 0.0;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_height(520.0)
            .show_rows(ui, row_height, num_rows, |ui, row_range| {
                for idx in row_range {
                    let svc = filtered[idx];
                    let is_even = idx % 2 == 0;
                    let is_selected = app
                        .selected_service_name
                        .as_ref()
                        .is_some_and(|n| n == &svc.name);

                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width().max(total_w), row_height),
                        egui::Sense::click(),
                    );

                    if row_resp.clicked() {
                        if is_selected {
                            app.selected_service_name = None;
                        } else {
                            app.selected_service_name = Some(svc.name.clone());
                        }
                    }

                    // Row background stripe & hover/selected effect
                    if is_selected {
                        let sel_fill = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(16, 185, 129, 30)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(16, 185, 129, 20)
                        };
                        ui.painter()
                            .rect_filled(row_rect, egui::Rounding::same(3.0), sel_fill);

                        // Accent left indicator bar
                        let ind_rect = egui::Rect::from_min_size(
                            row_rect.min,
                            egui::vec2(3.0, row_rect.height()),
                        );
                        ui.painter().rect_filled(
                            ind_rect,
                            egui::Rounding::same(1.5),
                            ThemePalette::ACCENT_PRIMARY,
                        );
                    } else if row_resp.hovered() {
                        let hover_fill = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 8)
                        };
                        ui.painter()
                            .rect_filled(row_rect, egui::Rounding::same(3.0), hover_fill);
                    } else if is_even {
                        let stripe_fill = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 3)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 3)
                        };
                        ui.painter()
                            .rect_filled(row_rect, egui::Rounding::same(3.0), stripe_fill);
                    }

                    ui.allocate_ui_at_rect(row_rect, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = spacing;

                            // Display Name (Dynamic full width allocation)
                            ui.allocate_ui_with_layout(
                                egui::vec2(display_w, row_height),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_width(display_w);
                                    let text_color = if is_selected {
                                        ThemePalette::ACCENT_PRIMARY
                                    } else {
                                        ThemePalette::text_primary(is_dark)
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&svc.display_name)
                                                .strong()
                                                .color(text_color),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(format!(
                                        "{}\nClick row to inspect / copy commands",
                                        svc.display_name
                                    ));
                                },
                            );

                            // Service Identifier (Monospace, click-to-copy)
                            ui.allocate_ui_with_layout(
                                egui::vec2(id_w, row_height),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_width(id_w);
                                    let copy_resp = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&svc.name)
                                                .monospace()
                                                .color(ThemePalette::text_secondary(is_dark)),
                                        )
                                        .truncate()
                                        .sense(egui::Sense::click()),
                                    )
                                    .on_hover_text(format!("Click to copy '{}'", svc.name));

                                    if copy_resp.clicked() {
                                        ui.output_mut(|o| o.copied_text = svc.name.clone());
                                    }
                                },
                            );

                            // State Pill
                            let state_c = service_state_color(&svc.state, is_dark);
                            ui.allocate_ui_with_layout(
                                egui::vec2(state_w, row_height),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_width(state_w);
                                    status_pill(ui, &svc.state.to_uppercase(), state_c, is_dark);
                                },
                            );

                            // Action buttons with Elevation Protection
                            ui.allocate_ui_with_layout(
                                egui::vec2(action_w, row_height),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_width(action_w);
                                    let is_running = svc.state.eq_ignore_ascii_case("running");
                                    let is_stopped = svc.state.eq_ignore_ascii_case("stopped");

                                    let tooltip = if !is_elevated {
                                        "Administrator privileges required to control services"
                                    } else {
                                        "Send service control command"
                                    };

                                    let start_btn = ui.add_enabled(
                                        is_elevated && !is_running,
                                        egui::Button::new(egui::RichText::new("Start").small()),
                                    );
                                    let stop_btn = ui.add_enabled(
                                        is_elevated && !is_stopped,
                                        egui::Button::new(
                                            egui::RichText::new("Stop").small().color(
                                                if is_elevated && !is_stopped {
                                                    ThemePalette::STATUS_CRITICAL
                                                } else {
                                                    ThemePalette::text_dimmed(is_dark)
                                                },
                                            ),
                                        ),
                                    );
                                    let restart_btn = ui.add_enabled(
                                        is_elevated && is_running,
                                        egui::Button::new(
                                            egui::RichText::new("Restart").small(),
                                        ),
                                    );

                                    if start_btn.on_hover_text(tooltip).clicked() {
                                        app.pending_service_action = Some(services::ServiceAction {
                                            name: svc.name.clone(),
                                            action: services::ServiceControlAction::Start,
                                        });
                                    }
                                    if stop_btn.on_hover_text(tooltip).clicked() {
                                        app.pending_service_action = Some(services::ServiceAction {
                                            name: svc.name.clone(),
                                            action: services::ServiceControlAction::Stop,
                                        });
                                    }
                                    if restart_btn.on_hover_text(tooltip).clicked() {
                                        app.pending_service_action = Some(services::ServiceAction {
                                            name: svc.name.clone(),
                                            action: services::ServiceControlAction::Restart,
                                        });
                                    }
                                },
                            );
                        });
                    });
                }
            });
    });

    if let Some(action) = app.pending_service_action.take() {
        app.queue_action(crate::app::commands::ActionCommand::ControlService {
            name: action.name,
            action: action.action,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_state_color_mapping() {
        assert_eq!(
            service_state_color("Running", true),
            ThemePalette::STATUS_HEALTHY
        );
        assert_eq!(
            service_state_color("RUNNING", false),
            ThemePalette::STATUS_HEALTHY
        );
        assert_eq!(
            service_state_color("Stopped", true),
            ThemePalette::text_dimmed(true)
        );
        assert_eq!(
            service_state_color("Paused", true),
            ThemePalette::STATUS_WARNING
        );
        assert_eq!(
            service_state_color("Start Pending", true),
            ThemePalette::STATUS_WARNING
        );
        assert_eq!(
            service_state_color("Stop Pending", false),
            ThemePalette::STATUS_WARNING
        );
    }

    #[test]
    fn test_sort_header_label() {
        assert_eq!(
            sort_header_label(
                "Display Name",
                services::ServiceSortColumn::DisplayName,
                services::ServiceSortColumn::DisplayName,
                true
            ),
            "Display Name ▲"
        );
        assert_eq!(
            sort_header_label(
                "Display Name",
                services::ServiceSortColumn::DisplayName,
                services::ServiceSortColumn::DisplayName,
                false
            ),
            "Display Name ▼"
        );
        assert_eq!(
            sort_header_label(
                "State",
                services::ServiceSortColumn::State,
                services::ServiceSortColumn::DisplayName,
                true
            ),
            "State"
        );
    }

    #[test]
    fn test_services_page_render_headless() {
        let mut app = crate::SystemMonitorApp::test_app();
        let mut data = SystemData::default();

        // 1. Initial empty services render
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });

        // 2. Populated services list render
        data.services = vec![
            crate::services::ServiceInfo {
                name: "ADBCSvc".to_string(),
                display_name: "Acer Display Backlight Control Service".to_string(),
                state: "Running".to_string(),
            },
            crate::services::ServiceInfo {
                name: "BITS".to_string(),
                display_name: "Background Intelligent Transfer Service".to_string(),
                state: "Running".to_string(),
            },
            crate::services::ServiceInfo {
                name: "AppIDSvc".to_string(),
                display_name: "Application Identity".to_string(),
                state: "Stopped".to_string(),
            },
        ];

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });

        // 3. Search and filter test
        app.service_search = "backlight".to_string();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });

        app.service_state_filter = Some("Stopped".to_string());
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });

        // 4. Selected service inspector render
        app.selected_service_name = Some("BITS".to_string());
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }
}
