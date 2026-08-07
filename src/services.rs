// src/services.rs
use serde::Deserialize;
use wmi::{WMIConnection, COMLibrary};

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_camel_case_types)]
struct Win32_Service {
    name: String,
    display_name: Option<String>,
    state: String,
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
