//! TelemetryHub is the central orchestrator for hardware and OS providers.
//!
//! Providers run at independent rates, history is bounded and downsampled,
//! and consumers read a replaceable latest snapshot instead of replaying a
//! queue of stale samples.

#[cfg(test)]
pub(crate) mod quality;
pub mod ring_buffer;
pub mod scheduler;

use crate::monitoring::snapshot::{MetricObservation, MetricState};
use crate::providers::TelemetryProvider;
use ring_buffer::{MetricStats, MultiResolutionHistory};
use scheduler::PollingScheduler;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, mpsc};
use std::time::Duration;
use std::time::SystemTime;

#[derive(Clone, Debug, Default)]
pub struct HistoryStats {
    pub sixty_seconds: MetricStats,
    pub five_minutes: MetricStats,
    pub thirty_minutes: MetricStats,
    pub one_hour: MetricStats,
}

/// Aggregated telemetry snapshot delivered to the application.
#[derive(Clone, Debug, Default)]
pub struct TelemetrySnapshot {
    /// Latest numeric values keyed by a stable provider-independent name.
    pub metrics: HashMap<String, f64>,
    /// Latest textual identity values such as GPU, BIOS and board names.
    pub labels: HashMap<String, String>,
    /// Provider availability as of the most recent attempted poll.
    pub provider_status: HashMap<String, bool>,
    /// Bounded statistics calculated independently for each time range.
    pub history_stats: HashMap<String, HistoryStats>,
    pub metric_status: HashMap<String, MetricObservation>,
}

#[derive(Clone, Debug, Default)]
struct SnapshotSlot {
    generation: u64,
    snapshot: TelemetrySnapshot,
}

/// Single-consumer view of the latest telemetry snapshot.
pub struct LatestSnapshotReader {
    slot: Arc<RwLock<SnapshotSlot>>,
    observed_generation: u64,
}

impl LatestSnapshotReader {
    pub fn latest_if_updated(&mut self) -> Option<TelemetrySnapshot> {
        let slot = self.slot.read().ok()?;
        if slot.generation == self.observed_generation {
            return None;
        }
        self.observed_generation = slot.generation;
        Some(slot.snapshot.clone())
    }
    pub fn latest(&self) -> Option<TelemetrySnapshot> {
        let slot = self.slot.read().ok()?;
        Some(slot.snapshot.clone())
    }
}

/// Commands sent from the application to the telemetry worker.
pub enum HubCommand {
    SetBackgroundMode(bool),
    SetPaused(bool),
    SetConsumerDemand { recording: bool, process_manager: bool },
    ForceRefresh,
    Shutdown,
}

pub struct TelemetryHub {
    providers: Vec<Box<dyn TelemetryProvider>>,
    scheduler: PollingScheduler,
    histories: HashMap<String, MultiResolutionHistory>,
    latest: TelemetrySnapshot,
    snapshot_slot: Arc<RwLock<SnapshotSlot>>,
    command_receiver: mpsc::Receiver<HubCommand>,
    /// Consecutive poll failures per provider; used to disable broken providers.
    consecutive_failures: HashMap<String, u32>,
    paused: bool,
    consumer_demand: (bool, bool),
    metric_owners: HashMap<String, String>,
}

impl TelemetryHub {
    pub fn new() -> (Self, LatestSnapshotReader, mpsc::SyncSender<HubCommand>) {
        let (command_tx, command_rx) = mpsc::sync_channel(16);
        let snapshot_slot = Arc::new(RwLock::new(SnapshotSlot::default()));
        let hub = Self {
            providers: Vec::new(),
            scheduler: PollingScheduler::new(),
            histories: HashMap::new(),
            latest: TelemetrySnapshot::default(),
            snapshot_slot: Arc::clone(&snapshot_slot),
            command_receiver: command_rx,
            paused: false,
            consecutive_failures: HashMap::new(),
            consumer_demand: (false, false),
            metric_owners: HashMap::new(),
        };
        let reader = LatestSnapshotReader {
            slot: snapshot_slot,
            observed_generation: 0,
        };
        (hub, reader, command_tx)
    }

    pub fn add_provider(&mut self, provider: Box<dyn TelemetryProvider>) {
        let name = provider.name().to_string();
        self.scheduler.register(&name, provider.poll_interval());
        self.latest.provider_status.insert(name, provider.is_available());
        self.providers.push(provider);
    }

    /// Run the hub on its dedicated worker thread until shutdown.
    pub fn run(&mut self) {
        loop {
            while let Ok(command) = self.command_receiver.try_recv() {
                match command {
                    HubCommand::SetBackgroundMode(background) => {
                        self.scheduler.set_background_mode(background);
                    }
                    HubCommand::SetConsumerDemand {
                        recording,
                        process_manager,
                    } => {
                        self.consumer_demand = (recording, process_manager);
                    }
                    HubCommand::SetPaused(paused) => {
                        self.paused = paused;
                        if paused {
                            for observation in self.latest.metric_status.values_mut() {
                                if observation.state == MetricState::Available {
                                    observation.state = MetricState::Stale;
                                }
                            }
                            self.latest.metrics.clear();
                            self.publish_snapshot();
                        }
                    }
                    HubCommand::ForceRefresh => self.poll_all(),
                    HubCommand::Shutdown => {
                        self.shutdown_providers();
                        return;
                    }
                }
            }
            if self.paused {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }

            let mut updated = false;
            for index in 0..self.providers.len() {
                let name = self.providers[index].name().to_string();
                if !self.scheduler.is_due(&name) {
                    continue;
                }
                let result = self.providers[index].poll();
                self.apply_provider_result(&name, result);
                self.scheduler.mark_polled(&name);
                updated = true;
            }

            if updated {
                self.publish_snapshot();
            }

            let wait = self.scheduler.time_until_next_poll();
            if wait > Duration::ZERO {
                std::thread::sleep(wait.min(Duration::from_millis(100)));
            }
        }
    }

    pub fn poll_all(&mut self) {
        if self.paused {
            return;
        }
        for index in 0..self.providers.len() {
            let name = self.providers[index].name().to_string();
            self.providers[index].reinitialize();
            let result = self.providers[index].poll();
            self.apply_provider_result(&name, result);
            self.scheduler.mark_polled(&name);
        }
        self.publish_snapshot();
    }

    /// Consecutive poll failures before a provider is disabled.
    const MAX_CONSECUTIVE_FAILURES: u32 = 5;

    fn apply_provider_result(
        &mut self,
        provider_name: &str,
        result: Result<crate::providers::ProviderData, crate::providers::ProviderError>,
    ) {
        let observed_at = SystemTime::now();
        for (key, owner) in &self.metric_owners {
            if owner == provider_name {
                self.latest.metrics.remove(key);
                self.latest.labels.remove(key);
                if let Some(observation) = self.latest.metric_status.get_mut(key) {
                    observation.state = MetricState::Stale;
                    observation.error = Some("Metric omitted by the latest provider poll".into());
                }
            }
        }
        match result {
            Ok(data) => {
                self.consecutive_failures.remove(provider_name);
                if self.scheduler.is_disabled(provider_name) {
                    tracing::info!(provider = provider_name, "Provider recovered; re-enabling");
                    self.scheduler.enable(provider_name);
                }
                self.latest.provider_status.insert(provider_name.to_string(), true);
                self.latest
                    .metric_status
                    .insert(provider_name.into(), MetricObservation::available(observed_at));
                for (key, value) in data {
                    self.metric_owners.insert(key.clone(), provider_name.into());
                    let observation = if matches!(value, crate::providers::MetricValue::Unavailable) {
                        MetricObservation {
                            observed_at: None,
                            state: MetricState::Unsupported,
                            error: None,
                        }
                    } else {
                        MetricObservation::available(observed_at)
                    };
                    self.latest.metric_status.insert(key.clone(), observation);
                    match value {
                        crate::providers::MetricValue::Text(text) => {
                            self.latest.labels.insert(key, text);
                        }
                        numeric if numeric.is_numeric() => self.record_metric(key, numeric.as_f64()),
                        _ => {}
                    }
                }
            }
            Err(error) => {
                let state = if matches!(error, crate::providers::ProviderError::Unavailable(_)) {
                    MetricState::Unsupported
                } else {
                    MetricState::Error
                };
                for (key, owner) in &self.metric_owners {
                    if owner == provider_name
                        && let Some(observation) = self.latest.metric_status.get_mut(key)
                    {
                        observation.state = state;
                        observation.error = Some(error.to_string());
                    }
                }
                let previous = self.latest.metric_status.get(provider_name).and_then(|o| o.observed_at);
                self.latest.metric_status.insert(
                    provider_name.into(),
                    MetricObservation {
                        observed_at: previous,
                        state,
                        error: Some(error.to_string()),
                    },
                );
                let failures = self.consecutive_failures.entry(provider_name.to_string()).or_insert(0);
                *failures += 1;
                if *failures >= Self::MAX_CONSECUTIVE_FAILURES {
                    if !self.scheduler.is_disabled(provider_name) {
                        tracing::warn!(
                            provider = provider_name,
                            failures = *failures,
                            error = %error,
                            "Provider failed repeatedly; disabling until manual refresh"
                        );
                        self.scheduler.disable(provider_name);
                    }
                } else if *failures == 1 {
                    tracing::warn!(provider = provider_name, error = %error, "Provider poll failed");
                }
                self.latest.provider_status.insert(provider_name.to_string(), false);
            }
        }
    }

    fn record_metric(&mut self, key: String, value: f64) {
        if !value.is_finite() {
            self.latest.metric_status.insert(
                key,
                MetricObservation {
                    observed_at: None,
                    state: MetricState::Error,
                    error: Some("Non-finite provider reading".into()),
                },
            );
            return;
        }
        self.latest.metrics.insert(key.clone(), value);
        let history = self.histories.entry(key.clone()).or_default();
        history.push(value);
        self.latest.history_stats.insert(
            key,
            HistoryStats {
                sixty_seconds: history.short.stats().clone(),
                five_minutes: history.medium.stats().clone(),
                thirty_minutes: history.long.stats().clone(),
                one_hour: history.extended.stats().clone(),
            },
        );
    }

    fn publish_snapshot(&mut self) {
        let now = std::time::Instant::now();
        for (key, history) in &mut self.histories {
            history.trim_at(now);
            self.latest.history_stats.insert(
                key.clone(),
                HistoryStats {
                    sixty_seconds: history.short.stats().clone(),
                    five_minutes: history.medium.stats().clone(),
                    thirty_minutes: history.long.stats().clone(),
                    one_hour: history.extended.stats().clone(),
                },
            );
        }
        for (key, owner) in &self.metric_owners {
            let max_age = self
                .providers
                .iter()
                .find(|p| p.name() == owner)
                .map(|p| p.poll_interval().saturating_mul(15))
                .unwrap_or(Duration::from_secs(15));
            if let Some(observation) = self.latest.metric_status.get_mut(key)
                && observation.state == MetricState::Available
                && observation
                    .observed_at
                    .and_then(|at| at.elapsed().ok())
                    .is_some_and(|age| age > max_age)
            {
                observation.state = MetricState::Stale;
                self.latest.metrics.remove(key);
            }
        }
        if let Ok(mut slot) = self.snapshot_slot.write() {
            slot.generation = slot.generation.wrapping_add(1);
            slot.snapshot = self.latest.clone();
        }
    }

    fn shutdown_providers(&mut self) {
        for provider in &mut self.providers {
            provider.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{MetricValue, ProviderData, ProviderError};

    #[test]
    fn missing_and_failed_metrics_do_not_remain_live() {
        let (mut hub, _, _) = TelemetryHub::new();
        let sample = || ProviderData::from([("gpu.0.temperature".into(), MetricValue::UInt(67))]);
        hub.apply_provider_result("gpu", Ok(sample()));
        let observed = hub.latest.metric_status["gpu.0.temperature"].observed_at;
        hub.apply_provider_result("gpu", Ok(ProviderData::new()));
        assert!(!hub.latest.metrics.contains_key("gpu.0.temperature"));
        assert_eq!(hub.latest.metric_status["gpu.0.temperature"].state, MetricState::Stale);
        assert_eq!(hub.latest.metric_status["gpu.0.temperature"].observed_at, observed);
        hub.apply_provider_result("gpu", Err(ProviderError::PollFailed("disconnected".into())));
        assert_eq!(hub.latest.metric_status["gpu.0.temperature"].state, MetricState::Error);
        hub.apply_provider_result("gpu", Ok(sample()));
        assert_eq!(hub.latest.metrics["gpu.0.temperature"], 67.0);
        assert!(hub.latest.metric_status["gpu.0.temperature"].is_fresh());
    }

    struct MockProvider {
        poll_count: u32,
    }

    impl TelemetryProvider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        fn poll_interval(&self) -> Duration {
            Duration::from_millis(50)
        }

        fn poll(&mut self) -> Result<ProviderData, ProviderError> {
            self.poll_count += 1;
            let mut data = ProviderData::new();
            data.insert("mock.value".into(), MetricValue::Float(self.poll_count as f64));
            Ok(data)
        }

        fn is_available(&self) -> bool {
            true
        }
    }

    #[test]
    fn hub_registers_provider() {
        let (mut hub, _reader, _sender) = TelemetryHub::new();
        hub.add_provider(Box::new(MockProvider { poll_count: 0 }));
        assert_eq!(hub.providers.len(), 1);
        assert_eq!(hub.latest.provider_status.get("mock"), Some(&true));
    }

    #[test]
    fn hub_publishes_latest_data_and_history() {
        let (mut hub, mut reader, command_sender) = TelemetryHub::new();
        hub.add_provider(Box::new(MockProvider { poll_count: 0 }));
        let handle = std::thread::spawn(move || hub.run());

        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        let snapshot = loop {
            if let Some(snapshot) = reader.latest_if_updated() {
                break snapshot;
            }
            assert!(std::time::Instant::now() < deadline, "telemetry snapshot timed out");
            std::thread::sleep(Duration::from_millis(10));
        };

        command_sender.send(HubCommand::Shutdown).unwrap();
        handle.join().unwrap();
        assert!(snapshot.metrics.contains_key("mock.value"));
        assert_eq!(snapshot.history_stats["mock.value"].sixty_seconds.sample_count, 1);
    }

    struct FailingProvider;

    impl TelemetryProvider for FailingProvider {
        fn name(&self) -> &str {
            "failing"
        }

        fn poll_interval(&self) -> Duration {
            Duration::from_millis(1)
        }

        fn poll(&mut self) -> Result<ProviderData, ProviderError> {
            Err(ProviderError::InitFailed("always fails".into()))
        }

        fn is_available(&self) -> bool {
            false
        }
    }

    #[test]
    fn failing_provider_is_disabled_after_repeated_failures() {
        let (mut hub, _reader, _sender) = TelemetryHub::new();
        hub.add_provider(Box::new(FailingProvider));

        // Drive enough consecutive failures to trip the backoff threshold.
        for _ in 0..TelemetryHub::MAX_CONSECUTIVE_FAILURES {
            let result = hub.providers[0].poll();
            hub.apply_provider_result("failing", result);
        }

        assert!(
            hub.scheduler.is_disabled("failing"),
            "provider should be disabled after {} consecutive failures",
            TelemetryHub::MAX_CONSECUTIVE_FAILURES
        );
        assert_eq!(hub.latest.provider_status.get("failing"), Some(&false));
    }

    #[test]
    fn success_resets_failure_count_and_reenables() {
        let (mut hub, _reader, _sender) = TelemetryHub::new();
        hub.add_provider(Box::new(FailingProvider));

        // Accumulate failures just below the disable threshold.
        for _ in 0..TelemetryHub::MAX_CONSECUTIVE_FAILURES - 1 {
            let result = hub.providers[0].poll();
            hub.apply_provider_result("failing", result);
        }
        assert!(!hub.scheduler.is_disabled("failing"));

        // A success clears the failure counter.
        hub.apply_provider_result("failing", Ok(ProviderData::new()));
        assert_eq!(hub.consecutive_failures.get("failing"), None);
        assert_eq!(hub.latest.provider_status.get("failing"), Some(&true));
    }

    #[test]
    #[ignore] // Hardware-dependent smoke test; deterministic flow is covered above.
    fn hardware_telemetry_smoke_test() {
        use crate::providers::nvml_provider::NvmlProvider;
        use crate::providers::sysinfo_provider::SysinfoProvider;
        use crate::providers::wmi_provider::WmiProvider;

        let (mut hub, mut reader, command_sender) = TelemetryHub::new();
        hub.add_provider(Box::new(SysinfoProvider::new()));
        hub.add_provider(Box::new(NvmlProvider::new()));
        hub.add_provider(Box::new(WmiProvider::new()));
        let handle = std::thread::spawn(move || hub.run());

        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut snapshots = 0;
        while std::time::Instant::now() < deadline {
            if reader.latest_if_updated().is_some() {
                snapshots += 1;
            }
            std::thread::sleep(Duration::from_millis(25));
        }

        command_sender.send(HubCommand::Shutdown).unwrap();
        handle.join().unwrap();
        assert!(snapshots > 0);
    }

    #[test]
    #[ignore] // Run explicitly on representative Windows hardware before release.
    fn hardware_telemetry_soak_test() {
        use crate::providers::nvml_provider::NvmlProvider;
        use crate::providers::sysinfo_provider::SysinfoProvider;
        use crate::providers::windows_gpu_provider::WindowsGpuProvider;
        use crate::providers::wmi_provider::WmiProvider;
        use crate::telemetry::quality::TelemetryQualityTracker;

        let seconds = std::env::var("SYSMON_SOAK_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(60)
            .clamp(10, 86_400);
        let (mut hub, mut reader, command_sender) = TelemetryHub::new();
        hub.add_provider(Box::new(SysinfoProvider::new()));
        hub.add_provider(Box::new(NvmlProvider::new()));
        hub.add_provider(Box::new(WmiProvider::new()));
        hub.add_provider(Box::new(WindowsGpuProvider::new()));
        let handle = std::thread::spawn(move || hub.run());

        let deadline = std::time::Instant::now() + Duration::from_secs(seconds);
        let mut tracker = TelemetryQualityTracker::default();
        while std::time::Instant::now() < deadline {
            if let Some(snapshot) = reader.latest_if_updated() {
                tracker.observe(std::time::Instant::now(), &snapshot);
            }
            std::thread::sleep(Duration::from_millis(25));
        }

        command_sender.send(HubCommand::Shutdown).unwrap();
        handle.join().unwrap();
        let report = tracker.report();
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        assert!(
            report.samples >= seconds * 2,
            "sampling rate fell below 2 Hz: {report:?}"
        );
        assert_eq!(report.invalid_values, 0, "non-finite telemetry observed: {report:?}");
        assert_eq!(
            report.out_of_range_percentages, 0,
            "percentage telemetry exceeded 0..=100: {report:?}"
        );
        assert!(
            report.maximum_gap_ms <= 1_500,
            "telemetry gap exceeded 1.5 seconds: {report:?}"
        );
    }
}
