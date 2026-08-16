use crate::ui::theme::ThemePalette;
use eframe::egui;

/// Modern section header with strong typography and flat Diagnostic Emerald accent underline.
pub(crate) fn paint_section_header(ui: &mut egui::Ui, text: &str, is_dark: bool) {
    ui.add_space(4.0);
    let r = ui.label(
        egui::RichText::new(text)
            .text_style(egui::TextStyle::Heading)
            .strong()
            .color(ThemePalette::text_primary(is_dark)),
    );
    let y = r.rect.bottom() + 4.0;

    let underline_w = r.rect.width();
    ui.painter().line_segment(
        [egui::pos2(r.rect.left(), y), egui::pos2(r.rect.left() + underline_w, y)],
        egui::Stroke::new(3.0, ThemePalette::ACCENT_PRIMARY),
    );
    ui.add_space(12.0);
}

/// Precision circular telemetry gauge with anti-aliased background track, smooth arc geometry,
/// and centered monospace percentage readout.
pub(crate) fn paint_circular_gauge(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    radius: f32,
    fraction: f32,
    color: egui::Color32,
    is_dark: bool,
) {
    let p = ui.painter();
    let track_color = ThemePalette::bg_track(is_dark);
    let stroke_width = 4.5;

    // Background track circle
    p.circle_stroke(center, radius, egui::Stroke::new(stroke_width, track_color));

    // Progress arc
    let frac = fraction.clamp(0.0, 1.0);
    if frac > 0.005 {
        use std::f32::consts::PI;
        let start_angle = -PI / 2.0;
        let sweep = frac * 2.0 * PI;

        // Subdivide arc points smoothly
        let num_points = ((sweep * 24.0) as usize).clamp(12, 64);
        let path: Vec<egui::Pos2> = (0..=num_points)
            .map(|i| {
                let t = i as f32 / num_points as f32;
                let angle = start_angle + sweep * t;
                center + egui::vec2(angle.cos() * radius, angle.sin() * radius)
            })
            .collect();

        p.add(egui::Shape::line(path, egui::Stroke::new(stroke_width, color)));
    }

    // Centered monospace percentage readout
    let pct_text = format!("{:.0}%", (frac * 100.0).round());
    let font_size = (radius * 0.44).clamp(10.0, 18.0);
    p.text(
        center,
        egui::Align2::CENTER_CENTER,
        pct_text,
        egui::FontId::monospace(font_size),
        ThemePalette::text_primary(is_dark),
    );
}

/// Structured card frame container with 1px border and 6.0px rounding.
#[allow(dead_code)]
pub(crate) fn card_frame(is_dark: bool) -> egui::Frame {
    egui::Frame::none()
        .fill(ThemePalette::bg_surface(is_dark))
        .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
}

/// Flat status indicator pill with tinted background, 1px border, and bold uppercase text.
#[allow(dead_code)]
pub(crate) fn status_pill(ui: &mut egui::Ui, label: &str, color: egui::Color32, is_dark: bool) {
    let frame = egui::Frame::none()
        .fill(color.gamma_multiply(if is_dark { 0.15 } else { 0.12 }))
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.4)))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(8.0, 3.0));
    frame.show(ui, |ui| {
        ui.label(egui::RichText::new(label).size(11.0).strong().color(color));
    });
}

/// Linear progress bar with anti-aliased track and fill.
pub(crate) fn paint_progress_bar(ui: &mut egui::Ui, fraction: f32, fill: egui::Color32, h: f32, is_dark: bool) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let rnd = (h / 2.0).min(3.0);

    // Track background
    ui.painter().rect_filled(rect, rnd, ThemePalette::bg_deepest(is_dark));
    ui.painter()
        .rect_stroke(rect, rnd, egui::Stroke::new(1.0, ThemePalette::bg_track(is_dark)));

    let frac = fraction.clamp(0.0, 1.0);
    if frac > 0.005 {
        let bar = egui::Rect::from_min_size(rect.min, egui::vec2(w * frac, h));
        ui.painter().rect_filled(bar, rnd, fill);
    }
}

/// Monospace-aligned details grid row.
pub(crate) fn details_row(ui: &mut egui::Ui, label: &str, value: &str, is_dark: bool) {
    ui.label(
        egui::RichText::new(label)
            .strong()
            .color(ThemePalette::text_secondary(is_dark)),
    );
    ui.label(
        egui::RichText::new(value)
            .monospace()
            .color(ThemePalette::text_primary(is_dark)),
    );
    ui.end_row();
}

/// Semantic threshold color mapping (<70% Emerald, 70-90% Amber, >90% Red).
pub(crate) fn get_usage_color(percentage: f32) -> egui::Color32 {
    if percentage < 70.0 {
        ThemePalette::STATUS_HEALTHY
    } else if percentage < 90.0 {
        ThemePalette::STATUS_WARNING
    } else {
        ThemePalette::STATUS_CRITICAL
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

pub(crate) fn format_started(epoch_secs: u64) -> String {
    chrono::DateTime::from_timestamp(epoch_secs as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "N/A".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_and_byte_formatting_are_monospace_safe() {
        assert_eq!(format_rate(0.0), "0 B/s");
        assert_eq!(format_rate(0.0005), "524 B/s");
        assert_eq!(format_rate(0.5), "512 KB/s");
        assert_eq!(format_rate(1.5), "1.50 MB/s");
        assert_eq!(format_rate(1024.0), "1.00 GB/s");

        assert_eq!(bytes_to_mb(1_048_576), 1.0);
        assert_eq!(bytes_to_gb(1_073_741_824), 1.0);
    }

    #[test]
    fn usage_color_thresholds_match_spec() {
        assert_eq!(get_usage_color(0.0), ThemePalette::STATUS_HEALTHY);
        assert_eq!(get_usage_color(50.0), ThemePalette::STATUS_HEALTHY);
        assert_eq!(get_usage_color(69.9), ThemePalette::STATUS_HEALTHY);
        assert_eq!(get_usage_color(70.0), ThemePalette::STATUS_WARNING);
        assert_eq!(get_usage_color(85.0), ThemePalette::STATUS_WARNING);
        assert_eq!(get_usage_color(89.9), ThemePalette::STATUS_WARNING);
        assert_eq!(get_usage_color(90.0), ThemePalette::STATUS_CRITICAL);
        assert_eq!(get_usage_color(100.0), ThemePalette::STATUS_CRITICAL);
    }

    #[test]
    fn card_frame_properties() {
        let dark_frame = card_frame(true);
        assert_eq!(dark_frame.fill, ThemePalette::bg_surface(true));
        assert_eq!(dark_frame.stroke.color, ThemePalette::border(true));
        assert_eq!(dark_frame.rounding, egui::Rounding::same(6.0));

        let light_frame = card_frame(false);
        assert_eq!(light_frame.fill, ThemePalette::bg_surface(false));
        assert_eq!(light_frame.stroke.color, ThemePalette::border(false));
        assert_eq!(light_frame.rounding, egui::Rounding::same(6.0));
    }

    #[test]
    fn format_started_test() {
        assert_eq!(format_started(0), "1970-01-01 00:00:00");
    }

    #[test]
    fn ui_components_render_without_panic() {
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                paint_section_header(ui, "Test Header", true);
                paint_section_header(ui, "Test Header Light", false);
                paint_circular_gauge(ui, egui::pos2(50.0, 50.0), 20.0, 0.45, egui::Color32::GREEN, true);
                paint_circular_gauge(ui, egui::pos2(150.0, 50.0), 20.0, 0.95, egui::Color32::RED, false);
                status_pill(ui, "ACTIVE", egui::Color32::GREEN, true);
                status_pill(ui, "STOPPED", egui::Color32::GRAY, false);
                paint_progress_bar(ui, 0.65, egui::Color32::BLUE, 6.0, true);
                paint_progress_bar(ui, 0.25, egui::Color32::BLUE, 6.0, false);
                egui::Grid::new("test_grid").show(ui, |ui| {
                    details_row(ui, "Key", "Value", true);
                    details_row(ui, "Key2", "Value2", false);
                });
            });
        });
    }
}
