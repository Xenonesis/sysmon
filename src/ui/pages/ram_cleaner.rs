use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "RAM Cleaner", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Current Memory Status Card ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("MEMORY STATUS")
                        .size(13.0)
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let total_gb = bytes_to_gb(data.memory_total);
                    let used_gb = bytes_to_gb(data.memory_used);
                    let free_gb = bytes_to_gb(data.memory_total.saturating_sub(data.memory_used));
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.2} GB / {:.2} GB · {:.2} GB Free",
                            used_gb, total_gb, free_gb
                        ))
                        .size(12.0)
                        .monospace()
                        .color(ThemePalette::text_secondary(is_dark)),
                    );
                });
            });

            ui.add_space(8.0);
            let color = get_usage_color(data.memory_percentage);
            paint_progress_bar(ui, data.memory_percentage / 100.0, color, 8.0, is_dark);

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{:.1}% Used", data.memory_percentage))
                        .size(14.0)
                        .strong()
                        .monospace()
                        .color(color),
                );
                ui.add_space(16.0);
                if privilege::is_app_elevated() {
                    status_pill(
                        ui,
                        "FULL SYSTEM MEMORY ACCESS",
                        ThemePalette::STATUS_HEALTHY,
                        is_dark,
                    );
                } else {
                    status_pill(
                        ui,
                        "USER PROCESSES ONLY (RUN AS ADMIN FOR FULL)",
                        ThemePalette::STATUS_WARNING,
                        is_dark,
                    );
                }
            });
        });

        ui.add_space(12.0);

        // ── 2. Manual Clean Control Card ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("MANUAL WORKING-SET CLEAN")
                        .size(13.0)
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.ram_cleaner_state.is_cleaning {
                        status_pill(ui, "CLEANING IN PROGRESS...", ThemePalette::ACCENT_PRIMARY, is_dark);
                    }
                });
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Frees physical memory by trimming working sets. Windows will smoothly reload active pages as needed.")
                    .size(12.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(10.0);

            let is_cleaning = app.ram_cleaner_state.is_cleaning;
            ui.add_enabled_ui(!is_cleaning, |ui| {
                let btn = egui::Button::new(
                    egui::RichText::new("Trim RAM Working Sets Now")
                        .size(13.5)
                        .strong()
                        .color(if is_cleaning { ThemePalette::text_dimmed(is_dark) } else { ThemePalette::bg_deepest(is_dark) }),
                )
                .fill(if is_cleaning { ThemePalette::bg_track(is_dark) } else { ThemePalette::ACCENT_PRIMARY })
                .rounding(egui::Rounding::same(4.0));

                if ui.add_sized([ui.available_width(), 34.0], btn).clicked() {
                    app.start_ram_clean(ui.ctx());
                }
            });
        });

        ui.add_space(12.0);

        // ── 3. Auto Clean Policy Card ──
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("AUTOMATIC CLEANING POLICY")
                    .size(13.0)
                    .strong()
                    .color(ThemePalette::text_primary(is_dark)),
            );
            ui.add_space(6.0);

            let mut settings_changed = false;
            if ui
                .checkbox(
                    &mut app.ram_cleaner_state.auto_clean_enabled,
                    egui::RichText::new("Enable Background Auto-Cleaning").strong(),
                )
                .changed()
            {
                app.settings.auto_ram_clean = app.ram_cleaner_state.auto_clean_enabled;
                settings_changed = true;
            }

            if app.ram_cleaner_state.auto_clean_enabled {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                egui::Grid::new("ram_cleaner_grid")
                    .spacing([24.0, 10.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Trigger Threshold:").color(ThemePalette::text_secondary(is_dark)));
                        if ui
                            .add(egui::Slider::new(&mut app.ram_cleaner_state.auto_clean_threshold, 1.0..=99.0).suffix("%"))
                            .changed()
                        {
                            app.settings.ram_clean_threshold = app.ram_cleaner_state.auto_clean_threshold;
                            settings_changed = true;
                        }
                        ui.end_row();

                        ui.label(egui::RichText::new("Target Usage:").color(ThemePalette::text_secondary(is_dark)));
                        if ui
                            .add(egui::Slider::new(&mut app.ram_cleaner_state.auto_clean_target, 1.0..=99.0).suffix("%"))
                            .changed()
                        {
                            app.settings.auto_clean_target = app.ram_cleaner_state.auto_clean_target;
                            settings_changed = true;
                        }
                        ui.end_row();

                        ui.label(egui::RichText::new("Cooldown Interval:").color(ThemePalette::text_secondary(is_dark)));
                        if ui
                            .add(
                                egui::Slider::new(&mut app.ram_cleaner_state.auto_clean_interval, 10..=7200).suffix(" s"),
                            )
                            .changed()
                        {
                            app.settings.auto_clean_interval = app.ram_cleaner_state.auto_clean_interval;
                            settings_changed = true;
                        }
                        ui.end_row();

                        ui.label(egui::RichText::new("Max Freed Budget:").color(ThemePalette::text_secondary(is_dark)));
                        if ui
                            .add(egui::Slider::new(&mut app.ram_cleaner_state.auto_clean_max_mb, 0..=16384).suffix(" MB"))
                            .on_hover_text("0 = unlimited; caps how much memory one auto-clean can free")
                            .changed()
                        {
                            app.settings.auto_clean_max_mb = app.ram_cleaner_state.auto_clean_max_mb;
                            settings_changed = true;
                        }
                        ui.end_row();
                    });

                ui.add_space(8.0);
                if ui
                    .checkbox(
                        &mut app.ram_cleaner_state.auto_clean_idle_only,
                        "Only clean when system is idle (>= 2m without input)",
                    )
                    .changed()
                {
                    app.settings.auto_clean_idle_only = app.ram_cleaner_state.auto_clean_idle_only;
                    settings_changed = true;
                }
                if ui
                    .checkbox(
                        &mut app.ram_cleaner_state.auto_clean_smart_only,
                        "Smart Clean (Skip focused foreground application)",
                    )
                    .changed()
                {
                    app.settings.auto_clean_smart_only = app.ram_cleaner_state.auto_clean_smart_only;
                    settings_changed = true;
                }
                if ui
                    .checkbox(
                        &mut app.ram_cleaner_state.auto_clean_notify,
                        "Show desktop notification after cleanup",
                    )
                    .changed()
                {
                    app.settings.auto_clean_notify = app.ram_cleaner_state.auto_clean_notify;
                    settings_changed = true;
                }

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Exclusion List (comma-separated executables):").size(12.0).color(ThemePalette::text_secondary(is_dark)));
                let mut exclusion_text = app.ram_cleaner_state.auto_clean_exclusions.join(", ");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut exclusion_text)
                            .hint_text("e.g. chrome.exe, firefox.exe, game.exe")
                            .desired_width(ui.available_width()),
                    )
                    .changed()
                {
                    app.ram_cleaner_state.auto_clean_exclusions = exclusion_text
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    app.settings.auto_clean_exclusions = app.ram_cleaner_state.auto_clean_exclusions.clone();
                    settings_changed = true;
                }
            }

            if settings_changed {
                let _ = app.settings.save();
            }
        });

        ui.add_space(12.0);

        // ── 4. Session Statistics Card ──
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("SESSION STATISTICS")
                    .size(13.0)
                    .strong()
                    .color(ThemePalette::text_primary(is_dark)),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Clean Passes:").color(ThemePalette::text_secondary(is_dark)));
                ui.label(
                    egui::RichText::new(format!("{}", app.ram_cleaner_state.clean_count))
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );

                ui.add_space(24.0);
                ui.label(egui::RichText::new("Total Freed:").color(ThemePalette::text_secondary(is_dark)));
                ui.label(
                    egui::RichText::new(format!("{:.2} MB", bytes_to_mb(app.ram_cleaner_state.bytes_freed)))
                        .monospace()
                        .strong()
                        .color(ThemePalette::STATUS_HEALTHY),
                );

                ui.add_space(24.0);
                ui.label(egui::RichText::new("Last Clean:").color(ThemePalette::text_secondary(is_dark)));
                if app.ram_cleaner_state.last_cleaned.is_some() {
                    ui.label(
                        egui::RichText::new(&app.ram_cleaner_state.last_cleaned_display)
                            .monospace()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                } else {
                    ui.label(egui::RichText::new("Never").color(ThemePalette::text_dimmed(is_dark)));
                }
            });
        });
    });
}
