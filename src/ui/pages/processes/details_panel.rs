use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

/// Renders the detailed process inspection card for the currently selected process.
pub(super) fn paint_process_details_card(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    pid: &crate::processes::ProcessIdentity,
    details: &crate::processes::ProcessDetails,
    is_dark: bool,
) {
    ui.add_space(12.0);
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading(
                egui::RichText::new(format!("Process Details — PID {}", pid.pid))
                    .color(ThemePalette::text_primary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("× Close").clicked() {
                    app.details_pid = None;
                }
            });
        });
        ui.separator();
        egui::Grid::new("process_details_grid")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                details_row(ui, "Executable", details.exe_path.as_deref().unwrap_or("N/A"), is_dark);
                details_row(ui, "Command Line", &details.command_line, is_dark);
                details_row(
                    ui,
                    "Working Directory",
                    details.cwd.as_deref().unwrap_or("N/A"),
                    is_dark,
                );
                details_row(ui, "Started", &format_started(details.start_time), is_dark);
                details_row(
                    ui,
                    "Run Time",
                    &format!("{}m {}s", details.run_time / 60, details.run_time % 60),
                    is_dark,
                );
                details_row(
                    ui,
                    "Parent PID",
                    &details
                        .parent_pid
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "—".to_string()),
                    is_dark,
                );
                details_row(
                    ui,
                    "Parent Name",
                    details.parent_name.as_deref().unwrap_or("—"),
                    is_dark,
                );
            });
    });
}
