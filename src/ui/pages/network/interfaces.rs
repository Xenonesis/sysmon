use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn format_bytes_human(bytes: u64) -> String {
    let mb = bytes_to_mb(bytes);
    if mb >= 1024.0 {
        format!("{:.2} GB", mb / 1024.0)
    } else {
        format!("{:.2} MB", mb)
    }
}

/// Renders the global network throughput history plots (Download & Upload).
pub(crate) fn paint_network_throughput_history(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    let total_rx_rate: f64 = data.network_info.iter().map(|n| n.received_rate).sum();
    let total_tx_rate: f64 = data.network_info.iter().map(|n| n.transmitted_rate).sum();

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("NETWORK THROUGHPUT HISTORY")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "↓ {}   ↑ {}",
                        format_rate(total_rx_rate),
                        format_rate(total_tx_rate)
                    ))
                    .monospace()
                    .strong()
                    .color(ThemePalette::text_primary(is_dark)),
                );
            });
        });

        ui.add_space(8.0);
        ui.columns(2, |cols| {
            // Download Graph
            cols[0].vertical(|ui| {
                ui.label(
                    egui::RichText::new("DOWNLOAD THROUGHPUT (MB/s)")
                        .size(10.0)
                        .strong()
                        .color(ThemePalette::STATUS_HEALTHY),
                );
                let download_points: PlotPoints = data
                    .network_download_history
                    .iter()
                    .map(|p| [p.time, p.value])
                    .collect();

                let line = Line::new(download_points)
                    .color(ThemePalette::STATUS_HEALTHY)
                    .width(1.5);

                Plot::new("network_download_plot_page")
                    .height(140.0)
                    .allow_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .y_axis_label("MB/s")
                    .show(ui, |plot_ui| {
                        plot_ui.line(line);
                    });
            });

            // Upload Graph
            cols[1].vertical(|ui| {
                ui.label(
                    egui::RichText::new("UPLOAD THROUGHPUT (MB/s)")
                        .size(10.0)
                        .strong()
                        .color(ThemePalette::STATUS_WARNING),
                );
                let upload_points: PlotPoints = data.network_upload_history.iter().map(|p| [p.time, p.value]).collect();

                let line = Line::new(upload_points).color(ThemePalette::STATUS_WARNING).width(1.5);

                Plot::new("network_upload_plot_page")
                    .height(140.0)
                    .allow_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .y_axis_label("MB/s")
                    .show(ui, |plot_ui| {
                        plot_ui.line(line);
                    });
            });
        });
    });
}

/// Renders the network adapters & interfaces list.
pub(crate) fn paint_network_interfaces(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("NETWORK ADAPTERS & INTERFACES")
                .size(11.0)
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{} adapter(s) detected", data.network_info.len()))
                    .monospace()
                    .size(11.0)
                    .color(ThemePalette::text_dimmed(is_dark)),
            );
        });
    });

    ui.add_space(6.0);

    for network in &data.network_info {
        let is_active = network.received_rate > 0.001 || network.transmitted_rate > 0.001;

        card_frame(is_dark).show(ui, |ui| {
            // Header row
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&network.interface)
                        .strong()
                        .size(13.5)
                        .color(ThemePalette::text_primary(is_dark)),
                );

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
                    let rx_color = if network.received_rate > 10.0 {
                        ThemePalette::STATUS_CRITICAL
                    } else if network.received_rate > 1.0 {
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
                    let tx_color = if network.transmitted_rate > 10.0 {
                        ThemePalette::STATUS_CRITICAL
                    } else if network.transmitted_rate > 1.0 {
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

        ui.add_space(8.0);
    }
}
