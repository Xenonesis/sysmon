use crate::processes::ProcessSortColumn;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) struct TableLayout {
    pub(super) total_w: f32,
    pub(super) spacing: f32,
    pub(super) pid_w: f32,
    pub(super) name_w: f32,
    pub(super) mem_w: f32,
    pub(super) vram_w: f32,
    pub(super) cpu_w: f32,
    pub(super) disk_read_w: f32,
    pub(super) disk_write_w: f32,
    pub(super) action_w: f32,
    pub(super) show_gpu: bool,
}

impl TableLayout {
    pub(super) fn compute(available_width: f32, show_gpu: bool) -> Self {
        let minimum_width = if show_gpu { 976.0 } else { 893.0 };
        let total_w = available_width.max(minimum_width);
        let spacing = 8.0;
        let pid_w = 60.0;
        let mem_w = 85.0;
        let vram_w = if show_gpu { 75.0 } else { 0.0 };
        let cpu_w = 70.0;
        let disk_read_w = 85.0;
        let disk_write_w = 85.0;
        let action_w = 175.0;

        let fixed_w = pid_w
            + mem_w
            + vram_w
            + cpu_w
            + disk_read_w
            + disk_write_w
            + action_w
            + (if show_gpu { 7.0 } else { 6.0 } * spacing);
        let name_w = (total_w - fixed_w).max(180.0);

        Self {
            total_w,
            spacing,
            pid_w,
            name_w,
            mem_w,
            vram_w,
            cpu_w,
            disk_read_w,
            disk_write_w,
            action_w,
            show_gpu,
        }
    }
}

pub(super) fn paint_table_header(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    layout: &TableLayout,
    is_dark: bool,
) {
    let sort_col = app.process_sort_column;
    let sort_asc = app.process_sort_ascending;

    let header_button = |ui: &mut egui::Ui,
                         label: &str,
                         width: f32,
                         col: ProcessSortColumn,
                         current_col: ProcessSortColumn,
                         asc: bool|
     -> egui::Response {
        let text = super::sort_header_label(label, col, current_col, asc);
        let is_active = col == current_col;
        let text_color = if is_active {
            ThemePalette::ACCENT_PRIMARY
        } else {
            ThemePalette::text_primary(is_dark)
        };
        let btn = egui::Button::new(egui::RichText::new(text).strong().size(11.5).color(text_color))
            .selected(is_active)
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::NONE);
        ui.add_sized([width, 22.0], btn)
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = layout.spacing;

        if header_button(ui, "PID", layout.pid_w, ProcessSortColumn::Pid, sort_col, sort_asc).clicked() {
            if app.process_sort_column == ProcessSortColumn::Pid {
                app.process_sort_ascending = !app.process_sort_ascending;
            } else {
                app.process_sort_column = ProcessSortColumn::Pid;
                app.process_sort_ascending = true;
            }
        }

        if header_button(
            ui,
            "Process Name",
            layout.name_w,
            ProcessSortColumn::Name,
            sort_col,
            sort_asc,
        )
        .clicked()
        {
            if app.process_sort_column == ProcessSortColumn::Name {
                app.process_sort_ascending = !app.process_sort_ascending;
            } else {
                app.process_sort_column = ProcessSortColumn::Name;
                app.process_sort_ascending = true;
            }
        }

        if header_button(
            ui,
            "Memory",
            layout.mem_w,
            ProcessSortColumn::Memory,
            sort_col,
            sort_asc,
        )
        .clicked()
        {
            if app.process_sort_column == ProcessSortColumn::Memory {
                app.process_sort_ascending = !app.process_sort_ascending;
            } else {
                app.process_sort_column = ProcessSortColumn::Memory;
                app.process_sort_ascending = false;
            }
        }

        if layout.show_gpu
            && header_button(ui, "VRAM", layout.vram_w, ProcessSortColumn::Vram, sort_col, sort_asc).clicked()
        {
            if app.process_sort_column == ProcessSortColumn::Vram {
                app.process_sort_ascending = !app.process_sort_ascending;
            } else {
                app.process_sort_column = ProcessSortColumn::Vram;
                app.process_sort_ascending = false;
            }
        }

        if header_button(ui, "CPU %", layout.cpu_w, ProcessSortColumn::Cpu, sort_col, sort_asc).clicked() {
            if app.process_sort_column == ProcessSortColumn::Cpu {
                app.process_sort_ascending = !app.process_sort_ascending;
            } else {
                app.process_sort_column = ProcessSortColumn::Cpu;
                app.process_sort_ascending = false;
            }
        }

        if header_button(
            ui,
            "Disk Read",
            layout.disk_read_w,
            ProcessSortColumn::Disk,
            sort_col,
            sort_asc,
        )
        .clicked()
        {
            if app.process_sort_column == ProcessSortColumn::Disk {
                app.process_sort_ascending = !app.process_sort_ascending;
            } else {
                app.process_sort_column = ProcessSortColumn::Disk;
                app.process_sort_ascending = false;
            }
        }

        ui.add_sized(
            [layout.disk_write_w, 22.0],
            egui::Label::new(
                egui::RichText::new("Disk Write")
                    .strong()
                    .size(11.5)
                    .color(ThemePalette::text_primary(is_dark)),
            ),
        );
        ui.allocate_ui_with_layout(
            egui::vec2(layout.action_w, 22.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(
                    egui::RichText::new("Actions")
                        .strong()
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            },
        );
    });
}
