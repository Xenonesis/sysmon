use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Renders the Live Metric Proximity & Headroom Matrix card.
pub(super) fn paint_proximity_matrix(
    app: &crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    data: &SystemData,
    is_dark: bool,
) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("LIVE THRESHOLD PROXIMITY & HEADROOM")
                    .size(11.5)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_pill(ui, "REAL-TIME GAUGES", ThemePalette::ACCENT_PRIMARY, is_dark);
            });
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Live sensor telemetry evaluated against automated guard thresholds:")
                .size(12.0)
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(8.0);

        // 1. CPU Saturation Metric
        let cpu_curr = data.cpu_usage;
        let cpu_thresh = app.settings.notification_cpu_threshold;
        let cpu_headroom = (cpu_thresh - cpu_curr).max(0.0);
        let cpu_color = if cpu_curr >= cpu_thresh {
            ThemePalette::STATUS_CRITICAL
        } else if cpu_curr >= cpu_thresh * 0.85 {
            ThemePalette::STATUS_WARNING
        } else {
            ThemePalette::STATUS_HEALTHY
        };

        paint_proximity_row(
            ui,
            ProximityRow {
                title: "CPU Saturation Limit",
                current_label: "Current Load:",
                current_value: format!("{:.1}%", cpu_curr),
                threshold_value: format!("> {:.0}%", cpu_thresh),
                threshold_fraction: cpu_thresh / 100.0,
                headroom: format!("+{:.1}% Headroom", cpu_headroom),
                fraction: cpu_curr / 100.0,
                color: cpu_color,
            },
            is_dark,
        );

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // 2. Memory Exhaustion Metric
        let mem_curr = data.memory_percentage;
        let mem_thresh = app.settings.notification_memory_threshold;
        let mem_headroom = (mem_thresh - mem_curr).max(0.0);
        let mem_color = if mem_curr >= mem_thresh {
            ThemePalette::STATUS_CRITICAL
        } else if mem_curr >= mem_thresh * 0.85 {
            ThemePalette::STATUS_WARNING
        } else {
            ThemePalette::STATUS_HEALTHY
        };

        paint_proximity_row(
            ui,
            ProximityRow {
                title: "Memory Exhaustion Limit",
                current_label: "Current Usage:",
                current_value: format!("{:.1}%", mem_curr),
                threshold_value: format!("> {:.0}%", mem_thresh),
                threshold_fraction: mem_thresh / 100.0,
                headroom: format!("+{:.1}% Headroom", mem_headroom),
                fraction: mem_curr / 100.0,
                color: mem_color,
            },
            is_dark,
        );

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // 3. GPU Thermal Boundary
        let gpu_temp_opt = data.gpu_info.first().and_then(|g| g.temperature);
        let gpu_thresh = app.settings.notification_temp_threshold as f32;
        let (gpu_val_str, gpu_headroom_str, gpu_frac, gpu_color) = if let Some(temp) = gpu_temp_opt {
            let temp_f = temp as f32;
            let headroom = (gpu_thresh - temp_f).max(0.0);
            let color = if temp_f >= gpu_thresh {
                ThemePalette::STATUS_CRITICAL
            } else if temp_f >= gpu_thresh * 0.85 {
                ThemePalette::STATUS_WARNING
            } else {
                ThemePalette::STATUS_HEALTHY
            };
            (
                format!("{:.0} °C", temp_f),
                format!("+{:.0} °C Margin", headroom),
                (temp_f / 100.0).clamp(0.0, 1.0),
                color,
            )
        } else {
            (
                "N/A".to_string(),
                "Thermal sensor offline".to_string(),
                0.0,
                ThemePalette::text_dimmed(is_dark),
            )
        };

        paint_proximity_row(
            ui,
            ProximityRow {
                title: "GPU Thermal Boundary",
                current_label: "Current Temp:",
                current_value: gpu_val_str,
                threshold_value: format!("> {:.0} °C", gpu_thresh),
                threshold_fraction: (gpu_thresh / 100.0).clamp(0.0, 1.0),
                headroom: gpu_headroom_str,
                fraction: gpu_frac,
                color: gpu_color,
            },
            is_dark,
        );

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // 4. Disk Space Warning Metric (Highest capacity disk)
        let max_disk_usage = data
            .disk_info
            .iter()
            .map(|d| d.usage_percentage)
            .fold(0.0f32, |acc, val| acc.max(val));
        let disk_thresh = app.settings.notification_disk_threshold;
        let disk_headroom = (disk_thresh - max_disk_usage).max(0.0);
        let disk_color = if max_disk_usage >= disk_thresh {
            ThemePalette::STATUS_CRITICAL
        } else if max_disk_usage >= disk_thresh * 0.85 {
            ThemePalette::STATUS_WARNING
        } else {
            ThemePalette::STATUS_HEALTHY
        };

        paint_proximity_row(
            ui,
            ProximityRow {
                title: "Disk Capacity Warning",
                current_label: "Used Space:",
                current_value: format!("{:.1}%", max_disk_usage),
                threshold_value: format!("> {:.0}%", disk_thresh),
                threshold_fraction: disk_thresh / 100.0,
                headroom: format!("+{:.1}% Threshold Headroom", disk_headroom),
                fraction: max_disk_usage / 100.0,
                color: disk_color,
            },
            is_dark,
        );
    });
}

/// Helper to render an individual metric threshold proximity row with progress bar and headroom badge.
pub(super) struct ProximityRow {
    title: &'static str,
    current_label: &'static str,
    current_value: String,
    threshold_value: String,
    threshold_fraction: f32,
    headroom: String,
    fraction: f32,
    color: egui::Color32,
}

pub(super) fn paint_proximity_row(ui: &mut egui::Ui, row: ProximityRow, is_dark: bool) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(row.title)
                .strong()
                .size(12.0)
                .color(ThemePalette::text_primary(is_dark)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(&row.headroom)
                    .monospace()
                    .size(11.0)
                    .strong()
                    .color(row.color),
            );
        });
    });

    ui.add_space(2.5);
    paint_proximity_bar(ui, row.fraction, row.threshold_fraction, row.color, 6.0, is_dark);
    ui.add_space(2.5);

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{} {}", row.current_label, row.current_value))
                .monospace()
                .size(11.0)
                .color(ThemePalette::text_primary(is_dark)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("Trigger Limit: {}", row.threshold_value))
                    .monospace()
                    .size(11.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
        });
    });
}

pub(super) fn paint_proximity_bar(
    ui: &mut egui::Ui,
    fraction: f32,
    threshold_fraction: f32,
    fill: egui::Color32,
    h: f32,
    is_dark: bool,
) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let rnd = (h / 2.0).min(3.0);

    // Track background
    ui.painter().rect_filled(rect, rnd, ThemePalette::bg_deepest(is_dark));
    ui.painter().rect_stroke(
        rect,
        rnd,
        egui::Stroke::new(1.0, ThemePalette::bg_track(is_dark)),
        egui::StrokeKind::Middle,
    );

    // Fill bar
    let frac = fraction.clamp(0.0, 1.0);
    if frac > 0.005 {
        let bar = egui::Rect::from_min_size(rect.min, egui::vec2(w * frac, h));
        ui.painter().rect_filled(bar, rnd, fill);
    }

    // Subtle threshold reference tick
    let thresh_frac = threshold_fraction.clamp(0.05, 0.98);
    let thresh_x = rect.min.x + w * thresh_frac;
    let thresh_color = if is_dark {
        egui::Color32::from_rgba_unmultiplied(245, 158, 11, 150)
    } else {
        egui::Color32::from_rgba_unmultiplied(217, 119, 6, 170)
    };
    ui.painter().line_segment(
        [
            egui::pos2(thresh_x, rect.min.y - 1.5),
            egui::pos2(thresh_x, rect.max.y + 1.5),
        ],
        egui::Stroke::new(1.5, thresh_color),
    );
}
