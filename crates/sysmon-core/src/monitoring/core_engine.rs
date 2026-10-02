use crate::app::models::*;
use crate::monitoring;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::time::Instant;
use sysinfo::{Disks, Networks, System};

#[cfg(target_os = "windows")]
use nvml_wrapper::Nvml;

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        #[cfg(target_os = "windows")]
        let nvml = Nvml::init().ok();

        #[cfg(target_os = "windows")]
        let wmi_thermal = wmi::WMIConnection::with_namespace_path("ROOT\\WMI").ok();

        SystemMonitor {
            sys,
            disks,
            networks,
            #[cfg(target_os = "windows")]
            nvml,
            #[cfg(target_os = "windows")]
            wmi_thermal,
            last_network_update: Instant::now(),
            last_disk_update: Instant::now(),
            previous_network_totals: std::collections::HashMap::new(),
            previous_disk_totals: std::collections::HashMap::new(),
        }
    }
}

