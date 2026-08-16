//! Background worker for executing guarded system actions off the UI thread.

use std::sync::mpsc::{Receiver, Sender};
use tracing::warn;

use crate::app::models::SystemMonitor;
use crate::app::{actions, commands, events};
use crate::{persistence, power, processes, services};

#[derive(Debug, Clone)]
pub(crate) enum ActionError {
    AccessDenied,
    #[allow(dead_code)]
    NotFound,
    #[allow(dead_code)]
    Unavailable,
    Failed(String),
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessDenied => write!(f, "Access denied; administrator privileges may be required"),
            Self::NotFound => write!(f, "Process or service not found"),
            Self::Unavailable => write!(f, "Operation unavailable on this system"),
            Self::Failed(message) => f.write_str(message),
        }
    }
}

/// Loop that consumes ActionCommands on a background thread and dispatches events back to the UI.
pub(crate) fn run_action_worker(commands: Receiver<commands::ActionCommand>, events: Sender<events::AppEvent>) {
    let mut monitor = SystemMonitor::new();
    while let Ok(command) = commands.recv() {
        let plan = actions::ActionPlan::from_command(command.clone());
        let result: Result<String, ActionError> = match command {
            commands::ActionCommand::KillProcess(pid) => monitor
                .kill_process(pid)
                .then_some(format!("Process {pid} killed"))
                .ok_or(ActionError::AccessDenied),
            commands::ActionCommand::SuspendProcess(pid) => monitor
                .suspend_process(pid)
                .then_some(format!("Process {pid} suspended"))
                .ok_or(ActionError::AccessDenied),
            commands::ActionCommand::ResumeProcess(pid) => monitor
                .resume_process(pid)
                .then_some(format!("Process {pid} resumed"))
                .ok_or(ActionError::AccessDenied),
            commands::ActionCommand::SetPriority { pid, priority } => {
                SystemMonitor::set_process_priority(pid, &priority)
                    .then_some(format!("Process {pid} priority set to {priority}"))
                    .ok_or(ActionError::AccessDenied)
            }
            commands::ActionCommand::CleanRam => Ok(format!("Freed {} bytes", monitor.clean_ram(&[], false))),
            commands::ActionCommand::ControlService { name, action } => services::send_service_control(&name, action)
                .then_some(format!("Service {name} action completed"))
                .ok_or(ActionError::Failed("Service action failed".into())),
            commands::ActionCommand::SetPowerPlan(guid) => power::set_active_power_plan(&guid)
                .map(|_| "Power plan changed".into())
                .map_err(ActionError::Failed),
            commands::ActionCommand::SetAffinity { pid, mask } => processes::set_process_affinity(pid, mask)
                .map(|_| format!("Process {pid} affinity set to {mask:#x}"))
                .map_err(ActionError::Failed),
            commands::ActionCommand::KillProcessTree(root) => {
                monitor.sys.refresh_processes();
                let tree = processes::build_process_tree(&monitor.sys);
                let order = processes::kill_order(&tree, root);
                let total = order.len();
                let killed = order.into_iter().filter(|pid| monitor.kill_process(*pid)).count();
                if killed == total {
                    Ok(format!("Killed {killed} processes"))
                } else {
                    Err(ActionError::Failed(format!("Killed {killed} of {total} processes")))
                }
            }
        };
        let audit_result = result.map_err(|error| error.to_string());
        let record = actions::ActionAuditRecord::from_result(&plan, &audit_result);
        if let Err(error) = persistence::action_log::append(&record) {
            warn!(%error, "Failed to persist action audit record");
        }
        let event = match audit_result {
            Ok(_) => events::AppEvent::ActionCompleted {
                command: plan.command,
                record,
                undo: plan.undo,
            },
            Err(_) => events::AppEvent::ActionFailed {
                command: plan.command,
                record,
            },
        };
        let _ = events.send(event);
    }
}
