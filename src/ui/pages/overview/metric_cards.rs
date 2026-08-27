use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(super) struct MetricCard {
    pub(super) title: &'static str,
    pub(super) accent: egui::Color32,
    pub(super) value_text: String,
    pub(super) subtitle: String,
    pub(super) fraction: f32,
    pub(super) color: egui::Color32,
    pub(super) status_label: &'static str,
}

pub(super) fn paint_metric_card(ui: &mut egui::Ui, cr: egui::Rect, card: &MetricCard, is_dark: bool) {
    let card_bg = ThemePalette::bg_card(is_dark);
    let card_border = egui::Stroke::new(1.0, ThemePalette::border(is_dark));
    let card_rnd = egui::CornerRadius::same(6);

    // Card background & 1px structural border
    ui.painter().rect_filled(cr, card_rnd, card_bg);
    ui.painter()
        .rect_stroke(cr, card_rnd, card_border, egui::StrokeKind::Middle);

    // Header Row: Accent indicator + Title
    ui.painter()
        .circle_filled(cr.min + egui::vec2(14.0, 14.0), 3.0, card.accent);

    ui.painter().text(
        cr.min + egui::vec2(22.0, 8.0),
        egui::Align2::LEFT_TOP,
        card.title,
        egui::FontId::monospace(10.5),
        ThemePalette::text_secondary(is_dark),
    );

    // Top-Right Status Badge
    let status_bg = card.color.gamma_multiply(if is_dark { 0.15 } else { 0.12 });
    let status_border = egui::Stroke::new(1.0, card.color.gamma_multiply(0.4));
    let badge_rect = egui::Rect::from_min_size(egui::pos2(cr.max.x - 72.0, cr.min.y + 8.0), egui::vec2(60.0, 18.0));
    ui.painter()
        .rect_filled(badge_rect, egui::CornerRadius::same(3), status_bg);
    ui.painter().rect_stroke(
        badge_rect,
        egui::CornerRadius::same(3),
        status_border,
        egui::StrokeKind::Middle,
    );
    ui.painter().text(
        badge_rect.center(),
        egui::Align2::CENTER_CENTER,
        card.status_label,
        egui::FontId::monospace(9.0),
        card.color,
    );

    // Large Bold Primary Telemetry Value
    ui.painter().text(
        cr.min + egui::vec2(14.0, 32.0),
        egui::Align2::LEFT_TOP,
        &card.value_text,
        egui::FontId::monospace(22.0),
        ThemePalette::text_primary(is_dark),
    );

    // Horizontal Precision Load Bar Track
    let bar_margin_x = 14.0;
    let bar_w = cr.width() - (bar_margin_x * 2.0);
    let bar_h = 4.5;
    let bar_y = cr.min.y + 66.0;
    let bar_track_rect =
        egui::Rect::from_min_size(egui::pos2(cr.min.x + bar_margin_x, bar_y), egui::vec2(bar_w, bar_h));
    let bar_rnd = egui::CornerRadius::same(2);

    ui.painter()
        .rect_filled(bar_track_rect, bar_rnd, ThemePalette::bg_deepest(is_dark));
    ui.painter().rect_stroke(
        bar_track_rect,
        bar_rnd,
        egui::Stroke::new(1.0, ThemePalette::bg_track(is_dark)),
        egui::StrokeKind::Middle,
    );

    let filled_w = (bar_w * card.fraction.clamp(0.0, 1.0)).max(2.0);
    let bar_fill_rect = egui::Rect::from_min_size(bar_track_rect.min, egui::vec2(filled_w, bar_h));
    ui.painter().rect_filled(bar_fill_rect, bar_rnd, card.color);

    // Footer Secondary Subtitle
    ui.painter().text(
        cr.min + egui::vec2(14.0, cr.height() - 11.0),
        egui::Align2::LEFT_BOTTOM,
        &card.subtitle,
        egui::FontId::monospace(10.0),
        ThemePalette::text_dimmed(is_dark),
    );
}

pub(super) fn build_overview_cards(data: &SystemData, is_dark: bool) -> [MetricCard; 5] {
    let cpu_c = get_usage_color(data.cpu_usage);
    let mem_c = get_usage_color(data.memory_percentage);

    let net_total_rate = data
        .network_info
        .iter()
        .map(|n| n.received_rate + n.transmitted_rate)
        .sum::<f64>();
    let net_download_rate = data.network_info.iter().map(|n| n.received_rate).sum::<f64>();
    let net_upload_rate = data.network_info.iter().map(|n| n.transmitted_rate).sum::<f64>();
    let net_c = if net_total_rate > 25.0 {
        ThemePalette::STATUS_CRITICAL
    } else if net_total_rate > 5.0 {
        ThemePalette::STATUS_WARNING
    } else if net_total_rate > 0.05 {
        ThemePalette::STATUS_HEALTHY
    } else {
        ThemePalette::text_dimmed(is_dark)
    };

    let disk_total_rate = data.disk_read_rate + data.disk_write_rate;
    let disk_c = if disk_total_rate > 100.0 {
        ThemePalette::STATUS_CRITICAL
    } else if disk_total_rate > 20.0 {
        ThemePalette::STATUS_WARNING
    } else if disk_total_rate > 0.05 {
        ThemePalette::STATUS_HEALTHY
    } else {
        ThemePalette::text_dimmed(is_dark)
    };

    let (gpu_sub, gpu_frac, gpu_c) = if let Some(gpu) = data.gpu_info.first() {
        let c = get_usage_color(gpu.utilization);
        let sub = if let (Some(u), Some(t)) = (gpu.memory_used, gpu.memory_total) {
            format!("{:.0}/{:.0} MB", bytes_to_mb(u), bytes_to_mb(t))
        } else if let Some(mhz) = gpu.clock_mhz {
            format!("{} MHz", mhz)
        } else {
            if gpu.name.chars().count() > 20 {
                let truncated: String = gpu.name.chars().take(18).collect();
                format!("{}…", truncated)
            } else {
                gpu.name.clone()
            }
        };
        (sub, (gpu.utilization / 100.0).clamp(0.0, 1.0), c)
    } else {
        ("Not detected".to_string(), 0.0, ThemePalette::text_dimmed(is_dark))
    };

    let cpu_sub = if let Some(temp) = data.cpu_temperature {
        format!("{} Cores · {:.0}°C", data.cpu_cores.len(), temp)
    } else {
        format!("{} Cores", data.cpu_cores.len())
    };

    [
        MetricCard {
            title: "CPU LOAD",
            accent: ThemePalette::ACCENT_PRIMARY,
            value_text: format!("{:.1}%", data.cpu_usage),
            subtitle: cpu_sub,
            fraction: (data.cpu_usage / 100.0).clamp(0.0, 1.0),
            color: cpu_c,
            status_label: if data.cpu_usage > 90.0 {
                "CRITICAL"
            } else if data.cpu_usage > 70.0 {
                "ELEVATED"
            } else {
                "NOMINAL"
            },
        },
        MetricCard {
            title: "MEMORY",
            accent: ThemePalette::ACCENT_ACTIVE,
            value_text: format!("{:.1}%", data.memory_percentage),
            subtitle: format!(
                "{:.1} / {:.1} GB",
                bytes_to_gb(data.memory_used),
                bytes_to_gb(data.memory_total)
            ),
            fraction: (data.memory_percentage / 100.0).clamp(0.0, 1.0),
            color: mem_c,
            status_label: if data.memory_percentage > 90.0 {
                "CRITICAL"
            } else if data.memory_percentage > 75.0 {
                "ELEVATED"
            } else {
                "NOMINAL"
            },
        },
        MetricCard {
            title: "GPU ENGINE",
            accent: ThemePalette::text_secondary(is_dark),
            value_text: if data.gpu_info.is_empty() {
                "N/A".to_string()
            } else {
                format!("{:.1}%", data.gpu_info[0].utilization)
            },
            subtitle: gpu_sub,
            fraction: gpu_frac,
            color: gpu_c,
            status_label: if data.gpu_info.is_empty() {
                "STANDBY"
            } else if data.gpu_info[0].utilization > 90.0 {
                "CRITICAL"
            } else {
                "ONLINE"
            },
        },
        MetricCard {
            title: "STORAGE I/O",
            accent: ThemePalette::text_secondary(is_dark),
            value_text: format_rate(disk_total_rate),
            subtitle: format!(
                "R: {} · W: {}",
                format_rate(data.disk_read_rate),
                format_rate(data.disk_write_rate)
            ),
            fraction: ((disk_total_rate / 200.0).clamp(0.0, 1.0) as f32),
            color: disk_c,
            status_label: if disk_total_rate > 100.0 {
                "CRITICAL"
            } else if disk_total_rate > 20.0 {
                "ACTIVE"
            } else {
                "IDLE"
            },
        },
        MetricCard {
            title: "NETWORK FLOW",
            accent: ThemePalette::text_secondary(is_dark),
            value_text: format_rate(net_total_rate),
            subtitle: format!(
                "↓ {} · ↑ {}",
                format_rate(net_download_rate),
                format_rate(net_upload_rate)
            ),
            fraction: ((net_total_rate / 10.0).clamp(0.0, 1.0) as f32),
            color: net_c,
            status_label: if net_total_rate > 25.0 {
                "HEAVY"
            } else if net_total_rate > 1.0 {
                "STREAM"
            } else {
                "QUIET"
            },
        },
    ]
}

pub(super) fn paint_overview_grid(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    let avail_w = ui.available_width();
    let cards = build_overview_cards(data, is_dark);

    let card_spacing = 8.0;
    let card_height = 104.0;
    let rows = super::calculate_metric_grid_rows(avail_w);

    for row_indices in rows {
        let count = row_indices.len() as f32;
        let card_w = if count == 1.0 && avail_w < 700.0 {
            (avail_w - card_spacing) / 2.0
        } else {
            (avail_w - card_spacing * (count - 1.0).max(0.0)) / count
        };

        let (row_rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, card_height), egui::Sense::hover());

        for (col_i, &card_i) in row_indices.iter().enumerate() {
            let x = row_rect.min.x + (card_w + card_spacing) * col_i as f32;
            let card_rect = egui::Rect::from_min_size(egui::pos2(x, row_rect.min.y), egui::vec2(card_w, card_height));
            paint_metric_card(ui, card_rect, &cards[card_i], is_dark);
        }
        ui.add_space(card_spacing);
    }
}
