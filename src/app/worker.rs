//! Background worker for executing guarded system actions off the UI thread.

use crate::app::models::SystemMonitor;
use crate::app::{actions, commands::ActionCommand, events};
use crate::{persistence, power, processes, services, startup};
use std::sync::mpsc::{Receiver, Sender};
use tracing::warn;

pub(crate) fn run_action_worker(commands: Receiver<ActionCommand>, events: Sender<events::AppEvent>) {
    let mut monitor = SystemMonitor::new();
    while let Ok(command) = commands.recv() {
        let plan = actions::ActionPlan::from_command(command.clone());
        let automatic = matches!(command, ActionCommand::AutoCleanRam { .. });
        let mut dynamic_undo = None;
        let mut ram_outcome = None;
        let result: Result<String, String> = match command {
            ActionCommand::KillProcess(identity) => monitor
                .kill_process(identity)
                .map(|_| format!("Process {identity} terminated")),
            ActionCommand::SuspendProcess(identity) => monitor
                .suspend_process(identity)
                .map(|_| format!("Process {identity} suspended")),
            ActionCommand::ResumeProcess(identity) => monitor
                .resume_process(identity)
                .map(|_| format!("Process {identity} resumed")),
            ActionCommand::SetPriority { identity, priority } => {
                SystemMonitor::set_process_priority(identity, &priority)
                    .map(|_| format!("Process {identity} priority set to {priority}"))
            }
            ActionCommand::SetAffinity { identity, preset } => processes::set_process_affinity(identity, preset)
                .map(|_| format!("Process {identity} affinity set to {preset:?}")),
            ActionCommand::CleanRam => monitor.clean_ram(&[], false, None, None, false).map(|outcome| {
                let message = outcome.to_string();
                ram_outcome = Some(outcome);
                message
            }),
            ActionCommand::AutoCleanRam {
                exclusions,
                smart_only,
                budget_bytes,
                target_percent,
                idle_only,
            } => monitor
                .clean_ram(&exclusions, smart_only, budget_bytes, Some(target_percent), idle_only)
                .map(|outcome| {
                    let message = outcome.to_string();
                    ram_outcome = Some(outcome);
                    message
                }),
            ActionCommand::ControlService { name, action } => services::send_service_control(&name, action)
                .map(|outcome| {
                    dynamic_undo = outcome.undo().map(|undo| ActionCommand::UndoService {
                        name: name.clone(),
                        undo,
                    });
                    format!("Service {name}: {outcome}")
                })
                .map_err(|error| error.to_string()),
            ActionCommand::UndoService { name, undo } => services::undo_service_control(&name, &undo)
                .map(|outcome| format!("Service {name}: {outcome}"))
                .map_err(|error| error.to_string()),
            ActionCommand::SetPowerPlan(guid) => {
                power::set_active_power_plan(&guid).map(|_| "Power plan changed".into())
            }
            ActionCommand::KillProcessTree(root) => kill_tree(&mut monitor, root),
            ActionCommand::DisableStartup { item_name, locator } => {
                startup::disable_startup(&locator).map(|_| format!("Startup item {item_name} disabled"))
            }
            ActionCommand::EnableStartup { item_name, locator } => {
                startup::enable_startup(&locator).map(|_| format!("Startup item {item_name} enabled"))
            }
            ActionCommand::QuarantineStartup { item_name, locator } => {
                startup::quarantine_startup(&item_name, &locator).map(|id| {
                    let review = startup::prepare_startup_restore(&id);
                    match review {
                        Ok(review) => {
                            dynamic_undo = Some(ActionCommand::RestoreStartup { review });
                            format!("Startup item {item_name} quarantined (backup {id})")
                        }
                        Err(error) => {
                            format!("Startup item {item_name} quarantined (backup {id}); Undo unavailable: {error}")
                        }
                    }
                })
            }
            ActionCommand::RestoreStartup { review } => {
                startup::restore_startup(&review).map(|_| format!("Startup item {} restored", review.item_name()))
            }
            ActionCommand::ReclaimStorageCaches(review) => {
                let outcome =
                    crate::storage::reclaimer::clean_reviewed(&review, &std::sync::atomic::AtomicBool::new(false));
                let mut message = outcome.summary();
                for issue in &outcome.issues {
                    message.push_str(&format!("\n{}: {}", issue.path.display(), issue.reason));
                }
                if outcome.failed > 0 || outcome.skipped > 0 || outcome.cancelled {
                    Err(message)
                } else {
                    Ok(message)
                }
            }
        };
        let undo = if result.is_ok() {
            dynamic_undo.or(plan.undo.clone())
        } else {
            None
        };
        let mut record = actions::ActionAuditRecord::from_result(&plan, &result);
        record.reversible = undo.is_some();
        if automatic {
            record.initiator = "automatic policy".into();
        }
        if let Some(ActionCommand::RestoreStartup { review }) = &undo {
            record.quarantine_id = Some(review.id().to_string());
        }
        if let Err(error) = persistence::action_log::append(&record) {
            warn!(%error, "Failed to persist action audit record");
        }
        let event = if result.is_ok() {
            events::AppEvent::ActionCompleted {
                command: plan.command,
                record,
                undo,
                ram_outcome,
            }
        } else {
            events::AppEvent::ActionFailed {
                command: plan.command,
                record,
            }
        };
        if events.send(event).is_err() {
            break;
        }
    }
}

fn kill_tree(monitor: &mut SystemMonitor, root: processes::ProcessIdentity) -> Result<String, String> {
    // Validate root before taking the snapshot; holding this handle pins its lifetime.
    let _root_handle =
        processes::open_process_for_action(root, windows_sys::Win32::System::Threading::PROCESS_TERMINATE)?;
    monitor.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let snapshot: Vec<_> = monitor
        .sys
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            processes::process_identity(pid.as_u32())
                .ok()
                .map(|identity| (identity, process.parent().map(|pid| pid.as_u32())))
        })
        .collect();
    let targets = processes::process_tree_targets(root, &snapshot)?;
    // Open and validate EVERY target before the first mutation. Guarded handles remain pinned.
    let handles: Vec<_> = targets
        .iter()
        .map(|identity| {
            processes::open_process_for_action(*identity, windows_sys::Win32::System::Threading::PROCESS_TERMINATE)
        })
        .collect::<Result<_, _>>()?;
    let mut terminated = 0;
    for (identity, handle) in targets.iter().zip(&handles) {
        if unsafe { windows_sys::Win32::System::Threading::TerminateProcess(handle.0, 1) } == 0 {
            return Err(format!(
                "Terminated {terminated} of {} snapshot processes; TerminateProcess({identity}): {}",
                targets.len(),
                std::io::Error::last_os_error()
            ));
        }
        terminated += 1;
    }
    Ok(format!(
        "Terminated {terminated} snapshot processes; descendants created after the snapshot are not included"
    ))
}
