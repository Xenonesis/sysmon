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
                    .desired_width(220.0),
            );
            if !app.process_search.is_empty() && ui.small_button("×").on_hover_text("Clear search filter").clicked() {
                app.process_search.clear();
            }

            ui.add_space(8.0);

            // Process count badge
            let count_label = format!("Showing {} / {}", filtered_count, total_count);
            status_pill(ui, &count_label, ThemePalette::ACCENT_PRIMARY, is_dark);

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
