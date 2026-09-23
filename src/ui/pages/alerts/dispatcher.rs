use crate::*;
use eframe::egui;

#[derive(Default)]
pub(super) struct AlertActions {
    pub(super) remove_alert_idx: Option<usize>,
    pub(super) clear_all_alerts: bool,
    pub(super) trigger_test_alert: bool,
    pub(super) navigate_tab: Option<Tab>,
    pub(super) run_ram_clean: bool,
}

impl AlertActions {
    pub(super) fn dispatch(self, app: &mut crate::SystemMonitorApp, ctx: &egui::Context) {
        if let Some(idx) = self.remove_alert_idx {
            let mut d = app.data.write();
            if idx < d.alerts.len() {
                d.alerts.remove(idx);
            }
        }

        if self.clear_all_alerts {
            app.data.write().alerts.clear();
        }

        if self.trigger_test_alert {
            play_alert_sound();
            if app.settings.show_notifications {
                let _ = notify_rust::Notification::new()
                    .summary("SysMon Alert Simulation")
                    .body("Diagnostic test alert triggered. Audio chime & notification verified.")
                    .timeout(notify_rust::Timeout::Milliseconds(5000))
                    .show();
            }
            let mut d = app.data.write();
            d.alerts.push(AlertInfo {
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                alert_type: AlertType::CpuHigh,
                source: AlertSource::Cpu,
                message: "Simulated Test Alert: CPU load threshold exceeded (Diagnostic Test)".to_string(),
                resolved_at: None,
                value: 95.0,
            });
        }

        if let Some(tab) = self.navigate_tab {
            app.selected_tab = tab;
        }

        if self.run_ram_clean {
            app.start_ram_clean(ctx);
        }
    }
}
