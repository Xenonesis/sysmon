use crate::processes::ProcessSortColumn;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;
pub(super) fn paint_process_toolbar(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    filtered_count: usize,
    total_count: usize,
    is_dark: bool,
) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            // Search Input with integrated Clear button
            ui.label(
                egui::RichText::new("Search:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.process_search)
                    .hint_text("Filter by name or PID...")
                    .desired_width(180.0),
            );
            if !app.process_search.is_empty() && ui.small_button("×").on_hover_text("Clear search filter").clicked() {
                app.process_search.clear();
            }

            ui.add_space(8.0);

            // Process count badge
            let count_label = format!("Showing {} / {}", filtered_count, total_count);
            status_pill(ui, &count_label, ThemePalette::ACCENT_PRIMARY, is_dark);

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.label(
                egui::RichText::new("Sort:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );

            let sort_options = [
                ("PID", ProcessSortColumn::Pid),
                ("Name", ProcessSortColumn::Name),
                ("Memory", ProcessSortColumn::Memory),
                ("VRAM", ProcessSortColumn::Vram),
                ("CPU", ProcessSortColumn::Cpu),
                ("Disk", ProcessSortColumn::Disk),
            ];

            let current_label = match app.process_sort_column {
                ProcessSortColumn::Pid => "PID",
                ProcessSortColumn::Name => "Name",
                ProcessSortColumn::Memory => "Memory",
                ProcessSortColumn::Vram => "VRAM",
                ProcessSortColumn::Cpu => "CPU",
                ProcessSortColumn::Disk => "Disk",
            };

            egui::ComboBox::from_id_salt("process_toolbar_sort_combo")
                .selected_text(current_label)
                .width(80.0)
                .show_ui(ui, |ui| {
                    for (label, col) in sort_options {
                        let is_selected = app.process_sort_column == col;
                        if ui.selectable_label(is_selected, label).clicked() {
                            if app.process_sort_column == col {
                                app.process_sort_ascending = !app.process_sort_ascending;
                            } else {
                                app.process_sort_column = col;
                                app.process_sort_ascending =
                                    matches!(col, ProcessSortColumn::Pid | ProcessSortColumn::Name);
                            }
                        }
                    }
                });

            let dir_icon = if app.process_sort_ascending { "▲" } else { "▼" };
            if ui
                .small_button(dir_icon)
                .on_hover_text("Toggle sort order (Ascending / Descending)")
                .clicked()
            {
                app.process_sort_ascending = !app.process_sort_ascending;
            }

            // Right-aligned management actions
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui::RichText::new("Export JSON").strong())
                    .on_hover_text("Export current process list to JSON")
                    .clicked()
                {
                    app.show_export = true;
                }
                if ui
                    .button(egui::RichText::new("Export CSV").strong())
                    .on_hover_text("Export current process list to CSV")
                    .clicked()
                {
                    app.show_export_csv = true;
                }
                if ui
                    .button(egui::RichText::new("Full Process Manager").strong())
                    .on_hover_text("Open advanced window with Kill, Suspend & Priority controls")
                    .clicked()
                {
                    app.show_process_manager = true;
                }
            });
        });
    });
}
