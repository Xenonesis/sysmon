use crate::privilege;
use crate::startup;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(crate) fn paint_summary_card(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
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
                    app.startup_items_loading = false;
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
}
