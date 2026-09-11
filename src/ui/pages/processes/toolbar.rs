use crate::processes::ProcessSortColumn;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

// ── Helper buttons (module-level to avoid nested borrow pain) ─────────────────

fn secondary_button(ui: &mut egui::Ui, text: &str, is_dark: bool) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text)
                .size(12.0)
                .strong()
                .color(ThemePalette::text_primary(is_dark)),
        )
        .fill(ThemePalette::bg_surface(is_dark))
        .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
        .corner_radius(egui::CornerRadius::same(4))
        .min_size(egui::vec2(0.0, 28.0)),
    )
}

fn paint_process_action_buttons(ui: &mut egui::Ui, app: &mut crate::SystemMonitorApp, is_dark: bool) {
    if secondary_button(ui, "Full Process Manager", is_dark)
        .on_hover_text("Open advanced window with Kill, Suspend & Priority controls")
        .clicked()
    {
        app.show_process_manager = true;
    }
    ui.add_space(6.0);
    if secondary_button(ui, "Export CSV", is_dark)
        .on_hover_text("Export current process list to CSV")
        .clicked()
    {
        app.show_export_csv = true;
    }
    ui.add_space(6.0);
    if secondary_button(ui, "Export JSON", is_dark)
        .on_hover_text("Export current process list to JSON")
        .clicked()
    {
        app.show_export = true;
    }
}

// ── Toolbar ──────────────────────────────────────────────────────────────────

pub(super) fn paint_process_toolbar(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    filtered_count: usize,
    total_count: usize,
    is_dark: bool,
) {
    card_frame(is_dark).show(ui, |ui| {
        paint_search_row(ui, app, filtered_count, total_count, is_dark);
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            paint_process_action_buttons(ui, app, is_dark);
        });
    });
}

fn paint_search_row(
    ui: &mut egui::Ui,
    app: &mut crate::SystemMonitorApp,
    filtered_count: usize,
    total_count: usize,
    is_dark: bool,
) {
    ui.horizontal_wrapped(|ui| {
        let search_label = ui.label(
            egui::RichText::new("Search:")
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.process_search)
                .hint_text("Filter by name or PID...")
                .desired_width(180.0),
        )
        .labelled_by(search_label.id);
        if !app.process_search.is_empty() && ui.small_button("Clear search").clicked() {
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

        egui::ComboBox::from_label("Process sort")
            .selected_text(current_label)
            .width(80.0)
            .show_ui(ui, |ui| {
                for (label, col) in sort_options {
                    if col == ProcessSortColumn::Vram && !app.settings.show_gpu {
                        continue;
                    }
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

        let direction = if app.process_sort_ascending {
            "Ascending"
        } else {
            "Descending"
        };
        if ui
            .small_button(direction)
            .on_hover_text("Toggle sort order (Ascending / Descending)")
            .clicked()
        {
            app.process_sort_ascending = !app.process_sort_ascending;
        }
    });
}
