use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_ram_cleaner_settings(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    changed: &mut bool,
    is_dark: bool,
) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("AUTOMATED RAM CLEANER CONFIGURATION")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        if ui
            .checkbox(&mut app.settings.auto_ram_clean, "Enable Automated RAM Cleaning")
            .changed()
        {
            app.ram_cleaner_state.auto_clean_enabled = app.settings.auto_ram_clean;
            *changed = true;
        }

        if app.settings.auto_ram_clean {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            egui::Grid::new("settings_ram_cleaner_grid")
                .spacing([24.0, 10.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Trigger Threshold:").color(ThemePalette::text_secondary(is_dark)));
                    if ui
                        .add(egui::Slider::new(&mut app.settings.ram_clean_threshold, 1.0..=99.0).suffix("%"))
                        .changed()
                    {
                        app.ram_cleaner_state.auto_clean_threshold = app.settings.ram_clean_threshold;
                        *changed = true;
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Target Usage:").color(ThemePalette::text_secondary(is_dark)));
                    if ui
                        .add(egui::Slider::new(&mut app.settings.auto_clean_target, 1.0..=99.0).suffix("%"))
                        .changed()
                    {
                        app.ram_cleaner_state.auto_clean_target = app.settings.auto_clean_target;
                        *changed = true;
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Cooldown Interval:").color(ThemePalette::text_secondary(is_dark)));
                    if ui
                        .add(egui::Slider::new(&mut app.settings.auto_clean_interval, 10..=7200).suffix(" s"))
                        .changed()
                    {
                        app.ram_cleaner_state.auto_clean_interval = app.settings.auto_clean_interval;
                        *changed = true;
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Max Freed Budget:").color(ThemePalette::text_secondary(is_dark)));
                    if ui
                        .add(egui::Slider::new(&mut app.settings.auto_clean_max_mb, 0..=16384).suffix(" MB"))
                        .on_hover_text("0 = unlimited; caps how much memory one auto-clean can free")
                        .changed()
                    {
                        app.ram_cleaner_state.auto_clean_max_mb = app.settings.auto_clean_max_mb;
                        *changed = true;
                    }
                    ui.end_row();
                });

            ui.add_space(8.0);
            if ui
                .checkbox(
                    &mut app.settings.auto_clean_idle_only,
                    "Only clean when system is idle (>= 2m without input)",
                )
                .changed()
            {
                app.ram_cleaner_state.auto_clean_idle_only = app.settings.auto_clean_idle_only;
                *changed = true;
            }
            if ui
                .checkbox(
                    &mut app.settings.auto_clean_smart_only,
                    "Smart Clean (Skip focused foreground application)",
                )
                .changed()
            {
                app.ram_cleaner_state.auto_clean_smart_only = app.settings.auto_clean_smart_only;
                *changed = true;
            }
            if ui
                .checkbox(
                    &mut app.settings.auto_clean_notify,
                    "Show desktop notification after cleanup",
                )
                .changed()
            {
                app.ram_cleaner_state.auto_clean_notify = app.settings.auto_clean_notify;
                *changed = true;
            }

            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Exclusion List (comma-separated executables):")
                    .size(12.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            let mut exclusion_text = app.settings.auto_clean_exclusions.join(", ");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut exclusion_text)
                        .hint_text("e.g. chrome.exe, firefox.exe, game.exe")
                        .desired_width(ui.available_width()),
                )
                .changed()
            {
                let parsed: Vec<String> = exclusion_text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                app.settings.auto_clean_exclusions = parsed.clone();
                app.ram_cleaner_state.auto_clean_exclusions = parsed;
                *changed = true;
            }
        }
    });
}
