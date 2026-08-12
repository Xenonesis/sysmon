use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(crate) fn paint_section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    let r = ui.label(
        egui::RichText::new(text)
            .text_style(egui::TextStyle::Heading)
            .strong()
            .color(ThemePalette::TEXT_PRIMARY),
    );
    let y = r.rect.bottom() + 4.0;

    // Modern thick rounded line highlight
    let underline_w = r.rect.width();
    ui.painter().line_segment(
        [egui::pos2(r.rect.left(), y), egui::pos2(r.rect.left() + underline_w, y)],
        egui::Stroke::new(3.5, ThemePalette::ACCENT_PRIMARY),
    );
    ui.add_space(12.0);
}

pub(crate) fn details_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).strong());
    ui.label(value);
    ui.end_row();
}

pub(crate) fn format_started(epoch_secs: u64) -> String {
    chrono::DateTime::from_timestamp(epoch_secs as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "N/A".to_string())
}

pub(crate) fn paint_progress_bar(ui: &mut egui::Ui, fraction: f32, fill: egui::Color32, h: f32) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let rnd = h / 2.0;

    // Track background
    ui.painter().rect_filled(rect, rnd, ThemePalette::BG_DEEPEST);
    ui.painter()
        .rect_stroke(rect, rnd, egui::Stroke::new(1.0, ThemePalette::BG_TRACK));

    let frac = fraction.clamp(0.0, 1.0);
    if frac > 0.005 {
        let bar = egui::Rect::from_min_size(rect.min, egui::vec2(w * frac, h));
        ui.painter().rect_filled(bar, rnd, fill);
    }
}

pub(crate) fn paint_circular_gauge(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    radius: f32,
    fraction: f32,
    color: egui::Color32,
    label: &str,
) {
    let p = ui.painter();
    let track_color = ThemePalette::BG_TRACK;

    // Track
    p.circle_stroke(center, radius, egui::Stroke::new(6.0, track_color));

    // Animate fraction if we had time context, but for now we draw the arc
    let frac = fraction.clamp(0.0, 1.0);
    if frac > 0.005 {
        use std::f32::consts::PI;
        // Start from top (-PI/2), sweep clockwise
        let start_angle = -PI / 2.0;
        let end_angle = start_angle + (frac * 2.0 * PI);

        let path: Vec<egui::Pos2> = (0..=30)
            .map(|i| {
                let t = i as f32 / 30.0;
                let angle = start_angle + (end_angle - start_angle) * t;
                center + egui::vec2(angle.cos() * radius, angle.sin() * radius)
            })
            .collect();

        // Outer glow
        p.add(egui::Shape::line(
            path.clone(),
            egui::Stroke::new(12.0, color.linear_multiply(0.15)),
        ));

        // Main arc
        p.add(egui::Shape::line(path, egui::Stroke::new(6.0, color)));
    }

    // Label in center
    let text_color = ThemePalette::TEXT_PRIMARY;
    p.text(
        center,
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(16.0),
        text_color,
    );
}

pub(crate) fn get_usage_color(percentage: f32) -> egui::Color32 {
    if percentage < 50.0 {
        ThemePalette::STATUS_HEALTHY // Mint green (#69f0ae)
    } else if percentage < 75.0 {
        ThemePalette::STATUS_WARNING // Amber (#ffab40)
    } else {
        ThemePalette::STATUS_CRITICAL // Saturated red (#ff5252)
    }
}

pub(crate) fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

pub(crate) fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

pub(crate) fn format_rate(mb_per_sec: f64) -> String {
    let bytes_per_sec = mb_per_sec * 1024.0 * 1024.0;
    if bytes_per_sec >= 1_073_741_824.0 {
        format!("{:.2} GB/s", bytes_per_sec / 1_073_741_824.0)
    } else if bytes_per_sec >= 1_048_576.0 {
        format!("{:.2} MB/s", bytes_per_sec / 1_048_576.0)
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.0} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}
