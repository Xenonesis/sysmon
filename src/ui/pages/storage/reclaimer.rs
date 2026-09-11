use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_reclaimer_card(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    app.storage_page.poll_background(ui.ctx());
    if !app.storage_page.reclaimer_scanned {
        app.storage_page.scan_caches();
    }

    let selected_bytes: u64 = app
        .storage_page
        .reclaimer_categories
        .iter()
        .filter(|c| app.storage_page.reclaimer_selected.contains(c.id))
        .map(|c| c.size_bytes)
        .sum();
    let selected_files: usize = app
        .storage_page
        .reclaimer_categories
        .iter()
        .filter(|c| app.storage_page.reclaimer_selected.contains(c.id))
        .map(|c| c.file_count)
        .sum();

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("STORAGE SPACE RECLAIMER")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(!app.storage_page.scan_busy(), egui::Button::new("Rescan")).clicked() {
                    app.storage_page.reclaimer_status = None;
                    app.storage_page.scan_caches();
                }
                if app.storage_page.scan_busy() && ui.small_button("Cancel scan").clicked() {
                    app.storage_page.cancel_scan();
                }
                status_pill(
                    ui,
                    &format!("{:.1} MB AVAILABLE", crate::ui::format::bytes_to_mb(selected_bytes)),
                    ThemePalette::ACCENT_PRIMARY,
                    is_dark,
                );
            });
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Review eligible files older than seven days. Cleanup is permanent, has no Undo, and preserves new, changed, linked and read-only files.",
            )
            .size(12.0)
            .color(ThemePalette::text_secondary(is_dark)),
        );

        ui.add_space(8.0);
        let mut toggle_id = None;
        for cat in &app.storage_page.reclaimer_categories {
            let is_selected = app.storage_page.reclaimer_selected.contains(cat.id);
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    let mut checked = is_selected;
                    if ui.add_enabled(cat.file_count > 0, egui::Checkbox::new(&mut checked, "")).changed() {
                        toggle_id = Some(cat.id);
                    }
                    ui.label(
                        egui::RichText::new(cat.label)
                            .strong()
                            .size(13.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(crate::ui::format::bytes_to_human(cat.size_bytes))
                                .monospace()
                                .strong()
                                .size(12.5)
                                .color(if cat.size_bytes > 0 {
                                    ThemePalette::ACCENT_PRIMARY
                                } else {
                                    ThemePalette::text_dimmed(is_dark)
                                }),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!("{} file(s)", cat.file_count))
                                .monospace()
                                .size(11.0)
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                    });
                });

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new(cat.description)
                            .size(11.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                });
                ui.label(format!("Excluded: {}; unreadable/failed: {}; coverage: {}", cat.skipped, cat.failed, if cat.complete { "finished" } else { "partial" }));
                if !cat.issues.is_empty() {
                    ui.collapsing("Exclusions and scan errors (first 100)", |ui| {
                        for issue in &cat.issues { ui.label(format!("{}: {}", issue.path.display(), issue.reason)); }
                    });
                }
            });
            ui.add_space(4.0);
        }

        if let Some(id) = toggle_id {
            app.storage_page.toggle_category(id);
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Selected: {} across {} file(s)",
                    crate::ui::format::bytes_to_human(selected_bytes),
                    selected_files
                ))
                .monospace()
                .size(12.0)
                .color(ThemePalette::text_primary(is_dark)),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clean_btn =
                    egui::Button::new(egui::RichText::new("Clean Selected Caches").size(12.0).strong().color(
                        if selected_bytes > 0 {
                            ThemePalette::STATUS_WARNING
                        } else {
                            ThemePalette::text_dimmed(is_dark)
                        },
                    ))
                    .fill(ThemePalette::STATUS_WARNING.gamma_multiply(if is_dark { 0.18 } else { 0.12 }))
                    .stroke(egui::Stroke::new(1.0, ThemePalette::STATUS_WARNING.gamma_multiply(0.5)))
                    .corner_radius(egui::CornerRadius::same(4));

                let has_selection = selected_files > 0 && !app.storage_page.scan_busy();
                ui.add_enabled_ui(has_selection, |ui| {
                    if ui.add(clean_btn).clicked() {
                        let reviewed = app.storage_page.reviewed_cleanup();
                        let accepted = app.queue_action(crate::app::commands::ActionCommand::ReclaimStorageCaches(reviewed));
                        app.storage_page.reclaimer_status = Some(if accepted {
                            "Awaiting confirmation for the reviewed file identities; nothing deleted yet."
                        } else {
                            "Cleanup request rejected: another action or confirmation is pending."
                        }.into());
                    }
                });
            });
        });

        if let Some(status) = &app.storage_page.reclaimer_status {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(status)
                    .monospace()
                    .size(11.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
        }
    });
}
