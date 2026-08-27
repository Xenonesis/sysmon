pub(crate) mod plots;
pub(crate) mod rate_summary;

use crate::SystemData;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

/// Coordinator function for rendering the Performance page.
pub(crate) fn show(app: &crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Performance Graphs", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Telemetry Window Summary Grid ──
        rate_summary::paint_history_summary(ui, data, is_dark);
        ui.add_space(10.0);

        // ── 2. Time-Series Performance Plots ──
        if app.settings.show_graphs {
            plots::paint_performance_plots(ui, data, is_dark);
        } else {
            card_frame(is_dark).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Performance graphs are currently disabled. Enable them in Settings.")
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
    fn test_performance_page_render_headless() {
        let app = crate::SystemMonitorApp::test_app();

        let mut cpu_history = crate::monitoring::history::BoundedHistory::new(60);
        cpu_history.push(DataPoint { time: 0.0, value: 20.0 });
        cpu_history.push(DataPoint { time: 1.0, value: 35.0 });
        cpu_history.push(DataPoint { time: 2.0, value: 42.5 });

        let mut memory_history = crate::monitoring::history::BoundedHistory::new(60);
        memory_history.push(DataPoint { time: 0.0, value: 65.0 });
        memory_history.push(DataPoint { time: 1.0, value: 67.0 });
        memory_history.push(DataPoint { time: 2.0, value: 68.0 });

        let mut gpu_history = crate::monitoring::history::BoundedHistory::new(60);
        gpu_history.push(DataPoint { time: 0.0, value: 10.0 });
        gpu_history.push(DataPoint { time: 1.0, value: 25.0 });

        let mut cpu_temp_history = crate::monitoring::history::BoundedHistory::new(60);
        cpu_temp_history.push(DataPoint { time: 0.0, value: 50.0 });
        cpu_temp_history.push(DataPoint { time: 1.0, value: 55.2 });

        let mut stats = std::collections::HashMap::new();
        stats.insert(
            "cpu.global_usage".to_string(),
            crate::telemetry::HistoryStats::default(),
        );
        stats.insert("memory.used".to_string(), crate::telemetry::HistoryStats::default());
        stats.insert(
            "gpu.0.utilization".to_string(),
            crate::telemetry::HistoryStats::default(),
        );

        let data = SystemData {
            cpu_usage: 42.5,
            memory_percentage: 68.0,
            cpu_temperature: Some(55.2),
            disk_read_rate: 12.5,
            disk_write_rate: 3.2,
            cpu_history,
            memory_history,
            gpu_history,
            disk_read_history: VecDeque::from([
                DataPoint { time: 0.0, value: 5.0 },
                DataPoint { time: 1.0, value: 12.5 },
            ]),
            disk_write_history: VecDeque::from([
                DataPoint { time: 0.0, value: 1.0 },
                DataPoint { time: 1.0, value: 3.2 },
            ]),
            network_download_history: VecDeque::from([
                DataPoint { time: 0.0, value: 0.5 },
                DataPoint { time: 1.0, value: 2.4 },
            ]),
            network_upload_history: VecDeque::from([
                DataPoint { time: 0.0, value: 0.1 },
                DataPoint { time: 1.0, value: 0.8 },
            ]),
            cpu_temp_history,
            telemetry_history_stats: stats,
            ..Default::default()
        };

        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                show(&app, ui, &data);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_performance_page_disabled_graphs() {
        let mut app = crate::SystemMonitorApp::test_app();
        app.settings.show_graphs = false;
        let data = SystemData::default();

        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                show(&app, ui, &data);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_subcomponents_direct() {
        let data = SystemData::default();
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                rate_summary::paint_history_summary(ui, &data, true);
                rate_summary::paint_history_summary(ui, &data, false);
                plots::paint_performance_plots(ui, &data, true);
                plots::paint_performance_plots(ui, &data, false);
            });
        })
        .textures_delta
        .clear();
    }
}
