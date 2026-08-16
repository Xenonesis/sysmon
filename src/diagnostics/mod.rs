use serde::Serialize;

use crate::monitoring::SystemSnapshot;
use crate::telemetry::HistoryStats;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum Severity {
    Healthy,
    Info,
    Warning,
    Critical,
}

impl Severity {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Healthy => "HEALTHY",
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Critical => "CRITICAL",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Finding {
    pub severity: Severity,
    pub title: String,
    pub evidence: String,
    pub recommendation: String,
    pub confidence: u8,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct DiagnosticReport {
    pub findings: Vec<Finding>,
}

pub(crate) fn analyze(
    snapshot: &SystemSnapshot,
    histories: &std::collections::HashMap<String, HistoryStats>,
) -> DiagnosticReport {
    let mut findings = Vec::new();

    let sustained_cpu = histories
        .get("cpu.global_usage")
        .map(|history| history.five_minutes.avg)
        .unwrap_or(snapshot.cpu_usage as f64);
    if snapshot.cpu_usage >= 90.0 || sustained_cpu >= 85.0 {
        let top = snapshot
            .processes
            .iter()
            .max_by(|a, b| a.cpu_usage.total_cmp(&b.cpu_usage));
        findings.push(Finding {
            severity: if snapshot.cpu_usage >= 95.0 {
                Severity::Critical
            } else {
                Severity::Warning
            },
            title: "CPU saturation".into(),
            evidence: top.map_or_else(
                || {
                    format!(
                        "CPU is {:.1}% (5-minute average {:.1}%).",
                        snapshot.cpu_usage, sustained_cpu
                    )
                },
                |process| {
                    format!(
                        "CPU is {:.1}% (5-minute average {:.1}%); {} (PID {}) is using {:.1}%.",
                        snapshot.cpu_usage, sustained_cpu, process.name, process.pid, process.cpu_usage
                    )
                },
            ),
            recommendation: "Inspect the leading process before changing power or priority settings.".into(),
            confidence: if top.is_some() { 92 } else { 75 },
        });
    }

    let sustained_memory = histories
        .get("memory.used")
        .and_then(|history| {
            (snapshot.memory_total > 0).then_some(history.five_minutes.avg / snapshot.memory_total as f64 * 100.0)
        })
        .unwrap_or(snapshot.memory_percentage as f64);
    if snapshot.memory_percentage >= 85.0 || sustained_memory >= 85.0 {
        let top = snapshot.processes.iter().max_by_key(|process| process.memory);
        findings.push(Finding {
            severity: if snapshot.memory_percentage >= 95.0 {
                Severity::Critical
            } else {
                Severity::Warning
            },
            title: "Memory pressure".into(),
            evidence: top.map_or_else(
                || format!("RAM usage is {:.1}%.", snapshot.memory_percentage),
                |process| {
                    format!(
                        "RAM usage is {:.1}%; {} (PID {}) holds {:.1} MB.",
                        snapshot.memory_percentage,
                        process.name,
                        process.pid,
                        process.memory as f64 / 1_048_576.0
                    )
                },
            ),
            recommendation: "Close or restart the responsible application; use working-set trimming only as a temporary diagnostic step."
                .into(),
            confidence: if top.is_some() { 90 } else { 70 },
        });
    }

    for disk in snapshot.disks.iter().filter(|disk| disk.usage_percentage >= 90.0) {
        findings.push(Finding {
            severity: if disk.usage_percentage >= 97.0 {
                Severity::Critical
            } else {
                Severity::Warning
            },
            title: format!("Low disk space on {}", disk.mount_point),
            evidence: format!("{} is {:.1}% full.", disk.name, disk.usage_percentage),
            recommendation: "Free space or move large files before Windows and applications lose working space.".into(),
            confidence: 98,
        });
    }

    for gpu in &snapshot.gpus {
        if gpu.temperature.is_some_and(|temperature| temperature >= 85) {
            findings.push(Finding {
                severity: Severity::Warning,
                title: format!("High GPU temperature: {}", gpu.name),
                evidence: format!(
                    "GPU temperature is {} C at {:.1}% utilization.",
                    gpu.temperature.unwrap_or_default(),
                    gpu.utilization
                ),
                recommendation: "Check airflow, fan operation, dust buildup and the active workload.".into(),
                confidence: 95,
            });
        }
    }

    for (provider, status) in snapshot.provider_status.iter().filter(|(_, status)| !status.available) {
        findings.push(Finding {
            severity: Severity::Info,
            title: format!("Telemetry provider unavailable: {provider}"),
            evidence: status
                .error
                .clone()
                .unwrap_or_else(|| "The provider did not return data on its latest poll.".into()),
            recommendation: "Install the matching driver or ignore this message when the hardware is not present."
                .into(),
            confidence: 100,
        });
    }

    if findings.is_empty() {
        findings.push(Finding {
            severity: Severity::Healthy,
            title: "No active bottleneck detected".into(),
            evidence: format!(
                "CPU {:.1}%, memory {:.1}%, and {} monitored disk(s) are within current thresholds.",
                snapshot.cpu_usage,
                snapshot.memory_percentage,
                snapshot.disks.len()
            ),
            recommendation: "Record a session while reproducing the slowdown to catch transient problems.".into(),
            confidence: 80,
        });
    }

    findings.sort_by_key(|finding| std::cmp::Reverse(finding.severity));
    DiagnosticReport { findings }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_cpu_produces_bottleneck_finding() {
        let snapshot = SystemSnapshot {
            cpu_usage: 96.0,
            ..Default::default()
        };
        let report = analyze(&snapshot, &Default::default());
        assert!(report.findings.iter().any(|finding| finding.title == "CPU saturation"));
    }

    #[test]
    fn quiet_snapshot_is_healthy() {
        let report = analyze(&SystemSnapshot::default(), &Default::default());
        assert!(matches!(report.findings[0].severity, Severity::Healthy));
    }
}
