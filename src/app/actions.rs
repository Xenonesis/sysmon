use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::commands::ActionCommand;
use crate::services::ServiceControlAction;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ActionPlan {
    pub command: ActionCommand,
    pub title: String,
    pub summary: String,
    pub risk: RiskLevel,
    pub requires_admin: bool,
    pub reversible: bool,
    pub undo: Option<ActionCommand>,
}

impl ActionPlan {
    pub(crate) fn from_command(command: ActionCommand) -> Self {
        match &command {
            ActionCommand::KillProcess(pid) => Self::new(
                command.clone(),
                format!("Terminate process {pid}"),
                "The process will exit immediately and unsaved work may be lost.",
                RiskLevel::High,
                true,
                None,
            ),
            ActionCommand::KillProcessTree(pid) => Self::new(
                command.clone(),
                format!("Terminate process tree {pid}"),
                "The process and every discovered child will be terminated deepest-first.",
                RiskLevel::Critical,
                true,
                None,
            ),
            ActionCommand::SuspendProcess(pid) => Self::new(
                command.clone(),
                format!("Suspend process {pid}"),
                "Execution will be frozen until the process is resumed.",
                RiskLevel::Medium,
                true,
                Some(ActionCommand::ResumeProcess(*pid)),
            ),
            ActionCommand::ResumeProcess(pid) => Self::new(
                command.clone(),
                format!("Resume process {pid}"),
                "Execution of the suspended process will continue.",
                RiskLevel::Low,
                true,
                None,
            ),
            ActionCommand::SetPriority { identity, priority } => Self::new(
                command.clone(),
                format!("Set process {identity} priority to {priority}"),
                "Changing scheduling priority can affect system responsiveness.",
                RiskLevel::Medium,
                true,
                None,
            ),
            ActionCommand::SetAffinity { identity, preset } => Self::new(
                command.clone(),
                format!("Set process {identity} CPU affinity to {preset:?}"),
                "Constrains execution to specified logical CPU processor cores.",
                RiskLevel::Medium,
                true,
                None,
            ),
            ActionCommand::CleanRam | ActionCommand::AutoCleanRam { .. } => Self::new(
                command.clone(),
                "Trim process working sets".into(),
                "Windows may need to page trimmed memory back in; short-lived slowdowns are possible.",
                RiskLevel::Medium,
                true,
                None,
            ),
            ActionCommand::ControlService { name, action } => {
                let mut plan = Self::new(
                    command.clone(),
                    format!(
                        "{} service {name}",
                        match action {
                            ServiceControlAction::Start => "Start",
                            ServiceControlAction::Stop => "Stop",
                            ServiceControlAction::Restart => "Restart",
                        }
                    ),
                    "Dependent applications or Windows components may be interrupted. Undo is offered only after an observed transition.",
                    if matches!(action, ServiceControlAction::Start) {
                        RiskLevel::Medium
                    } else {
                        RiskLevel::High
                    },
                    true,
                    None,
                );
                // Undo is derived from the observed previous state after execution, not a static command.
                if !matches!(action, ServiceControlAction::Restart) {
                    plan.reversible = true;
                }
                plan
            }
            ActionCommand::UndoService { name, .. } => Self::new(
                command.clone(),
                format!("Restore service {name}"),
                "Restores the observed previous stable state only if the service still matches the completed action.",
                RiskLevel::High,
                true,
                None,
            ),
            ActionCommand::SetPowerPlan(guid) => Self::new(
                command.clone(),
                "Change active power plan".into(),
                format!("Windows will activate power scheme {guid}."),
                RiskLevel::Low,
                false,
                None,
            ),
            ActionCommand::DisableStartup { item_name, locator } => Self::new(
                command.clone(),
                format!("Disable startup item {item_name}"),
                "The exact startup entry will be disabled without deleting it.",
                RiskLevel::Medium,
                locator.requires_admin(),
                Some(ActionCommand::EnableStartup {
                    item_name: item_name.clone(),
                    locator: locator.clone(),
                }),
            ),
            ActionCommand::EnableStartup { item_name, locator } => Self::new(
                command.clone(),
                format!("Enable startup item {item_name}"),
                "The exact startup entry will run again at the next applicable sign-in.",
                RiskLevel::Medium,
                locator.requires_admin(),
                Some(ActionCommand::DisableStartup {
                    item_name: item_name.clone(),
                    locator: locator.clone(),
                }),
            ),
            ActionCommand::QuarantineStartup { item_name, .. } => {
                let mut plan = Self::new(
                    command.clone(),
                    format!("Quarantine startup item {item_name}"),
                    "The exact entry will be backed up in the administrator-protected store and removed from its startup source.",
                    RiskLevel::High,
                    true,
                    None,
                );
                plan.reversible = true;
                plan
            }
            ActionCommand::RestoreStartup { review } => Self::new(
                command.clone(),
                format!("Restore quarantined startup item {}", review.item_name()),
                review.summary(),
                RiskLevel::Medium,
                true,
                None,
            ),
            ActionCommand::ReclaimStorageCaches(review) => {
                let requires_admin = review.category_ids().iter().any(|id| id == "windows_update");
                Self::new(
                    command.clone(),
                    format!(
                        "Reclaim {} reviewed files ({} bytes)",
                        review.file_count(),
                        review.size_bytes()
                    ),
                    format!(
                        "Only the reviewed manifest is eligible. Categories: {}. Replaced, changed, or unsafe files will be skipped.",
                        review.category_ids().join(", ")
                    ),
                    RiskLevel::Low,
                    requires_admin,
                    None,
                )
            }
        }
    }

    fn new(
        command: ActionCommand,
        title: String,
        summary: impl Into<String>,
        risk: RiskLevel,
        requires_admin: bool,
        undo: Option<ActionCommand>,
    ) -> Self {
        Self {
            reversible: undo.is_some(),
            command,
            title,
            summary: summary.into(),
            risk,
            requires_admin,
            undo,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ActionAuditRecord {
    pub timestamp: String,
    pub action: String,
    pub risk: RiskLevel,
    pub succeeded: bool,
    pub message: String,
    pub reversible: bool,
    #[serde(default = "default_initiator")]
    pub initiator: String,
    #[serde(default)]
    pub quarantine_id: Option<String>,
}

fn default_initiator() -> String {
    "user".into()
}

impl ActionAuditRecord {
    pub(crate) fn from_result(plan: &ActionPlan, result: &Result<String, String>) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            action: plan.title.clone(),
            risk: plan.risk,
            succeeded: result.is_ok(),
            message: result.as_ref().map_or_else(Clone::clone, Clone::clone),
            reversible: plan.reversible && result.is_ok(),
            initiator: default_initiator(),
            quarantine_id: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn automatic(action: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            action: action.into(),
            risk: RiskLevel::Low,
            succeeded: true,
            message: message.into(),
            reversible: false,
            initiator: "automatic policy".into(),
            quarantine_id: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ActionHistoryEntry {
    pub record: ActionAuditRecord,
    pub undo: Option<ActionCommand>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_actions_are_never_marked_reversible() {
        let plan = ActionPlan::from_command(ActionCommand::SuspendProcess(crate::processes::ProcessIdentity {
            pid: 42,
            creation_time: 123,
        }));
        let record = ActionAuditRecord::from_result(&plan, &Err("access denied".into()));
        assert!(!record.succeeded);
        assert!(!record.reversible);
    }
}
