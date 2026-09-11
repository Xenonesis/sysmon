use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_lock_inspector_card(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    app.storage_page.poll_background(ui.ctx());
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("FILE, FOLDER & VOLUME USER INSPECTOR")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(res) = &app.storage_page.lock_result {
                    if res.error.is_some() {
                        status_pill(ui, "ERROR", ThemePalette::STATUS_CRITICAL, is_dark);
                    } else if res.partial {
                        status_pill(ui, "PARTIAL", ThemePalette::STATUS_WARNING, is_dark);
                    } else if res.processes.is_empty() {
                        status_pill(ui, "NO USERS FOUND", ThemePalette::text_dimmed(is_dark), is_dark);
                    } else {
                        status_pill(
                            ui,
                            &format!("{} USERS", res.processes.len()),
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
                "Inspect Restart Manager file users and bounded folder contents. Volume queries may be unsupported. Results do not guarantee safe deletion or ejection.",
            )
            .size(12.0)
            .color(ThemePalette::text_secondary(is_dark)),
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let avail_w = ui.available_width();
            let input_w = (avail_w - 270.0).max(140.0);
            let path_input = egui::TextEdit::singleline(&mut app.storage_page.lock_path)
                .hint_text("Absolute file, folder or volume path (e.g. D:\\)...")
                .desired_width(input_w);
            if ui.add(path_input).changed() {
                app.storage_page.cancel_inspection();
            }

            if ui
                .button(egui::RichText::new("File...").size(12.0))
                .on_hover_text("Browse for a file to inspect")
                .clicked()
                && let Some(file_path) = rfd::FileDialog::new().pick_file() {
                    app.storage_page.cancel_inspection();
                    app.storage_page.lock_path = file_path.to_string_lossy().to_string();
                }

            if ui
                .button(egui::RichText::new("Folder...").size(12.0))
                .on_hover_text("Browse for a folder or drive to inspect")
                .clicked()
                && let Some(folder_path) = rfd::FileDialog::new().pick_folder() {
                    app.storage_page.cancel_inspection();
                    app.storage_page.lock_path = folder_path.to_string_lossy().to_string();
                }

            let inspect_btn = egui::Button::new(
                egui::RichText::new("Inspect Locks")
                    .size(12.0)
                    .strong()
                    .color(ThemePalette::ACCENT_PRIMARY),
            )
            .fill(ThemePalette::ACCENT_PRIMARY.gamma_multiply(if is_dark { 0.18 } else { 0.12 }))
            .stroke(egui::Stroke::new(1.0, ThemePalette::ACCENT_PRIMARY.gamma_multiply(0.5)))
            .corner_radius(egui::CornerRadius::same(4));

            if ui.add_enabled(!app.storage_page.lock_busy(), inspect_btn).clicked() {
                app.storage_page.inspect_locks();
            }
            if app.storage_page.lock_busy() && ui.button("Cancel").clicked() {
                app.storage_page.cancel_inspection();
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
            ui.label(format!("{:?}: {} files queried; {} skipped; {}.", res.kind, res.files_scanned, res.entries_skipped, if res.cancelled { "cancelled" } else if res.partial { "partial coverage" } else { "bounded inspection finished" }));
            for note in &res.coverage {
                ui.label(egui::RichText::new(note).size(11.0).color(ThemePalette::STATUS_WARNING));
            }
            if res.processes.is_empty() && res.error.is_none() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        status_pill(ui, "NO USERS FOUND", ThemePalette::text_dimmed(is_dark), is_dark);
                        ui.label(
                            egui::RichText::new(format!(
                                "No users reported for the inspected coverage of \"{}\". This is not a deletion or ejection safety check.",
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
                        "Reported users of \"{}\":",
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
                                .corner_radius(egui::CornerRadius::same(4));

                                if ui.add_enabled(proc.identity.is_some(), kill_btn).on_hover_text("Request termination using the captured native process creation identity; confirmation and process safety checks apply.").clicked() {
                                    kill_pid = proc.identity;
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
