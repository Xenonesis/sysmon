use crate::app::models::*;
use crate::monitoring::engine::SystemMonitorApp;
use eframe::egui;

impl SystemMonitorApp {
    pub(crate) fn export_diagnostics(
        &self,
        destination: &std::path::Path,
    ) -> Result<std::path::PathBuf, std::io::Error> {
        let snapshot = self
            .latest_snapshot
            .as_ref()
            .cloned()
            .unwrap_or_else(|| crate::snapshot_from_data(&self.data.read()));
        crate::persistence::diagnostics::export(destination, &snapshot, &self.settings)
    }

    pub(crate) fn export_to_csv(data: &SystemData) -> Result<String, Box<dyn std::error::Error>> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Header
        wtr.write_record(["Category", "Metric", "Value"])?;

        // System info
        wtr.write_record(["System", "Timestamp", &data.last_update])?;
        wtr.write_record(["CPU", "Usage %", &format!("{:.2}", data.cpu_usage)])?;
        wtr.write_record([
            "Memory",
            "Total GB",
            &format!("{:.2}", crate::ui::components::bytes_to_gb(data.memory_total)),
        ])?;
        wtr.write_record([
            "Memory",
            "Used GB",
            &format!("{:.2}", crate::ui::components::bytes_to_gb(data.memory_used)),
        ])?;
        wtr.write_record(["Memory", "Usage %", &format!("{:.2}", data.memory_percentage)])?;

        // GPU
        for gpu in &data.gpu_info {
            wtr.write_record(["GPU", "Name", &gpu.name])?;
            wtr.write_record([
                "GPU",
                "Usage %",
                &gpu.utilization
                    .map(|v| format!("{v:.2}"))
                    .unwrap_or_else(|| "N/A".into()),
            ])?;
            if let Some(temp) = gpu.temperature {
                wtr.write_record(["GPU", "Temperature C", &format!("{}", temp)])?;
            }
            if let Some(clock) = gpu.clock_mhz {
                wtr.write_record(["GPU", "Clock MHz", &clock.to_string()])?;
            }
            if let Some(power) = gpu.power_watts {
                wtr.write_record(["GPU", "Power W", &format!("{:.1}", power)])?;
            }
            if let Some(fan) = gpu.fan_percent {
                wtr.write_record(["GPU", "Fan %", &fan.to_string()])?;
            }
        }

        // Top processes (kept as 3 fields: Category/Metric/Value to keep the CSV rectangular)
        for proc in &data.top_processes {
            let vram_str = proc
                .vram_bytes
                .map(|b| format!("{:.2}", crate::ui::components::bytes_to_mb(b)))
                .unwrap_or_else(|| "-".to_string());
            wtr.write_record([
                "Process",
                &format!("PID {} ({})", proc.pid, proc.name),
                &format!(
                    "Memory: {:.2} MB, VRAM: {} MB, CPU: {:.2}%",
                    crate::ui::components::bytes_to_mb(proc.memory),
                    vram_str,
                    proc.cpu_usage
                ),
            ])?;
        }

        let csv_data = String::from_utf8(wtr.into_inner()?)?;
        Ok(csv_data)
    }

    pub(crate) fn export_data_to_json(&self, data: &SystemData) -> Result<String, Box<dyn std::error::Error>> {
        use serde::Serialize;
        #[derive(Serialize)]
        struct ExportData {
            timestamp: String,
            cpu_usage: f32,
            memory_used: u64,
            memory_total: u64,
            memory_percentage: f32,
            gpu_info: Option<GpuInfo>,
            top_processes: Vec<crate::processes::ProcessInfo>,
            disk_info: Vec<DiskInfo>,
            network_info: Vec<NetworkInfo>,
            system_info: SystemInfo,
            startup_item_count: usize,
            high_impact_startup_count: usize,
            boot_diagnostics: Option<crate::startup::BootDiagnostics>,
            swap_info: SwapInfo,
            battery_info: Option<BatteryInfo>,
            network_download_history: Vec<DataPoint>,
            network_upload_history: Vec<DataPoint>,
            disk_read_history: Vec<DataPoint>,
            disk_write_history: Vec<DataPoint>,
        }

        let export = ExportData {
            timestamp: data.last_update.clone(),
            cpu_usage: data.cpu_usage,
            memory_used: data.memory_used,
            memory_total: data.memory_total,
            memory_percentage: data.memory_percentage,
            gpu_info: data.gpu_info.first().cloned(),
            top_processes: data.top_processes.clone(),
            disk_info: data.disk_info.clone(),
            network_info: data.network_info.clone(),
            system_info: data.system_info.clone(),
            startup_item_count: self.startup_items.len(),
            high_impact_startup_count: crate::startup::high_impact_count(&self.startup_items),
            boot_diagnostics: self.boot_diagnostics.clone(),
            swap_info: data.swap_info.clone(),
            battery_info: data.battery_info.clone(),
            network_download_history: data.network_download_history.iter().copied().collect(),
            network_upload_history: data.network_upload_history.iter().copied().collect(),
            disk_read_history: data.disk_read_history.iter().copied().collect(),
            disk_write_history: data.disk_write_history.iter().copied().collect(),
        };

        Ok(serde_json::to_string_pretty(&export)?)
    }

    pub fn queue_action(&mut self, command: crate::app::commands::ActionCommand) -> bool {
        if self.action_pending || self.pending_action_plan.is_some() {
            self.action_status = Some("Another system action is already pending.".into());
            return false;
        }
        self.pending_action_plan = Some(crate::app::actions::ActionPlan::from_command(command));
        true
    }

    pub fn start_ram_clean(&mut self, _ctx: &egui::Context) {
        self.queue_action(crate::app::commands::ActionCommand::CleanRam);
    }
}

#[cfg(test)]
mod csv_export_tests {
    use super::*;

    // ponytail: smallest check that fails if export mixes 3- and 5-field records again
    #[test]
    fn export_csv_is_rectangular_and_parseable() {
        let data = SystemData {
            last_update: "2026-01-01 00:00:00".into(),
            cpu_usage: 12.5,
            top_processes: vec![crate::processes::ProcessInfo {
                pid: 1234,
                start_time: 0,
                identity: None,
                name: "test_a.exe".into(),
                parent_pid: Some(4),
                cpu_usage: 3.2,
                memory: 50 * 1024 * 1024,
                vram_bytes: Some(1024 * 1024),
                status: "Running".into(),
                disk_read_bytes: 0,
                disk_written_bytes: 0,
                disk_read_bytes_per_second: None,
                disk_written_bytes_per_second: None,
            }],
            ..Default::default()
        };
        let csv_data = SystemMonitorApp::export_to_csv(&data).expect("export must succeed");
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(false)
            .from_reader(csv_data.as_bytes());
        for record in rdr.records() {
            record.expect("every CSV record must have 3 fields");
        }
    }
}
