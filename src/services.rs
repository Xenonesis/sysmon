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
        if ascending {
            cmp
        } else {
            cmp.reverse()
        }
    });
}

#[derive(Debug, Clone)]
pub struct ServiceAction {
    pub name: String,
    pub action: ServiceControlAction,
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
    pub final_state: ServiceState,
}

impl fmt::Display for ServiceControlOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} completed; service is {:?}", self.action, self.final_state)
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
    let manager = windows_service::service_manager::ServiceManager::local_computer(
        None::<&str>,
        windows_service::service_manager::ServiceManagerAccess::CONNECT,
    )
    .map_err(|error| ServiceControlError::OpenManager(error.to_string()))?;
    let desired_access = windows_service::service::ServiceAccess::QUERY_STATUS
        | windows_service::service::ServiceAccess::START
        | windows_service::service::ServiceAccess::STOP;
    let service = manager
        .open_service(name, desired_access)
        .map_err(|error| ServiceControlError::OpenService(error.to_string()))?;
    match action {
        ServiceControlAction::Start => {
            if service
                .query_status()
                .is_ok_and(|status| status.current_state == ServiceState::Running)
            {
                return Ok(ServiceControlOutcome {
                    action,
                    final_state: ServiceState::Running,
                });
            }
            let empty: Vec<String> = Vec::new();
            service
                .start(&empty)
                .map_err(|error| ServiceControlError::RequestFailed {
                    phase: "start",
                    detail: error.to_string(),
                })?;
            let status = wait_for_state(&service, ServiceState::Running, "start")?;
            Ok(ServiceControlOutcome {
                action,
                final_state: status.current_state,
            })
        }
        ServiceControlAction::Stop => {
            if service
                .query_status()
                .is_ok_and(|status| status.current_state == ServiceState::Stopped)
            {
                return Ok(ServiceControlOutcome {
                    action,
                    final_state: ServiceState::Stopped,
                });
            }
            service.stop().map_err(|error| ServiceControlError::RequestFailed {
                phase: "stop",
                detail: error.to_string(),
            })?;
            let status = wait_for_state(&service, ServiceState::Stopped, "stop")?;
            Ok(ServiceControlOutcome {
                action,
                final_state: status.current_state,
            })
        }
        ServiceControlAction::Restart => {
            if !service
                .query_status()
                .is_ok_and(|status| status.current_state == ServiceState::Stopped)
            {
                service.stop().map_err(|error| ServiceControlError::RequestFailed {
                    phase: "restart stop",
                    detail: error.to_string(),
                })?;
                wait_for_state(&service, ServiceState::Stopped, "restart stop")?;
            }

            let empty: Vec<String> = Vec::new();
            service
                .start(&empty)
                .map_err(|error| ServiceControlError::PartialRestart {
                    detail: error.to_string(),
                })?;
            let status = wait_for_state(&service, ServiceState::Running, "restart start").map_err(|error| {
                ServiceControlError::PartialRestart {
                    detail: error.to_string(),
                }
            })?;
            Ok(ServiceControlOutcome {
                action,
                final_state: status.current_state,
            })
        }
    }
}

pub fn get_services() -> Vec<ServiceInfo> {
    get_services_with_com(None)
}

pub fn get_services_with_com(com: Option<&std::rc::Rc<wmi::COMLibrary>>) -> Vec<ServiceInfo> {
    let mut result = Vec::new();

    if let Some(com_lib) = com {
        if let Ok(wmi_con) = WMIConnection::new(com_lib.clone()) {
            let results: Result<Vec<Win32_Service>, _> =
                wmi_con.raw_query("SELECT Name, DisplayName, State FROM Win32_Service");
            if let Ok(services) = results {
                for svc in services {
                    result.push(ServiceInfo {
                        name: svc.name,
                        display_name: svc.display_name.unwrap_or_default(),
                        state: svc.state,
                    });
                }
            }
        }
    } else if let Ok(com_lib) = crate::providers::init_com() {
        if let Ok(wmi_con) = WMIConnection::new(std::rc::Rc::new(com_lib)) {
            let results: Result<Vec<Win32_Service>, _> =
                wmi_con.raw_query("SELECT Name, DisplayName, State FROM Win32_Service");
            if let Ok(services) = results {
                for svc in services {
                    result.push(ServiceInfo {
                        name: svc.name,
                        display_name: svc.display_name.unwrap_or_default(),
                        state: svc.state,
                    });
                }
            }
        }
    }

    result.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

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
