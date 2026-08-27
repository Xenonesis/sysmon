//! Provider trait abstraction for modular telemetry data sources.
//!
//! Each hardware/OS data source implements `TelemetryProvider`, allowing
//! the TelemetryHub to poll them independently at configurable rates
//! without vendor-specific logic leaking into the UI.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

/// Normalized metric value from any provider.
#[derive(Clone, Debug)]
pub enum MetricValue {
    Float(f64),
    Int(i64),
    UInt(u64),
    Text(String),
    Bool(bool),
    Unavailable,
}

impl MetricValue {
    /// Extract as f64, returning 0.0 if not numeric.
    pub fn as_f64(&self) -> f64 {
        match self {
            MetricValue::Float(v) => *v,
            MetricValue::Int(v) => *v as f64,
            MetricValue::UInt(v) => *v as f64,
            _ => 0.0,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, MetricValue::Float(_) | MetricValue::Int(_) | MetricValue::UInt(_))
    }
}

/// A collection of named metrics returned by a single provider poll.
pub type ProviderData = HashMap<String, MetricValue>;

/// Error types for provider operations.
#[derive(Debug)]
pub enum ProviderError {
    /// Hardware not present (e.g., no NVIDIA GPU)
    Unavailable(String),
    /// Initialization failed (e.g., NVML init error)
    InitFailed(String),
    /// Polling failed but may recover
    PollFailed(String),
    /// Permission denied
    PermissionDenied(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderError::Unavailable(msg) => write!(f, "Unavailable: {}", msg),
            ProviderError::InitFailed(msg) => write!(f, "Init failed: {}", msg),
            ProviderError::PollFailed(msg) => write!(f, "Poll failed: {}", msg),
            ProviderError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
        }
    }
}

impl std::error::Error for ProviderError {}

/// The core trait that every telemetry data source must implement.
///
/// Providers are polled by the `TelemetryHub` scheduler at their
/// declared `poll_interval()`. If `poll()` returns an error, the hub
/// logs it and continues — no single provider can crash the application.
pub trait TelemetryProvider: Send {
    /// Human-readable name for logging and UI display.
    fn name(&self) -> &str;

    /// The recommended polling interval for this provider.
    fn poll_interval(&self) -> Duration;

    /// Perform a single poll, returning normalized metrics.
    fn poll(&mut self) -> Result<ProviderData, ProviderError>;

    /// Whether this provider is currently available and functional.
    fn is_available(&self) -> bool;

    /// Gracefully shut down this provider, releasing resources.
    fn shutdown(&mut self) {}
}

pub mod nvml_provider;
pub mod sysinfo_provider;
pub mod windows_gpu_provider;
pub mod wmi_provider;
