mod table;
mod toolbar;

use crate::processes::ProcessSortColumn;
use crate::ui::components::*;
use crate::*;
use eframe::egui;

pub(crate) fn sort_header_label(
    label: &str,
    col: ProcessSortColumn,
    current_col: ProcessSortColumn,
    asc: bool,
) -> String {
    if col == current_col {
        let arrow = if asc { " ▲" } else { " ▼" };
        format!("{}{}", label, arrow)
    } else {
        label.to_string()
    }
}

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Process Monitor", is_dark);

    // Filter and Sort processes upfront
    let mut filtered_processes = processes::filter_processes(&data.top_processes, &app.process_search);
    let ascending = app.process_sort_ascending;
    processes::sort_processes_refs(&mut filtered_processes, app.process_sort_column, ascending);

    // ── Integrated Toolbar Container ──
    toolbar::paint_process_toolbar(app, ui, filtered_processes.len(), data.top_processes.len(), is_dark);

    ui.add_space(8.0);

    // ── Responsive Process Table ──
    table::paint_process_table(app, ui, &filtered_processes, data, is_dark);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_header_label_indicators() {
        assert_eq!(
            sort_header_label("PID", ProcessSortColumn::Pid, ProcessSortColumn::Pid, true),
            "PID ▲"
        );
        assert_eq!(
            sort_header_label("PID", ProcessSortColumn::Pid, ProcessSortColumn::Pid, false),
            "PID ▼"
        );
        assert_eq!(
            sort_header_label("CPU %", ProcessSortColumn::Cpu, ProcessSortColumn::Memory, false),
            "CPU %"
        );
        assert_eq!(
            sort_header_label("Memory", ProcessSortColumn::Memory, ProcessSortColumn::Memory, false),
            "Memory ▼"
        );
    }

    #[test]
    fn test_processes_page_render_headless() {
        let mut app = crate::SystemMonitorApp::test_app();
        let mut data = SystemData::default();

        // 1. Initial empty process list render
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });

        // 2. Populated process list render
        data.top_processes = vec![
            crate::processes::ProcessInfo {
                parent_pid: None,
                pid: 1001,
                name: "system_service.exe".to_string(),
                cpu_usage: 25.0,
                memory: 600 * 1024 * 1024,
                disk_read_bytes: 5_000_000,
                disk_written_bytes: 2_500_000,
                status: "Running".to_string(),
            },
            crate::processes::ProcessInfo {
                parent_pid: None,
                pid: 1002,
                name: "browser_worker.exe".to_string(),
                cpu_usage: 12.0,
                memory: 300 * 1024 * 1024,
                disk_read_bytes: 1_000_000,
                disk_written_bytes: 500_000,
                status: "Running".to_string(),
            },
            crate::processes::ProcessInfo {
                parent_pid: None,
                pid: 1003,
                name: "background_daemon.exe".to_string(),
                cpu_usage: 1.5,
                memory: 50 * 1024 * 1024,
                disk_read_bytes: 100_000,
                disk_written_bytes: 50_000,
                status: "Running".to_string(),
            },
        ];

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });

        // 3. Search filtered render
        app.process_search = "browser".to_string();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });

        // 4. Details panel expanded render
        app.process_search.clear();
        app.details_pid = Some(1001);
        data.selected_process_details = Some((
            1001,
            crate::processes::ProcessDetails {
                exe_path: Some("C:\\Windows\\System32\\system_service.exe".to_string()),
                command_line: "system_service.exe --daemon".to_string(),
                cwd: Some("C:\\Windows\\System32".to_string()),
                start_time: 1700000000,
                run_time: 3600,
                parent_pid: Some(4),
                parent_name: Some("System".to_string()),
            },
        ));

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }

    #[test]
    fn test_processes_subcomponents_direct() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData {
            cpu_cores: vec![
                crate::CpuCoreInfo {
                    core_id: 0,
                    usage: 10.0,
                    name: "Core 0".to_string(),
                },
                crate::CpuCoreInfo {
                    core_id: 1,
                    usage: 20.0,
                    name: "Core 1".to_string(),
                },
                crate::CpuCoreInfo {
                    core_id: 2,
                    usage: 30.0,
                    name: "Core 2".to_string(),
                },
                crate::CpuCoreInfo {
                    core_id: 3,
                    usage: 40.0,
                    name: "Core 3".to_string(),
                },
            ],
            top_processes: vec![crate::processes::ProcessInfo {
                parent_pid: None,
                pid: 2048,
                name: "test_very_long_process_name_exceeding_thirty_six_characters_limit.exe".to_string(),
                cpu_usage: 5.0,
                memory: 100 * 1024 * 1024,
                disk_read_bytes: 0,
                disk_written_bytes: 0,
                status: "Running".to_string(),
            }],
            ..Default::default()
        };

        app.suspended_pids.insert(2048);

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                toolbar::paint_process_toolbar(&mut app, ui, 1, 1, true);
                let refs: Vec<_> = data.top_processes.iter().collect();
                table::paint_process_table(&mut app, ui, &refs, &data, true);
            });
        });
    }
}
