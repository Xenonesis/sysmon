use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::SystemData;
use eframe::egui;

/// Renders the active TCP/UDP sockets and process connections table with search filter toolbar.
pub(crate) fn paint_socket_connections(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    data: &SystemData,
    is_dark: bool,
) {
    let all_conns = &data.socket_connections;
    let filtered_conns = crate::network::filter_connections(all_conns, &app.network_socket_search);

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("ACTIVE NETWORK CONNECTIONS & PROCESS SOCKETS")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_pill(
                    ui,
                    &format!("{} SOCKETS", filtered_conns.len()),
                    ThemePalette::ACCENT_PRIMARY,
                    is_dark,
                );
            });
        });

        ui.add_space(8.0);

        // Search filter toolbar
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Filter:")
                    .size(11.5)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.network_socket_search)
                    .hint_text("Filter by PID, Port, Remote IP, Process...")
                    .desired_width(260.0),
            );
            if !app.network_socket_search.is_empty() && ui.small_button("×").clicked() {
                app.network_socket_search.clear();
            }
        });

        ui.add_space(6.0);

        // Responsive Socket Table
        egui::Grid::new("network_sockets_grid")
            .striped(true)
            .spacing([14.0, 5.0])
            .min_col_width(60.0)
            .show(ui, |ui| {
                ui.strong("Proto");
                ui.strong("Local Address");
                ui.strong("Remote Address");
                ui.strong("State");
                ui.strong("PID");
                ui.strong("Process Name");
                ui.end_row();

                for conn in filtered_conns.iter().take(150) {
                    let proto_color = if conn.protocol == "TCP" {
                        ThemePalette::ACCENT_PRIMARY
                    } else {
                        ThemePalette::ACCENT_ACTIVE
                    };
                    ui.label(
                        egui::RichText::new(conn.protocol)
                            .monospace()
                            .strong()
                            .color(proto_color),
                    );
                    ui.label(egui::RichText::new(&conn.local_addr).monospace());
                    ui.label(egui::RichText::new(&conn.remote_addr).monospace());

                    let state_color = match conn.state {
                        "ESTABLISHED" => ThemePalette::STATUS_HEALTHY,
                        "LISTEN" => ThemePalette::ACCENT_PRIMARY,
                        "TIME_WAIT" | "CLOSE_WAIT" => ThemePalette::STATUS_WARNING,
                        _ => ThemePalette::text_dimmed(is_dark),
                    };
                    ui.label(egui::RichText::new(conn.state).monospace().color(state_color));
                    ui.label(egui::RichText::new(conn.pid.to_string()).monospace());

                    let proc_display = conn.process_name.as_deref().unwrap_or("—");
                    ui.label(
                        egui::RichText::new(proc_display)
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.end_row();
                }
            });
    });
}
