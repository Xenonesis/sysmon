use crate::app::actions::ActionAuditRecord;
use crate::app::commands::ActionCommand;
use crate::monitoring::snapshot::SystemSnapshot;

#[derive(Debug, Clone)]
pub(crate) enum AppEvent {
    Snapshot(Box<SystemSnapshot>),
    MonitoringPaused(bool),
    ActionCompleted {
        command: ActionCommand,
        record: ActionAuditRecord,
        undo: Option<ActionCommand>,
        ram_outcome: Option<crate::monitoring::process_actions::RamCleanOutcome>,
    },
    ActionFailed {
        command: ActionCommand,
        record: ActionAuditRecord,
    },
}
