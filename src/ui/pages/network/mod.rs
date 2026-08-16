pub(crate) mod interfaces;
pub(crate) mod sockets;

use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::SystemData;
use eframe::egui;

/// Coordinator function for rendering the Network Interfaces & Telemetry page.
pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Network Interfaces & Telemetry", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Global Network Traffic History ──
        if app.settings.show_graphs && !data.network_download_history.is_empty() {
            interfaces::paint_network_throughput_history(ui, data, is_dark);
            ui.add_space(10.0);
        }

        // ── 2. Network Interfaces List ──
        interfaces::paint_network_interfaces(ui, data, is_dark);

        // ── 3. Active Sockets & Process Connections Table ──
        ui.add_space(8.0);
        sockets::paint_socket_connections(app, ui, data, is_dark);

        if data.network_info.is_empty() {
            card_frame(is_dark).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("No network interfaces or adapters detected.")
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataPoint;
    use std::collections::VecDeque;

    #[test]
    fn test_format_bytes_human() {
        assert_eq!(interfaces::format_bytes_human(0), "0.00 MB");
        assert_eq!(interfaces::format_bytes_human(1024 * 1024), "1.00 MB");
        assert_eq!(interfaces::format_bytes_human(500 * 1024 * 1024), "500.00 MB");
        assert_eq!(interfaces::format_bytes_human(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(interfaces::format_bytes_human(1536 * 1024 * 1024), "1.50 GB");
        assert_eq!(interfaces::format_bytes_human(10 * 1024 * 1024 * 1024), "10.00 GB");
    }

    #[test]
    fn test_network_page_render_headless() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData {
            network_info: vec![
                crate::NetworkInfo {
                    interface: "Ethernet".to_string(),
                    received: 1024 * 1024 * 1024,
                    transmitted: 512 * 1024 * 1024,
                    received_rate: 1.5,
                    transmitted_rate: 0.8,
                },
                crate::NetworkInfo {
                    interface: "Wi-Fi".to_string(),
                    received: 50 * 1024 * 1024,
                    transmitted: 10 * 1024 * 1024,
                    received_rate: 0.0,
                    transmitted_rate: 0.0,
                },
            ],
            network_download_history: VecDeque::from([
                DataPoint { time: 0.0, value: 0.5 },
                DataPoint { time: 1.0, value: 1.5 },
            ]),
            network_upload_history: VecDeque::from([
                DataPoint { time: 0.0, value: 0.2 },
                DataPoint { time: 1.0, value: 0.8 },
            ]),
            top_processes: vec![crate::processes::ProcessInfo {
                pid: 1001,
                name: "browser.exe".to_string(),
                cpu_usage: 5.0,
                memory: 500 * 1024 * 1024,
                disk_read_bytes: 0,
                disk_written_bytes: 0,
                status: "Running".to_string(),
            }],
            ..Default::default()
        };

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }

    #[test]
    fn test_network_page_empty_interfaces() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData::default();

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }

    #[test]
    fn test_network_page_socket_filter() {
        let mut app = crate::SystemMonitorApp::test_app();
        app.network_socket_search = "127.0.0.1".to_string();
        let data = SystemData::default();

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }

    #[test]
    fn test_network_subcomponents_direct() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData::default();
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                interfaces::paint_network_throughput_history(ui, &data, true);
                interfaces::paint_network_interfaces(ui, &data, true);
                sockets::paint_socket_connections(&mut app, ui, &data, true);
            });
        });
    }
}
