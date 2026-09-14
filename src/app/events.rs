use crate::app::actions::ActionAuditRecord;
use crate::app::commands::ActionCommand;
use crate::app::models::SystemData;
use crate::monitoring::snapshot::SystemSnapshot;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) enum AppEvent {
    Snapshot {
        snapshot: Box<SystemSnapshot>,
        data_arc: Arc<SystemData>,
    },
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

impl std::fmt::Debug for AppEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot { snapshot, .. } => f
                .debug_struct("Snapshot")
                .field("snapshot", snapshot)
                .finish_non_exhaustive(),
            Self::MonitoringPaused(paused) => f.debug_tuple("MonitoringPaused").field(paused).finish(),
            Self::ActionCompleted {
                command,
                record,
                undo,
                ram_outcome,
            } => f
                .debug_struct("ActionCompleted")
                .field("command", command)
                .field("record", record)
                .field("undo", undo)
                .field("ram_outcome", ram_outcome)
                .finish(),
            Self::ActionFailed { command, record } => f
                .debug_struct("ActionFailed")
                .field("command", command)
                .field("record", record)
                .finish(),
        }
    }
}
