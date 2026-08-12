use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    paint_section_header(ui, "Network Interfaces");

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Network graphs
        if app.settings.show_graphs && !data.network_download_history.is_empty() {
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("Network Activity History")
                        .size(15.0)
                        .strong()
                        .color(ThemePalette::TEXT_PRIMARY),
                );

                // Download graph
                ui.label(
                    egui::RichText::new("Download Rate (MB/s)")
                        .size(12.0)
                        .color(ThemePalette::STATUS_HEALTHY),
                );
                let download_points: PlotPoints = data
                    .network_download_history
                    .iter()
                    .map(|p| [p.time, p.value])
                    .collect();

                let line = Line::new(download_points).color(ThemePalette::STATUS_HEALTHY);

                Plot::new("network_download_plot")
                    .height(150.0)
                    .allow_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .y_axis_label("MB/s")
                    .show(ui, |plot_ui| {
                        plot_ui.line(line);
                    });

                ui.add_space(10.0);

                // Upload graph
                ui.label(
                    egui::RichText::new("Upload Rate (MB/s)")
                        .size(12.0)
                        .color(ThemePalette::ACCENT_PRIMARY),
                );
                let upload_points: PlotPoints = data.network_upload_history.iter().map(|p| [p.time, p.value]).collect();

                let line = Line::new(upload_points).color(ThemePalette::ACCENT_PRIMARY);

                Plot::new("network_upload_plot")
                    .height(150.0)
                    .allow_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .y_axis_label("MB/s")
                    .show(ui, |plot_ui| {
                        plot_ui.line(line);
                    });
            });

            ui.add_space(10.0);
        }

        // Network interfaces list
        for network in &data.network_info {
            ui.group(|ui| {
                ui.heading(&network.interface);
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Total Received:");
                    ui.strong(format!("{:.2} MB", bytes_to_mb(network.received)));
                });

                ui.horizontal(|ui| {
                    ui.label("Total Transmitted:");
                    ui.strong(format!("{:.2} MB", bytes_to_mb(network.transmitted)));
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Download Rate:");
                    let color = if network.received_rate > 10.0 {
                        ThemePalette::STATUS_CRITICAL
                    } else if network.received_rate > 1.0 {
                        ThemePalette::STATUS_WARNING
                    } else {
                        ThemePalette::TEXT_TERTIARY
                    };
                    ui.colored_label(color, format_rate(network.received_rate));
                });

                ui.horizontal(|ui| {
                    ui.label("Upload Rate:");
                    let color = if network.transmitted_rate > 10.0 {
                        ThemePalette::STATUS_CRITICAL
                    } else if network.transmitted_rate > 1.0 {
                        ThemePalette::STATUS_WARNING
                    } else {
                        ThemePalette::TEXT_TERTIARY
                    };
                    ui.colored_label(color, format_rate(network.transmitted_rate));
                });
            });

            ui.add_space(10.0);
        }

        if data.network_info.is_empty() {
            ui.label("No network interfaces detected.");
        }
    });
}
