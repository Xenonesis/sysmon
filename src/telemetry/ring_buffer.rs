//! Bounded ring-buffer for metric history with multi-resolution time ranges.
//!
//! Each `MetricHistory` stores a fixed-capacity circular buffer of `MetricPoint`s
//! and maintains running statistics (min, max, average, peak time).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// A single timestamped metric measurement.
#[derive(Clone, Debug)]
pub struct MetricPoint {
    pub timestamp: Instant,
    pub value: f64,
}

/// Running statistics over a metric history window.
#[derive(Clone, Debug, Default)]
pub struct MetricStats {
    pub current: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub peak_time: Option<Instant>,
    pub sample_count: u64,
}

/// A bounded circular buffer of metric points with automatic eviction
/// of the oldest entries when capacity is reached.
#[derive(Clone, Debug)]
pub struct MetricHistory {
    buffer: VecDeque<MetricPoint>,
    capacity: usize,
    stats: MetricStats,
    sum: f64,
}

impl MetricHistory {
    /// Create a new ring buffer with the given maximum capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            stats: MetricStats {
                min: f64::MAX,
                max: f64::MIN,
                ..Default::default()
            },
            sum: 0.0,
        }
    }

    /// Push a new metric value. Automatically evicts the oldest point
    /// if the buffer is at capacity.
    pub fn push(&mut self, value: f64) {
        let now = Instant::now();

        // Evict oldest if at capacity
        if self.buffer.len() >= self.capacity
            && let Some(old) = self.buffer.pop_front()
        {
            self.sum -= old.value;
        }

        self.buffer.push_back(MetricPoint { timestamp: now, value });

        // Update running statistics. Min/max are recalculated from the bounded
        // window so evicted peaks do not leak into current-window summaries.
        self.sum += value;
        self.stats.current = value;
        self.stats.sample_count += 1;
        self.recalculate_window_stats();
    }

    /// Current running statistics.
    pub fn stats(&self) -> &MetricStats {
        &self.stats
    }

    /// Number of points currently stored.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Maximum capacity of this buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Iterate over all stored points (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = &MetricPoint> {
        self.buffer.iter()
    }

    /// Get the most recent N points as (relative_seconds, value) pairs
    /// suitable for plotting.
    pub fn plot_data(&self, max_points: usize) -> Vec<(f64, f64)> {
        let now = Instant::now();
        self.buffer
            .iter()
            .rev()
            .take(max_points)
            .map(|p| {
                let age = now.saturating_duration_since(p.timestamp).as_secs_f64();
                (-age, p.value)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Clear all stored data and reset statistics.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.sum = 0.0;
        self.stats = MetricStats {
            min: f64::MAX,
            max: f64::MIN,
            ..Default::default()
        };
    }

    /// Trim entries older than the given duration from now.
    pub fn trim_older_than(&mut self, max_age: std::time::Duration) {
        let Some(cutoff) = Instant::now().checked_sub(max_age) else {
            return;
        };
        while let Some(front) = self.buffer.front() {
            if front.timestamp < cutoff {
                if let Some(old) = self.buffer.pop_front() {
                    self.sum -= old.value;
                }
            } else {
                break;
            }
        }
        self.recalculate_window_stats();
    }

    fn recalculate_window_stats(&mut self) {
        if self.buffer.is_empty() {
            self.stats.current = 0.0;
            self.stats.min = 0.0;
            self.stats.max = 0.0;
            self.stats.avg = 0.0;
            self.stats.peak_time = None;
            return;
        }

        self.stats.current = self.buffer.back().map_or(0.0, |point| point.value);
        self.stats.avg = self.sum / self.buffer.len() as f64;
        if let Some(minimum) = self.buffer.iter().min_by(|a, b| a.value.total_cmp(&b.value)) {
            self.stats.min = minimum.value;
        }
        if let Some(maximum) = self.buffer.iter().max_by(|a, b| a.value.total_cmp(&b.value)) {
            self.stats.max = maximum.value;
            self.stats.peak_time = Some(maximum.timestamp);
        }
    }
}

/// Pre-configured history buffers for common time ranges.
#[derive(Clone, Debug)]
pub struct MultiResolutionHistory {
    /// Last 60 seconds (~5Hz = 300 points)
    pub short: MetricHistory,
    /// Last 5 minutes (~1Hz = 300 points)
    pub medium: MetricHistory,
    /// Last 30 minutes (~0.2Hz = 360 points)
    pub long: MetricHistory,
    /// Last hour (~0.1Hz = 360 points)
    pub extended: MetricHistory,
    last_medium: Option<Instant>,
    last_long: Option<Instant>,
    last_extended: Option<Instant>,
}

impl MultiResolutionHistory {
    pub fn new() -> Self {
        Self {
            short: MetricHistory::new(300),    // 60s @ 5Hz
            medium: MetricHistory::new(300),   // 5min @ 1Hz
            long: MetricHistory::new(360),     // 30min @ 0.2Hz
            extended: MetricHistory::new(360), // 1hr @ 0.1Hz
            last_medium: None,
            last_long: None,
            last_extended: None,
        }
    }

    /// Push at the native rate and downsample longer windows automatically.
    pub fn push(&mut self, value: f64) {
        let now = Instant::now();
        self.short.push(value);
        if Self::due(self.last_medium, now, Duration::from_secs(1)) {
            self.medium.push(value);
            self.last_medium = Some(now);
        }
        if Self::due(self.last_long, now, Duration::from_secs(5)) {
            self.long.push(value);
            self.last_long = Some(now);
        }
        if Self::due(self.last_extended, now, Duration::from_secs(10)) {
            self.extended.push(value);
            self.last_extended = Some(now);
        }
    }

    fn due(previous: Option<Instant>, now: Instant, interval: Duration) -> bool {
        previous.is_none_or(|sampled| now.saturating_duration_since(sampled) >= interval)
    }
}

impl Default for MultiResolutionHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_stats() {
        let mut h = MetricHistory::new(10);
        h.push(5.0);
        h.push(10.0);
        h.push(3.0);

        assert_eq!(h.len(), 3);
        assert_eq!(h.stats().current, 3.0);
        assert_eq!(h.stats().min, 3.0);
        assert_eq!(h.stats().max, 10.0);
        assert!((h.stats().avg - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn eviction_at_capacity() {
        let mut h = MetricHistory::new(3);
        h.push(1.0);
        h.push(2.0);
        h.push(3.0);
        h.push(4.0); // evicts 1.0

        assert_eq!(h.len(), 3);
        let values: Vec<f64> = h.iter().map(|p| p.value).collect();
        assert_eq!(values, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn clear_resets() {
        let mut h = MetricHistory::new(10);
        h.push(42.0);
        h.clear();

        assert!(h.is_empty());
        assert_eq!(h.stats().sample_count, 0);
    }

    #[test]
    fn plot_data_returns_negative_ages() {
        let mut h = MetricHistory::new(10);
        h.push(1.0);
        h.push(2.0);

        let data = h.plot_data(10);
        assert_eq!(data.len(), 2);
        // Ages should be negative (in the past)
        for (age, _) in &data {
            assert!(*age <= 0.0);
        }
    }

    #[test]
    fn multi_resolution_creates_all_tiers() {
        let mr = MultiResolutionHistory::new();
        assert_eq!(mr.short.capacity(), 300);
        assert_eq!(mr.medium.capacity(), 300);
        assert_eq!(mr.long.capacity(), 360);
        assert_eq!(mr.extended.capacity(), 360);
    }

    #[test]
    fn evicted_peak_no_longer_affects_stats() {
        let mut history = MetricHistory::new(2);
        history.push(100.0);
        history.push(2.0);
        history.push(3.0);
        assert_eq!(history.stats().max, 3.0);
        assert_eq!(history.stats().min, 2.0);
    }

    #[test]
    fn multi_resolution_pushes_short_window_immediately() {
        let mut history = MultiResolutionHistory::new();
        history.push(42.0);
        assert_eq!(history.short.len(), 1);
        assert_eq!(history.medium.len(), 1);
        assert_eq!(history.long.len(), 1);
        assert_eq!(history.extended.len(), 1);
    }

    #[test]
    fn trim_older_than_huge_duration_does_not_panic() {
        let mut history = MetricHistory::new(10);
        history.push(10.0);
        history.push(20.0);
        history.trim_older_than(Duration::from_secs(365 * 24 * 3600));
        assert_eq!(history.len(), 2);
    }
}
