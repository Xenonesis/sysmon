mod battery_diag;
mod cpu_arch;
mod gpu_display;
mod memory_specs;
mod os_platform;

use crate::ui::components::*;
use crate::*;
use eframe::egui;

/// Renders the complete System Information & Hardware Specs page.
pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "System Information & Hardware Specs", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Operating System & Platform ──
        os_platform::paint_os_platform_card(ui, data, is_dark);
        ui.add_space(10.0);

        // ── Power Management & Battery Diagnostics ──
        battery_diag::paint_power_schemes_card(app, ui, is_dark);
        ui.add_space(10.0);

        // ── 2. Processor (CPU) ──
        cpu_arch::paint_cpu_arch_card(ui, data, is_dark);
        ui.add_space(10.0);

        // ── 3. Memory & Virtual Memory (RAM / Page File) ──
        memory_specs::paint_memory_specs_card(ui, data, is_dark);
        ui.add_space(10.0);

        // ── 4. Graphics Hardware (GPU) ──
        gpu_display::paint_gpu_display_card(ui, data, is_dark);
        ui.add_space(10.0);

        // ── 5. Battery Health (if present) ──
        battery_diag::paint_battery_diagnostics_card(ui, data, is_dark);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_render_default_mock() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData {
            system_info: SystemInfo {
                os_name: "Windows 11 Pro".to_string(),
                os_version: "10.0.22631".to_string(),
                kernel_version: "22631.3007".to_string(),
                hostname: "DESKTOP-TEST".to_string(),
                uptime: 123456,
                cpu_brand: "12th Gen Intel(R) Core(TM) i9-12900H".to_string(),
                cpu_count: 14,
                os_build: Some("22631.3007".to_string()),
                motherboard: Some("Micro-Star International Co., Ltd. MS-1582".to_string()),
                bios_version: Some("E1582IMS.10B".to_string()),
                gpu_driver: Some("551.86".to_string()),
            },
            cpu_cores: vec![
                CpuCoreInfo {
                    core_id: 0,
                    usage: 12.5,
                    name: "Core 0".to_string(),
                },
                CpuCoreInfo {
                    core_id: 1,
                    usage: 8.0,
                    name: "Core 1".to_string(),
                },
            ],
            cpu_usage: 10.25,
            cpu_temperature: Some(52.0),
            memory_total: 32 * 1024 * 1024 * 1024,
            memory_used: 12 * 1024 * 1024 * 1024,
            memory_percentage: 37.5,
            swap_info: crate::SwapInfo {
                total: 16 * 1024 * 1024 * 1024,
                used: 4 * 1024 * 1024 * 1024,
                percentage: 25.0,
            },
            gpu_info: vec![GpuInfo {
                name: "NVIDIA GeForce RTX 3070 Ti Laptop GPU".to_string(),
                utilization: 45.0,
                memory_used: Some(3 * 1024 * 1024 * 1024),
                memory_total: Some(8 * 1024 * 1024 * 1024),
                temperature: Some(60),
                clock_mhz: Some(1485),
                power_watts: Some(80.0),
                fan_percent: Some(50),
            }],
            battery_info: Some(crate::BatteryInfo {
                present: true,
                design_capacity: 80000,
                full_charge_capacity: 75000,
                status: 1,
                discharge_state: Some("AC Online / Fully Charged".to_string()),
            }),
            ..Default::default()
        };

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }

    #[test]
    fn test_system_info_render_no_gpu_no_battery() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData {
            system_info: SystemInfo {
                os_name: "Ubuntu 22.04 LTS".to_string(),
                os_version: "22.04".to_string(),
                kernel_version: "5.15.0-generic".to_string(),
                hostname: "ubuntu-server".to_string(),
                uptime: 3600,
                cpu_brand: "AMD EPYC 7763".to_string(),
                cpu_count: 64,
                os_build: None,
                motherboard: None,
                bios_version: None,
                gpu_driver: None,
            },
            memory_total: 64 * 1024 * 1024 * 1024,
            memory_used: 16 * 1024 * 1024 * 1024,
            memory_percentage: 25.0,
            swap_info: crate::SwapInfo {
                total: 0,
                used: 0,
                percentage: 0.0,
            },
            gpu_info: Vec::new(),
            battery_info: None,
            ..Default::default()
        };

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }

    #[test]
    fn test_system_info_render_multi_gpu_and_battery_wear() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData {
            system_info: SystemInfo {
                os_name: "Windows 11 Enterprise".to_string(),
                os_version: "10.0.26100".to_string(),
                kernel_version: "26100.1150".to_string(),
                hostname: "WORKSTATION-01".to_string(),
                uptime: 999999,
                cpu_brand: "AMD Ryzen 9 7950X 16-Core Processor".to_string(),
                cpu_count: 16,
                os_build: Some("26100.1150".to_string()),
                motherboard: Some("ASUS ROG CROSSHAIR X670E HERO".to_string()),
                bios_version: Some("2007".to_string()),
                gpu_driver: Some("552.22".to_string()),
            },
            cpu_usage: 95.0,
            cpu_temperature: Some(88.5),
            gpu_info: vec![
                GpuInfo {
                    name: "NVIDIA GeForce RTX 4090 #1".to_string(),
                    utilization: 95.0,
                    memory_used: Some(22 * 1024 * 1024 * 1024),
                    memory_total: Some(24 * 1024 * 1024 * 1024),
                    temperature: Some(78),
                    clock_mhz: Some(2520),
                    power_watts: Some(410.0),
                    fan_percent: Some(85),
                },
                GpuInfo {
                    name: "NVIDIA GeForce RTX 4090 #2".to_string(),
                    utilization: 12.0,
                    memory_used: Some(2 * 1024 * 1024 * 1024),
                    memory_total: Some(24 * 1024 * 1024 * 1024),
                    temperature: Some(40),
                    clock_mhz: Some(1500),
                    power_watts: Some(50.0),
                    fan_percent: Some(30),
                },
            ],
            battery_info: Some(crate::BatteryInfo {
                present: true,
                design_capacity: 100000,
                full_charge_capacity: 65000,
                status: 2,
                discharge_state: Some("Discharging".to_string()),
            }),
            ..Default::default()
        };

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }

    #[test]
    fn test_individual_subcomponents() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData::default();

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                os_platform::paint_os_platform_card(ui, &data, true);
                battery_diag::paint_power_schemes_card(&mut app, ui, true);
                cpu_arch::paint_cpu_arch_card(ui, &data, true);
                memory_specs::paint_memory_specs_card(ui, &data, true);
                gpu_display::paint_gpu_display_card(ui, &data, true);
                battery_diag::paint_battery_diagnostics_card(ui, &data, true);
            });
        });
    }
}
