use crate::*;
use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "RAM Cleaner");

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Current Memory Status
            ui.group(|ui| {
                ui.heading("Memory Overview");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Total RAM:");
                    ui.strong(format!("{:.2} GB", bytes_to_gb(data.memory_total)));
                });
                ui.horizontal(|ui| {
                    ui.label("Used RAM:");
                    let color = get_usage_color(data.memory_percentage);
                    ui.colored_label(
                        color,
                        format!(
                            "{:.2} GB ({:.1}%)",
                            bytes_to_gb(data.memory_used),
                            data.memory_percentage
                        ),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Free RAM:");
                    ui.strong(format!(
                        "{:.2} GB",
                        bytes_to_gb(data.memory_total.saturating_sub(data.memory_used))
                    ));
                });
                let color = get_usage_color(data.memory_percentage);
                paint_progress_bar(ui, data.memory_percentage / 100.0, color, 8.0);
            });

            ui.add_space(10.0);

            // Manual Clean button
            ui.group(|ui| {
                ui.heading("Manual Clean");
                ui.separator();
                ui.label("Frees up unused RAM by emptying process working sets.");
                ui.label("This is safe and Windows will reload memory as needed.");
                ui.add_space(5.0);

                if privilege::is_app_elevated() {
                    ui.colored_label(ThemePalette::STATUS_HEALTHY, "Running as Administrator: Full memory cleaning enabled.");
                } else {
                    ui.colored_label(ThemePalette::STATUS_WARNING, "Standard Privileges: User processes only. Run as Admin to clean system memory.");
                }
                ui.add_space(5.0);

                let is_cleaning = app.ram_cleaner_state.is_cleaning;
                ui.add_enabled_ui(!is_cleaning, |ui| {
                    if ui
                        .button(egui::RichText::new("🧹 Clean RAM Now").size(16.0).strong())
                        .on_hover_text("Free working sets of all running processes")
                        .clicked()
                    {
                        app.start_ram_clean(ui.ctx());
                    }
                });

                if is_cleaning {
                    ui.colored_label(ThemePalette::ACCENT_PRIMARY, "Cleaning in progress...");
                }
            });

            ui.add_space(10.0);

            // Auto Clean settings
            ui.group(|ui| {
                ui.heading("Auto Clean");
                ui.separator();

                let mut settings_changed = false;
                if ui.checkbox(
                    &mut app.ram_cleaner_state.auto_clean_enabled,
                    "Enable automatic RAM cleaning",
                ).changed() {
                    app.settings.auto_ram_clean = app.ram_cleaner_state.auto_clean_enabled;
                    settings_changed = true;
                }

                if app.ram_cleaner_state.auto_clean_enabled {
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.label("Clean when RAM usage exceeds:");
                        if ui.add(
                            egui::Slider::new(&mut app.ram_cleaner_state.auto_clean_threshold, 50.0..=95.0)
                                .suffix("%"),
                        ).changed() {
                            app.settings.ram_clean_threshold = app.ram_cleaner_state.auto_clean_threshold;
                            settings_changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Minimum interval between cleans:");
                        if ui.add(
                            egui::Slider::new(&mut app.ram_cleaner_state.auto_clean_interval, 60..=1800)
                                .suffix(" sec"),
                        ).changed() {
                            app.settings.auto_clean_interval = app.ram_cleaner_state.auto_clean_interval;
                            settings_changed = true;
                        }
                    });
                }

                if settings_changed {
                    let _ = app.settings.save();
                }
            });

            ui.add_space(10.0);

            // Statistics
            ui.group(|ui| {
                ui.heading("Cleaning Statistics");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Total cleans this session:");
                    ui.strong(format!("{}", app.ram_cleaner_state.clean_count));
                });

                ui.horizontal(|ui| {
                    ui.label("Total RAM freed this session:");
                    ui.strong(format!("{:.2} MB", bytes_to_mb(app.ram_cleaner_state.bytes_freed)));
                });

                ui.horizontal(|ui| {
                    ui.label("Last cleaned:");
                    if app.ram_cleaner_state.last_cleaned.is_some() {
                        ui.strong(&app.ram_cleaner_state.last_cleaned_display);
                    } else {
                        ui.label("Never");
                    }
                });
            });
        });
    }
