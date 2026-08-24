//! Sampling-quality measurements for deterministic tests and hardware soak runs.

use super::TelemetrySnapshot;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default, serde::Serialize)]
pub(crate) struct TelemetryQualityReport {
    pub(crate) samples: u64,
    pub(crate) invalid_values: u64,
    pub(crate) out_of_range_percentages: u64,
    pub(crate) average_gap_ms: f64,
    pub(crate) maximum_gap_ms: u128,
}

#[derive(Default)]
pub(crate) struct TelemetryQualityTracker {
    samples: u64,
    invalid_values: u64,
    out_of_range_percentages: u64,
    last_observed: Option<Instant>,
    total_gap: Duration,
    maximum_gap: Duration,
}

impl TelemetryQualityTracker {
    pub(crate) fn observe(&mut self, observed_at: Instant, snapshot: &TelemetrySnapshot) {
        if let Some(previous) = self.last_observed.replace(observed_at) {
            let gap = observed_at.saturating_duration_since(previous);
            self.total_gap += gap;
            self.maximum_gap = self.maximum_gap.max(gap);
        }
        self.samples += 1;

        for (key, value) in &snapshot.metrics {
            if !value.is_finite() {
                self.invalid_values += 1;
            } else if is_percentage_metric(key) && !(0.0..=100.0).contains(value) {
                self.out_of_range_percentages += 1;
            }
        }
    }

    pub(crate) fn report(&self) -> TelemetryQualityReport {
        let gaps = self.samples.saturating_sub(1);
        TelemetryQualityReport {
            samples: self.samples,
            invalid_values: self.invalid_values,
            out_of_range_percentages: self.out_of_range_percentages,
            average_gap_ms: if gaps == 0 {
                0.0
            } else {
                self.total_gap.as_secs_f64() * 1_000.0 / gaps as f64
            },
            maximum_gap_ms: self.maximum_gap.as_millis(),
        }
    }
}

fn is_percentage_metric(key: &str) -> bool {
    key == "cpu.global_usage"
        || (key.starts_with("cpu.core.") && key.ends_with(".usage"))
        || key.ends_with(".utilization")
        || key.ends_with(".memory_util")
        || key.ends_with(".fan_speed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_sampling_gaps_and_invalid_values() {
        let start = Instant::now();
        let mut tracker = TelemetryQualityTracker::default();
        let mut first = TelemetrySnapshot::default();
        first.metrics.insert("cpu.global_usage".into(), 20.0);
        tracker.observe(start, &first);

        let mut second = TelemetrySnapshot::default();
        second.metrics.insert("cpu.global_usage".into(), 140.0);
        second.metrics.insert("bad.metric".into(), f64::NAN);
        tracker.observe(start + Duration::from_millis(250), &second);

        let report = tracker.report();
        assert_eq!(report.samples, 2);
        assert_eq!(report.invalid_values, 1);
        assert_eq!(report.out_of_range_percentages, 1);
        assert_eq!(report.maximum_gap_ms, 250);
        assert_eq!(report.average_gap_ms, 250.0);
    }
}
