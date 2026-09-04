use crate::privilege;
use crate::startup::{self, ImpactTier};
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;
use std::sync::Arc;
use std::thread;

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

        if let Some(diag) = &*app.boot_diagnostics_share.lock()
            && !app.boot_diagnostics_loaded
        {
            app.boot_diagnostics = Some(diag.clone());
            app.boot_diagnostics_loaded = true;
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
        super::summary_card::paint_summary_card(app, ui, is_dark);

        ui.add_space(8.0);

        // ── Search & Filter Toolbar ──
        super::filter_bar::paint_filter_bar(app, ui, is_dark);

        ui.add_space(8.0);

        // ── Apply filters and sort ──
        let filtered_indices = super::filter_and_sort_indices(app);

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
            let mut action = None;
            let is_elevated = privilege::is_app_elevated();

            for &idx in &filtered_indices {
                if idx >= app.startup_items.len() {
                    continue;
                }
                let item = &app.startup_items[idx];
                if let Some(act) = super::item_card::paint_startup_item_card(
                    &mut app.startup_show_confirm,
                    ui,
                    item,
                    is_dark,
                    is_elevated,
                ) {
                    action = Some(act);
                }
                ui.add_space(4.0);
            }

            // Process actions safely by matching (name, source)
            if let Some(act) = action {
                super::action_handler::handle_startup_action(app, act);
            }
        }

        // ── Optimization History ──
        super::action_handler::paint_action_history(app, ui, is_dark);
    });
}
