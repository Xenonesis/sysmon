use crate::*;
use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui) {
        paint_section_header(ui, "Startup Programs");

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Load data lazily in a background thread ──
            if !app.startup_items_loaded && !app.startup_items_loading {
                app.startup_items_loading = true;
                let ctx = ui.ctx().clone();
                let startup_items_share = Arc::clone(&app.startup_items_share);
                let boot_diagnostics_share = Arc::clone(&app.boot_diagnostics_share);
                thread::Builder::new()
                    .name("startup_loader".to_string())
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move || {
                        let items = startup::get_startup_items();
                        let diag = startup::get_boot_diagnostics();
                        {
                            let mut share = startup_items_share.lock();
                            *share = Some(items);
                        }
                        {
                            let mut share = boot_diagnostics_share.lock();
                            *share = diag;
                        }
                        ctx.request_repaint();
                    })
                    .expect("failed to spawn startup loader thread");
            }

            // Sync loaded data to app state
            let is_loading = {
                let share = app.startup_items_share.lock();
                if let Some(ref items) = *share {
                    app.startup_items = items.clone();
                    app.startup_items_loaded = true;
                    app.startup_items_loading = false;
                    
                    let high_impact_count = items.iter().filter(|i| i.impact_tier == ImpactTier::High && i.enabled).count();
                    app.data.lock().high_impact_startup_count = high_impact_count;
                    
                    false
                } else {
                    true
                }
            };

            if let Some(ref diag) = *app.boot_diagnostics_share.lock() {
                app.boot_diagnostics = Some(diag.clone());
                app.boot_diagnostics_loaded = true;
            }

            if is_loading {
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Analyzing startup configuration...").strong().color(ThemePalette::TEXT_SECONDARY));
                });
                return;
            }

            // ── Header card with boot info ──
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("Startup Items");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🔄 Refresh").clicked() {
                            app.startup_items_loaded = false;
                            app.boot_diagnostics_loaded = false;
                            *app.startup_items_share.lock() = None;
                            *app.boot_diagnostics_share.lock() = None;
                            app.startup_show_confirm = None;
                        }
                    });
                });
                ui.separator();

                // Boot diagnostics summary
                ui.horizontal(|ui| {
                    let total = app.startup_items.len();
                    let high = startup::high_impact_count(&app.startup_items);

                    let mut boot_shown = false;
                    if let Some(ref bd) = app.boot_diagnostics {
                        if let Some(ms) = bd.boot_duration_ms {
                            let secs = ms as f64 / 1000.0;
                            let c = if secs < 30.0 { ThemePalette::STATUS_HEALTHY }
                                    else if secs < 60.0 { ThemePalette::STATUS_WARNING }
                                    else { ThemePalette::STATUS_CRITICAL };
                            ui.colored_label(c, format!("Boot: {:.1}s", secs));
                            ui.separator();
                            boot_shown = true;
                        }
                    }
                    if !boot_shown {
                        if privilege::is_app_elevated() {
                            ui.colored_label(ThemePalette::STATUS_WARNING, "Boot: Unknown");
                            ui.separator();
                        } else {
                            ui.colored_label(ThemePalette::STATUS_WARNING, "Boot: (Requires Admin)")
                                .on_hover_text("Reading boot diagnostics event logs requires Administrator privileges");
                            ui.separator();
                        }
                    }
                    if high > 0 {
                        ui.colored_label(ThemePalette::STATUS_CRITICAL, format!("{} high-impact", high));
                    } else {
                        ui.colored_label(ThemePalette::STATUS_HEALTHY, "No high-impact items");
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(format!("{} total", total)).color(ThemePalette::TEXT_LABEL));
                });
            });

            ui.add_space(8.0);

            // ── Search & Filter toolbar ──
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.add(egui::TextEdit::singleline(&mut app.startup_search)
                        .hint_text("Search by name, command, publisher...")
                        .desired_width(250.0));

                    ui.separator();

                    // Impact filter
                    egui::ComboBox::from_id_source("impact_filter")
                        .selected_text(match &app.startup_filter_impact {
                            Some(ImpactTier::High) => "High",
                            Some(ImpactTier::Medium) => "Medium",
                            Some(ImpactTier::Low) => "Low",
                            _ => "Impact: All",
                        })
                        .show_ui(ui, |ui: &mut egui::Ui| {
                            if ui.selectable_label(app.startup_filter_impact.is_none(), "All").clicked() {
                                app.startup_filter_impact = None;
                            }
                            if ui.selectable_label(app.startup_filter_impact == Some(ImpactTier::High), "High").clicked() {
                                app.startup_filter_impact = Some(ImpactTier::High);
                            }
                            if ui.selectable_label(app.startup_filter_impact == Some(ImpactTier::Medium), "Medium").clicked() {
                                app.startup_filter_impact = Some(ImpactTier::Medium);
                            }
                            if ui.selectable_label(app.startup_filter_impact == Some(ImpactTier::Low), "Low").clicked() {
                                app.startup_filter_impact = Some(ImpactTier::Low);
                            }
                        });

                    // Signed filter
                    egui::ComboBox::from_id_source("signed_filter")
                        .selected_text(match app.startup_filter_signed {
                            Some(true) => "Signed",
                            Some(false) => "Unsigned",
                            None => "Signed: All",
                        })
                        .show_ui(ui, |ui: &mut egui::Ui| {
                            if ui.selectable_label(app.startup_filter_signed.is_none(), "All").clicked() {
                                app.startup_filter_signed = None;
                            }
                            if ui.selectable_label(app.startup_filter_signed == Some(true), "Signed").clicked() {
                                app.startup_filter_signed = Some(true);
                            }
                            if ui.selectable_label(app.startup_filter_signed == Some(false), "Unsigned").clicked() {
                                app.startup_filter_signed = Some(false);
                            }
                        });

                    ui.checkbox(&mut app.startup_filter_broken, "Broken only");
                });

                // Sort controls
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Sort:").color(ThemePalette::TEXT_LABEL).small());

                    let sorts = [
                        (StartupSortColumn::Impact, "Impact"),
                        (StartupSortColumn::Name, "Name"),
                        (StartupSortColumn::Source, "Source"),
                        (StartupSortColumn::Publisher, "Publisher"),
                    ];
                    for (col, label) in &sorts {
                        let is_active = app.startup_sort == *col;
                        let text = if is_active {
                            let arrow = if app.startup_sort_ascending { "^" } else { "v" };
                            format!("{} {}", label, arrow)
                        } else {
                            label.to_string()
                        };
                        if ui.selectable_label(is_active,
                            egui::RichText::new(text).small()
                        ).clicked() {
                            if is_active {
                                app.startup_sort_ascending = !app.startup_sort_ascending;
                            } else {
                                app.startup_sort = *col;
                                app.startup_sort_ascending = true;
                            }
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // ── Apply filters and sort ──
            let search_lower = app.startup_search.to_lowercase();
            let mut filtered_indices: Vec<usize> = app.startup_items.iter().enumerate()
                .filter(|(_, item)| {
                    // Search filter
                    if !search_lower.is_empty() {
                        let matches = item.name.to_lowercase().contains(&search_lower)
                            || item.command.to_lowercase().contains(&search_lower)
                            || item.publisher.as_ref().map(|p| p.to_lowercase().contains(&search_lower)).unwrap_or(false);
                        if !matches { return false; }
                    }
                    // Impact filter
                    if let Some(ref filter) = app.startup_filter_impact {
                        if item.impact_tier != *filter { return false; }
                    }
                    // Signed filter
                    if let Some(filter_signed) = app.startup_filter_signed {
                        if item.is_signed != Some(filter_signed) { return false; }
                    }
                    // Broken filter
                    if app.startup_filter_broken && item.exe_exists { return false; }
                    true
                })
                .map(|(i, _)| i)
                .collect();

            // Sort the filtered view
            {
                let items_ref = &app.startup_items;
                let sort_col = app.startup_sort;
                let ascending = app.startup_sort_ascending;
                filtered_indices.sort_by(|a, b| {
                    let ia = &items_ref[*a];
                    let ib = &items_ref[*b];
                    let cmp = match sort_col {
                        StartupSortColumn::Name => ia.name.to_lowercase().cmp(&ib.name.to_lowercase()),
                        StartupSortColumn::Impact => ia.impact_tier.sort_key().cmp(&ib.impact_tier.sort_key()),
                        StartupSortColumn::Source => ia.source.cmp(&ib.source),
                        StartupSortColumn::Publisher => {
                            let pa = ia.publisher.as_deref().unwrap_or("zzz").to_lowercase();
                            let pb = ib.publisher.as_deref().unwrap_or("zzz").to_lowercase();
                            pa.cmp(&pb)
                        }
                    };
                    if ascending { cmp } else { cmp.reverse() }
                });
            }

            if filtered_indices.is_empty() {
                ui.group(|ui| {
                    ui.add_space(20.0);
                    if app.startup_items.is_empty() {
                        ui.label("No startup items found.");
                    } else {
                        ui.label("No items match the current filters.");
                    }
                    ui.add_space(20.0);
                });
            } else {
                ui.label(egui::RichText::new(format!("Showing {} of {} item(s)", filtered_indices.len(), app.startup_items.len()))
                    .small().color(ThemePalette::TEXT_LABEL));
                ui.add_space(4.0);

                let mut action: Option<(usize, &str)> = None;

                for &idx in &filtered_indices {
                    let item = &app.startup_items[idx];
                    let is_confirming = app.startup_show_confirm == Some(idx);

                    ui.group(|ui| {
                        // ── Row 1: Impact badge + Name + Source ──
                        ui.horizontal(|ui| {
                            // Impact badge
                            let (badge_text, badge_color) = match item.impact_tier {
                                ImpactTier::High => ("HIGH", ThemePalette::STATUS_CRITICAL),
                                ImpactTier::Medium => ("MED", ThemePalette::STATUS_WARNING),
                                ImpactTier::Low => ("LOW", ThemePalette::STATUS_HEALTHY),
                                ImpactTier::Unknown => ("?", ThemePalette::TEXT_DIMMED),
                            };
                            ui.colored_label(badge_color,
                                egui::RichText::new(badge_text).size(11.0).strong());
                            ui.separator();
                            if item.enabled {
                                ui.strong(&item.name);
                            } else {
                                ui.label(egui::RichText::new(&item.name).strong().strikethrough().color(ThemePalette::TEXT_DIMMED));
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.colored_label(ThemePalette::TEXT_TERTIARY,
                                    egui::RichText::new(&item.source).small());
                            });
                        });

                        // ── Row 2: Command path ──
                        let cmd_display = if item.command.chars().count() > 90 {
                            let truncated: String = item.command.chars().take(87).collect();
                            format!("{}...", truncated)
                        } else {
                            item.command.clone()
                        };
                        ui.label(egui::RichText::new(cmd_display).small().color(ThemePalette::TEXT_DIMMED));

                        // ── Row 3: Publisher + Signed status ──
                        ui.horizontal(|ui| {
                            if let Some(ref pub_name) = item.publisher {
                                ui.label(egui::RichText::new(format!("Publisher: {}", pub_name))
                                    .small().color(ThemePalette::TEXT_LABEL));
                            }
                            match item.is_signed {
                                Some(true) => { ui.colored_label(ThemePalette::STATUS_HEALTHY,
                                    egui::RichText::new("Signed").small()); }
                                Some(false) => { ui.colored_label(ThemePalette::STATUS_CRITICAL,
                                    egui::RichText::new("Unsigned").small()); }
                                None => {}
                            }
                            if !item.exe_exists && item.exe_path.is_some() {
                                ui.colored_label(ThemePalette::STATUS_CRITICAL,
                                    egui::RichText::new("File missing").small());
                            }
                        });

                        // ── Row 4: Recommendation + Reason ──
                        ui.horizontal(|ui| {
                            let rec_color = match item.recommendation {
                                Recommendation::Keep => ThemePalette::STATUS_HEALTHY,
                                Recommendation::Review => ThemePalette::STATUS_WARNING,
                                Recommendation::Disable => ThemePalette::STATUS_CRITICAL,
                                Recommendation::Cleanup => ThemePalette::STATUS_CRITICAL,
                            };
                            ui.colored_label(rec_color,
                                egui::RichText::new(format!("> {}", item.recommendation.label()))
                                    .small().strong());
                            ui.label(egui::RichText::new(format!("— {}", item.reason))
                                .small().color(ThemePalette::TEXT_LABEL_SUB));
                        });

                        // ── Row 5: Actions ──
                        if is_confirming {
                            // Confirmation dialog
                            ui.horizontal(|ui| {
                                ui.colored_label(ThemePalette::STATUS_WARNING,
                                    egui::RichText::new(format!("Disable \"{}\" from startup?", item.name)).strong());
                                if ui.button("Yes, disable").clicked() {
                                    action = Some((idx, "disable"));
                                    app.startup_show_confirm = None;
                                }
                                if ui.button("Cancel").clicked() {
                                    app.startup_show_confirm = None;
                                }
                            });
                        } else {
                            ui.horizontal(|ui| {
                                let is_elevated = privilege::is_app_elevated();
                                let can_modify = item.source.contains("HKCU")
                                    || item.source.contains("Startup Folder")
                                    || (is_elevated && (item.source.contains("HKLM") || item.source.contains("Task Scheduler")));
                                let is_keep = item.recommendation == Recommendation::Keep;

                                // Disable/Enable button
                                if item.enabled {
                                    ui.add_enabled_ui(can_modify && !is_keep, |ui| {
                                        if ui.button("Disable").on_hover_text(
                                            if is_keep { "System component — disabling not recommended" }
                                            else if !can_modify { "Requires Administrator privileges" }
                                            else { "Disable this startup item (reversible)" }
                                        ).clicked() {
                                            app.startup_show_confirm = Some(idx);
                                        }
                                    });
                                } else {
                                    ui.add_enabled_ui(can_modify, |ui| {
                                        if ui.button("Enable").on_hover_text(
                                            if !can_modify { "Requires Administrator privileges" }
                                            else { "Re-enable this startup item" }
                                        ).clicked() {
                                            action = Some((idx, "enable"));
                                        }
                                    });
                                }

                                // Open location
                                if let Some(ref path) = item.exe_path {
                                    if item.exe_exists {
                                        let path_clone = path.clone();
                                        if ui.button("Open").on_hover_text("Open file location in Explorer").clicked() {
                                            startup::open_file_location(&path_clone);
                                        }
                                    }
                                }

                                // Copy command
                                if ui.button("Copy").on_hover_text("Copy full command to clipboard").clicked() {
                                    ui.output_mut(|o| o.copied_text = item.command.clone());
                                }

                                // Search online
                                let name_clone = item.name.clone();
                                if ui.button("Search").on_hover_text("Search online for info about this item").clicked() {
                                    startup::search_online(&name_clone);
                                }

                                // Remove button (permanent delete for HKCU/Startup Folder/HKLM/Task Scheduler items)
                                if can_modify && !item.enabled {
                                    if ui.button("Remove").on_hover_text("Permanently remove this startup item").clicked() {
                                        action = Some((idx, "remove"));
                                    }
                                }

                                // Admin message for HKLM/Task Scheduler items when not elevated
                                if !can_modify {
                                    ui.colored_label(ThemePalette::TEXT_DIMMED,
                                        egui::RichText::new("(Requires Admin)").small());
                                }
                            });
                        }
                    });
                    ui.add_space(3.0);
                }

                // Process actions
                if let Some((idx, act)) = action {
                    let item = &app.startup_items[idx];
                    let item_name = item.name.clone();
                    let item_source = item.source.clone();
                    let item_command = item.command.clone();
                    let tier_before = item.impact_tier.label().to_string();
                    let high_before = startup::high_impact_count(&app.startup_items);

                    let success = match act {
                        "disable" => startup::disable_startup_item(&item_name, &item_source, &item_command),
                        "enable" => startup::reenable_startup_item(&item_name, &item_source),
                        "remove" => startup::remove_startup_item(&item_name, &item_source),
                        _ => false,
                    };

                    if success {
                        let high_after = if app.startup_items[idx].impact_tier == ImpactTier::High {
                            if act == "disable" { high_before.saturating_sub(1) } else { high_before + 1 }
                        } else {
                            high_before
                        };

                        app.settings.startup_optimization_history.push(StartupOptimizationEntry {
                            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                            action: act.to_string(),
                            item_name: item_name.clone(),
                            item_source,
                            impact_tier_before: tier_before,
                            high_impact_count_before: high_before,
                            high_impact_count_after: high_after,
                        });
                        let _ = app.settings.save();
                        app.startup_items_loaded = false;
                        *app.startup_items_share.lock() = None;
                        *app.boot_diagnostics_share.lock() = None;
                    }
                }
            }

            // ── Optimization History ──
            if !app.settings.startup_optimization_history.is_empty() {
                ui.add_space(16.0);
                ui.group(|ui| {
                    ui.heading("Optimization History");
                    ui.separator();

                    let history = &app.settings.startup_optimization_history;
                    let show_count = history.len().min(10);
                    for entry in history.iter().rev().take(show_count) {
                        ui.horizontal(|ui| {
                            ui.colored_label(ThemePalette::TEXT_LABEL,
                                egui::RichText::new(&entry.timestamp).small());
                            ui.label(egui::RichText::new(format!("{} \"{}\"",
                                entry.action, entry.item_name)).small());
                            let delta = entry.high_impact_count_before as i32 - entry.high_impact_count_after as i32;
                            if delta > 0 {
                                ui.colored_label(ThemePalette::STATUS_HEALTHY,
                                    egui::RichText::new(format!("-{} high", delta)).small());
                            }
                        });
                    }
                });
            }
        });
    }
