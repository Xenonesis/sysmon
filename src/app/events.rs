use crate::monitoring::snapshot::SystemSnapshot;

#[derive(Debug, Clone)]
pub(crate) enum AppEvent {
    Snapshot(SystemSnapshot),
    ActionCompleted { action: String, message: String },
    ActionFailed { action: String, error: String },
    ProviderFailed { provider: String, error: String },
}
