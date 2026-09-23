use crate::persistence;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;

pub(super) fn paint_policy_config_card(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
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
                        .add(
                            egui::Slider::new(
                                &mut app.ram_cleaner_state.auto_clean_threshold,
                                persistence::settings::CLEAN_THRESHOLD,
                            )
                            .suffix("%"),
                        )
                        .changed()
                    {
                        app.settings.ram_clean_threshold = app.ram_cleaner_state.auto_clean_threshold;
                        settings_changed = true;
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Target Usage:").color(ThemePalette::text_secondary(is_dark)));
                    if ui
                        .add(
                            egui::Slider::new(
                                &mut app.ram_cleaner_state.auto_clean_target,
                                persistence::settings::CLEAN_TARGET,
                            )
                            .suffix("%"),
                        )
                        .changed()
                    {
                        app.settings.auto_clean_target = app.ram_cleaner_state.auto_clean_target;
                        settings_changed = true;
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Cooldown Interval:").color(ThemePalette::text_secondary(is_dark)));
                    if ui
                        .add(
                            egui::Slider::new(
                                &mut app.ram_cleaner_state.auto_clean_interval,
                                persistence::settings::CLEAN_INTERVAL,
                            )
                            .suffix(" s"),
                        )
                        .changed()
                    {
                        app.settings.auto_clean_interval = app.ram_cleaner_state.auto_clean_interval;
                        settings_changed = true;
                    }
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Best-Effort Trim Budget:").color(ThemePalette::text_secondary(is_dark)),
                    );
                    if ui
                        .add(
                            egui::Slider::new(
                                &mut app.ram_cleaner_state.auto_clean_max_mb,
                                persistence::settings::CLEAN_BUDGET_MB,
                            )
                            .suffix(" MB"),
                        )
                        .on_hover_text(
                            "0 = unlimited. Checked before each process; a single working-set trim can \
                             overshoot this observed-decrease threshold.",
                        )
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
            ui.label(
                egui::RichText::new("Exclusion List (comma-separated executables):")
                    .size(12.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
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
            crate::ui::pages::settings::commit_settings(app);
        }
    });
}
