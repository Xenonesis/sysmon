use crate::ui::components::card_frame;
use crate::ui::theme::ThemePalette;
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};

pub(super) fn paint_utilization_plot(ui: &mut egui::Ui, window: &crate::timeline::TimelineWindow, is_dark: bool) {
    let origin = window.query.start_ms;
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("UTILIZATION")
                .strong()
                .size(11.0)
                .color(ThemePalette::ACCENT_PRIMARY),
        );
        let cpu: PlotPoints = envelope(&window.metrics, origin, |sample| Some(sample.cpu_pct)).into();
        let memory: PlotPoints = envelope(&window.metrics, origin, |sample| Some(sample.memory_pct)).into();
        let has_gpu = window.metrics.iter().any(|sample| sample.gpu_pct.is_some());
        let gpu: PlotPoints = envelope(&window.metrics, origin, |sample| sample.gpu_pct).into();
        Plot::new("timeline_utilization")
            .height(220.0)
            .legend(Legend::default())
            .allow_scroll(false)
            .include_y(0.0)
            .include_y(100.0)
            .x_axis_label("Seconds in selected range")
            .y_axis_label("Usage %")
            .show(ui, |plot| {
                plot.line(Line::new("CPU", cpu).color(ThemePalette::ACCENT_PRIMARY));
                plot.line(Line::new("Memory", memory).color(ThemePalette::STATUS_HEALTHY));
                if has_gpu {
                    plot.line(Line::new("GPU", gpu).color(ThemePalette::STATUS_WARNING));
                }
            });
    });
}

pub(super) fn paint_io_plot(ui: &mut egui::Ui, window: &crate::timeline::TimelineWindow, is_dark: bool) {
    let origin = window.query.start_ms;
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("DISK & NETWORK THROUGHPUT")
                .strong()
                .size(11.0)
                .color(ThemePalette::STATUS_WARNING),
        );
        let disk: PlotPoints = envelope(&window.metrics, origin, |sample| {
            Some((sample.disk_read_bps + sample.disk_write_bps) / 1_048_576.0)
        })
        .into();
        let network: PlotPoints = envelope(&window.metrics, origin, |sample| {
            Some((sample.network_down_bps + sample.network_up_bps) / 1_048_576.0)
        })
        .into();
        Plot::new("timeline_io")
            .height(190.0)
            .legend(Legend::default())
            .allow_scroll(false)
            .x_axis_label("Seconds in selected range")
            .y_axis_label("MB/s")
            .show(ui, |plot| {
                plot.line(Line::new("Disk", disk).color(ThemePalette::STATUS_WARNING));
                plot.line(Line::new("Network", network).color(ThemePalette::ACCENT_PRIMARY));
            });
    });
}

pub(super) fn envelope(
    samples: &[crate::timeline::TimelineMetricSample],
    origin: i64,
    value: impl Fn(&crate::timeline::TimelineMetricSample) -> Option<f64>,
) -> Vec<[f64; 2]> {
    let bucket_size = samples.len().div_ceil(1_000).max(1);
    let mut points = Vec::with_capacity(2_000);
    for bucket in samples.chunks(bucket_size) {
        let mut extrema: Option<([f64; 2], [f64; 2])> = None;
        for sample in bucket {
            let Some(y) = value(sample).filter(|value| value.is_finite()) else {
                continue;
            };
            let point = [seconds_from(origin, sample.timestamp_ms), y];
            match &mut extrema {
                None => extrema = Some((point, point)),
                Some((min, max)) => {
                    if y < min[1] {
                        *min = point;
                    }
                    if y > max[1] {
                        *max = point;
                    }
                }
            }
        }
        if let Some((a, b)) = extrema {
            if a[0] <= b[0] {
                points.extend([a, b]);
            } else {
                points.extend([b, a]);
            }
        }
    }
    points
}

pub(super) fn seconds_from(origin_ms: i64, timestamp_ms: i64) -> f64 {
    timestamp_ms.saturating_sub(origin_ms) as f64 / 1_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::TimelineMetricSample;

    #[test]
    fn test_seconds_from() {
        assert_eq!(seconds_from(1000, 3500), 2.5);
        assert_eq!(seconds_from(5000, 2000), -3.0);
    }

    #[test]
    fn test_envelope_empty() {
        let points = envelope(&[], 0, |_| Some(10.0));
        assert!(points.is_empty());
    }

    #[test]
    fn test_envelope_extrema() {
        let samples = vec![
            TimelineMetricSample {
                timestamp_ms: 1000,
                cpu_pct: 10.0,
                ..Default::default()
            },
            TimelineMetricSample {
                timestamp_ms: 2000,
                cpu_pct: 90.0,
                ..Default::default()
            },
        ];
        let points = envelope(&samples, 1000, |s| Some(s.cpu_pct));
        // When bucket_size is 1, each sample forms its own bucket yielding [min, max] = 2 points per bucket
        assert_eq!(points.len(), 4);
        assert_eq!(points[0], [0.0, 10.0]);
        assert_eq!(points[2], [1.0, 90.0]);
    }
}
