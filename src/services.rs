// src/services.rs
use serde::Deserialize;
use wmi::{WMIConnection, COMLibrary};

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: String,
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

#[derive(Deserialize, Debug)]
#[allow(non_camel_case_types)]
struct Win32_Service {
    name: String,
    display_name: Option<String>,
    state: String,
}

pub fn send_service_control(name: &str, action: ServiceControlAction) -> bool {
    let manager = match windows_service::service_manager::ServiceManager::local_computer(None::<&str>, windows_service::service_manager::ServiceManagerAccess::CONNECT) {
        Ok(m) => m,
        Err(_) => return false,
    };
    let desired_access = windows_service::service::ServiceAccess::QUERY_STATUS | windows_service::service::ServiceAccess::START | windows_service::service::ServiceAccess::STOP;
    let service = match manager.open_service(name, desired_access) {
        Ok(s) => s,
        Err(_) => return false,
    };
    match action {
        ServiceControlAction::Start => {
            let empty: Vec<String> = Vec::new();
            service.start(&empty).is_ok()
        }
        ServiceControlAction::Stop => service.stop().is_ok(),
        ServiceControlAction::Restart => {
            let stop_ok = service.stop().is_ok();
            let _ = std::thread::sleep(std::time::Duration::from_millis(250));
            let empty: Vec<String> = Vec::new();
            let start_ok = service.start(&empty).is_ok();
            stop_ok || start_ok
        }
    }
}

pub fn get_services() -> Vec<ServiceInfo> {
    let mut result = Vec::new();
    
    // Fallback to WMI because `windows-service` crate does not have `enumerate_services_status`
    // and `ServiceAccess` is a private struct in the `service_manager` module.
    if let Ok(com_lib) = COMLibrary::new() {
        if let Ok(wmi_con) = WMIConnection::new(com_lib.into()) {
            let results: Result<Vec<Win32_Service>, _> = wmi_con.raw_query("SELECT Name, DisplayName, State FROM Win32_Service");
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
