use super::interfaces::format_bytes_human;
use crate::NetworkInfo;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

/// Renders a single network adapter card with status pills and transmission telemetry.
pub(super) fn paint_adapter_card(ui: &mut egui::Ui, network: &NetworkInfo, is_dark: bool) {
    let is_active = network.received_rate > 1024.0 || network.transmitted_rate > 1024.0;

    card_frame(is_dark).show(ui, |ui| {
        // Header row
        ui.horizontal(|ui| {
            let icon = if network.interface.to_lowercase().contains("wi-fi")
                || network.interface.to_lowercase().contains("wireless")
            {
                "📶"
            } else {
                "🌐"
            };

            ui.label(
                egui::RichText::new(format!("{icon} {}", network.interface))
                    .strong()
                    .size(13.5)
                    .color(ThemePalette::text_primary(is_dark)),
            );

            ui.add_space(4.0);

            if is_active {
                status_pill(ui, "ACTIVE", ThemePalette::STATUS_HEALTHY, is_dark);
            } else {
                status_pill(ui, "IDLE", ThemePalette::text_dimmed(is_dark), is_dark);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "↓ {}   ↑ {}",
                        format_rate(network.received_rate),
                        format_rate(network.transmitted_rate)
                    ))
                    .monospace()
                    .strong()
                    .size(12.5)
                    .color(if is_active {
                        ThemePalette::text_primary(is_dark)
                    } else {
                        ThemePalette::text_dimmed(is_dark)
                    }),
                );
            });
        });

        ui.add_space(8.0);

        // Monospace Metrics Grid
        egui::Grid::new(format!("net_grid_{}", network.interface))
            .num_columns(4)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Download Rate:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                let rx_color = if network.received_rate > 10.0 * 1_048_576.0 {
                    ThemePalette::STATUS_CRITICAL
                } else if network.received_rate > 1_048_576.0 {
                    ThemePalette::STATUS_WARNING
                } else {
                    ThemePalette::STATUS_HEALTHY
                };
                ui.label(
                    egui::RichText::new(format_rate(network.received_rate))
                        .monospace()
                        .strong()
                        .color(rx_color),
                );

                ui.label(
                    egui::RichText::new("Total Received:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format_bytes_human(network.received))
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.end_row();

                ui.label(
                    egui::RichText::new("Upload Rate:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                let tx_color = if network.transmitted_rate > 10.0 * 1_048_576.0 {
                    ThemePalette::STATUS_CRITICAL
                } else if network.transmitted_rate > 1_048_576.0 {
                    ThemePalette::STATUS_WARNING
                } else {
                    ThemePalette::STATUS_HEALTHY
                };
                ui.label(
                    egui::RichText::new(format_rate(network.transmitted_rate))
                        .monospace()
                        .strong()
                        .color(tx_color),
                );

                ui.label(
                    egui::RichText::new("Total Transmitted:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(
                    egui::RichText::new(format_bytes_human(network.transmitted))
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                );
                ui.end_row();
            });
    });
}
