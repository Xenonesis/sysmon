// src/services.rs
use serde::Deserialize;
use std::fmt;
use std::time::{Duration, Instant};
use windows_service::service::{ServiceState, ServiceStatus};
use wmi::WMIConnection;

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServiceSortColumn {
    #[default]
    DisplayName,
    Name,
    State,
}

pub fn sort_services_refs(services: &mut [&ServiceInfo], column: ServiceSortColumn, ascending: bool) {
    services.sort_by(|a, b| {
        let cmp = match column {
            ServiceSortColumn::DisplayName => a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()),
            ServiceSortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            ServiceSortColumn::State => {
                let sa = a.state.to_lowercase();
                let sb = b.state.to_lowercase();
                sa.cmp(&sb)
                    .then_with(|| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
            }
        };
        if ascending { cmp } else { cmp.reverse() }
    });
}

#[derive(Debug, Clone, Copy)]
pub enum ServiceControlAction {
    Start,
    Stop,
    Restart,
}

impl fmt::Display for ServiceControlAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
        })
    }
}

#[derive(Debug, Clone)]
pub struct ServiceControlOutcome {
    pub action: ServiceControlAction,
    pub initial_state: ServiceState,
    pub transitioned: bool,
    pub final_process: Option<crate::processes::ProcessIdentity>,
    pub final_state: ServiceState,
}

#[derive(Debug, Clone)]
pub struct ServiceUndo {
    pub expected_state: ServiceState,
    pub expected_process: Option<crate::processes::ProcessIdentity>,
    pub restore_state: ServiceState,
}

impl ServiceControlOutcome {
    pub fn undo(&self) -> Option<ServiceUndo> {
        if !self.transitioned || self.initial_state == self.final_state {
            return None;
        }
        if self.final_state == ServiceState::Running && self.final_process.is_none() {
            return None;
        }
        Some(ServiceUndo {
            expected_state: self.final_state,
            expected_process: self.final_process,
            restore_state: self.initial_state,
        })
    }
}

fn validate_undo(
    undo: &ServiceUndo,
    state: ServiceState,
    process: Option<crate::processes::ProcessIdentity>,
) -> Result<(), ServiceControlError> {
    if state != undo.expected_state || process != undo.expected_process {
        return Err(ServiceControlError::RequestFailed {
            phase: "undo",
            detail: "Service changed since the original action; refusing stale Undo".into(),
        });
    }
    Ok(())
}

impl fmt::Display for ServiceControlOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} completed; {:?} -> {:?}; {}",
            self.action,
            self.initial_state,
            self.final_state,
            if self.transitioned {
                "transition observed"
            } else {
                "no change"
            }
        )
    }
}

#[derive(Debug, Clone)]
pub enum ServiceControlError {
    OpenManager(String),
    OpenService(String),
    RequestFailed {
        phase: &'static str,
        detail: String,
    },
    TimedOut {
        phase: &'static str,
        last_state: ServiceState,
    },
    PartialRestart {
        detail: String,
    },
}

impl fmt::Display for ServiceControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenManager(detail) => write!(f, "Could not open the Windows service manager: {detail}"),
            Self::OpenService(detail) => write!(f, "Could not open the service: {detail}"),
            Self::RequestFailed { phase, detail } => write!(f, "Service {phase} request failed: {detail}"),
            Self::TimedOut { phase, last_state } => {
                write!(
                    f,
                    "Service {phase} timed out after 30 seconds (last state: {last_state:?})"
                )
            }
            Self::PartialRestart { detail } => write!(f, "Service stopped but could not be restarted: {detail}"),
        }
    }
}

fn wait_for_state(
    service: &windows_service::service::Service,
    target: ServiceState,
    phase: &'static str,
) -> Result<ServiceStatus, ServiceControlError> {
    let hard_deadline = Instant::now() + Duration::from_secs(30);
    let mut last_progress = None;
    let mut progress_deadline = hard_deadline;

    loop {
        let status = service
            .query_status()
            .map_err(|error| ServiceControlError::RequestFailed {
                phase,
                detail: error.to_string(),
            })?;
        if status.current_state == target {
            return Ok(status);
        }

        let now = Instant::now();
        let progress = (status.current_state, status.checkpoint);
        if last_progress != Some(progress) {
            last_progress = Some(progress);
            let hint = if status.wait_hint.is_zero() {
                Duration::from_secs(5)
            } else {
                status.wait_hint.clamp(Duration::from_secs(1), Duration::from_secs(10))
            };
            progress_deadline = (now + hint).min(hard_deadline);
        }
        if now >= hard_deadline || now >= progress_deadline {
            return Err(ServiceControlError::TimedOut {
                phase,
                last_state: status.current_state,
            });
        }

        let poll = if status.wait_hint.is_zero() {
            Duration::from_millis(250)
        } else {
            (status.wait_hint / 10).clamp(Duration::from_millis(100), Duration::from_secs(1))
        };
        std::thread::sleep(poll);
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[allow(non_camel_case_types)]
struct Win32_Service {
    name: String,
    display_name: Option<String>,
    state: String,
}

pub fn send_service_control(
    name: &str,
    action: ServiceControlAction,
) -> Result<ServiceControlOutcome, ServiceControlError> {
    control_service(name, action, None)
}

pub fn undo_service_control(name: &str, undo: &ServiceUndo) -> Result<ServiceControlOutcome, ServiceControlError> {
    let action = match undo.restore_state {
        ServiceState::Stopped => ServiceControlAction::Stop,
        ServiceState::Running => ServiceControlAction::Start,
        _ => {
            return Err(ServiceControlError::RequestFailed {
                phase: "undo",
                detail: "Unsupported original service state".into(),
            });
        }
    };
    control_service(name, action, Some(undo))
}

fn control_service(
    name: &str,
    action: ServiceControlAction,
    undo: Option<&ServiceUndo>,
) -> Result<ServiceControlOutcome, ServiceControlError> {
    use windows_service::service::ServiceAccess;
    let manager = windows_service::service_manager::ServiceManager::local_computer(
        None::<&str>,
        windows_service::service_manager::ServiceManagerAccess::CONNECT,
    )
    .map_err(|error| ServiceControlError::OpenManager(error.to_string()))?;
    let rights = match action {
        ServiceControlAction::Start => ServiceAccess::START,
        ServiceControlAction::Stop => ServiceAccess::STOP,
        ServiceControlAction::Restart => ServiceAccess::START | ServiceAccess::STOP,
    };
    let service = manager
        .open_service(name, ServiceAccess::QUERY_STATUS | rights)
        .map_err(|error| ServiceControlError::OpenService(error.to_string()))?;
    let before = service
        .query_status()
        .map_err(|error| ServiceControlError::RequestFailed {
            phase: "query",
            detail: error.to_string(),
        })?;
    let identity = |status: &ServiceStatus| {
        status
            .process_id
            .and_then(|pid| crate::processes::process_identity(pid).ok())
    };
    if let Some(undo) = undo {
        validate_undo(undo, before.current_state, identity(&before))?;
    }
    if !matches!(before.current_state, ServiceState::Running | ServiceState::Stopped) {
        return Err(ServiceControlError::RequestFailed {
            phase: "control",
            detail: format!(
                "Cannot apply {action} while service is {:?}; wait for a stable Running/Stopped state",
                before.current_state
            ),
        });
    }
    let target = match action {
        ServiceControlAction::Stop => ServiceState::Stopped,
        _ => ServiceState::Running,
    };
    if before.current_state == target && !matches!(action, ServiceControlAction::Restart) {
        return Ok(ServiceControlOutcome {
            action,
            initial_state: before.current_state,
            final_state: before.current_state,
            transitioned: false,
            final_process: identity(&before),
        });
    }
    let mut stopped = false;
    if matches!(action, ServiceControlAction::Stop | ServiceControlAction::Restart)
        && before.current_state == ServiceState::Running
    {
        service.stop().map_err(|error| ServiceControlError::RequestFailed {
            phase: "stop",
            detail: error.to_string(),
        })?;
        wait_for_state(&service, ServiceState::Stopped, "stop")?;
        stopped = true;
    }
    if target == ServiceState::Running {
        let start_error = |detail: String| {
            if stopped {
                ServiceControlError::PartialRestart { detail }
            } else {
                ServiceControlError::RequestFailed { phase: "start", detail }
            }
        };
        service
            .start::<&str>(&[])
            .map_err(|error| start_error(error.to_string()))?;
        wait_for_state(&service, ServiceState::Running, "start").map_err(|error| start_error(error.to_string()))?;
    }
    let after = service
        .query_status()
        .map_err(|error| ServiceControlError::RequestFailed {
            phase: "final query",
            detail: error.to_string(),
        })?;
    if after.current_state != target {
        return Err(ServiceControlError::RequestFailed {
            phase: "final query",
            detail: format!("Service changed externally to {:?}", after.current_state),
        });
    }
    Ok(ServiceControlOutcome {
        action,
        initial_state: before.current_state,
        final_state: after.current_state,
        transitioned: true,
        final_process: identity(&after),
    })
}

pub fn get_services() -> Result<Vec<ServiceInfo>, String> {
    let connection = WMIConnection::new().map_err(|error| format!("Service inventory unavailable: {error}"))?;
    let services: Vec<Win32_Service> = connection
        .raw_query("SELECT Name, DisplayName, State FROM Win32_Service")
        .map_err(|error| format!("Service inventory query failed: {error}"))?;
    let mut result: Vec<_> = services
        .into_iter()
        .map(|svc| ServiceInfo {
            display_name: svc.display_name.unwrap_or_else(|| svc.name.clone()),
            name: svc.name,
            state: svc.state,
        })
        .collect();
    result.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_service_start_cannot_offer_stop_undo() {
        let outcome = ServiceControlOutcome {
            action: ServiceControlAction::Start,
            initial_state: ServiceState::Running,
            final_state: ServiceState::Running,
            transitioned: false,
            final_process: Some(crate::processes::ProcessIdentity {
                pid: 99,
                creation_time: 10,
            }),
        };
        assert!(outcome.undo().is_none());
    }

    #[test]
    fn service_undo_refuses_external_restart_or_state_change() {
        let identity = crate::processes::ProcessIdentity {
            pid: 99,
            creation_time: 10,
        };
        let outcome = ServiceControlOutcome {
            action: ServiceControlAction::Start,
            initial_state: ServiceState::Stopped,
            final_state: ServiceState::Running,
            transitioned: true,
            final_process: Some(identity),
        };
        let undo = outcome.undo().unwrap();
        assert!(validate_undo(&undo, ServiceState::Running, Some(identity)).is_ok());
        assert!(validate_undo(&undo, ServiceState::Stopped, None).is_err());
        assert!(
            validate_undo(
                &undo,
                ServiceState::Running,
                Some(crate::processes::ProcessIdentity {
                    creation_time: 11,
                    ..identity
                })
            )
            .is_err()
        );
    }

    #[test]
    fn test_sort_services_refs() {
        let s1 = ServiceInfo {
            name: "svc_c".to_string(),
            display_name: "Apple Service".to_string(),
            state: "Stopped".to_string(),
        };
        let s2 = ServiceInfo {
            name: "svc_a".to_string(),
            display_name: "Zebra Service".to_string(),
            state: "Running".to_string(),
        };
        let s3 = ServiceInfo {
            name: "svc_b".to_string(),
            display_name: "Mango Service".to_string(),
            state: "Running".to_string(),
        };

        let mut list = vec![&s1, &s2, &s3];

        // Sort by Display Name ascending
        sort_services_refs(&mut list, ServiceSortColumn::DisplayName, true);
        assert_eq!(list[0].display_name, "Apple Service");
        assert_eq!(list[1].display_name, "Mango Service");
        assert_eq!(list[2].display_name, "Zebra Service");

        // Sort by Display Name descending
        sort_services_refs(&mut list, ServiceSortColumn::DisplayName, false);
        assert_eq!(list[0].display_name, "Zebra Service");
        assert_eq!(list[2].display_name, "Apple Service");

        // Sort by Identifier Name ascending
        sort_services_refs(&mut list, ServiceSortColumn::Name, true);
        assert_eq!(list[0].name, "svc_a");
        assert_eq!(list[1].name, "svc_b");
        assert_eq!(list[2].name, "svc_c");

        // Sort by State ascending (Running before Stopped)
        sort_services_refs(&mut list, ServiceSortColumn::State, true);
        assert_eq!(list[0].state, "Running");
        assert_eq!(list[1].state, "Running");
        assert_eq!(list[2].state, "Stopped");
    }
}
