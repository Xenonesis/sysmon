use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::commands::ActionCommand;
use crate::services::ServiceControlAction;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
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
                Some(ActionCommand::SuspendProcess(*pid)),
            ),
            ActionCommand::SetPriority { pid, priority } => Self::new(
                command.clone(),
                format!("Set process {pid} priority to {priority}"),
                "Changing scheduling priority can affect system responsiveness.",
                RiskLevel::Medium,
                true,
                None,
            ),
            ActionCommand::SetAffinity { pid, mask } => Self::new(
                command.clone(),
                format!("Set process {pid} CPU affinity to {mask:#x}"),
                "Constrains execution to specified logical CPU processor cores.",
                RiskLevel::Medium,
                true,
                None,
            ),
            ActionCommand::CleanRam => Self::new(
                command.clone(),
                "Trim process working sets".into(),
                "Windows may need to page trimmed memory back in; short-lived slowdowns are possible.",
                RiskLevel::Medium,
                true,
                None,
            ),
            ActionCommand::ControlService { name, action } => {
                let (verb, risk, undo) = match action {
                    ServiceControlAction::Start => (
                        "Start",
                        RiskLevel::Medium,
                        Some(ActionCommand::ControlService {
                            name: name.clone(),
                            action: ServiceControlAction::Stop,
                        }),
                    ),
                    ServiceControlAction::Stop => (
                        "Stop",
                        RiskLevel::High,
                        Some(ActionCommand::ControlService {
                            name: name.clone(),
                            action: ServiceControlAction::Start,
                        }),
                    ),
                    ServiceControlAction::Restart => ("Restart", RiskLevel::High, None),
                };
                Self::new(
                    command.clone(),
                    format!("{verb} service {name}"),
                    "Dependent applications or Windows components may be interrupted.",
                    risk,
                    true,
                    undo,
                )
            }
            ActionCommand::SetPowerPlan(guid) => Self::new(
                command.clone(),
                "Change active power plan".into(),
                format!("Windows will activate power scheme {guid}."),
                RiskLevel::Low,
                false,
                None,
            ),
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
        }
    }

    pub(crate) fn automatic(action: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            action: action.into(),
            risk: RiskLevel::Low,
            succeeded: true,
            message: message.into(),
            reversible: false,
            initiator: "automatic policy".into(),
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
    fn kill_tree_is_critical_and_irreversible() {
        let plan = ActionPlan::from_command(ActionCommand::KillProcessTree(42));
        assert!(matches!(plan.risk, RiskLevel::Critical));
        assert!(!plan.reversible);
    }

    #[test]
    fn suspend_has_resume_undo() {
        let plan = ActionPlan::from_command(ActionCommand::SuspendProcess(42));
        assert!(matches!(plan.undo, Some(ActionCommand::ResumeProcess(42))));
    }
}
