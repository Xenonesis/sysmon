//! TelemetryHub — the central orchestrator for all telemetry providers.
//!
//! Replaces the monolithic `SystemMonitor` with a modular, multi-tier
//! architecture that polls providers at independent rates and delivers
//! normalized snapshots to the UI via bounded channels.

pub mod ring_buffer;
pub mod scheduler;

use crate::providers::TelemetryProvider;
use ring_buffer::MetricHistory;
use scheduler::PollingScheduler;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

/// Aggregated telemetry snapshot delivered to the UI each tick.
#[derive(Clone, Debug, Default)]
pub struct TelemetrySnapshot {
    /// All latest metric values keyed by "provider.metric_name".
    pub metrics: HashMap<String, f64>,
    /// Provider availability status.
    pub provider_status: HashMap<String, bool>,
}

/// Commands the UI can send to the TelemetryHub.
pub enum HubCommand {
    /// Switch to background/tray mode (reduced polling).
    SetBackgroundMode(bool),
    /// Force an immediate poll of all providers.
    ForceRefresh,
    /// Shut down the hub gracefully.
    Shutdown,
}

/// The TelemetryHub manages all providers, the polling scheduler,
/// and metric history buffers. It runs on a dedicated background thread.
pub struct TelemetryHub {
    providers: Vec<Box<dyn TelemetryProvider>>,
    scheduler: PollingScheduler,
    histories: HashMap<String, MetricHistory>,
    latest: TelemetrySnapshot,
    snapshot_sender: mpsc::SyncSender<TelemetrySnapshot>,
    command_receiver: mpsc::Receiver<HubCommand>,
    /// Default history capacity per metric.
    history_capacity: usize,
}

impl TelemetryHub {
    /// Create a new hub. Returns the hub plus channels for snapshot delivery and commands.
    pub fn new() -> (
        Self,
        mpsc::Receiver<TelemetrySnapshot>,
        mpsc::SyncSender<HubCommand>,
    ) {
        // Bounded channel capacity 2 — UI always gets latest, never replays stale
        let (snapshot_tx, snapshot_rx) = mpsc::sync_channel(2);
        let (command_tx, command_rx) = mpsc::sync_channel(16);

        let hub = Self {
            providers: Vec::new(),
            scheduler: PollingScheduler::new(),
            histories: HashMap::new(),
            latest: TelemetrySnapshot::default(),
            snapshot_sender: snapshot_tx,
            command_receiver: command_rx,
            history_capacity: 300, // ~60s at 5Hz
        };

        (hub, snapshot_rx, command_tx)
    }

    /// Register a telemetry provider with the hub.
    pub fn add_provider(&mut self, provider: Box<dyn TelemetryProvider>) {
        let name = provider.name().to_string();
        let interval = provider.poll_interval();
        self.scheduler.register(&name, interval);
        self.latest
            .provider_status
            .insert(name.clone(), provider.is_available());
        self.providers.push(provider);
    }

    /// Run the hub loop on the current thread (blocking).
    /// Call this from a dedicated background thread.
    pub fn run(&mut self) {
        loop {
            // Process any pending commands
            while let Ok(cmd) = self.command_receiver.try_recv() {
                match cmd {
                    HubCommand::SetBackgroundMode(bg) => {
                        self.scheduler.set_background_mode(bg);
                    }
                    HubCommand::ForceRefresh => {
                        self.poll_all();
                    }
                    HubCommand::Shutdown => {
                        self.shutdown_providers();
                        return;
                    }
                }
            }

            // Poll providers that are due
            let mut any_polled = false;
            for provider in &mut self.providers {
                let name = provider.name().to_string();
                if self.scheduler.is_due(&name) {
                    match provider.poll() {
                        Ok(data) => {
                            self.latest.provider_status.insert(name.clone(), true);
                            for (key, value) in data {
                                let f = value.as_f64();
                                self.latest.metrics.insert(key.clone(), f);
                                let history = self
                                    .histories
                                    .entry(key)
                                    .or_insert_with(|| MetricHistory::new(self.history_capacity));
                                history.push(f);
                            }
                        }
                        Err(e) => {
                            tracing::warn!(provider = %name, error = %e, "Provider poll failed");
                            self.latest.provider_status.insert(name.clone(), false);
                        }
                    }
                    self.scheduler.mark_polled(&name);
                    any_polled = true;
                }
            }

            // Send snapshot if anything was updated
            if any_polled {
                let _ = self.snapshot_sender.try_send(self.latest.clone());
            }

            // Sleep until next provider is due
            let wait = self.scheduler.time_until_next_poll();
            if wait > Duration::ZERO {
                std::thread::sleep(wait.min(Duration::from_millis(100)));
            }
        }
    }

    /// Poll all providers regardless of schedule.
    fn poll_all(&mut self) {
        for provider in &mut self.providers {
            let name = provider.name().to_string();
            match provider.poll() {
                Ok(data) => {
                    self.latest.provider_status.insert(name.clone(), true);
                    for (key, value) in data {
                        let f = value.as_f64();
                        self.latest.metrics.insert(key.clone(), f);
                        let history = self
                            .histories
                            .entry(key)
                            .or_insert_with(|| MetricHistory::new(self.history_capacity));
                        history.push(f);
                    }
                }
                Err(e) => {
                    tracing::warn!(provider = %name, error = %e, "Provider poll failed");
                    self.latest.provider_status.insert(name.clone(), false);
                }
            }
            self.scheduler.mark_polled(&name);
        }
        let _ = self.snapshot_sender.try_send(self.latest.clone());
    }

    /// Gracefully shut down all providers.
    fn shutdown_providers(&mut self) {
        for provider in &mut self.providers {
            provider.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{MetricValue, ProviderError};

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
            data.insert(
                "mock.value".into(),
                MetricValue::Float(self.poll_count as f64),
            );
            Ok(data)
        }

        fn is_available(&self) -> bool {
            true
        }
    }

    #[test]
    fn hub_creates_channels() {
        let (hub, _rx, _tx) = TelemetryHub::new();
        assert!(hub.providers.is_empty());
    }

    #[test]
    fn hub_registers_provider() {
        let (mut hub, _rx, _tx) = TelemetryHub::new();
        hub.add_provider(Box::new(MockProvider { poll_count: 0 }));
        assert_eq!(hub.providers.len(), 1);
        assert_eq!(
            hub.latest.provider_status.get("mock"),
            Some(&true)
        );
    }

    #[test]
    #[ignore] // Run manually with: cargo test test_deep_telemetry_flow -- --nocapture --ignored
    fn test_deep_telemetry_flow() {
        use crate::providers::sysinfo_provider::SysinfoProvider;
        use crate::providers::nvml_provider::NvmlProvider;
        use crate::providers::wmi_provider::WmiProvider;

        println!("--- Deep Testing TelemetryHub ---");

        let (mut hub, snapshot_rx, command_tx) = TelemetryHub::new();

        println!("Registering SysinfoProvider...");
        hub.add_provider(Box::new(SysinfoProvider::new()));
        
        println!("Registering NvmlProvider...");
        hub.add_provider(Box::new(NvmlProvider::new()));
        
        println!("Registering WmiProvider...");
        hub.add_provider(Box::new(WmiProvider::new()));

        let hub_handle = std::thread::spawn(move || {
            hub.run();
        });

        println!("Waiting for telemetry snapshots (collecting for 3 seconds)...");
        let start = std::time::Instant::now();
        let mut snapshot_count = 0;

        while start.elapsed() < Duration::from_secs(3) {
            if let Ok(snapshot) = snapshot_rx.recv_timeout(Duration::from_millis(500)) {
                snapshot_count += 1;
                println!("\n--- Snapshot {} ---", snapshot_count);
                
                println!("Provider Status:");
                for (provider, status) in &snapshot.provider_status {
                    println!("  {}: {}", provider, if *status { "OK" } else { "FAIL/UNAVAILABLE" });
                }

                println!("Selected Metrics:");
                for key in &["cpu.global_usage", "memory.used", "gpu.0.temperature", "board.manufacturer", "cpu.name"] {
                    if let Some(val) = snapshot.metrics.get(*key) {
                        println!("  {}: {:.2}", key, val);
                    }
                }
            }
        }

        println!("\nTotal snapshots received: {}", snapshot_count);
        let _ = command_tx.send(HubCommand::Shutdown);
        let _ = hub_handle.join();
        assert!(snapshot_count > 0, "Failed to receive any snapshots!");
    }
}
