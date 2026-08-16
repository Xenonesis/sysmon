use crate::startup::ImpactTier;
use crate::startup::Recommendation;
use crate::startup::StartupOptimizationEntry;
use crate::startup::StartupSortColumn;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;
use std::sync::Arc;
use std::thread;

/// Resolves badge text and high-contrast semantic color for startup impact tiers.
pub(crate) fn impact_tier_badge_color(tier: &ImpactTier, is_dark: bool) -> (&'static str, egui::Color32) {
    match tier {
        ImpactTier::High => ("HIGH", ThemePalette::STATUS_CRITICAL),
        ImpactTier::Medium => ("MED", ThemePalette::STATUS_WARNING),
        ImpactTier::Low => ("LOW", ThemePalette::STATUS_HEALTHY),
        ImpactTier::Unknown => ("UNKNOWN", ThemePalette::text_dimmed(is_dark)),
    }
}

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Startup Programs", is_dark);

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
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(startup::get_startup_data));
                    match result {
                        Ok((items, diag)) => {
                            *startup_items_share.lock() = Some(items);
                            *boot_diagnostics_share.lock() = diag;
                        }
                        Err(_) => {
                            // Panic in startup data collection — provide empty
                            // results so the UI doesn't get stuck on "loading…"
                            *startup_items_share.lock() = Some(Vec::new());
                        }
                    }
                    ctx.request_repaint();
                })
                .ok();
        }

        // Sync loaded data to app state (only when loading completes)
        let is_loading = {
            let share = app.startup_items_share.lock();
            if let Some(items) = &*share {
                if !app.startup_items_loaded {
                    app.startup_items = items.clone();
                    app.startup_items_loaded = true;
                    app.startup_items_loading = false;

                    let high_impact_count = items
                        .iter()
                        .filter(|i| i.impact_tier == ImpactTier::High && i.enabled)
                        .count();
                    app.data.write().high_impact_startup_count = high_impact_count;
                }
                false
            } else {
                true
            }
        };

        if let Some(diag) = &*app.boot_diagnostics_share.lock() {
            if !app.boot_diagnostics_loaded {
                app.boot_diagnostics = Some(diag.clone());
                app.boot_diagnostics_loaded = true;
            }
        }
        if is_loading {
            ui.add_space(20.0);
            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Analyzing startup items and boot diagnostics...")
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                });
            });
            return;
        }

        // ── Summary Header Card ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("Startup Telemetry").color(ThemePalette::text_primary(is_dark)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("Refresh").strong())
                        .on_hover_text("Re-scan startup programs and boot logs")
                        .clicked()
                    {
                        app.startup_items_loaded = false;
                        app.boot_diagnostics_loaded = false;
                        *app.startup_items_share.lock() = None;
                        *app.boot_diagnostics_share.lock() = None;
                        app.startup_show_confirm = None;
                    }
                });
            });

            ui.separator();

            // Boot diagnostics benchmark summary readout
            ui.horizontal(|ui| {
                let total = app.startup_items.len();
                let high = startup::high_impact_count(&app.startup_items);

                let mut boot_shown = false;
                if let Some(bd) = &app.boot_diagnostics {
                    if let Some(ms) = bd.boot_duration_ms {
                        let secs = ms as f64 / 1000.0;
                        let c = if secs < 30.0 {
                            ThemePalette::STATUS_HEALTHY
                        } else if secs < 60.0 {
                            ThemePalette::STATUS_WARNING
                        } else {
                            ThemePalette::STATUS_CRITICAL
                        };
                        status_pill(ui, &format!("BOOT: {:.1}s", secs), c, is_dark);
                        boot_shown = true;
                    }
                }
                if !boot_shown {
                    if privilege::is_app_elevated() {
                        status_pill(ui, "BOOT: UNKNOWN", ThemePalette::STATUS_WARNING, is_dark);
                    } else {
                        status_pill(ui, "BOOT: ADMIN REQ", ThemePalette::STATUS_WARNING, is_dark);
                    }
                }

                ui.add_space(6.0);

                if high > 0 {
                    status_pill(
                        ui,
                        &format!("{} HIGH IMPACT", high),
                        ThemePalette::STATUS_CRITICAL,
                        is_dark,
                    );
                } else {
                    status_pill(ui, "0 HIGH IMPACT", ThemePalette::STATUS_HEALTHY, is_dark);
                }

                ui.add_space(6.0);
                status_pill(
                    ui,
                    &format!("{} TOTAL ITEMS", total),
                    ThemePalette::ACCENT_PRIMARY,
                    is_dark,
                );
            });
        });

        ui.add_space(8.0);

        // ── Search & Filter Toolbar ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                // Search Input with integrated Clear button
                ui.label(
                    egui::RichText::new("Search:")
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut app.startup_search)
                        .hint_text("Search name, command, publisher...")
                        .desired_width(240.0),
                );
                if !app.startup_search.is_empty() && ui.small_button("×").on_hover_text("Clear search filter").clicked()
                {
                    app.startup_search.clear();
                }

                ui.add_space(8.0);

                // Impact filter
                ui.label(
                    egui::RichText::new("Impact:")
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                egui::ComboBox::from_id_source("startup_impact_filter")
                    .selected_text(match &app.startup_filter_impact {
                        Some(ImpactTier::High) => "High",
                        Some(ImpactTier::Medium) => "Medium",
                        Some(ImpactTier::Low) => "Low",
                        _ => "All",
                    })
                    .show_ui(ui, |ui: &mut egui::Ui| {
                        if ui
                            .selectable_label(app.startup_filter_impact.is_none(), "All")
                            .clicked()
                        {
                            app.startup_filter_impact = None;
                        }
                        if ui
                            .selectable_label(app.startup_filter_impact == Some(ImpactTier::High), "High")
                            .clicked()
                        {
                            app.startup_filter_impact = Some(ImpactTier::High);
                        }
                        if ui
                            .selectable_label(app.startup_filter_impact == Some(ImpactTier::Medium), "Medium")
                            .clicked()
                        {
                            app.startup_filter_impact = Some(ImpactTier::Medium);
                        }
                        if ui
                            .selectable_label(app.startup_filter_impact == Some(ImpactTier::Low), "Low")
                            .clicked()
                        {
                            app.startup_filter_impact = Some(ImpactTier::Low);
                        }
                    });

                ui.add_space(8.0);

                // Signed filter
                ui.label(
                    egui::RichText::new("Publisher:")
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                egui::ComboBox::from_id_source("startup_signed_filter")
                    .selected_text(match app.startup_filter_signed {
                        Some(true) => "Signed",
                        Some(false) => "Unsigned",
                        None => "All",
                    })
                    .show_ui(ui, |ui: &mut egui::Ui| {
                        if ui
                            .selectable_label(app.startup_filter_signed.is_none(), "All")
                            .clicked()
                        {
                            app.startup_filter_signed = None;
                        }
                        if ui
                            .selectable_label(app.startup_filter_signed == Some(true), "Signed")
                            .clicked()
                        {
                            app.startup_filter_signed = Some(true);
                        }
                        if ui
                            .selectable_label(app.startup_filter_signed == Some(false), "Unsigned")
                            .clicked()
                        {
                            app.startup_filter_signed = Some(false);
                        }
                    });

                ui.add_space(8.0);
                ui.checkbox(&mut app.startup_filter_broken, "Broken only");

                let has_active_filters = !app.startup_search.is_empty()
                    || app.startup_filter_impact.is_some()
                    || app.startup_filter_signed.is_some()
                    || app.startup_filter_broken;

                if has_active_filters
                    && ui
                        .small_button("Reset")
                        .on_hover_text("Reset all search and filtering")
                        .clicked()
                {
                    app.startup_search.clear();
                    app.startup_filter_impact = None;
                    app.startup_filter_signed = None;
                    app.startup_filter_broken = false;
                }
            });

            ui.add_space(4.0);

            // Sort controls
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Sort by:")
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );

                let sorts = [
                    (StartupSortColumn::Impact, "Impact"),
                    (StartupSortColumn::Name, "Name"),
                    (StartupSortColumn::Source, "Source"),
                    (StartupSortColumn::Publisher, "Publisher"),
                ];
                for (col, label) in &sorts {
                    let is_active = app.startup_sort == *col;
                    let text = if is_active {
                        let arrow = if app.startup_sort_ascending { " ▲" } else { " ▼" };
                        format!("{}{}", label, arrow)
                    } else {
                        label.to_string()
                    };
                    let text_color = if is_active {
                        ThemePalette::ACCENT_PRIMARY
                    } else {
                        ThemePalette::text_primary(is_dark)
                    };
                    if ui
                        .button(egui::RichText::new(text).small().strong().color(text_color))
                        .clicked()
                    {
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
        let mut filtered_indices: Vec<usize> = app
            .startup_items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                // Search filter
                if !search_lower.is_empty() {
                    let matches = item.name.to_lowercase().contains(&search_lower)
                        || item.command.to_lowercase().contains(&search_lower)
                        || item
                            .publisher
                            .as_ref()
                            .map(|p| p.to_lowercase().contains(&search_lower))
                            .unwrap_or(false);
                    if !matches {
                        return false;
                    }
                }
                // Impact filter
                if let Some(filter) = &app.startup_filter_impact {
                    if item.impact_tier != *filter {
                        return false;
                    }
                }
                // Signed filter
                if let Some(filter_signed) = app.startup_filter_signed {
                    if item.is_signed != Some(filter_signed) {
                        return false;
                    }
                }
                // Broken filter
                if app.startup_filter_broken && item.exe_exists {
                    return false;
                }
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
                if ascending {
                    cmp
                } else {
                    cmp.reverse()
                }
            });
        }

        if filtered_indices.is_empty() {
            card_frame(is_dark).show(ui, |ui| {
                ui.add_space(12.0);
                if app.startup_items.is_empty() {
                    ui.label(
                        egui::RichText::new("No startup items found.").color(ThemePalette::text_secondary(is_dark)),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("No startup items match the active filter criteria.")
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                }
                ui.add_space(12.0);
            });
        } else {
            let mut action: Option<(String, String, String, &str)> = None;
            let is_elevated = privilege::is_app_elevated();

            for &idx in &filtered_indices {
                if idx >= app.startup_items.len() {
                    continue;
                }
                let item = &app.startup_items[idx];
                let is_confirming = app.startup_show_confirm.as_deref() == Some(&item.name);
                card_frame(is_dark).show(ui, |ui| {
                    // ── Row 1: High-Contrast Impact Badge + Signed Badge + Name + Source ──
                    ui.horizontal(|ui| {
                        // Impact tier badge
                        let (badge_text, badge_color) = impact_tier_badge_color(&item.impact_tier, is_dark);
                        status_pill(ui, badge_text, badge_color, is_dark);

                        // Signed/Verified Publisher Badge
                        match item.is_signed {
                            Some(true) => {
                                status_pill(ui, "SIGNED", ThemePalette::STATUS_HEALTHY, is_dark);
                            }
                            Some(false) => {
                                status_pill(ui, "UNSIGNED", ThemePalette::STATUS_CRITICAL, is_dark);
                            }
                            None => {}
                        }

                        if !item.exe_exists && item.exe_path.is_some() {
                            status_pill(ui, "FILE MISSING", ThemePalette::STATUS_CRITICAL, is_dark);
                        }

                        ui.add_space(4.0);

                        if item.enabled {
                            ui.strong(egui::RichText::new(&item.name).color(ThemePalette::text_primary(is_dark)));
                        } else {
                            ui.label(
                                egui::RichText::new(&item.name)
                                    .strong()
                                    .strikethrough()
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(&item.source)
                                    .monospace()
                                    .size(11.0)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                        });
                    });

                    ui.add_space(2.0);

                    // ── Row 2: Command path (Monospace) ──
                    let clean_cmd = item.command.replace('\0', "");
                    let cmd_display = if clean_cmd.chars().count() > 90 {
                        let truncated: String = clean_cmd.chars().take(87).collect();
                        format!("{}...", truncated)
                    } else {
                        clean_cmd.clone()
                    };
                    ui.label(
                        egui::RichText::new(cmd_display)
                            .monospace()
                            .size(11.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    )
                    .on_hover_text(&clean_cmd);

                    ui.add_space(2.0);

                    // ── Row 3: Publisher + Recommendation Reason ──
                    ui.horizontal(|ui| {
                        if let Some(pub_name) = &item.publisher {
                            ui.label(
                                egui::RichText::new(format!("Publisher: {}", pub_name))
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            ui.separator();
                        }

                        let rec_color = match item.recommendation {
                            Recommendation::Keep => ThemePalette::STATUS_HEALTHY,
                            Recommendation::Review => ThemePalette::STATUS_WARNING,
                            Recommendation::Disable | Recommendation::Cleanup => ThemePalette::STATUS_CRITICAL,
                        };
                        ui.label(
                            egui::RichText::new(format!("> {}", item.recommendation.label()))
                                .strong()
                                .size(11.5)
                                .color(rec_color),
                        );
                        ui.label(
                            egui::RichText::new(format!("— {}", item.reason))
                                .size(11.5)
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                    });

                    ui.add_space(4.0);

                    // ── Row 4: Action Controls ──
                    if is_confirming {
                        // Confirmation dialog
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Disable \"{}\" from startup?", item.name))
                                    .strong()
                                    .color(ThemePalette::STATUS_WARNING),
                            );
                            if ui.button(egui::RichText::new("Yes, disable").strong()).clicked() {
                                action =
                                    Some((item.name.clone(), item.source.clone(), item.command.clone(), "disable"));
                                app.startup_show_confirm = None;
                            }
                            if ui.button("Cancel").clicked() {
                                app.startup_show_confirm = None;
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            let can_modify = item.source.contains("HKCU")
                                || item.source.contains("Startup Folder")
                                || (is_elevated
                                    && (item.source.contains("HKLM") || item.source.contains("Task Scheduler")));
                            let is_keep = item.recommendation == Recommendation::Keep;

                            // Disable/Enable button
                            if item.enabled {
                                ui.add_enabled_ui(can_modify && !is_keep, |ui| {
                                    if ui
                                        .button(egui::RichText::new("Disable").small())
                                        .on_hover_text(if is_keep {
                                            "System component — disabling not recommended"
                                        } else if !can_modify {
                                            "Requires Administrator privileges"
                                        } else {
                                            "Disable this startup item (reversible)"
                                        })
                                        .clicked()
                                    {
                                        app.startup_show_confirm = Some(item.name.clone());
                                    }
                                });
                            } else {
                                ui.add_enabled_ui(can_modify, |ui| {
                                    if ui
                                        .button(
                                            egui::RichText::new("Enable")
                                                .small()
                                                .color(ThemePalette::STATUS_HEALTHY),
                                        )
                                        .on_hover_text(if !can_modify {
                                            "Requires Administrator privileges"
                                        } else {
                                            "Re-enable this startup item"
                                        })
                                        .clicked()
                                    {
                                        action = Some((
                                            item.name.clone(),
                                            item.source.clone(),
                                            item.command.clone(),
                                            "enable",
                                        ));
                                    }
                                });
                            }

                            // Open location
                            if let Some(path) = &item.exe_path {
                                if item.exe_exists {
                                    let path_clone = path.clone();
                                    if ui
                                        .button(egui::RichText::new("Open").small())
                                        .on_hover_text("Open file location in Explorer")
                                        .clicked()
                                    {
                                        startup::open_file_location(&path_clone);
                                    }
                                }
                            }

                            // Copy command
                            if ui
                                .button(egui::RichText::new("Copy").small())
                                .on_hover_text("Copy full command to clipboard")
                                .clicked()
                            {
                                ui.output_mut(|o| o.copied_text = item.command.clone());
                            }

                            // Search online
                            let name_clone = item.name.clone();
                            if ui
                                .button(egui::RichText::new("Search Online").small())
                                .on_hover_text("Search online for info about this item")
                                .clicked()
                            {
                                startup::search_online(&name_clone);
                            }

                            // Remove button (permanent delete for HKCU/Startup Folder/HKLM/Task Scheduler items)
                            if can_modify
                                && !item.enabled
                                && ui
                                    .button(
                                        egui::RichText::new("Remove")
                                            .small()
                                            .color(ThemePalette::STATUS_CRITICAL),
                                    )
                                    .on_hover_text("Permanently remove this startup item")
                                    .clicked()
                            {
                                action = Some((item.name.clone(), item.source.clone(), item.command.clone(), "remove"));
                            }

                            // Admin requirement notice
                            if !can_modify {
                                ui.label(
                                    egui::RichText::new("(Requires Admin)")
                                        .size(11.0)
                                        .color(ThemePalette::text_dimmed(is_dark)),
                                );
                            }
                        });
                    }
                });

                ui.add_space(4.0);
            }

            // Process actions safely by matching (name, source)
            if let Some((name, source, cmd, act)) = action {
                let success = match act {
                    "disable" => startup::disable_startup_item(&name, &source, &cmd),
                    "enable" => startup::reenable_startup_item(&name, &source),
                    "remove" => startup::remove_startup_item(&name, &source),
                    _ => false,
                };

                if success {
                    if let Some(pos) = app
                        .startup_items
                        .iter()
                        .position(|it| it.name == name && it.source == source)
                    {
                        let tier_before = app.startup_items[pos].impact_tier.label().to_string();
                        let high_before = startup::high_impact_count(&app.startup_items);

                        if act == "disable" {
                            app.startup_items[pos].enabled = false;
                        } else if act == "enable" {
                            app.startup_items[pos].enabled = true;
                        } else if act == "remove" {
                            app.startup_items.remove(pos);
                        }

                        let high_after = startup::high_impact_count(&app.startup_items);
                        app.data.write().high_impact_startup_count = high_after;

                        app.settings
                            .startup_optimization_history
                            .push(StartupOptimizationEntry {
                                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                                action: act.to_string(),
                                item_name: name,
                                item_source: source,
                                impact_tier_before: tier_before,
                                high_impact_count_before: high_before,
                                high_impact_count_after: high_after,
                            });
                        let _ = app.settings.save();
                    }
                }
            }
        }

        // ── Optimization History ──
        if !app.settings.startup_optimization_history.is_empty() {
            ui.add_space(16.0);
            card_frame(is_dark).show(ui, |ui| {
                ui.heading(egui::RichText::new("Optimization History").color(ThemePalette::text_primary(is_dark)));
                ui.separator();

                let history = &app.settings.startup_optimization_history;
                let show_count = history.len().min(10);
                for entry in history.iter().rev().take(show_count) {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&entry.timestamp)
                                .monospace()
                                .size(11.0)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{} \"{}\"", entry.action, entry.item_name))
                                .size(11.5)
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        let delta = entry.high_impact_count_before as i32 - entry.high_impact_count_after as i32;
                        if delta > 0 {
                            status_pill(ui, &format!("-{} HIGH", delta), ThemePalette::STATUS_HEALTHY, is_dark);
                        }
                    });
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impact_tier_badge_colors() {
        let (label_high, color_high) = impact_tier_badge_color(&ImpactTier::High, true);
        assert_eq!(label_high, "HIGH");
        assert_eq!(color_high, ThemePalette::STATUS_CRITICAL);

        let (label_med, color_med) = impact_tier_badge_color(&ImpactTier::Medium, true);
        assert_eq!(label_med, "MED");
        assert_eq!(color_med, ThemePalette::STATUS_WARNING);

        let (label_low, color_low) = impact_tier_badge_color(&ImpactTier::Low, true);
        assert_eq!(label_low, "LOW");
        assert_eq!(color_low, ThemePalette::STATUS_HEALTHY);
    }

    #[test]
    fn test_startup_manager_render_all_states() {
        let mut app = SystemMonitorApp::test_app();
        app.startup_items_loaded = true;
        app.startup_items_loading = false;
        app.startup_items = vec![
            crate::startup::StartupItem {
                name: "Test App Normal".into(),
                command: r#""C:\Program Files\Test\app.exe" --silent"#.into(),
                enabled: true,
                source: "Registry (HKCU)".into(),
                exe_path: Some(r#"C:\Program Files\Test\app.exe"#.into()),
                exe_exists: true,
                publisher: Some("Test Corp".into()),
                is_signed: Some(true),
                impact_tier: ImpactTier::High,
                recommendation: Recommendation::Keep,
                reason: "Test reason".into(),
            },
            crate::startup::StartupItem {
                name: "Broken App <Unicode> 日本語".into(),
                command: r#"C:\NonExistent\broken.exe"#.into(),
                enabled: false,
                source: "Task Scheduler".into(),
                exe_path: Some(r#"C:\NonExistent\broken.exe"#.into()),
                exe_exists: false,
                publisher: None,
                is_signed: Some(false),
                impact_tier: ImpactTier::High,
                recommendation: Recommendation::Cleanup,
                reason: "File not found".into(),
            },
            crate::startup::StartupItem {
                name: "Unsigned App with Null\0Byte".into(),
                command: "C:\\App\\app.exe\0--arg".into(),
                enabled: true,
                source: "Startup Folder (User)".into(),
                exe_path: None,
                exe_exists: false,
                publisher: Some("Unknown Pub".into()),
                is_signed: None,
                impact_tier: ImpactTier::Medium,
                recommendation: Recommendation::Review,
                reason: "Review for necessity".into(),
            },
        ];

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui);
            });
        });

        // Test with confirmation dialog open
        app.startup_show_confirm = Some("Test App Normal".into());
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui);
            });
        });

        // Test with empty items and filters active
        app.startup_search = "NonExistentFilter999".into();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui);
            });
        });
    }
}
