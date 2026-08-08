use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CounterRate {
    pub value_per_second: f64,
    pub initialized: bool,
}

pub fn counter_rate(previous: Option<u64>, current: u64, elapsed: Duration) -> CounterRate {
    let Some(previous) = previous else {
        return CounterRate { value_per_second: 0.0, initialized: false };
    };
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0 {
        return CounterRate { value_per_second: 0.0, initialized: true };
    }
    CounterRate {
        value_per_second: current.saturating_sub(previous) as f64 / seconds,
        initialized: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sample_is_zero() {
        assert_eq!(counter_rate(None, 100, Duration::from_secs(1)), CounterRate { value_per_second: 0.0, initialized: false });
    }

    #[test]
    fn computes_delta_rate() {
        assert_eq!(counter_rate(Some(100), 300, Duration::from_secs(2)).value_per_second, 100.0);
    }

    #[test]
    fn counter_reset_does_not_spike() {
        assert_eq!(counter_rate(Some(300), 100, Duration::from_secs(1)).value_per_second, 0.0);
    }

    #[test]
    fn zero_elapsed_is_safe() {
        assert_eq!(counter_rate(Some(100), 200, Duration::ZERO).value_per_second, 0.0);
    }
}
