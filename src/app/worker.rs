//! Background worker for executing guarded system actions off the UI thread.

use std::sync::mpsc::{Receiver, Sender};
use tracing::warn;

use crate::app::models::SystemMonitor;
use crate::app::{actions, commands, events};
use crate::{persistence, power, processes, services, startup};

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
        let mut dynamic_undo = None;
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
                .map(|outcome| format!("Service {name}: {outcome}"))
                .map_err(|error| ActionError::Failed(error.to_string())),
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
            commands::ActionCommand::DisableStartup { item_name, locator } => startup::disable_startup(&locator)
                .map(|_| format!("Startup item {item_name} disabled"))
                .map_err(ActionError::Failed),
            commands::ActionCommand::EnableStartup { item_name, locator } => startup::enable_startup(&locator)
                .map(|_| format!("Startup item {item_name} enabled"))
                .map_err(ActionError::Failed),
            commands::ActionCommand::QuarantineStartup { item_name, locator } => {
                startup::quarantine_startup(&item_name, &locator)
                    .map(|quarantine_id| {
                        dynamic_undo = Some(commands::ActionCommand::RestoreStartup {
                            item_name: item_name.clone(),
                            quarantine_id: quarantine_id.clone(),
                        });
                        format!("Startup item {item_name} quarantined (backup {quarantine_id})")
                    })
                    .map_err(ActionError::Failed)
            }
            commands::ActionCommand::RestoreStartup {
                item_name,
                quarantine_id,
            } => startup::restore_startup(&quarantine_id)
                .map(|_| format!("Startup item {item_name} restored"))
                .map_err(ActionError::Failed),
            commands::ActionCommand::ReclaimStorageCaches(ids) => {
                let mut total_bytes = 0u64;
                let mut total_files = 0usize;
                let mut failure = None;
                for id in &ids {
                    match crate::storage::reclaimer::clean_reclaimable_category(id) {
                        Ok((bytes, files)) => {
                            total_bytes += bytes;
                            total_files += files;
                        }
                        Err(e) => {
                            failure = Some(e);
                            break;
                        }
                    }
                }
                if let Some(e) = failure {
                    Err(ActionError::Failed(e))
                } else {
                    Ok(format!(
                        "Reclaimed {total_bytes} bytes across {total_files} files ({})",
                        ids.join(", ")
                    ))
                }
            }
        };
        let audit_result = result.map_err(|error| error.to_string());
        let mut record = actions::ActionAuditRecord::from_result(&plan, &audit_result);
        let undo = dynamic_undo.or(plan.undo);
        if let Some(commands::ActionCommand::RestoreStartup { quarantine_id, .. }) = &undo {
            record.quarantine_id = Some(quarantine_id.clone());
        }
        if let Err(error) = persistence::action_log::append(&record) {
            warn!(%error, "Failed to persist action audit record");
        }
        let event = match audit_result {
            Ok(_) => events::AppEvent::ActionCompleted {
                command: plan.command,
                record,
                undo,
            },
            Err(_) => events::AppEvent::ActionFailed {
                command: plan.command,
                record,
            },
        };
        let _ = events.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn test_action_worker_handles_reclaim_storage_caches() {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (evt_tx, evt_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            run_action_worker(cmd_rx, evt_tx);
        });

        cmd_tx
            .send(commands::ActionCommand::ReclaimStorageCaches(vec![]))
            .expect("send command");

        let event = evt_rx.recv().expect("receive event");
        match event {
            events::AppEvent::ActionCompleted { record, .. } => {
                assert!(record.succeeded);
                assert!(record.message.contains("Reclaimed 0 bytes across 0 files"));
            }
            events::AppEvent::ActionFailed { .. } => {
                panic!("Action should have succeeded");
            }
            _ => panic!("Unexpected event"),
        }

        drop(cmd_tx);
        let _ = handle.join();
    }

    #[test]
    fn test_action_worker_handles_reclaim_storage_caches_error_on_unknown_category() {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (evt_tx, evt_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            run_action_worker(cmd_rx, evt_tx);
        });

        cmd_tx
            .send(commands::ActionCommand::ReclaimStorageCaches(vec![
                "invalid_category_xyz".into(),
            ]))
            .expect("send command");

        let event = evt_rx.recv().expect("receive event");
        match event {
            events::AppEvent::ActionFailed { record, .. } => {
                assert!(!record.succeeded);
                assert!(
                    record
                        .message
                        .contains("Unknown reclaim category 'invalid_category_xyz'")
                );
            }
            events::AppEvent::ActionCompleted { .. } => {
                panic!("Action should have failed on invalid category");
            }
            _ => panic!("Unexpected event"),
        }

        drop(cmd_tx);
        let _ = handle.join();
    }
}
