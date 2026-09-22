use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

mod lock_inspector;
mod perf;
mod reclaimer;
mod volumes;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Storage Devices & Partitions", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        volumes::paint_volumes(ui, data, is_dark);

        // ── 3b. Disk Latency & Queue Depth ──
        perf::paint_disk_perf_card(ui, &data.disk_perf, is_dark);

        // ── 4. File & USB Drive Lock Inspector ──
        lock_inspector::paint_lock_inspector_card(app, ui, is_dark);

        // ── 5. Storage Space Reclaimer ──
        reclaimer::paint_reclaimer_card(app, ui, is_dark);

        if data.disk_info.is_empty() {
            card_frame(is_dark).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("No storage devices or mounted partitions detected.")
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_page_headless_render() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData::default();
        let ctx = egui::Context::default();

        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
        })
        .textures_delta
        .clear();

        assert!(app.storage_page.reclaimer_scanned);
    }

    #[test]
    fn test_storage_page_state_inspection_and_toggles() {
        let mut state = crate::app::page_state::StoragePageState::default();
        assert!(!state.reclaimer_selected.is_empty());

        state.toggle_category("shader_cache");
        assert!(!state.reclaimer_selected.contains("shader_cache"));
        state.toggle_category("shader_cache");
        assert!(state.reclaimer_selected.contains("shader_cache"));

        state.lock_path = "".to_string();
        state.inspect_locks();
        assert!(state.lock_status.is_some());
        assert!(state.lock_result.is_none());

        let exe = std::env::current_exe().expect("current exe");
        state.lock_path = exe.to_str().unwrap().to_string();
        state.inspect_locks();
        assert!(state.lock_busy());
        let ctx = egui::Context::default();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while state.lock_busy() && std::time::Instant::now() < deadline {
            state.poll_background(&ctx);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(state.lock_result.is_some());
    }

    #[test]
    fn test_storage_page_render_with_locks_and_categories() {
        let mut app = crate::SystemMonitorApp::test_app();
        let mut data = SystemData::default();
        data.disk_info.push(DiskInfo {
            name: "C:\\".to_string(),
            mount_point: "C:\\".to_string(),
            total_space: 500 * 1024 * 1024 * 1024,
            available_space: 250 * 1024 * 1024 * 1024,
            file_system: "NTFS".to_string(),
            usage_percentage: 50.0,
        });
        data.disk_read_rate = 1024.0;
        data.disk_write_rate = 2048.0;

        app.storage_page.lock_result = Some(crate::storage::file_locks::FileLockResult {
            path: "C:\\test\\locked.dll".into(),
            kind: crate::storage::file_locks::InspectionKind::File,
            processes: vec![crate::storage::file_locks::LockingProcess {
                identity: None,
                pid: 1234,
                name: "test_process.exe".into(),
                app_type: "Desktop App".into(),
                is_service: false,
                handles: Vec::new(),
            }],
            error: None,
            files_scanned: 1,
            entries_skipped: 0,
            partial: false,
            cancelled: false,
            coverage: Vec::new(),
            handles: Vec::new(),
        });

        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
        })
        .textures_delta
        .clear();
    }

    #[derive(Debug)]
    struct DummyDroppedFile(std::path::PathBuf);
    impl egui::DroppedFile for DummyDroppedFile {
        fn path(&self) -> &std::path::Path {
            &self.0
        }
        fn bytes(&self) -> Result<Vec<u8>, String> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn test_storage_page_drag_and_drop_file_inspection() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData::default();
        let ctx = egui::Context::default();

        let dummy = std::sync::Arc::new(DummyDroppedFile(std::path::PathBuf::from("C:\\test\\dropped_file.exe")));
        let mut raw_input = egui::RawInput::default();
        raw_input.dropped_files.push(dummy);

        ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
        })
        .textures_delta
        .clear();

        assert_eq!(app.storage_page.lock_path, "C:\\test\\dropped_file.exe");
        assert!(app.storage_page.lock_busy() || app.storage_page.lock_result.is_some());
    }

    #[test]
    fn test_storage_page_drag_and_drop_cancels_previous_inspection() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData::default();
        let ctx = egui::Context::default();

        // Populate initial inspection state
        app.storage_page.lock_path = "C:\\test\\old_file.txt".to_string();
        app.storage_page.lock_result = Some(crate::storage::file_locks::FileLockResult {
            path: "C:\\test\\old_file.txt".into(),
            kind: crate::storage::file_locks::InspectionKind::File,
            processes: Vec::new(),
            error: None,
            files_scanned: 1,
            entries_skipped: 0,
            partial: false,
            cancelled: false,
            coverage: Vec::new(),
            handles: Vec::new(),
        });
        let initial_generation = app.storage_page.lock_generation;

        // Simulate dropping a new file
        let dummy = std::sync::Arc::new(DummyDroppedFile(std::path::PathBuf::from("C:\\test\\new_file.docx")));
        let mut raw_input = egui::RawInput::default();
        raw_input.dropped_files.push(dummy);

        ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
        })
        .textures_delta
        .clear();

        // Verify that cancel_inspection was invoked before inspect_locks:
        // lock_path is updated to the new dropped file,
        // lock_generation was advanced due to cancellation,
        // and a new inspection was scheduled.
        assert_eq!(app.storage_page.lock_path, "C:\\test\\new_file.docx");
        assert!(app.storage_page.lock_generation > initial_generation);
    }

    #[test]
    fn test_storage_page_render_with_handles_and_unlock_buttons() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData::default();

        let handle1 = crate::storage::file_locks::LockedHandleInfo::new(1234, 0x40, "C:\\test\\locked.dll", 0x100);
        let handle2 = crate::storage::file_locks::LockedHandleInfo::new(1234, 0x44, "C:\\test\\locked_sub.dll", 0x200);

        app.storage_page.lock_result = Some(crate::storage::file_locks::FileLockResult {
            path: "C:\\test\\locked.dll".into(),
            kind: crate::storage::file_locks::InspectionKind::File,
            processes: vec![crate::storage::file_locks::LockingProcess {
                identity: None,
                pid: 1234,
                name: "test_process.exe".into(),
                app_type: "Desktop App".into(),
                is_service: false,
                handles: vec![handle1, handle2],
            }],
            error: None,
            files_scanned: 1,
            entries_skipped: 0,
            partial: false,
            cancelled: false,
            coverage: Vec::new(),
            handles: Vec::new(),
        });

        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_storage_page_unlock_and_close_handle_actions() {
        let mut app = crate::SystemMonitorApp::test_app();
        assert!(!app.action_pending);

        let unlock_cmd = crate::app::commands::ActionCommand::UnlockAllProcessesForPath {
            path: "C:\\test\\locked.dll".into(),
        };
        let queued = app.queue_action(unlock_cmd);
        assert!(queued);
        assert!(app.pending_action_plan.is_some());

        let plan = app.pending_action_plan.as_ref().unwrap();
        assert_eq!(plan.title, "Unlock file path");

        // Clear pending plan to test close handle action
        app.pending_action_plan = None;

        let close_cmd = crate::app::commands::ActionCommand::CloseFileHandle {
            pid: 1234,
            handle: 0x40,
            path: "C:\\test\\locked.dll".into(),
        };
        let queued = app.queue_action(close_cmd);
        assert!(queued);
        assert!(app.pending_action_plan.is_some());

        let plan = app.pending_action_plan.as_ref().unwrap();
        assert_eq!(plan.title, "Close remote file handle");
    }
}
