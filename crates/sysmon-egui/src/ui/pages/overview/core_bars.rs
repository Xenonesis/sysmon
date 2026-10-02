use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(super) fn paint_per_core_bars(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("PER-CORE CPU USAGE")
                .size(11.0)
                .monospace()
                .strong()
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(6.0);
        let cols = data.cpu_cores.len().min(16);
        ui.columns(cols, |col_uis| {
            for (i, core) in data.cpu_cores.iter().take(cols).enumerate() {
                let frac = (core.usage / 100.0).clamp(0.0, 1.0);
                let c = get_usage_color(core.usage);
                col_uis[i].label(
                    egui::RichText::new(format!("C{:02}", core.core_id))
                        .size(9.0)
                        .monospace()
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
                paint_progress_bar(&mut col_uis[i], frac, c, 4.0, is_dark);
                col_uis[i].label(
                    egui::RichText::new(format!("{:.0}%", core.usage))
                        .size(9.0)
                        .monospace()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            }
        });
    });
}
