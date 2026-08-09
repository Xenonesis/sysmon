use crate::monitoring::snapshot::SystemSnapshot;

#[derive(Debug, Clone)]
pub(crate) enum AppEvent {
    Snapshot(SystemSnapshot),
    // action label kept for the alert center UI
    #[allow(dead_code)]
    ActionCompleted { action: String, message: String },
    #[allow(dead_code)]
    ActionFailed { action: String, error: String },
    // emitted by provider polling when an optional source fails
    #[allow(dead_code)]
    ProviderFailed { provider: String, error: String },
}
