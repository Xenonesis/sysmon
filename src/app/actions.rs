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
            ActionCommand::QuarantineStartup { item_name, locator } => {
                let mut plan = Self::new(
                    command.clone(),
                    format!("Quarantine startup item {item_name}"),
                    "The exact entry will be backed up in local app data and removed from its startup source.",
                    RiskLevel::High,
                    locator.requires_admin(),
                    None,
                );
                plan.reversible = true;
                plan
            }
            ActionCommand::RestoreStartup { item_name, .. } => Self::new(
                command.clone(),
                format!("Restore quarantined startup item {item_name}"),
                "The saved entry will be restored to its exact original source.",
                RiskLevel::Medium,
                true,
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
    use crate::startup::{StartupLocator, StartupRegistryHive};

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

    #[test]
    fn privileged_action_contracts_are_explicit() {
        let current_user_startup = StartupLocator::Registry {
            hive: StartupRegistryHive::CurrentUser,
            value_path: "Software\\Test".into(),
            enabled_value_path: "Software\\Test".into(),
            approved_path: "Software\\Approved".into(),
            value_name: "Example".into(),
        };
        let machine_startup = StartupLocator::Registry {
            hive: StartupRegistryHive::LocalMachine,
            value_path: "Software\\Test".into(),
            enabled_value_path: "Software\\Test".into(),
            approved_path: "Software\\Approved".into(),
            value_name: "Example".into(),
        };
        let cases = [
            (ActionCommand::KillProcess(7), RiskLevel::High, true, false),
            (ActionCommand::CleanRam, RiskLevel::Medium, true, false),
            (
                ActionCommand::ControlService {
                    name: "Example".into(),
                    action: ServiceControlAction::Stop,
                },
                RiskLevel::High,
                true,
                true,
            ),
            (
                ActionCommand::SetPowerPlan("balanced".into()),
                RiskLevel::Low,
                false,
                false,
            ),
            (
                ActionCommand::DisableStartup {
                    item_name: "User item".into(),
                    locator: current_user_startup,
                },
                RiskLevel::Medium,
                false,
                true,
            ),
            (
                ActionCommand::DisableStartup {
                    item_name: "Machine item".into(),
                    locator: machine_startup,
                },
                RiskLevel::Medium,
                true,
                true,
            ),
        ];

        for (command, risk, requires_admin, reversible) in cases {
            let plan = ActionPlan::from_command(command);
            assert_eq!(plan.risk, risk, "unexpected risk for {}", plan.title);
            assert_eq!(
                plan.requires_admin, requires_admin,
                "unexpected elevation for {}",
                plan.title
            );
            assert_eq!(
                plan.reversible, reversible,
                "unexpected Undo contract for {}",
                plan.title
            );
            assert!(!plan.summary.trim().is_empty());
        }
    }

    #[test]
    fn failed_actions_are_never_marked_reversible() {
        let plan = ActionPlan::from_command(ActionCommand::SuspendProcess(42));
        let record = ActionAuditRecord::from_result(&plan, &Err("access denied".into()));
        assert!(!record.succeeded);
        assert!(!record.reversible);
    }
}
