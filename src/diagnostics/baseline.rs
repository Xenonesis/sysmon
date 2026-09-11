//! Shared baseline comparison used by live session and timeline diagnostics.

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SignalSample {
    pub(crate) cpu_pct: f64,
    pub(crate) memory_pct: f64,
    pub(crate) disk_bps: f64,
    pub(crate) network_bps: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ComparisonState {
    Change,
    NoMeaningfulChange,
}

#[derive(Clone, Debug)]
pub(crate) struct BaselineComparison {
    pub(crate) state: ComparisonState,
    pub(crate) primary_signal: &'static str,
    pub(crate) primary_score: f64,
    pub(crate) summary: String,
    pub(crate) confidence: &'static str,
    pub(crate) evidence: Vec<String>,
}

pub(crate) fn compare_to_baseline(baseline: &[SignalSample], incident: SignalSample) -> Option<BaselineComparison> {
    if baseline.len() < 3 || !is_finite(incident) {
        return None;
    }

    if baseline.iter().any(|sample| !is_finite(*sample)) {
        return None;
    }
    let valid = baseline;

    let average = |select: fn(SignalSample) -> f64| valid.iter().copied().map(select).sum::<f64>() / valid.len() as f64;
    let cpu_base = average(|sample| sample.cpu_pct);
    let memory_base = average(|sample| sample.memory_pct);
    let disk_base = average(|sample| sample.disk_bps);
    let network_base = average(|sample| sample.network_bps);

    let noise = |select: fn(SignalSample) -> f64, mean: f64, floor: f64| {
        let variance = valid.iter().copied().map(|s| (select(s) - mean).powi(2)).sum::<f64>() / valid.len() as f64;
        floor.max(variance.sqrt() * 2.0)
    };
    let mut ranked = [
        (
            "CPU",
            relative_increase(incident.cpu_pct, cpu_base, noise(|s| s.cpu_pct, cpu_base, 5.0)),
            format!("CPU was {:.1}% versus a {:.1}% baseline.", incident.cpu_pct, cpu_base),
        ),
        (
            "Memory",
            relative_increase(
                incident.memory_pct,
                memory_base,
                noise(|s| s.memory_pct, memory_base, 5.0),
            ),
            format!(
                "Memory was {:.1}% versus a {:.1}% baseline.",
                incident.memory_pct, memory_base
            ),
        ),
        (
            "Disk I/O",
            relative_increase(
                incident.disk_bps,
                disk_base,
                noise(|s| s.disk_bps, disk_base, 1_048_576.0),
            ),
            format!(
                "Disk throughput was {:.1} MB/s versus {:.1} MB/s baseline.",
                incident.disk_bps / 1_048_576.0,
                disk_base / 1_048_576.0
            ),
        ),
        (
            "Network",
            relative_increase(
                incident.network_bps,
                network_base,
                noise(|s| s.network_bps, network_base, 1_048_576.0),
            ),
            format!(
                "Network throughput was {:.1} MB/s versus {:.1} MB/s baseline.",
                incident.network_bps / 1_048_576.0,
                network_base / 1_048_576.0
            ),
        ),
    ];
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    let primary = &ranked[0];
    let changed = primary.1 > 0.0;
    let confidence = if !changed {
        "not applicable"
    } else if valid.len() >= 10 {
        "medium"
    } else {
        "low"
    };

    Some(BaselineComparison {
        state: if changed {
            ComparisonState::Change
        } else {
            ComparisonState::NoMeaningfulChange
        },
        primary_signal: if changed { primary.0 } else { "No meaningful change" },
        primary_score: primary.1,
        summary: if changed {
            primary.2.clone()
        } else {
            "No measured increase exceeded the baseline variability and noise floors.".into()
        },
        confidence,
        evidence: ranked.into_iter().map(|entry| entry.2).collect(),
    })
}

fn relative_increase(value: f64, baseline: f64, noise_floor: f64) -> f64 {
    if value - baseline < noise_floor {
        0.0
    } else {
        ((value - baseline) / baseline.max(noise_floor) * 100.0).min(1_000.0)
    }
}

fn is_finite(sample: SignalSample) -> bool {
    sample.cpu_pct.is_finite()
        && sample.memory_pct.is_finite()
        && sample.disk_bps.is_finite()
        && sample.network_bps.is_finite()
        && (0.0..=100.0).contains(&sample.cpu_pct)
        && (0.0..=100.0).contains(&sample.memory_pct)
        && sample.disk_bps >= 0.0
        && sample.network_bps >= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_and_below_noise_samples_have_no_primary_cause() {
        let baseline = [SignalSample {
            cpu_pct: 20.0,
            memory_pct: 40.0,
            ..Default::default()
        }; 30];
        for cpu_pct in [10.0, 20.0, 24.9] {
            let result = compare_to_baseline(&baseline, SignalSample { cpu_pct, ..baseline[0] }).unwrap();
            assert_eq!(result.state, ComparisonState::NoMeaningfulChange);
            assert_eq!(result.primary_score, 0.0);
            assert_ne!(result.confidence, "high");
        }
    }

    #[test]
    fn ranks_largest_change_with_noise_floor() {
        let baseline = vec![
            SignalSample {
                cpu_pct: 10.0,
                memory_pct: 40.0,
                ..Default::default()
            };
            10
        ];
        let incident = SignalSample {
            cpu_pct: 90.0,
            memory_pct: 42.0,
            disk_bps: 200_000.0,
            network_bps: 100_000.0,
        };

        let comparison = compare_to_baseline(&baseline, incident).expect("comparison");
        assert_eq!(comparison.primary_signal, "CPU");
        assert_eq!(comparison.confidence, "medium");
    }

    #[test]
    fn refuses_short_or_non_finite_baselines() {
        assert!(compare_to_baseline(&[SignalSample::default(); 2], SignalSample::default()).is_none());
        let invalid = SignalSample {
            cpu_pct: f64::NAN,
            ..Default::default()
        };
        assert!(compare_to_baseline(&[invalid; 3], SignalSample::default()).is_none());
    }
}
