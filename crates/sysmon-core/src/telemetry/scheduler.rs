//! Polling scheduler that manages per-provider tick intervals.
//!
//! Separates render frequency (60 FPS) from telemetry frequency (1–5 Hz)
//! and supports background/tray mode throttling.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks when each provider was last polled and whether it's due.
pub struct PollingScheduler {
    intervals: HashMap<String, Duration>,
    last_poll: HashMap<String, Instant>,
    background_mode: bool,
    /// Multiplier applied to all intervals in background/tray mode.
    background_multiplier: u32,
    /// Providers disabled after repeated consecutive failures.
    disabled: std::collections::HashSet<String>,
}

impl PollingScheduler {
    pub fn new() -> Self {
        Self {
            intervals: HashMap::new(),
            last_poll: HashMap::new(),
            background_mode: false,
            background_multiplier: 5, // 5x slower in background
            disabled: std::collections::HashSet::new(),
        }
    }

    /// Register a provider with its desired polling interval.
    pub fn register(&mut self, name: &str, interval: Duration) {
        self.intervals.insert(name.to_string(), interval);
        // Omit from last_poll so `is_due` returns true immediately on the first tick
        // without unsafe Instant subtraction underflow.
        self.last_poll.remove(name);
    }

    /// Check if a provider is due for polling.
    pub fn is_due(&self, name: &str) -> bool {
        if self.disabled.contains(name) {
            return false;
        }
        let Some(interval) = self.intervals.get(name) else {
            return false;
        };
        let effective_interval = if self.background_mode {
            *interval * self.background_multiplier
        } else {
            *interval
        };

        match self.last_poll.get(name) {
            Some(last) => last.elapsed() >= effective_interval,
            None => true,
        }
    }

    /// Mark a provider as just polled.
    pub fn mark_polled(&mut self, name: &str) {
        self.last_poll.insert(name.to_string(), Instant::now());
    }

    /// Disable a provider so it is never scheduled again (e.g. after
    /// repeated consecutive failures). Re-enable with `enable`.
    pub fn disable(&mut self, name: &str) {
        self.disabled.insert(name.to_string());
    }

    /// Re-enable a previously disabled provider.
    pub fn enable(&mut self, name: &str) {
        self.disabled.remove(name);
    }

    /// Whether a provider is currently disabled.
    pub fn is_disabled(&self, name: &str) -> bool {
        self.disabled.contains(name)
    }

    /// Enable or disable background mode (reduces polling rates).
    pub fn set_background_mode(&mut self, enabled: bool) {
        self.background_mode = enabled;
    }

    /// Whether background mode is active.
    pub fn is_background_mode(&self) -> bool {
        self.background_mode
    }

    /// Returns the shortest sleep duration until any provider is next due.
    pub fn time_until_next_poll(&self) -> Duration {
        let mut min_wait = Duration::from_secs(1);

        for (name, interval) in &self.intervals {
            if self.disabled.contains(name) {
                continue;
            }
            let effective_interval = if self.background_mode {
                *interval * self.background_multiplier
            } else {
                *interval
            };

            if let Some(last) = self.last_poll.get(name) {
                let elapsed = last.elapsed();
                if elapsed >= effective_interval {
                    return Duration::ZERO;
                }
                let remaining = effective_interval - elapsed;
                if remaining < min_wait {
                    min_wait = remaining;
                }
            } else {
                return Duration::ZERO;
            }
        }

        min_wait
    }
}

impl Default for PollingScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_poll_is_always_due() {
        let mut sched = PollingScheduler::new();
        sched.register("test", Duration::from_secs(10));
        assert!(sched.is_due("test"));
    }

    #[test]
    fn not_due_immediately_after_poll() {
        let mut sched = PollingScheduler::new();
        sched.register("test", Duration::from_secs(10));
        sched.mark_polled("test");
        assert!(!sched.is_due("test"));
    }

    #[test]
    fn unknown_provider_not_due() {
        let sched = PollingScheduler::new();
        assert!(!sched.is_due("nonexistent"));
    }

    #[test]
    fn background_mode_increases_interval() {
        let mut sched = PollingScheduler::new();
        sched.register("test", Duration::from_millis(100));
        sched.mark_polled("test");
        sched.set_background_mode(true);
        // In background mode, 100ms * 5 = 500ms, so should not be due after 100ms
        std::thread::sleep(Duration::from_millis(110));
        assert!(!sched.is_due("test"));
    }

    #[test]
    fn register_with_huge_interval_does_not_panic() {
        let mut sched = PollingScheduler::new();
        sched.register("large_interval", Duration::from_secs(365 * 24 * 3600));
        assert!(sched.is_due("large_interval"));
    }
}
