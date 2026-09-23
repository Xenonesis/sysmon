mod manual_clean;
mod policy_config;
mod stats_card;
mod status_card;

use crate::SystemData;
use crate::ui::components::*;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "RAM Cleaner", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Current Memory Status Card ──
        status_card::paint_status_card(ui, data, is_dark);

        ui.add_space(12.0);

        // ── 2. Manual Clean Control Card ──
        manual_clean::paint_manual_clean_card(app, ui, is_dark);

        ui.add_space(12.0);

        // ── 3. Auto Clean Policy Card ──
        policy_config::paint_policy_config_card(app, ui, is_dark);

        ui.add_space(12.0);

        // ── 4. Session Statistics Card ──
        stats_card::paint_stats_card(app, ui, is_dark);
    });
}
