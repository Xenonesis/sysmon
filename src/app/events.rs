use crate::app::actions::ActionAuditRecord;
use crate::app::commands::ActionCommand;
use crate::monitoring::snapshot::SystemSnapshot;

#[derive(Debug, Clone)]
pub(crate) enum AppEvent {
    Snapshot(Box<SystemSnapshot>),
    AuditRecorded(ActionAuditRecord),
    ActionCompleted {
        command: ActionCommand,
        record: ActionAuditRecord,
        undo: Option<ActionCommand>,
    },
    ActionFailed {
        command: ActionCommand,
        record: ActionAuditRecord,
    },
}
