use crate::app::models::SystemMonitor;
use crate::app::models::{AlertInfo, AlertSource, AlertType, AppSettings, SystemData};
use chrono::Local;

impl SystemMonitor {
    pub(crate) fn check_alerts(settings: &AppSettings, data: &SystemData) -> Vec<AlertInfo> {
        let mut alerts = Vec::new();
        let timestamp = Local::now().format("%H:%M:%S").to_string();

        // CPU alert
        if data.cpu_usage > settings.notification_cpu_threshold {
            alerts.push(AlertInfo {
                timestamp: timestamp.clone(),
                alert_type: AlertType::CpuHigh,
                source: AlertSource::Cpu,
                message: format!("CPU usage is high: {:.1}%", data.cpu_usage),
                value: data.cpu_usage,
            });
        }

        // Memory alert
        if data.memory_percentage > settings.notification_memory_threshold {
            alerts.push(AlertInfo {
                timestamp: timestamp.clone(),
                alert_type: AlertType::MemoryHigh,
                source: AlertSource::Memory,
                message: format!("Memory usage is high: {:.1}%", data.memory_percentage),
                value: data.memory_percentage,
            });
        }

        // GPU temperature alert
        for (index, gpu) in data.gpu_info.iter().enumerate() {
            if let Some(temp) = gpu.temperature
                && temp > settings.notification_temp_threshold
            {
                alerts.push(AlertInfo {
                    timestamp: timestamp.clone(),
                    alert_type: AlertType::GpuTempHigh,
                    source: AlertSource::Gpu {
                        index,
                        name: gpu.name.clone(),
                    },
                    message: format!("GPU temperature is high: {}°C ({})", temp, gpu.name),
                    value: temp as f32,
                });
            }
        }

        // Disk space alerts
        for disk in &data.disk_info {
            if disk.usage_percentage > settings.notification_disk_threshold {
                alerts.push(AlertInfo {
                    timestamp: timestamp.clone(),
                    alert_type: AlertType::DiskSpaceLow,
                    source: AlertSource::Disk {
                        mount_point: disk.mount_point.clone(),
                        name: disk.name.clone(),
                    },
                    message: format!("Disk {} is almost full: {:.1}%", disk.name, disk.usage_percentage),
                    value: disk.usage_percentage,
                });
            }
        }

        // Startup High Impact alert
        if data.high_impact_startup_count > 0 {
            alerts.push(AlertInfo {
                timestamp: timestamp.clone(),
                alert_type: AlertType::StartupHighImpact,
                source: AlertSource::Startup,
                message: format!(
                    "{} startup item(s) have High impact on boot time",
                    data.high_impact_startup_count
                ),
                value: data.high_impact_startup_count as f32,
            });
        }

        alerts
    }
}

#[cfg(test)]
mod alert_tests {
    use super::*;
    use crate::app::models::DiskInfo;

    #[test]
    fn in_app_alerts_do_not_require_desktop_notifications() {
        let settings = AppSettings {
            show_notifications: false,
            notification_cpu_threshold: 80.0,
            ..Default::default()
        };
        let data = SystemData {
            cpu_usage: 85.0,
            ..Default::default()
        };
        let alerts = SystemMonitor::check_alerts(&settings, &data);
        assert!(alerts.iter().any(|alert| alert.source == AlertSource::Cpu));
    }

    #[test]
    fn disk_alert_uses_configured_threshold_and_typed_source() {
        let settings = AppSettings {
            notification_disk_threshold: 75.0,
            ..Default::default()
        };
        let data = SystemData {
            disk_info: vec![DiskInfo {
                name: "Data".into(),
                mount_point: "D:\\".into(),
                total_space: 100,
                available_space: 20,
                usage_percentage: 80.0,
                file_system: "NTFS".into(),
            }],
            ..Default::default()
        };
        let alerts = SystemMonitor::check_alerts(&settings, &data);
        assert_eq!(alerts.len(), 1);
        assert_eq!(
            alerts[0].source,
            AlertSource::Disk {
                mount_point: "D:\\".into(),
                name: "Data".into(),
            }
        );
        assert_eq!(alerts[0].key(), "disk:D:\\");
    }
}
