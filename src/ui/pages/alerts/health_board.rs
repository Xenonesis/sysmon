use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Renders the Nominal System Health & Reliability Board when zero active alerts are present.
pub(super) fn paint_nominal_health_board(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("SYSTEM HEALTH & GUARD STATUS")
                    .size(11.5)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_pill(ui, "ACTIVE MONITORING", ThemePalette::STATUS_HEALTHY, is_dark);
            });
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Automated diagnostic telemetry engines actively policing hardware parameters:")
                .size(12.0)
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        // Guard Status Items with live operational badges
        let guards = [
            (
                "CPU Thermal & Saturation Guard",
                "Continuous global & per-core throttling watchdog",
                "PASS",
                ThemePalette::STATUS_HEALTHY,
            ),
            (
                "Memory Working Set Watchdog",
                "RAM exhaustion detection & auto-cleanup readiness",
                "PASS",
                ThemePalette::STATUS_HEALTHY,
            ),
            (
                "Storage Volume Exhaustion Sentinel",
                "NTFS & ReFS partition capacity limit watchdog",
                "PASS",
                ThemePalette::STATUS_HEALTHY,
            ),
            (
                "Windows Startup Degradation Scanner",
                "Boot Diagnostics telemetry monitor for rogue startup impact",
                "PASS",
                ThemePalette::STATUS_HEALTHY,
            ),
        ];

        for (name, desc, status_text, status_color) in guards {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("✓ {}", name))
                        .strong()
                        .size(11.5)
                        .color(ThemePalette::STATUS_HEALTHY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_pill(ui, status_text, status_color, is_dark);
                });
            });
            ui.label(
                egui::RichText::new(desc)
                    .size(10.5)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(4.0);
        }

        ui.separator();
        ui.add_space(4.0);

        // Live Health Telemetry Summary Grid (Compact 4-metric 2-row grid)
        ui.label(
            egui::RichText::new("LIVE DIAGNOSTIC TELEMETRY SCOPE")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(4.0);

        egui::Grid::new("health_board_summary_grid")
            .num_columns(4)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Uptime:")
                        .size(11.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format_uptime(data.system_info.uptime))
                        .monospace()
                        .strong()
                        .size(11.0)
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.label(
                    egui::RichText::new("Processes:")
                        .size(11.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format!("{} active", data.top_processes.len()))
                        .monospace()
                        .strong()
                        .size(11.0)
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Network:")
                        .size(11.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format!("{} polled", data.network_info.len()))
                        .monospace()
                        .strong()
                        .size(11.0)
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.label(
                    egui::RichText::new("Volumes:")
                        .size(11.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format!("{} mounted", data.disk_info.len()))
                        .monospace()
                        .strong()
                        .size(11.0)
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.end_row();
            });
    });
}
