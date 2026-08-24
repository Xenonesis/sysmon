//! Shared baseline comparison used by live session and timeline diagnostics.

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SignalSample {
    pub(crate) cpu_pct: f64,
    pub(crate) memory_pct: f64,
    pub(crate) disk_bps: f64,
    pub(crate) network_bps: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct BaselineComparison {
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

    let valid: Vec<_> = baseline.iter().copied().filter(|sample| is_finite(*sample)).collect();
    if valid.len() < 3 {
        return None;
    }

    let average = |select: fn(SignalSample) -> f64| valid.iter().copied().map(select).sum::<f64>() / valid.len() as f64;
    let cpu_base = average(|sample| sample.cpu_pct);
    let memory_base = average(|sample| sample.memory_pct);
    let disk_base = average(|sample| sample.disk_bps);
    let network_base = average(|sample| sample.network_bps);

    let mut ranked = [
        (
            "CPU",
            relative_increase(incident.cpu_pct, cpu_base, 5.0),
            format!("CPU was {:.1}% versus a {:.1}% baseline.", incident.cpu_pct, cpu_base),
        ),
        (
            "Memory",
            relative_increase(incident.memory_pct, memory_base, 5.0),
            format!(
                "Memory was {:.1}% versus a {:.1}% baseline.",
                incident.memory_pct, memory_base
            ),
        ),
        (
            "Disk I/O",
            relative_increase(incident.disk_bps, disk_base, 1_048_576.0),
            format!(
                "Disk throughput was {:.1} MB/s versus {:.1} MB/s baseline.",
                incident.disk_bps / 1_048_576.0,
                disk_base / 1_048_576.0
            ),
        ),
        (
            "Network",
            relative_increase(incident.network_bps, network_base, 1_048_576.0),
            format!(
                "Network throughput was {:.1} MB/s versus {:.1} MB/s baseline.",
                incident.network_bps / 1_048_576.0,
                network_base / 1_048_576.0
            ),
        ),
    ];
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    let primary = &ranked[0];
    let confidence = if valid.len() >= 30 {
        "high"
    } else if valid.len() >= 10 {
        "medium"
    } else {
        "low"
    };

    Some(BaselineComparison {
        primary_signal: primary.0,
        primary_score: primary.1,
        summary: primary.2.clone(),
        confidence,
        evidence: ranked.into_iter().map(|entry| entry.2).collect(),
    })
}

fn relative_increase(value: f64, baseline: f64, noise_floor: f64) -> f64 {
    ((value - baseline).max(0.0) / baseline.max(noise_floor) * 100.0).min(1_000.0)
}

fn is_finite(sample: SignalSample) -> bool {
    sample.cpu_pct.is_finite()
        && sample.memory_pct.is_finite()
        && sample.disk_bps.is_finite()
        && sample.network_bps.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

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
