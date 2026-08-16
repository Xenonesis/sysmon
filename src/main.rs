#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
pub(crate) mod app;
pub(crate) mod ui;
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use chrono::Local;
mod diagnostics;
mod monitoring;
mod network;
mod persistence;
mod power;
mod privilege;
mod processes;
pub mod providers;
mod services;
mod startup;
mod storage;
pub mod telemetry;
mod updater;
use eframe::egui;

use rfd::FileDialog;
use tracing::{error, info, warn};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) use crate::app::models::*;
pub(crate) use crate::monitoring::engine::*;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
#[cfg(target_os = "windows")]
use tray_icon::menu::MenuEvent;

impl eframe::App for SystemMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let ctx_clone = ctx.clone();
        self.data.write().last_activity = Instant::now();
        while let Ok(event) = self.app_channels.event_receiver.try_recv() {
            match event {
                app::events::AppEvent::Snapshot(snapshot) => {
                    if let Err(error) = self.session_recorder.record(&snapshot) {
                        self.session_status = Some(format!("Session recording failed: {error}"));
                    }
                    self.latest_snapshot = Some(*snapshot);
                }
                app::events::AppEvent::AuditRecorded(record) => {
                    self.action_history
                        .push(app::actions::ActionHistoryEntry { record, undo: None });
                }
                app::events::AppEvent::ActionCompleted { command, record, undo } => {
                    self.action_pending = false;
                    self.action_status = Some(record.message.clone());
                    if matches!(command, app::commands::ActionCommand::CleanRam) {
                        self.ram_cleaner_state.is_cleaning = false;
                        self.ram_cleaner_state.last_cleaned = Some(Instant::now());
                        self.ram_cleaner_state.last_cleaned_display = Local::now().format("%H:%M:%S").to_string();
                        self.ram_cleaner_state.clean_count += 1;
                        if let Some(bytes) = record
                            .message
                            .strip_prefix("Freed ")
                            .and_then(|value| value.split_whitespace().next())
                            .and_then(|value| value.parse::<u64>().ok())
                        {
                            self.ram_cleaner_state.bytes_freed =
                                self.ram_cleaner_state.bytes_freed.saturating_add(bytes);
                            let mut data = self.data.write();
                            data.ram_clean_freed_bytes = data.ram_clean_freed_bytes.saturating_add(bytes);
                        }
                    }
                    self.action_history
                        .push(app::actions::ActionHistoryEntry { record, undo });
                }
                app::events::AppEvent::ActionFailed { command, record } => {
                    self.action_pending = false;
                    self.action_status = Some(record.message.clone());
                    if matches!(command, app::commands::ActionCommand::CleanRam) {
                        self.ram_cleaner_state.is_cleaning = false;
                    }
                    self.action_history
                        .push(app::actions::ActionHistoryEntry { record, undo: None });
                    if self.settings.enable_sounds {
                        play_alert_sound();
                    }
                }
            }
        }
        {
            let mut data = self.data.write();
            data.is_hidden = self.is_hidden;
            data.selected_tab = self.selected_tab;
            if let Some(items) = &*self.startup_items_share.lock() {
                data.high_impact_startup_count = startup::high_impact_count(items);
            }
        }
        // Apply minimized setting immediately
        if !self.start_minimized_applied {
            self.start_minimized_applied = true;
            if self.settings.start_minimized {
                if self.settings.minimize_to_tray {
                    self.is_hidden = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
                if let Some(hk) = &self.clean_ram_hotkey {
                    if event.id == hk.id() {
                        self.queue_action(app::commands::ActionCommand::CleanRam);
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if Some(&event.id) == self.tray_menu_quit_id.as_ref() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if Some(&event.id) == self.tray_menu_show_id.as_ref() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.is_hidden = false;
            } else if Some(&event.id) == self.tray_menu_clean_id.as_ref() {
                self.queue_action(app::commands::ActionCommand::CleanRam);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.is_hidden = false;
            } else if Some(&event.id) == self.tray_menu_procman_id.as_ref() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.is_hidden = false;
                let _ = self
                    .app_channels
                    .monitoring_sender
                    .send(app::commands::MonitoringCommand::SetHidden(false));
                self.show_process_manager = true;
            } else if Some(&event.id) == self.tray_menu_pause_id.as_ref() {
                let paused = {
                    let mut d = self.data.write();
                    d.monitoring_paused = !d.monitoring_paused;
                    d.monitoring_paused
                };
                let _ = self
                    .app_channels
                    .monitoring_sender
                    .send(app::commands::MonitoringCommand::SetPaused(paused));
                if let Some(item) = &self.tray_menu_pause_item {
                    item.set_checked(paused);
                }
            } else if let Some(plan_guid) = self.tray_menu_power_guids.get(&event.id) {
                let plan_guid = plan_guid.clone();
                self.queue_action(app::commands::ActionCommand::SetPowerPlan(plan_guid));
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) && self.settings.minimize_to_tray {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.is_hidden = true;
            let _ = self
                .app_channels
                .monitoring_sender
                .send(app::commands::MonitoringCommand::SetHidden(true));
        }

        // Update tray tooltip with CPU/RAM usage
        #[cfg(target_os = "windows")]
        if let Some(tray) = &mut self.tray_icon {
            let data = self.data.read();
            let tooltip = if data.monitoring_paused {
                format!(
                    "⏸ SysMon Paused — CPU {:.0}% | RAM {:.0}%",
                    data.cpu_usage, data.memory_percentage
                )
            } else {
                format!(
                    "SysMon: CPU {:.0}% | RAM {:.0}%",
                    data.cpu_usage, data.memory_percentage
                )
            };
            let _ = tray.set_tooltip(Some(tooltip));
        }

        // Ensure repaint for continuous updates but without CPU lock
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // Check for updates automatically (once every 24 hours)
        if self.update_check_time.is_none_or(|t| t.elapsed().as_secs() > 86400) {
            let mut updater = self.updater.clone();
            let update_info_share = self.update_info_share.clone();
            thread::Builder::new()
                .name("auto_updater_check".to_string())
                .stack_size(8 * 1024 * 1024)
                .spawn(move || {
                    if let Ok(update_info) = updater.check_for_updates() {
                        *update_info_share.lock() = Some(update_info.clone());
                    }
                })
                .expect("failed to spawn auto updater check thread");
            self.update_check_time = Some(Instant::now());
        }

        // Poll background installer result each frame.
        let installer_result = self.update_result_share.lock().take();
        if let Some(result) = installer_result {
            self.update_downloading = false;
            match result {
                Ok(()) => {
                    // Installer launched successfully — hide banner.
                    self.show_update_notification = false;
                    self.update_error = None;
                }
                Err(msg) => {
                    self.update_error = Some(msg);
                }
            }
        }

        // Show update notification banner
        let update_info_opt = self.update_info_share.lock().clone();
        if let Some(update_info) = update_info_opt {
            if update_info.update_available && self.show_update_notification {
                let mut frame = egui::Frame::none().fill(ThemePalette::BG_SURFACE);
                frame.inner_margin = egui::Margin::symmetric(16.0, 12.0);

                egui::TopBottomPanel::top("update_notification")
                    .frame(frame)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                ThemePalette::ACCENT_PRIMARY,
                                egui::RichText::new("UPDATE AVAILABLE").strong(),
                            );
                            ui.add_space(8.0);
                            ui.label(format!(
                                "Version {} is ready. You are currently on v{}.",
                                update_info.latest_version, update_info.current_version
                            ));

                            // Show inline error message if the last attempt failed.
                            if let Some(err) = &self.update_error {
                                ui.add_space(8.0);
                                ui.colored_label(egui::Color32::from_rgb(220, 80, 70), format!("⚠ {}", err));
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if !self.update_downloading {
                                    if ui.button("Dismiss").clicked() {
                                        self.show_update_notification = false;
                                        self.update_error = None;
                                    }
                                    ui.add_space(8.0);
                                }
                                if self.update_downloading {
                                    ui.add_enabled(
                                        false,
                                        egui::Button::new(egui::RichText::new("⏳ Downloading…").strong()),
                                    );
                                } else if ui.button(egui::RichText::new("Install Update").strong()).clicked() {
                                    let download_url = update_info.download_url.clone();
                                    let checksum_url = update_info.checksum_url.clone();
                                    let result_share = self.update_result_share.clone();
                                    self.update_downloading = true;
                                    self.update_error = None;
                                    thread::Builder::new()
                                        .name("updater_downloader".to_string())
                                        .stack_size(8 * 1024 * 1024)
                                        .spawn(move || {
                                            let result = updater::Updater::new()
                                                .download_and_install_update(&download_url, &checksum_url);
                                            *result_share.lock() = Some(result);
                                        })
                                        .expect("failed to spawn updater downloader thread");
                                }
                            });
                        });
                    });
            }
        }

        // Keyboard shortcuts
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F5) {
                // Refresh (reset statistics)
                {
                    let mut data = self.data.write();
                    data.cpu_history.clear();
                    data.memory_history.clear();
                    data.gpu_history.clear();
                }
            }
            if i.modifiers.ctrl {
                let mut new_tab = None;
                if i.key_pressed(egui::Key::Num1) {
                    new_tab = Some(Tab::Overview);
                }
                if i.key_pressed(egui::Key::Num2) {
                    new_tab = Some(Tab::Performance);
                }
                if i.key_pressed(egui::Key::Num3) {
                    new_tab = Some(Tab::Processes);
                }
                if i.key_pressed(egui::Key::Num4) {
                    new_tab = Some(Tab::CpuCores);
                }
                if i.key_pressed(egui::Key::Num5) {
                    new_tab = Some(Tab::Storage);
                }
                if i.key_pressed(egui::Key::Num6) {
                    new_tab = Some(Tab::Network);
                }
                if i.key_pressed(egui::Key::Num7) {
                    new_tab = Some(Tab::SystemInfo);
                }
                if i.key_pressed(egui::Key::Num8) {
                    new_tab = Some(Tab::Alerts);
                }
                if i.key_pressed(egui::Key::Num9) {
                    new_tab = Some(Tab::RamCleaner);
                }
                if i.key_pressed(egui::Key::Num0) {
                    new_tab = Some(Tab::StartupManager);
                }

                if let Some(tab) = new_tab {
                    if tab != Tab::CpuCores || self.settings.show_cpu_cores {
                        self.selected_tab = tab;
                    }
                }
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::E) {
                // Ctrl+E = Export
                self.show_export = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::B) {
                // Ctrl+B = Toggle Sidebar
                self.settings.sidebar_collapsed = !self.settings.sidebar_collapsed;
                let _ = self.settings.save();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::M) {
                // Ctrl+M = Toggle Mini-Widget / HUD
                self.widget_open = !self.widget_open;
                self.settings.show_widget = self.widget_open;
                let _ = self.settings.save();
                {
                    let mut shared = self.shared_settings.lock();
                    *shared = self.settings.clone();
                }
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Comma) {
                // Ctrl+, = Settings
                self.show_settings = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::U) {
                // Ctrl+U = Check for updates manually
                let mut updater = self.updater.clone();
                let update_info_share = self.update_info_share.clone();
                let repaint_ctx = ctx_clone.clone();
                thread::Builder::new()
                    .name("manual_updater_check".to_string())
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move || {
                        if let Ok(update_info) = updater.check_for_updates() {
                            *update_info_share.lock() = Some(update_info);
                            repaint_ctx.request_repaint();
                        }
                    })
                    .expect("failed to spawn manual updater check thread");
            }
        });

        // Mirror details selection into shared state so the monitor thread computes details
        {
            let mut d = self.data.write();
            if d.selected_process_pid != self.details_pid {
                d.selected_process_pid = self.details_pid;
                d.selected_process_details = None;
            }
        }

        let data_arc_local = self.data.clone();
        let mut data = data_arc_local.read();

        // Handle process kill actions
        if let Some(pid) = self.selected_process_pid.take() {
            self.queue_action(app::commands::ActionCommand::KillProcess(pid));
        }

        // Handle process tree kill actions (background thread; tree walk + kills can take seconds)
        if let Some(root) = self.kill_tree_pid.take() {
            self.queue_action(app::commands::ActionCommand::KillProcessTree(root));
        }

        // Handle process suspend actions
        if let Some(pid) = self.suspend_process_pid.take() {
            self.queue_action(app::commands::ActionCommand::SuspendProcess(pid));
        }

        // Handle process resume actions
        if let Some(pid) = self.resume_process_pid.take() {
            self.queue_action(app::commands::ActionCommand::ResumeProcess(pid));
        }

        // Handle process priority changes
        if let Some((pid, priority)) = self.priority_change.take() {
            self.queue_action(app::commands::ActionCommand::SetPriority { pid, priority });
        }

        // Handle process CPU affinity changes
        if let Some((pid, mask)) = self.affinity_change.take() {
            self.queue_action(app::commands::ActionCommand::SetAffinity { pid, mask });
        }

        // Auto RAM cleaning
        if self.ram_cleaner_state.auto_clean_enabled && !self.ram_cleaner_state.is_cleaning {
            let idle_ok = {
                let d = self.data.read();
                !self.ram_cleaner_state.auto_clean_idle_only || d.last_activity.elapsed().as_secs() > 120
            };
            let should_clean = if let Some(last) = self.ram_cleaner_state.last_cleaned {
                last.elapsed().as_secs() >= self.ram_cleaner_state.auto_clean_interval
                    && data.memory_percentage >= self.ram_cleaner_state.auto_clean_threshold
            } else {
                data.memory_percentage >= self.ram_cleaner_state.auto_clean_threshold
            };
            if should_clean && idle_ok {
                self.ram_cleaner_state.is_cleaning = true;
                self.ram_cleaner_state.last_cleaned = Some(Instant::now());
                self.ram_cleaner_state.last_cleaned_display = Local::now().format("%H:%M:%S").to_string();
                self.ram_cleaner_state.clean_count += 1;
                let data_arc = Arc::clone(&self.data);
                let repaint_ctx = ctx_clone.clone();
                let enable_sounds = self.settings.enable_sounds;
                let target = self.ram_cleaner_state.auto_clean_target;
                let max_mb = self.ram_cleaner_state.auto_clean_max_mb;
                let notify = self.ram_cleaner_state.auto_clean_notify;
                let exclusions = self.ram_cleaner_state.auto_clean_exclusions.clone();
                let smart_only = self.ram_cleaner_state.auto_clean_smart_only;
                let total_ram = data.memory_total;
                let auto_event_sender = self.app_channels.event_sender.clone();
                thread::Builder::new()
                    .name("ram_cleaner_auto".to_string())
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move || {
                        // ponytail: bounded passes + budget; a truly stuck
                        // process set just stops after 5 passes
                        let mut monitor = SystemMonitor::new();
                        let mut freed_total = 0u64;
                        for _pass in 0..5 {
                            let freed = monitor.clean_ram(&exclusions, smart_only);
                            let budget_left = if max_mb == 0 {
                                u64::MAX
                            } else {
                                (max_mb * 1024 * 1024).saturating_sub(freed_total)
                            };
                            freed_total = freed_total.saturating_add(freed);
                            monitor.sys.refresh_memory();
                            let usage_pct = if total_ram > 0 {
                                monitor.sys.used_memory() as f64 / total_ram as f64 * 100.0
                            } else {
                                0.0
                            };
                            if should_stop_cleaning(usage_pct, target as f64, freed, budget_left) {
                                break;
                            }
                        }
                        if enable_sounds {
                            play_success_sound();
                        }
                        if notify {
                            let _ = notify_rust::Notification::new()
                                .summary("Auto RAM Clean")
                                .body(&format!("Freed {:.1} MB of RAM", freed_total as f64 / 1024.0 / 1024.0))
                                .timeout(notify_rust::Timeout::Milliseconds(5000))
                                .show();
                        }
                        let audit = app::actions::ActionAuditRecord::automatic(
                            "Automatic RAM working-set cleanup",
                            format!("Freed {freed_total} bytes using the configured cleanup policy"),
                        );
                        if let Err(error) = persistence::action_log::append(&audit) {
                            warn!(%error, "Failed to persist automatic action audit record");
                        }
                        let _ = auto_event_sender.send(app::events::AppEvent::AuditRecorded(audit));
                        // Store freed bytes in SystemData for the UI to pick up
                        {
                            let mut d = data_arc.write();
                            d.ram_clean_freed_bytes += freed_total;
                            d.ram_clean_is_cleaning = false;
                        }
                        repaint_ctx.request_repaint();
                    })
                    .expect("failed to spawn auto ram cleaner thread");
                // Mark cleaning in shared data too
                drop(data);
                self.data.write().ram_clean_is_cleaning = true;
                data = data_arc_local.read();
            }
        }
        // Sync back from shared data
        {
            let d = self.data.read();
            if !d.ram_clean_is_cleaning && self.ram_cleaner_state.is_cleaning {
                self.ram_cleaner_state.is_cleaning = false;
            }
            self.ram_cleaner_state.bytes_freed = d.ram_clean_freed_bytes;
        }

        // CSV Export window
        let mut show_export_csv = self.show_export_csv;
        if show_export_csv {
            let csv_result = self.export_to_csv(&data);
            egui::Window::new("Export to CSV")
                .open(&mut show_export_csv)
                .resizable(true)
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.heading("Export System Data to CSV");
                    ui.separator();

                    match csv_result {
                        Ok(csv_data) => {
                            ui.label("Data exported successfully. Copy the CSV below:");
                            ui.add_space(5.0);

                            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                ui.text_edit_multiline(&mut csv_data.as_str());
                            });

                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                if ui.button("📋 Copy to Clipboard").clicked() {
                                    ui.output_mut(|o| o.copied_text = csv_data.clone());
                                }
                                if ui.button("💾 Save to File...").clicked() {
                                    let date_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
                                    if let Some(path) = FileDialog::new()
                                        .set_file_name(format!("sysmon_export_{}.csv", date_str))
                                        .add_filter("CSV File", &["csv"])
                                        .save_file()
                                    {
                                        if std::fs::write(&path, &csv_data).is_ok() {
                                            #[cfg(target_os = "windows")]
                                            play_success_sound();
                                        }
                                    }
                                }
                            });

                            ui.add_space(5.0);
                            ui.label("Tip: Open in Excel or any spreadsheet application");
                        }
                        Err(e) => {
                            ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
                        }
                    }
                });
        }
        self.show_export_csv = show_export_csv;

        // JSON Export window
        let mut show_export = self.show_export;
        if show_export {
            let json_result = self.export_data_to_json(&data);
            egui::Window::new("Export Data")
                .open(&mut show_export)
                .resizable(true)
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.heading("Export System Data to JSON");
                    ui.separator();

                    match json_result {
                        Ok(json_data) => {
                            ui.label("Data exported successfully. Copy the JSON below:");
                            ui.add_space(5.0);

                            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                ui.text_edit_multiline(&mut json_data.as_str());
                            });

                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                if ui.button("📋 Copy to Clipboard").clicked() {
                                    ui.output_mut(|o| o.copied_text = json_data.clone());
                                }
                                if ui.button("💾 Save to File...").clicked() {
                                    let date_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
                                    if let Some(path) = FileDialog::new()
                                        .set_file_name(format!("sysmon_export_{}.json", date_str))
                                        .add_filter("JSON File", &["json"])
                                        .save_file()
                                    {
                                        if std::fs::write(&path, &json_data).is_ok() {
                                            #[cfg(target_os = "windows")]
                                            play_success_sound();
                                        }
                                    }
                                }
                            });

                            ui.add_space(5.0);
                            ui.label("Tip: You can paste this into a .json file");
                        }
                        Err(e) => {
                            ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
                        }
                    }
                });
        }
        self.show_export = show_export;

        // Alerts window
        let mut show_alerts = self.show_alerts;
        let mut clear_alerts = false;
        if show_alerts {
            egui::Window::new("System Alerts")
                .open(&mut show_alerts)
                .resizable(true)
                .default_width(600.0)
                .show(ctx, |ui| {
                    ui.heading("Active System Alerts");
                    ui.separator();

                    if data.alerts.is_empty() {
                        ui.label("✅ No active alerts. System is running normally.");
                    } else {
                        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                            for alert in &data.alerts {
                                ui.group(|ui| {
                                    let (icon, color) = match alert.alert_type {
                                        AlertType::CpuHigh => ("CPU", egui::Color32::YELLOW),
                                        AlertType::MemoryHigh => ("RAM", egui::Color32::YELLOW),
                                        AlertType::GpuTempHigh => ("GPU", egui::Color32::RED),
                                        AlertType::DiskSpaceLow => ("DISK", egui::Color32::RED),
                                        AlertType::StartupHighImpact => ("STARTUP", egui::Color32::YELLOW),
                                    };

                                    ui.horizontal(|ui| {
                                        ui.colored_label(color, icon);
                                        ui.colored_label(color, &alert.message);
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(&alert.timestamp);
                                        });
                                    });
                                });
                                ui.add_space(5.0);
                            }
                        });

                        ui.separator();
                        if ui.button("Clear All Alerts").clicked() {
                            clear_alerts = true;
                        }
                    }
                });
        }
        self.show_alerts = show_alerts;
        if clear_alerts {
            drop(data);
            self.data.write().alerts.clear();
            data = data_arc_local.read();
        }

        let is_dark = ThemePalette::is_dark_mode(self.settings.theme);

        let is_collapsed = self.settings.sidebar_collapsed;
        let sidebar_width = if is_collapsed { 52.0 } else { 190.0 };
        let sidebar_frame = egui::Frame::none()
            .fill(ThemePalette::bg_surface(is_dark))
            .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)));

        // Modern sleek SidePanel for navigation
        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .exact_width(sidebar_width)
            .frame(sidebar_frame)
            .show(ctx, |ui| {
                ui.add_space(14.0);

                if !is_collapsed {
                    // Brand Header (Expanded)
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.add(
                            egui::Image::new(egui::include_image!("../assets/icon.png"))
                                .max_width(20.0)
                                .max_height(20.0),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("Sys")
                                .size(18.0)
                                .strong()
                                .color(ThemePalette::ACCENT_PRIMARY),
                        );
                        ui.label(
                            egui::RichText::new("Mon")
                                .size(18.0)
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(8.0);
                            let collapse_btn = egui::Button::new(
                                egui::RichText::new("◀")
                                    .size(11.0)
                                    .monospace()
                                    .color(ThemePalette::text_secondary(is_dark)),
                            )
                            .fill(ThemePalette::bg_track(is_dark))
                            .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
                            .rounding(egui::Rounding::same(3.0));

                            if ui
                                .add(collapse_btn)
                                .on_hover_text("Collapse Sidebar (Ctrl+B)")
                                .clicked()
                            {
                                self.settings.sidebar_collapsed = true;
                                let _ = self.settings.save();
                            }
                        });
                    });
                } else {
                    // Brand Header (Collapsed)
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::click());
                    let is_hovered = response.hovered();
                    if response.on_hover_text("Expand Sidebar (Ctrl+B)").clicked() {
                        self.settings.sidebar_collapsed = false;
                        let _ = self.settings.save();
                    }
                    if is_hovered {
                        let hover_fill = if is_dark {
                            egui::Color32::from_rgb(32, 32, 36)
                        } else {
                            egui::Color32::from_rgb(235, 235, 238)
                        };
                        ui.painter().rect_filled(rect, egui::Rounding::same(4.0), hover_fill);
                    }
                    let logo_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(22.0, 22.0));
                    egui::Image::new(egui::include_image!("../assets/icon.png")).paint_at(ui, logo_rect);
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // Navigation Categories
                // Navigation Categories
                struct NavItem {
                    tab: Tab,
                    label: &'static str,
                    icon: &'static str,
                }

                struct NavGroup {
                    title: &'static str,
                    items: Vec<NavItem>,
                }

                let groups = [
                    NavGroup {
                        title: "TELEMETRY",
                        items: {
                            let mut items = vec![
                                NavItem {
                                    tab: Tab::Overview,
                                    label: "Overview",
                                    icon: "📊",
                                },
                                NavItem {
                                    tab: Tab::Performance,
                                    label: "Performance",
                                    icon: "📈",
                                },
                            ];
                            if self.settings.show_cpu_cores {
                                items.push(NavItem {
                                    tab: Tab::CpuCores,
                                    label: "CPU Cores",
                                    icon: "⚡",
                                });
                            }
                            items.push(NavItem {
                                tab: Tab::Storage,
                                label: "Storage",
                                icon: "💾",
                            });
                            items.push(NavItem {
                                tab: Tab::Network,
                                label: "Network",
                                icon: "🌐",
                            });
                            items
                        },
                    },
                    NavGroup {
                        title: "SYSTEM CONTROL",
                        items: vec![
                            NavItem {
                                tab: Tab::Processes,
                                label: "Processes",
                                icon: "📋",
                            },
                            NavItem {
                                tab: Tab::Services,
                                label: "Services",
                                icon: "⚙",
                            },
                            NavItem {
                                tab: Tab::StartupManager,
                                label: "Startup Apps",
                                icon: "🚀",
                            },
                            NavItem {
                                tab: Tab::RamCleaner,
                                label: "RAM Cleaner",
                                icon: "🧹",
                            },
                        ],
                    },
                    NavGroup {
                        title: "DIAGNOSTICS & HEALTH",
                        items: vec![
                            NavItem {
                                tab: Tab::Diagnostics,
                                label: "Diagnostics",
                                icon: "🩺",
                            },
                            NavItem {
                                tab: Tab::SystemInfo,
                                label: "System Info",
                                icon: "💻",
                            },
                            NavItem {
                                tab: Tab::Alerts,
                                label: "Alerts",
                                icon: "🔔",
                            },
                        ],
                    },
                ];

                for (g_idx, group) in groups.iter().enumerate() {
                    if !is_collapsed {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(group.title)
                                    .size(9.5)
                                    .strong()
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                        });
                        ui.add_space(2.0);
                    } else if g_idx > 0 {
                        ui.add_space(3.0);
                        ui.separator();
                        ui.add_space(3.0);
                    }

                    ui.spacing_mut().item_spacing.y = 2.0;
                    for item in &group.items {
                        let is_selected = self.selected_tab == item.tab;
                        let item_h = if is_collapsed { 30.0 } else { 28.0 };
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(ui.available_width(), item_h), egui::Sense::click());

                        let tooltip_text = if item.tab == Tab::Alerts && !data.alerts.is_empty() {
                            format!("{} ({} active)", item.label, data.alerts.len())
                        } else {
                            item.label.to_string()
                        };

                        let is_hovered = response.hovered();
                        if response.on_hover_text(tooltip_text).clicked() {
                            self.selected_tab = item.tab;
                        }

                        if is_selected {
                            let fill = if is_dark {
                                egui::Color32::from_rgb(32, 32, 36)
                            } else {
                                egui::Color32::from_rgb(235, 235, 238)
                            };
                            ui.painter().rect_filled(rect, egui::Rounding::same(4.0), fill);

                            // 3px solid #10B981 vertical left edge indicator
                            let edge_rect =
                                egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.left() + 3.0, rect.bottom()));
                            ui.painter().rect_filled(
                                edge_rect,
                                egui::Rounding {
                                    nw: 4.0,
                                    sw: 4.0,
                                    ne: 0.0,
                                    se: 0.0,
                                },
                                ThemePalette::ACCENT_PRIMARY,
                            );
                        } else if is_hovered {
                            let hover_fill = if is_dark {
                                egui::Color32::from_rgb(28, 28, 31)
                            } else {
                                egui::Color32::from_rgb(240, 240, 243)
                            };
                            ui.painter().rect_filled(rect, egui::Rounding::same(4.0), hover_fill);
                        }

                        if !is_collapsed {
                            let text_color = if is_selected || is_hovered {
                                ThemePalette::text_primary(is_dark)
                            } else {
                                ThemePalette::text_secondary(is_dark)
                            };

                            let icon_pos = egui::pos2(rect.left() + 10.0, rect.center().y);
                            ui.painter().text(
                                icon_pos,
                                egui::Align2::LEFT_CENTER,
                                item.icon,
                                egui::FontId::proportional(13.0),
                                if is_selected {
                                    ThemePalette::ACCENT_PRIMARY
                                } else {
                                    text_color
                                },
                            );

                            let text_pos = egui::pos2(rect.left() + 30.0, rect.center().y);
                            ui.painter().text(
                                text_pos,
                                egui::Align2::LEFT_CENTER,
                                item.label,
                                egui::FontId::proportional(12.5),
                                text_color,
                            );

                            // Dynamic alert count pill [ N ] when item is Alerts and alerts exist
                            if item.tab == Tab::Alerts && !data.alerts.is_empty() {
                                let alerts_count = data.alerts.len();
                                let badge_text = format!("{alerts_count}");
                                let badge_color = ThemePalette::STATUS_WARNING;
                                let badge_bg = badge_color.gamma_multiply(if is_dark { 0.2 } else { 0.15 });
                                let badge_pos = egui::pos2(rect.right() - 8.0, rect.center().y);
                                let badge_rect = egui::Rect::from_center_size(
                                    badge_pos + egui::vec2(-8.0, 0.0),
                                    egui::vec2(20.0, 15.0),
                                );
                                ui.painter()
                                    .rect_filled(badge_rect, egui::Rounding::same(4.0), badge_bg);
                                ui.painter().rect_stroke(
                                    badge_rect,
                                    egui::Rounding::same(4.0),
                                    egui::Stroke::new(1.0, badge_color.gamma_multiply(0.4)),
                                );
                                ui.painter().text(
                                    badge_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    &badge_text,
                                    egui::FontId::monospace(9.5),
                                    badge_color,
                                );
                            }
                        } else {
                            // Collapsed mode: Centered Icon Glyph
                            let icon_color = if is_selected {
                                ThemePalette::ACCENT_PRIMARY
                            } else if is_hovered {
                                ThemePalette::text_primary(is_dark)
                            } else {
                                ThemePalette::text_secondary(is_dark)
                            };

                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                item.icon,
                                egui::FontId::proportional(13.5),
                                icon_color,
                            );

                            // Alert indicator dot on collapsed token
                            if item.tab == Tab::Alerts && !data.alerts.is_empty() {
                                let dot_pos = rect.right_top() + egui::vec2(-7.0, 6.0);
                                ui.painter().circle_filled(dot_pos, 3.0, ThemePalette::STATUS_WARNING);
                            }
                        }
                    }
                }

                // Pinned Bottom Utility Dock
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(10.0);

                    if !is_collapsed {
                        // Live heartbeat label (Updated: HH:MM:SS)
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            let dot_color = ThemePalette::STATUS_HEALTHY;
                            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot_rect.center(), 2.5, dot_color);
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!("Updated: {}", data.last_update))
                                    .size(10.5)
                                    .monospace()
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                        });
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Utility buttons (Expanded)
                        let util_btn = |ui: &mut egui::Ui, icon: &str, text: &str| {
                            let btn = egui::Button::new(
                                egui::RichText::new(format!("{icon}  {text}"))
                                    .size(12.0)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                            .frame(false);
                            ui.add_sized([ui.available_width(), 24.0], btn)
                        };

                        if util_btn(ui, "ℹ", "About").on_hover_text("About SysMon").clicked() {
                            self.selected_tab = Tab::About;
                        }
                        if util_btn(ui, "⌨", "Shortcuts")
                            .on_hover_text("Keyboard shortcuts (Ctrl+B, F5...)")
                            .clicked()
                        {
                            self.show_shortcuts = true;
                        }
                        if util_btn(ui, "⚙", "Settings")
                            .on_hover_text("Application settings (Ctrl+,)")
                            .clicked()
                        {
                            self.show_settings = true;
                        }
                    } else {
                        // Collapsed utility dock
                        let (dot_rect, dot_resp) =
                            ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.0), egui::Sense::hover());
                        dot_resp.on_hover_text(format!("Live Heartbeat · Updated: {}", data.last_update));
                        ui.painter()
                            .circle_filled(dot_rect.center(), 3.0, ThemePalette::STATUS_HEALTHY);

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        let util_compact_btn = |ui: &mut egui::Ui, icon: &str, tip: &str| -> egui::Response {
                            let btn = egui::Button::new(
                                egui::RichText::new(icon)
                                    .size(13.0)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                            .frame(false);
                            ui.add_sized([ui.available_width(), 24.0], btn).on_hover_text(tip)
                        };

                        if util_compact_btn(ui, "ℹ", "About SysMon").clicked() {
                            self.selected_tab = Tab::About;
                        }
                        if util_compact_btn(ui, "⌨", "Keyboard Shortcuts").clicked() {
                            self.show_shortcuts = true;
                        }
                        if util_compact_btn(ui, "⚙", "Settings (Ctrl+,)").clicked() {
                            self.show_settings = true;
                        }
                    }
                });
            });

        // Process Manager window
        if self.show_process_manager {
            crate::ui::windows::process_manager::show(self, ctx, &data);
        }

        // Keyboard Shortcuts dialog
        crate::ui::dialogs::render_shortcuts_dialog(self, ctx, is_dark);

        // Settings window
        if self.show_settings {
            let mut show_settings = self.show_settings;
            egui::Window::new("Settings")
                .open(&mut show_settings)
                .resizable(true)
                .default_width(600.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    crate::ui::pages::settings::show(self, ui);
                });
            self.show_settings = show_settings;
        }

        // Desktop mini-widget: a compact always-visible telemetry window
        if self.widget_open {
            egui::Window::new("SysMon Widget")
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(0.0, 0.0))
                .resizable(false)
                .title_bar(true)
                .collapsible(false)
                .show(ctx, |ui| {
                    crate::ui::hud::render_hud(self, ui, &data);
                });
        }

        // Global always-visible status bar header
        let status_bar_frame = egui::Frame::none()
            .fill(ThemePalette::bg_deepest(is_dark))
            .inner_margin(egui::Margin::symmetric(14.0, 0.0))
            .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)));

        egui::TopBottomPanel::top("global_status_bar")
            .exact_height(42.0)
            .frame(status_bar_frame)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if is_collapsed {
                        let expand_btn =
                            egui::Button::new(egui::RichText::new("☰").size(13.0).color(ThemePalette::ACCENT_PRIMARY))
                                .fill(ThemePalette::bg_surface(is_dark))
                                .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
                                .rounding(egui::Rounding::same(4.0));

                        if ui.add(expand_btn).on_hover_text("Expand Sidebar (Ctrl+B)").clicked() {
                            self.settings.sidebar_collapsed = false;
                            let _ = self.settings.save();
                        }
                        ui.add_space(4.0);
                    }

                    // Telemetry Ribbon: CPU, RAM, GPU, NET with live micro progress tracks
                    let cpu_c = get_usage_color(data.cpu_usage);
                    paint_telemetry_chip(
                        ui,
                        "CPU",
                        &format!("{:.1}%", data.cpu_usage),
                        Some(data.cpu_usage / 100.0),
                        cpu_c,
                        is_dark,
                    );
                    ui.add_space(3.0);

                    let mem_c = get_usage_color(data.memory_percentage);
                    paint_telemetry_chip(
                        ui,
                        "RAM",
                        &format!("{:.1}%", data.memory_percentage),
                        Some(data.memory_percentage / 100.0),
                        mem_c,
                        is_dark,
                    );
                    ui.add_space(3.0);

                    if let Some(gpu) = data.gpu_info.first() {
                        let gpu_c = get_usage_color(gpu.utilization);
                        paint_telemetry_chip(
                            ui,
                            "GPU",
                            &format!("{:.1}%", gpu.utilization),
                            Some(gpu.utilization / 100.0),
                            gpu_c,
                            is_dark,
                        );
                    } else {
                        paint_telemetry_chip(ui, "GPU", "N/A", None, ThemePalette::text_dimmed(is_dark), is_dark);
                    }
                    ui.add_space(3.0);

                    let net_total_rate: f64 = data
                        .network_info
                        .iter()
                        .map(|n| n.received_rate + n.transmitted_rate)
                        .sum();
                    let net_c = if net_total_rate > 50.0 {
                        ThemePalette::STATUS_CRITICAL
                    } else if net_total_rate > 10.0 {
                        ThemePalette::STATUS_WARNING
                    } else {
                        ThemePalette::STATUS_HEALTHY
                    };
                    paint_telemetry_chip(ui, "NET", &format!("{:.1} MB/s", net_total_rate), None, net_c, is_dark);

                    // Right side Quick Action Hub
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(4.0);

                        // Alerts badge / button
                        if !data.alerts.is_empty() {
                            let alert_count = data.alerts.len();
                            let alert_btn = egui::Button::new(
                                egui::RichText::new(format!("⚠ {alert_count} ALERTS"))
                                    .size(11.0)
                                    .strong()
                                    .color(ThemePalette::STATUS_WARNING),
                            )
                            .fill(ThemePalette::STATUS_WARNING.gamma_multiply(if is_dark { 0.18 } else { 0.12 }))
                            .stroke(egui::Stroke::new(1.0, ThemePalette::STATUS_WARNING.gamma_multiply(0.5)))
                            .rounding(egui::Rounding::same(4.0));

                            if ui.add(alert_btn).on_hover_text("View active system alerts").clicked() {
                                self.selected_tab = Tab::Alerts;
                            }
                        } else {
                            let nominal_btn = egui::Button::new(
                                egui::RichText::new("✓ NORMAL")
                                    .size(10.5)
                                    .strong()
                                    .color(ThemePalette::STATUS_HEALTHY),
                            )
                            .fill(ThemePalette::bg_surface(is_dark))
                            .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
                            .rounding(egui::Rounding::same(4.0));

                            if ui
                                .add(nominal_btn)
                                .on_hover_text("All metric thresholds nominal")
                                .clicked()
                            {
                                self.selected_tab = Tab::Alerts;
                            }
                        }

                        ui.add_space(5.0);

                        // Diagnostic Session Record toggle button
                        let is_recording = self.session_recorder.is_recording();
                        let (rec_text, rec_color) = if is_recording {
                            (
                                format!("⏹ Rec ({})", self.session_recorder.sample_count()),
                                ThemePalette::STATUS_CRITICAL,
                            )
                        } else {
                            ("⏺ Record".to_string(), ThemePalette::text_secondary(is_dark))
                        };

                        let rec_btn =
                            egui::Button::new(egui::RichText::new(&rec_text).size(11.0).strong().color(rec_color))
                                .fill(if is_recording {
                                    ThemePalette::STATUS_CRITICAL.gamma_multiply(if is_dark { 0.20 } else { 0.12 })
                                } else {
                                    ThemePalette::bg_surface(is_dark)
                                })
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if is_recording {
                                        ThemePalette::STATUS_CRITICAL.gamma_multiply(0.5)
                                    } else {
                                        ThemePalette::border(is_dark)
                                    },
                                ))
                                .rounding(egui::Rounding::same(4.0));

                        if ui
                            .add(rec_btn)
                            .on_hover_text(if is_recording {
                                "Stop diagnostic recording session"
                            } else {
                                "Start diagnostic recording session"
                            })
                            .clicked()
                        {
                            let was_recording = is_recording;
                            self.session_status = Some(match self.session_recorder.toggle() {
                                Ok(Some(path)) => {
                                    if was_recording {
                                        format!("Session saved to {}", path.display())
                                    } else {
                                        format!("Recording to {}", path.display())
                                    }
                                }
                                Ok(None) => "Session stopped".into(),
                                Err(error) => format!("Session recorder error: {error}"),
                            });
                        }

                        ui.add_space(5.0);

                        // Clean RAM button
                        let is_cleaning = self.ram_cleaner_state.is_cleaning;
                        let clean_text = if is_cleaning {
                            "🧹 Cleaning..."
                        } else {
                            "🧹 Free RAM"
                        };
                        let clean_btn = egui::Button::new(egui::RichText::new(clean_text).size(11.0).strong().color(
                            if is_cleaning {
                                ThemePalette::text_dimmed(is_dark)
                            } else {
                                ThemePalette::ACCENT_PRIMARY
                            },
                        ))
                        .fill(if is_cleaning {
                            ThemePalette::bg_track(is_dark)
                        } else {
                            ThemePalette::bg_surface(is_dark)
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if is_cleaning {
                                ThemePalette::border(is_dark)
                            } else {
                                ThemePalette::ACCENT_PRIMARY.gamma_multiply(0.45)
                            },
                        ))
                        .rounding(egui::Rounding::same(4.0));

                        ui.add_enabled_ui(!is_cleaning, |ui| {
                            if ui
                                .add(clean_btn)
                                .on_hover_text("Free working sets of running processes")
                                .clicked()
                            {
                                self.start_ram_clean(ctx);
                            }
                        });

                        ui.add_space(5.0);

                        // Mini-Widget / HUD Toggle button
                        let hud_open = self.widget_open;
                        let hud_btn = egui::Button::new(
                            egui::RichText::new(if hud_open { "◰ HUD ON" } else { "◰ HUD" })
                                .size(11.0)
                                .strong()
                                .color(if hud_open {
                                    ThemePalette::ACCENT_PRIMARY
                                } else {
                                    ThemePalette::text_secondary(is_dark)
                                }),
                        )
                        .fill(if hud_open {
                            ThemePalette::ACCENT_PRIMARY.gamma_multiply(if is_dark { 0.18 } else { 0.12 })
                        } else {
                            ThemePalette::bg_surface(is_dark)
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if hud_open {
                                ThemePalette::ACCENT_PRIMARY.gamma_multiply(0.5)
                            } else {
                                ThemePalette::border(is_dark)
                            },
                        ))
                        .rounding(egui::Rounding::same(4.0));

                        if ui
                            .add(hud_btn)
                            .on_hover_text("Toggle Desktop Floating Mini-HUD (Ctrl+M)")
                            .clicked()
                        {
                            self.widget_open = !self.widget_open;
                            self.settings.show_widget = self.widget_open;
                            let _ = self.settings.save();
                            {
                                let mut shared = self.shared_settings.lock();
                                *shared = self.settings.clone();
                            }
                        }
                    });
                });
            });
        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| match self.selected_tab {
            Tab::Overview => crate::ui::pages::overview::show(self, ui, &data),
            Tab::Performance => crate::ui::pages::performance::show(self, ui, &data),
            Tab::Processes => crate::ui::pages::processes::show(self, ui, &data),
            Tab::CpuCores => crate::ui::pages::cpu_cores::show(self, ui, &data),
            Tab::Storage => crate::ui::pages::storage::show(self, ui, &data),
            Tab::Network => crate::ui::pages::network::show(self, ui, &data),
            Tab::SystemInfo => crate::ui::pages::system_info::show(self, ui, &data),
            Tab::Alerts => crate::ui::pages::alerts::show(self, ui, &data),
            Tab::RamCleaner => crate::ui::pages::ram_cleaner::show(self, ui, &data),
            Tab::StartupManager => crate::ui::pages::startup_manager::show(self, ui),
            Tab::Services => crate::ui::pages::services::show(self, ui, &data),
            Tab::Diagnostics => crate::ui::pages::diagnostics::show(self, ui, &data),
            Tab::About => crate::ui::pages::about::show(self, ui, &data),
        });
        crate::ui::dialogs::render_action_confirmation(self, ctx);
        crate::ui::dialogs::render_action_history(self, ctx);
    }
}

// ─── Custom UI helpers ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sysmon-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    }

    #[test]
    fn validation_clamps_user_ranges() {
        let settings = AppSettings {
            refresh_interval: 0,
            process_count: 999,
            ram_clean_threshold: 1.0,
            ..Default::default()
        };
        let checked = crate::persistence::settings::validated(settings);
        assert_eq!(checked.refresh_interval, 1);
        assert_eq!(checked.process_count, 100);
        assert_eq!(checked.ram_clean_threshold, 50.0);
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_path("settings.json");
        let settings = AppSettings::default();
        crate::persistence::settings::save(&path, &settings).unwrap();
        let loaded = crate::persistence::settings::load(&path).unwrap();
        assert_eq!(loaded.refresh_interval, settings.refresh_interval);
        let _ = std::fs::remove_file(path);
    }
}
fn main() {
    // ── 1. Single-Instance Enforcement ──────────────────────────────────
    // Prevent multiple copies from running simultaneously using a Windows named mutex.
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn CreateMutexW(
                lp_mutex_attributes: *const std::ffi::c_void,
                b_initial_owner: i32,
                lp_name: *const u16,
            ) -> *mut std::ffi::c_void;
            fn GetLastError() -> u32;
        }

        let mutex_name: Vec<u16> = "Global\\SystemMonitorSingleInstance\0".encode_utf16().collect();
        let _handle = unsafe { CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr()) };
        let last_error = unsafe { GetLastError() };

        const ERROR_ALREADY_EXISTS: u32 = 183;
        if last_error == ERROR_ALREADY_EXISTS {
            use windows::core::PCWSTR;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

            let title: Vec<u16> = "System Monitor\0".encode_utf16().collect();
            let msg: Vec<u16> = "System Monitor is already running.\n\nCheck your system tray or taskbar.\0"
                .encode_utf16()
                .collect();
            unsafe {
                let _ = MessageBoxW(
                    None,
                    PCWSTR(msg.as_ptr()),
                    PCWSTR(title.as_ptr()),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            std::process::exit(0);
        }
    }

    // ── 2. Crash Report Directory ───────────────────────────────────────
    let log_dir = directories::ProjectDirs::from("com", "Xenonesis", "SystemMonitor")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("SystemMonitor"));
    let crash_dir = log_dir.join("crash-reports");
    let logs_dir = log_dir.join("logs");
    let _ = std::fs::create_dir_all(&crash_dir);
    let _ = std::fs::create_dir_all(&logs_dir);

    // ── 3. Global Panic Handler ─────────────────────────────────────────
    // On panic: write a crash report to disk and show a MessageBox.
    let crash_dir_clone = crash_dir.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let crash_file = crash_dir_clone.join(format!("crash_{}.log", timestamp));

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());

        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        let report = format!(
            "═══════════════════════════════════════════════\n\
             SYSTEM MONITOR — CRASH REPORT\n\
             ═══════════════════════════════════════════════\n\
             Version:   {}\n\
             Timestamp: {}\n\
             Location:  {}\n\
             \n\
             Error:\n\
             {}\n\
             \n\
             Please report this issue at:\n\
             https://github.com/Xenonesis/sysmon/issues\n\
             ═══════════════════════════════════════════════\n",
            APP_VERSION,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            location,
            payload,
        );

        let _ = std::fs::write(&crash_file, &report);

        // Show a MessageBox on Windows so the user sees feedback instead of silent crash
        #[cfg(target_os = "windows")]
        {
            use windows::core::PCWSTR;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

            let title: Vec<u16> = "System Monitor — Unexpected Error\0".encode_utf16().collect();
            let msg_text = format!(
                "System Monitor encountered an unexpected error and needs to close.\n\n\
                 Error: {}\n\
                 Location: {}\n\n\
                 A crash report has been saved to:\n{}\n\n\
                 Please report this issue on GitHub.\0",
                payload,
                location,
                crash_file.display()
            );
            let msg: Vec<u16> = msg_text.encode_utf16().collect();
            unsafe {
                MessageBoxW(None, PCWSTR(msg.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR);
            }
        }
    }));

    // ── 4. Structured Logging ───────────────────────────────────────────
    let file_appender = tracing_appender::rolling::daily(&logs_dir, "system-monitor.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .init();

    info!(version = APP_VERSION, "System Monitor starting — Enterprise Edition");
    info!("Log directory: {}", logs_dir.display());
    info!("Crash report directory: {}", crash_dir.display());

    // ── 5. Launch GUI ───────────────────────────────────────────────────
    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 800.0])
        .with_min_inner_size([900.0, 600.0])
        .with_title(format!("System Monitor v{}", APP_VERSION));

    if let Some(icon) = load_icon() {
        viewport_builder = viewport_builder.with_icon(std::sync::Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };

    info!("Launching GUI window");

    let result = eframe::run_native(
        "System Monitor",
        options,
        Box::new(|cc| {
            let app = SystemMonitorApp::new(cc);
            Ok(Box::new(app))
        }),
    );

    match result {
        Ok(()) => {
            info!("System Monitor shut down gracefully");
        }
        Err(e) => {
            error!("GUI failed to start: {}", e);

            #[cfg(target_os = "windows")]
            {
                use windows::core::PCWSTR;
                use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

                let title: Vec<u16> = "System Monitor — Startup Error\0".encode_utf16().collect();
                let msg_text = format!(
                    "System Monitor failed to start.\n\n\
                     Error: {}\n\n\
                     Please ensure your graphics drivers are up to date.\0",
                    e
                );
                let msg: Vec<u16> = msg_text.encode_utf16().collect();
                unsafe {
                    MessageBoxW(None, PCWSTR(msg.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR);
                }
            }

            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn test_battery_info_default() {
        let b = BatteryInfo::default();
        assert_eq!(b.design_capacity, 0);
        assert!(!b.present);
    }
}
#[cfg(test)]
mod ram_cleaner_tests {
    use super::*;

    fn ex(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exclusion_matches_case_insensitively() {
        assert!(is_excluded("Chrome.EXE", &ex(&["chrome.exe"])));
        assert!(is_excluded("firefox", &ex(&["FireFox"])));
        assert!(!is_excluded("notepad", &ex(&["chrome.exe"])));
        assert!(!is_excluded("chrome", &ex(&["chrome.exe"])));
    }

    #[test]
    fn stop_conditions_cover_target_budget_and_empty() {
        assert!(should_stop_cleaning(65.0, 70.0, 10, 100)); // under target
        assert!(!should_stop_cleaning(80.0, 70.0, 10, 100)); // still over target
        assert!(should_stop_cleaning(90.0, 70.0, 0, 100)); // nothing freed
        assert!(should_stop_cleaning(90.0, 70.0, 100, 100)); // budget exhausted
    }

    #[test]
    fn settings_defaults_and_clamps() {
        let s = AppSettings::default();
        assert_eq!(s.auto_clean_target, 70.0);
        assert!(s.auto_clean_notify);
        assert_eq!(s.auto_clean_max_mb, 0);
        let s2 = AppSettings {
            auto_clean_target: 10.0,
            auto_clean_max_mb: 99999,
            ..Default::default()
        };
        let c = crate::persistence::settings::validated(s2);
        assert_eq!(c.auto_clean_target, 30.0);
        assert_eq!(c.auto_clean_max_mb, 4096);
    }

    #[test]
    fn settings_sidebar_collapsed_default_and_serde() {
        let s = AppSettings::default();
        assert!(!s.sidebar_collapsed);
        let json = serde_json::to_string(&s).unwrap();
        let mut deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.sidebar_collapsed);
        deserialized.sidebar_collapsed = true;
        let json2 = serde_json::to_string(&deserialized).unwrap();
        let deserialized2: AppSettings = serde_json::from_str(&json2).unwrap();
        assert!(deserialized2.sidebar_collapsed);
    }
}
