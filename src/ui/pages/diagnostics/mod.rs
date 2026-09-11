mod guided;

use crate::ui::components::paint_section_header;
use crate::{SystemData, SystemMonitorApp};
use eframe::egui;

mod bsod;
mod findings;
mod recorder;

pub(crate) fn show(app: &mut SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Diagnostics & Session Recorder", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        guided::show(app, ui, is_dark);
        ui.add_space(8.0);

        recorder::paint_recorder(app, ui, is_dark);

        ui.add_space(12.0);

        findings::paint_findings(app, ui, data, is_dark);

        ui.add_space(10.0);

        bsod::paint_bsod_history_card(app, ui, is_dark);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn diagnostics_guided_flow_renders_ready_and_review_states() {
        let root = std::env::temp_dir().join(format!("sysmon-diagnostics-ui-{}", std::process::id()));
        crate::app_paths::with_test_data_local_dir(root.clone(), || {
            let mut app = crate::SystemMonitorApp::test_app();
            let data = SystemData::default();
            let ctx = egui::Context::default();

            ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
            })
            .textures_delta
            .clear();

            app.session_recorder.start().unwrap();
            for index in 0..20 {
                let snapshot = crate::monitoring::SystemSnapshot {
                    sampled_at: SystemTime::UNIX_EPOCH + Duration::from_secs(index),
                    cpu_usage: if index < 6 { 10.0 } else { 75.0 },
                    memory_percentage: 45.0,
                    ..Default::default()
                };
                app.session_recorder.record(&snapshot).unwrap();
            }
            app.session_recorder.stop().unwrap();

            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 700.0))),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
                },
            )
            .textures_delta
            .clear();
        });
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_diagnostics_renders_with_crashes() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData::default();
        app.crash_reports.outcome = Some(crate::diagnostics::minidump::CrashScanOutcome {
            reports: vec![
                crate::diagnostics::minidump::MinidumpCrashReport {
                    file_name: "MEMORY.DMP".into(),
                    timestamp: Some("2026-08-27 10:00:00 UTC".into()),
                    kind: crate::diagnostics::minidump::CrashKind::Kernel64,
                    code: 0x00000116,
                    code_name: "VIDEO_TDR_ERROR".into(),
                    parameters: Vec::new(),
                    explanation: "Display driver timed out".into(),
                    address_module: Some("nvlddmkm.sys".into()),
                    recommendation: "Reinstall GPU drivers".into(),
                },
                crate::diagnostics::minidump::MinidumpCrashReport {
                    file_name: "CRASH.DMP".into(),
                    timestamp: Some("2026-08-26 14:00:00 UTC".into()),
                    kind: crate::diagnostics::minidump::CrashKind::Kernel64,
                    code: 0x0000003B,
                    code_name: "SYSTEM_SERVICE_EXCEPTION".into(),
                    parameters: Vec::new(),
                    explanation: "System service routine error".into(),
                    address_module: None,
                    recommendation: "Check driver updates".into(),
                },
            ],
            ..Default::default()
        });

        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
        })
        .textures_delta
        .clear();

        assert_eq!(app.crash_reports.outcome.as_ref().unwrap().reports.len(), 2);
    }
}
