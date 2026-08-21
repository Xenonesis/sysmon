mod core_bars;
mod hardware_banner;
mod health_deck;
mod metric_cards;
mod top_processes;

use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Calculates row groupings of card indices based on available width.
/// Breakpoints:
/// - Desktop Wide (avail_w >= 1050.0): 5 cards across in 1 row [0, 1, 2, 3, 4]
/// - Standard (700.0 <= avail_w < 1050.0): Row 1 [0, 1, 2], Row 2 [3, 4]
/// - Compact (avail_w < 700.0): Row 1 [0, 1], Row 2 [2, 3], Row 3 [4]
pub(crate) fn calculate_metric_grid_rows(avail_w: f32) -> Vec<Vec<usize>> {
    if avail_w >= 1050.0 {
        vec![vec![0, 1, 2, 3, 4]]
    } else if avail_w >= 700.0 {
        vec![vec![0, 1, 2], vec![3, 4]]
    } else {
        vec![vec![0, 1], vec![2, 3], vec![4]]
    }
}

pub(crate) fn format_uptime(uptime_secs: u64) -> String {
    let d = uptime_secs / 86400;
    let h = (uptime_secs % 86400) / 3600;
    let m = (uptime_secs % 3600) / 60;
    format!("{}d {}h {}m", d, h, m)
}

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "System Overview", is_dark);

    // Show loading state until first telemetry data arrives
    if data.memory_total == 0 {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("Initializing telemetry engines...")
                    .size(14.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Waiting for system telemetry snapshot")
                    .size(11.0)
                    .monospace()
                    .color(ThemePalette::text_dimmed(is_dark)),
            );
        });
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Dynamic Breakpoint-Aware Metric Cards Grid ──
        metric_cards::paint_overview_grid(ui, data, is_dark);
        ui.add_space(4.0);

        // ── 2. Hardware Spec & Uptime Banner ──
        hardware_banner::paint_hardware_banner(ui, data, is_dark);
        ui.add_space(10.0);

        // ── 3. Per-core CPU usage bars (if enabled) ──
        if app.settings.show_per_core_cpu && !data.cpu_cores.is_empty() {
            core_bars::paint_per_core_bars(ui, data, is_dark);
            ui.add_space(10.0);
        }

        // ── 4. Two-Column System Health & Storage Insights Deck ──
        health_deck::paint_health_deck(app, ui, data, is_dark);
        ui.add_space(10.0);

        // ── 5. Top Processes Table Preview ──
        if app.settings.show_processes {
            top_processes::paint_top_processes_table(app, ui, data, is_dark);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_metric_grid_rows_breakpoints() {
        // Desktop Wide >= 1050
        let wide_rows = calculate_metric_grid_rows(1200.0);
        assert_eq!(wide_rows, vec![vec![0, 1, 2, 3, 4]]);

        let edge_wide = calculate_metric_grid_rows(1050.0);
        assert_eq!(edge_wide, vec![vec![0, 1, 2, 3, 4]]);

        // Standard 700..1050
        let std_rows = calculate_metric_grid_rows(900.0);
        assert_eq!(std_rows, vec![vec![0, 1, 2], vec![3, 4]]);

        let edge_std = calculate_metric_grid_rows(700.0);
        assert_eq!(edge_std, vec![vec![0, 1, 2], vec![3, 4]]);

        // Compact < 700
        let compact_rows = calculate_metric_grid_rows(650.0);
        assert_eq!(compact_rows, vec![vec![0, 1], vec![2, 3], vec![4]]);

        let very_compact = calculate_metric_grid_rows(400.0);
        assert_eq!(very_compact, vec![vec![0, 1], vec![2, 3], vec![4]]);
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(0), "0d 0h 0m");
        assert_eq!(format_uptime(59), "0d 0h 0m");
        assert_eq!(format_uptime(60), "0d 0h 1m");
        assert_eq!(format_uptime(3665), "0d 1h 1m");
        assert_eq!(format_uptime(86400 + 7200 + 180), "1d 2h 3m");
        assert_eq!(format_uptime(112500), "1d 7h 15m");
    }

    #[test]
    fn test_overview_ui_render_headless() {
        let mut app = crate::SystemMonitorApp::test_app();
        let mut data = SystemData::default();

        // 1. Initial empty/loading state render
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });

        // 2. Populated telemetry data state render
        data.memory_total = 16 * 1024 * 1024 * 1024;
        data.memory_used = 8 * 1024 * 1024 * 1024;
        data.memory_percentage = 50.0;
        data.cpu_usage = 25.5;
        data.cpu_temperature = Some(45.0);
        data.cpu_cores = vec![
            crate::CpuCoreInfo {
                core_id: 0,
                usage: 30.0,
                name: "Core 0".to_string(),
            },
            crate::CpuCoreInfo {
                core_id: 1,
                usage: 21.0,
                name: "Core 1".to_string(),
            },
        ];
        data.gpu_info = vec![crate::GpuInfo {
            name: "NVIDIA RTX 4070".to_string(),
            utilization: 15.0,
            memory_used: Some(2048 * 1024 * 1024),
            memory_total: Some(12288 * 1024 * 1024),
            temperature: Some(42),
            clock_mhz: Some(2400),
            power_watts: Some(65.0),
            fan_percent: Some(30),
        }];
        data.disk_info = vec![crate::DiskInfo {
            name: "C:".to_string(),
            mount_point: "C:\\".to_string(),
            total_space: 1_000_000_000_000,
            available_space: 400_000_000_000,
            usage_percentage: 60.0,
            file_system: "NTFS".to_string(),
        }];
        data.top_processes = vec![crate::processes::ProcessInfo {
            parent_pid: None,
            pid: 1234,
            start_time: 0,
            name: "sysmon.exe".to_string(),
            cpu_usage: 1.2,
            memory: 128 * 1024 * 1024,
            disk_read_bytes: 1000,
            disk_written_bytes: 2000,
            status: "Running".to_string(),
        }];

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });

        // 3. Compact width render
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 800.0))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(&mut app, ui, &data);
                });
            },
        );
    }
}
