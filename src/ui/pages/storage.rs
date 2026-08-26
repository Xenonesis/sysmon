use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Storage Devices & Partitions", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Global Disk I/O Banner ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("DISK I/O TELEMETRY")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{} volume(s) detected", data.disk_info.len()))
                            .monospace()
                            .size(11.0)
                            .color(ThemePalette::text_dimmed(is_dark)),
                    );
                });
            });

            ui.add_space(8.0);
            ui.columns(2, |cols| {
                cols[0].horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Total Read Rate:")
                            .size(12.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format_rate(data.disk_read_rate))
                            .monospace()
                            .strong()
                            .size(13.0)
                            .color(ThemePalette::STATUS_HEALTHY),
                    );
                });

                cols[1].horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Total Write Rate:")
                            .size(12.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format_rate(data.disk_write_rate))
                            .monospace()
                            .strong()
                            .size(13.0)
                            .color(ThemePalette::STATUS_WARNING),
                    );
                });
            });
        });

        ui.add_space(10.0);

        // ── 2. Storage Volume Cards ──
        for disk in &data.disk_info {
            let color = get_usage_color(disk.usage_percentage);
            let used_bytes = disk.total_space.saturating_sub(disk.available_space);

            card_frame(is_dark).show(ui, |ui| {
                // Header: Volume Name + FS Pill + Usage Pill + Monospace Percentage
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&disk.name)
                            .strong()
                            .size(14.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    if !disk.file_system.is_empty() {
                        status_pill(ui, &disk.file_system, ThemePalette::ACCENT_PRIMARY, is_dark);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{:.1}%", disk.usage_percentage))
                                .monospace()
                                .strong()
                                .size(13.0)
                                .color(color),
                        );
                        if disk.usage_percentage >= 90.0 {
                            status_pill(ui, "CRITICAL", ThemePalette::STATUS_CRITICAL, is_dark);
                        } else if disk.usage_percentage >= 70.0 {
                            status_pill(ui, "ELEVATED", ThemePalette::STATUS_WARNING, is_dark);
                        } else {
                            status_pill(ui, "HEALTHY", ThemePalette::STATUS_HEALTHY, is_dark);
                        }
                    });
                });

                ui.add_space(8.0);
                paint_progress_bar(ui, disk.usage_percentage / 100.0, color, 8.0, is_dark);
                ui.add_space(10.0);

                // Monospace Metrics Grid
                egui::Grid::new(format!("disk_grid_{}", disk.mount_point))
                    .num_columns(4)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Mount Point:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(&disk.mount_point)
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );

                        ui.label(
                            egui::RichText::new("Used Space:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2} GB", bytes_to_gb(used_bytes)))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("File System:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(&disk.file_system)
                                .monospace()
                                .color(ThemePalette::text_primary(is_dark)),
                        );

                        ui.label(
                            egui::RichText::new("Available:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2} GB", bytes_to_gb(disk.available_space)))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Total Capacity:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2} GB", bytes_to_gb(disk.total_space)))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();
                    });

                // High usage warning
                if disk.usage_percentage >= 90.0 {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        status_pill(ui, "LOW STORAGE WARNING", ThemePalette::STATUS_CRITICAL, is_dark);
                        ui.label(
                            egui::RichText::new(format!(
                                "Only {:.2} GB remaining on this volume. Clean temporary files or expand capacity.",
                                bytes_to_gb(disk.available_space)
                            ))
                            .size(11.5)
                            .color(ThemePalette::STATUS_CRITICAL),
                        );
                    });
                }
            });

            ui.add_space(8.0);
        }

        // ── 3. Physical Drive Hardware & S.M.A.R.T. Health (Cached from background thread) ──
        let physical_drives = &data.physical_disks;
        if !physical_drives.is_empty() {
            ui.add_space(6.0);
            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("PHYSICAL HARDWARE DRIVES & S.M.A.R.T. HEALTH")
                            .size(11.0)
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_pill(
                            ui,
                            &format!("{} DRIVES", physical_drives.len()),
                            ThemePalette::ACCENT_PRIMARY,
                            is_dark,
                        );
                    });
                });

                ui.add_space(8.0);

                for drive in physical_drives {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&drive.model)
                                    .size(13.0)
                                    .strong()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                            status_pill(ui, &drive.media_type, ThemePalette::ACCENT_PRIMARY, is_dark);

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let smart_color = if drive.status == "HEALTHY" {
                                    ThemePalette::STATUS_HEALTHY
                                } else {
                                    ThemePalette::STATUS_CRITICAL
                                };
                                status_pill(ui, &drive.smart_status, smart_color, is_dark);
                            });
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let size_gb = (drive.size_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
                            ui.label(
                                egui::RichText::new(format!("Capacity: {:.1} GB", size_gb))
                                    .monospace()
                                    .size(11.0)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            ui.add_space(12.0);
                            ui.label(
                                egui::RichText::new(format!("Device ID: {}", drive.device_id))
                                    .monospace()
                                    .size(10.5)
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                            if let Some(wear) = drive.wear_percentage {
                                ui.add_space(12.0);
                                ui.label(
                                    egui::RichText::new(format!("Estimated Life: {}%", wear))
                                        .monospace()
                                        .size(11.0)
                                        .strong()
                                        .color(ThemePalette::STATUS_HEALTHY),
                                );
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
            });
        }

        ui.add_space(10.0);

        // ── 4. File & USB Drive Lock Inspector ──
        paint_lock_inspector_card(app, ui, is_dark);

        ui.add_space(10.0);

        // ── 5. Storage Space Reclaimer ──
        paint_reclaimer_card(app, ui, is_dark);

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

fn paint_lock_inspector_card(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("FILE & USB DRIVE LOCK INSPECTOR")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(res) = &app.storage_page.lock_result {
                    if res.error.is_some() {
                        status_pill(ui, "ERROR", ThemePalette::STATUS_CRITICAL, is_dark);
                    } else if res.processes.is_empty() {
                        status_pill(ui, "UNLOCKED", ThemePalette::STATUS_HEALTHY, is_dark);
                    } else {
                        status_pill(
                            ui,
                            &format!("{} LOCKED", res.processes.len()),
                            ThemePalette::STATUS_WARNING,
                            is_dark,
                        );
                    }
                } else {
                    status_pill(ui, "READY", ThemePalette::text_dimmed(is_dark), is_dark);
                }
            });
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Detect which processes or services are preventing a file, folder, or USB drive from being modified, ejected, or deleted.",
            )
            .size(12.0)
            .color(ThemePalette::text_secondary(is_dark)),
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let avail_w = ui.available_width();
            let input_w = (avail_w - 270.0).max(140.0);
            let path_input = egui::TextEdit::singleline(&mut app.storage_page.lock_path)
                .hint_text("Enter file path or drive letter (e.g. D:\\ or C:\\file.ext)...")
                .desired_width(input_w);
            ui.add(path_input);

            if ui
                .button(egui::RichText::new("📁 File...").size(12.0))
                .on_hover_text("Browse for a file to inspect")
                .clicked()
            {
                if let Some(file_path) = rfd::FileDialog::new().pick_file() {
                    app.storage_page.lock_path = file_path.to_string_lossy().to_string();
                }
            }

            if ui
                .button(egui::RichText::new("📂 Folder...").size(12.0))
                .on_hover_text("Browse for a folder or drive to inspect")
                .clicked()
            {
                if let Some(folder_path) = rfd::FileDialog::new().pick_folder() {
                    app.storage_page.lock_path = folder_path.to_string_lossy().to_string();
                }
            }

            let inspect_btn = egui::Button::new(
                egui::RichText::new("Inspect Locks")
                    .size(12.0)
                    .strong()
                    .color(ThemePalette::ACCENT_PRIMARY),
            )
            .fill(ThemePalette::ACCENT_PRIMARY.gamma_multiply(if is_dark { 0.18 } else { 0.12 }))
            .stroke(egui::Stroke::new(1.0, ThemePalette::ACCENT_PRIMARY.gamma_multiply(0.5)))
            .rounding(egui::Rounding::same(4.0));

            if ui.add(inspect_btn).clicked() {
                app.storage_page.inspect_locks();
            }
        });

        if let Some(status) = &app.storage_page.lock_status {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(status)
                    .size(11.5)
                    .color(ThemePalette::STATUS_WARNING),
            );
        }

        if let Some(res) = &app.storage_page.lock_result {
            ui.add_space(8.0);
            if res.processes.is_empty() && res.error.is_none() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        status_pill(ui, "UNLOCKED", ThemePalette::STATUS_HEALTHY, is_dark);
                        ui.label(
                            egui::RichText::new(format!(
                                "No active processes are locking \"{}\". Resource can be safely ejected or deleted.",
                                res.path
                            ))
                            .size(12.0)
                            .color(ThemePalette::text_primary(is_dark)),
                        );
                    });
                });
            } else if !res.processes.is_empty() {
                ui.label(
                    egui::RichText::new(format!(
                        "Processes holding lock on \"{}\":",
                        res.path
                    ))
                    .size(12.0)
                    .strong()
                    .color(ThemePalette::text_primary(is_dark)),
                );
                ui.add_space(4.0);

                let mut kill_pid = None;
                for proc in &res.processes {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("PID {}", proc.pid))
                                    .monospace()
                                    .size(12.0)
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(&proc.name)
                                    .strong()
                                    .size(13.0)
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                            let app_color = if proc.is_service {
                                ThemePalette::STATUS_WARNING
                            } else {
                                ThemePalette::ACCENT_PRIMARY
                            };
                            status_pill(ui, &proc.app_type, app_color, is_dark);

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let kill_btn = egui::Button::new(
                                    egui::RichText::new("Terminate Process")
                                        .size(11.0)
                                        .strong()
                                        .color(ThemePalette::STATUS_CRITICAL),
                                )
                                .fill(ThemePalette::STATUS_CRITICAL.gamma_multiply(if is_dark { 0.18 } else { 0.12 }))
                                .stroke(egui::Stroke::new(1.0, ThemePalette::STATUS_CRITICAL.gamma_multiply(0.5)))
                                .rounding(egui::Rounding::same(4.0));

                                if ui.add(kill_btn).on_hover_text("Terminate locking process via ActionPlan").clicked() {
                                    kill_pid = Some(proc.pid);
                                }
                            });
                        });
                    });
                    ui.add_space(3.0);
                }

                if let Some(pid) = kill_pid {
                    app.queue_action(crate::app::commands::ActionCommand::KillProcess(pid));
                }
            }
        }
    });
}

fn paint_reclaimer_card(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    if !app.storage_page.reclaimer_scanned {
        app.storage_page.scan_caches();
    }

    let selected_bytes: u64 = app
        .storage_page
        .reclaimer_categories
        .iter()
        .filter(|c| app.storage_page.reclaimer_selected.contains(c.id))
        .map(|c| c.size_bytes)
        .sum();
    let selected_files: usize = app
        .storage_page
        .reclaimer_categories
        .iter()
        .filter(|c| app.storage_page.reclaimer_selected.contains(c.id))
        .map(|c| c.file_count)
        .sum();

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("STORAGE SPACE RECLAIMER")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("⟳ Rescan").clicked() {
                    app.storage_page.scan_caches();
                    app.storage_page.reclaimer_status = Some("Caches re-scanned.".into());
                }
                status_pill(
                    ui,
                    &format!("{:.1} MB AVAILABLE", crate::ui::format::bytes_to_mb(selected_bytes)),
                    ThemePalette::ACCENT_PRIMARY,
                    is_dark,
                );
            });
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Safely scan and clean temporary caches, DirectX/GPU shader caches, and diagnostic dump files.",
            )
            .size(12.0)
            .color(ThemePalette::text_secondary(is_dark)),
        );

        ui.add_space(8.0);
        let mut toggle_id = None;
        for cat in &app.storage_page.reclaimer_categories {
            let is_selected = app.storage_page.reclaimer_selected.contains(cat.id);
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    let mut checked = is_selected;
                    if ui.checkbox(&mut checked, "").changed() {
                        toggle_id = Some(cat.id);
                    }
                    ui.label(
                        egui::RichText::new(cat.label)
                            .strong()
                            .size(13.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(crate::ui::format::bytes_to_human(cat.size_bytes))
                                .monospace()
                                .strong()
                                .size(12.5)
                                .color(if cat.size_bytes > 0 {
                                    ThemePalette::ACCENT_PRIMARY
                                } else {
                                    ThemePalette::text_dimmed(is_dark)
                                }),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!("{} file(s)", cat.file_count))
                                .monospace()
                                .size(11.0)
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                    });
                });

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new(cat.description)
                            .size(11.0)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                });
            });
            ui.add_space(4.0);
        }

        if let Some(id) = toggle_id {
            app.storage_page.toggle_category(id);
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Selected: {} across {} file(s)",
                    crate::ui::format::bytes_to_human(selected_bytes),
                    selected_files
                ))
                .monospace()
                .size(12.0)
                .color(ThemePalette::text_primary(is_dark)),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clean_btn = egui::Button::new(
                    egui::RichText::new("Clean Selected Caches")
                        .size(12.0)
                        .strong()
                        .color(if selected_bytes > 0 {
                            ThemePalette::STATUS_WARNING
                        } else {
                            ThemePalette::text_dimmed(is_dark)
                        }),
                )
                .fill(ThemePalette::STATUS_WARNING.gamma_multiply(if is_dark { 0.18 } else { 0.12 }))
                .stroke(egui::Stroke::new(1.0, ThemePalette::STATUS_WARNING.gamma_multiply(0.5)))
                .rounding(egui::Rounding::same(4.0));

                let has_selection = !app.storage_page.reclaimer_selected.is_empty();
                ui.add_enabled_ui(has_selection, |ui| {
                    if ui.add(clean_btn).clicked() {
                        let ids = app.storage_page.selected_category_ids();
                        app.queue_action(crate::app::commands::ActionCommand::ReclaimStorageCaches(ids));
                        app.storage_page.reclaimer_status = Some("Cleaning queued via ActionPlan...".into());
                    }
                });
            });
        });

        if let Some(status) = &app.storage_page.reclaimer_status {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(status)
                    .monospace()
                    .size(11.0)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
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

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| show(&mut app, ui, &data));
        });

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
            processes: vec![crate::storage::file_locks::LockingProcess {
                pid: 1234,
                name: "test_process.exe".into(),
                app_type: "Desktop App".into(),
                is_service: false,
            }],
            error: None,
        });

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| show(&mut app, ui, &data));
        });
    }
}
