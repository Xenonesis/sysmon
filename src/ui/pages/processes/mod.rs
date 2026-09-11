mod row_actions;
mod table;
mod toolbar;

use crate::processes::ProcessSortColumn;
use crate::ui::components::*;
use crate::*;
use eframe::egui;

pub(crate) fn sort_header_label(
    label: &str,
    col: ProcessSortColumn,
    current_col: ProcessSortColumn,
    asc: bool,
) -> String {
    if col == current_col {
        let arrow = if asc { " ▲" } else { " ▼" };
        format!("{}{}", label, arrow)
    } else {
        label.to_string()
    }
}

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Process Monitor", is_dark);
    if !app.settings.show_gpu && app.process_sort_column == ProcessSortColumn::Vram {
        app.process_sort_column = ProcessSortColumn::Memory;
    }

    // Filter and Sort processes upfront
    let mut filtered_processes = processes::filter_processes(&data.top_processes, &app.process_search);
    let ascending = app.process_sort_ascending;
    processes::sort_processes_refs(&mut filtered_processes, app.process_sort_column, ascending);

    egui::ScrollArea::vertical().id_salt("process_page").show(ui, |ui| {
        // ── Integrated Toolbar Container ──
        toolbar::paint_process_toolbar(app, ui, filtered_processes.len(), data.top_processes.len(), is_dark);

        ui.add_space(8.0);

        // ── Responsive Process Table ──
        table::paint_process_table(app, ui, &filtered_processes, data, is_dark);
    });
}
