//! Application shell: per-frame UI body extracted verbatim from
//! `eframe::App::ui` in `main.rs`. Original section order preserved.

use crate::app;
use crate::app::models::*;
use crate::monitoring::engine::{SystemMonitorApp, Tab};
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::{privilege, startup, updater};
use chrono::Local;
use eframe::egui;
use rfd::FileDialog;
use std::thread;
use std::time::Instant;
use tracing::warn;
#[cfg(target_os = "windows")]
use tray_icon::menu::MenuEvent;

pub(crate) fn request_update_check(app: &mut SystemMonitorApp, ctx: &egui::Context) {
    if app.update_check_pending {
        return;
    }
    app.update_check_pending = true;
    app.update_check_status = Some("Checking for updates…".into());
    app.update_error = None;
    app.update_check_time = Some(Instant::now());
    let mut updater = app.updater.clone();
    let result_share = app.update_check_result_share.clone();
    let ctx = ctx.clone();
    if let Err(error) = thread::Builder::new().name("update_check".into()).spawn(move || {
        *result_share.lock() = Some(updater.check_for_updates());
        ctx.request_repaint();
    }) {
        app.update_check_pending = false;
        app.update_check_status = Some(format!("Could not start update check: {error}"));
    }
}

fn set_hidden(app: &mut SystemMonitorApp, ctx: &egui::Context, hidden: bool) {
    app.is_hidden = hidden;
    app.data.write().is_hidden = hidden;
    ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Visible(!hidden));
    if !hidden {
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
    }
    if let Err(error) = app
        .app_channels
        .monitoring_sender
        .send(app::commands::MonitoringCommand::SetHidden(hidden))
    {
        app.action_status = Some(format!("Monitoring visibility update failed: {error}"));
    }
}

pub(crate) fn logic_shell(app: &mut SystemMonitorApp, ctx: &egui::Context) {
    let ctx = ctx.clone();
    app.session_recorder.poll();
    if let Some(status) = app.session_recorder.take_status() {
        app.session_status = Some(status);
    }
    app.storage_page.poll_background(&ctx);
    crate::ui::windows::process_manager::update_window_picker(app, &ctx);
    while let Ok(event) = app.app_channels.event_receiver.try_recv() {
        match event {
            app::events::AppEvent::MonitoringPaused(paused) => {
                app.timeline.record_monitoring_transition(paused);
                app.last_monitoring_paused = paused;
                if let Some(item) = &app.tray_menu_pause_item {
                    item.set_checked(paused);
                }
            }
            app::events::AppEvent::Snapshot(snapshot) => {
                if let Err(error) = app.session_recorder.record(&snapshot) {
                    app.session_status = Some(format!("Session recording failed: {error}"));
                }
                let snapshot = *snapshot;
                app.timeline.record_snapshot(snapshot.clone());
                app.latest_snapshot = Some(snapshot);
            }
            app::events::AppEvent::ActionCompleted {
                command,
                record,
                undo,
                ram_outcome,
            } => {
                app.timeline
                    .record_event(crate::timeline::TimelineEvent::from_audit(&record));
                app.action_pending = false;
                app.action_status = Some(record.message.clone());
                if matches!(
                    &command,
                    app::commands::ActionCommand::CleanRam | app::commands::ActionCommand::AutoCleanRam { .. }
                ) {
                    app.ram_cleaner_state.is_cleaning = false;
                    app.ram_cleaner_state.last_cleaned = Some(Instant::now());
                    app.ram_cleaner_state.last_cleaned_display = Local::now().format("%H:%M:%S").to_string();
                    app.ram_cleaner_state.clean_count += 1;
                    if let Some(outcome) = ram_outcome {
                        let bytes = outcome.working_set_reduction;
                        app.ram_cleaner_state.bytes_freed = app.ram_cleaner_state.bytes_freed.saturating_add(bytes);
                        let mut data = app.data.write();
                        data.ram_clean_freed_bytes = data.ram_clean_freed_bytes.saturating_add(bytes);
                    }
                }
                match &command {
                    app::commands::ActionCommand::SuspendProcess(pid) => {
                        app.suspended_pids.insert(*pid);
                    }
                    app::commands::ActionCommand::ResumeProcess(pid) => {
                        app.suspended_pids.remove(pid);
                    }
                    app::commands::ActionCommand::KillProcess(pid)
                    | app::commands::ActionCommand::KillProcessTree(pid) => {
                        app.suspended_pids.remove(pid);
                        app.storage_page.locks_invalidated();
                    }
                    app::commands::ActionCommand::DisableStartup { locator, .. } => {
                        if let Some(item) = app.startup_items.iter_mut().find(|item| item.locator == *locator) {
                            item.enabled = false;
                        }
                    }
                    app::commands::ActionCommand::EnableStartup { locator, .. } => {
                        if let Some(item) = app.startup_items.iter_mut().find(|item| item.locator == *locator) {
                            item.enabled = true;
                        }
                    }
                    app::commands::ActionCommand::QuarantineStartup { locator, .. } => {
                        app.startup_items.retain(|item| item.locator != *locator);
                    }
                    app::commands::ActionCommand::RestoreStartup { review } => {
                        for entry in &mut app.action_history {
                            if matches!(
                                &entry.undo,
                                Some(app::commands::ActionCommand::RestoreStartup { review: existing })
                                    if existing.id() == review.id()
                            ) {
                                entry.undo = None;
                            }
                        }
                    }
                    app::commands::ActionCommand::ReclaimStorageCaches(_) => {
                        app.storage_page.cleanup_finished(record.message.clone());
                    }
                    _ => {}
                }
                if matches!(
                    &command,
                    app::commands::ActionCommand::DisableStartup { .. }
                        | app::commands::ActionCommand::EnableStartup { .. }
                        | app::commands::ActionCommand::QuarantineStartup { .. }
                        | app::commands::ActionCommand::RestoreStartup { .. }
                ) {
                    app.startup_items_loaded = false;
                    app.startup_items_loading = false;
                    *app.startup_items_share.lock() = None;
                }
                app.data.write().high_impact_startup_count = startup::high_impact_count(&app.startup_items);
                app.action_history
                    .push(app::actions::ActionHistoryEntry { record, undo });
            }
            app::events::AppEvent::ActionFailed { command, record } => {
                app.timeline
                    .record_event(crate::timeline::TimelineEvent::from_audit(&record));
                app.action_pending = false;
                app.action_status = Some(record.message.clone());
                if matches!(
                    &command,
                    app::commands::ActionCommand::CleanRam | app::commands::ActionCommand::AutoCleanRam { .. }
                ) {
                    app.ram_cleaner_state.is_cleaning = false;
                }
                if matches!(&command, app::commands::ActionCommand::ReclaimStorageCaches(_)) {
                    app.storage_page.cleanup_finished(record.message.clone());
                }
                app.action_history
                    .push(app::actions::ActionHistoryEntry { record, undo: None });
                if app.settings.enable_sounds {
                    play_alert_sound();
                }
            }
        }
    }
    if let Some(result) = app.timeline.take_query_result() {
        match result {
            Ok(window) => {
                app.timeline_ui.window = Some(std::sync::Arc::new(window));
                app.timeline_ui.last_refresh = Some(Instant::now());
                app.timeline_ui.message = None;
            }
            Err(error) => app.timeline_ui.message = Some(error),
        }
    }
    if let Some(result) = app.timeline.take_export_result() {
        app.timeline_ui.message = Some(match result {
            Ok(path) => format!("Incident exported to {}", path.display()),
            Err(error) => format!("Incident export failed: {error}"),
        });
    }
    let active_alerts = app.data.read().alerts.clone();
    if let Some(result) = app.timeline.take_clear_result() {
        app.timeline_ui.message = Some(match result {
            Ok(()) => "Timeline cleared; history storage acknowledged the removal.".into(),
            Err(error) => format!("Timeline clear failed: {error}"),
        });
        app.timeline_ui.last_refresh = None;
    }
    let active_keys: std::collections::HashSet<_> = active_alerts.iter().map(AlertInfo::key).collect();
    for alert in &active_alerts {
        let key = alert.key();
        if !app.timeline_ui.active_alert_keys.contains(&key) {
            app.timeline.record_event(crate::timeline::TimelineEvent::new(
                crate::timeline::TimelineEventKind::AlertTriggered,
                key,
                "warning",
                alert.message.clone(),
                format!("Observed value: {:.2}", alert.value),
            ));
        }
    }
    for resolved in app.timeline_ui.active_alert_keys.difference(&active_keys) {
        app.timeline.record_event(crate::timeline::TimelineEvent::new(
            crate::timeline::TimelineEventKind::AlertResolved,
            resolved.clone(),
            "info",
            format!("{} alert resolved", resolved),
            "The metric returned below its configured threshold.",
        ));
    }
    app.timeline_ui.active_alert_keys = active_keys;
    if app.timeline.status().enabled {
        let services = {
            let data = app.data.read();
            (!data.services.is_empty()).then(|| {
                data.services
                    .iter()
                    .map(|service| (service.name.clone(), service.state.clone()))
                    .collect()
            })
        };
        if let Some(event) = app.timeline_ui.service_inventory_event(services) {
            app.timeline.record_event(event);
        }
        let startup = app.startup_items_loaded.then(|| {
            app.startup_items
                .iter()
                .map(|item| (format!("{:?}", item.locator), item.enabled))
                .collect()
        });
        if let Some(event) = app.timeline_ui.startup_inventory_event(startup) {
            app.timeline.record_event(event);
        }
    } else {
        app.timeline_ui.service_states = None;
        app.timeline_ui.startup_states = None;
    }
    {
        let mut data = app.data.write();
        data.is_hidden = app.is_hidden;
        data.selected_tab = app.selected_tab;
    }
    // Apply minimized setting immediately
    if !app.start_minimized_applied {
        app.start_minimized_applied = true;
        if app.settings.start_minimized {
            if app.settings.minimize_to_tray {
                set_hidden(app, &ctx, true);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv()
            && let Some(hk) = &app.clean_ram_hotkey
            && event.id == hk.id()
        {
            app.queue_action(app::commands::ActionCommand::CleanRam);
        }
    }

    #[cfg(target_os = "windows")]
    if let Ok(event) = MenuEvent::receiver().try_recv() {
        if Some(&event.id) == app.tray_menu_quit_id.as_ref() {
            app.quit_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if Some(&event.id) == app.tray_menu_show_id.as_ref() {
            set_hidden(app, &ctx, false);
        } else if Some(&event.id) == app.tray_menu_clean_id.as_ref() {
            app.queue_action(app::commands::ActionCommand::CleanRam);
            set_hidden(app, &ctx, false);
        } else if Some(&event.id) == app.tray_menu_procman_id.as_ref() {
            set_hidden(app, &ctx, false);
            app.show_process_manager = true;
        } else if Some(&event.id) == app.tray_menu_pause_id.as_ref() {
            let paused = !app.data.read().monitoring_paused;
            if let Err(error) = app
                .app_channels
                .monitoring_sender
                .send(app::commands::MonitoringCommand::SetPaused(paused))
            {
                app.action_status = Some(format!("Could not change monitoring state: {error}"));
            }
        } else if let Some(plan_guid) = app.tray_menu_power_guids.get(&event.id) {
            let plan_guid = plan_guid.clone();
            app.queue_action(app::commands::ActionCommand::SetPowerPlan(plan_guid));
        }
    }

    if ctx.input(|i| i.viewport().close_requested()) && !app.quit_requested {
        if app.settings.minimize_to_tray && app.tray_icon.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            set_hidden(app, &ctx, true);
        } else {
            app.quit_requested = true;
        }
    }
    if app.quit_requested {
        if let Err(error) = app.session_recorder.shutdown(std::time::Duration::from_secs(2)) {
            warn!(%error, "Recording did not finish flushing before shutdown");
        }
        crate::ui::windows::process_manager::cancel_window_picker(app, &ctx);
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        return;
    }

    // Update tray tooltip with CPU/RAM usage
    #[cfg(target_os = "windows")]
    if let Some(tray) = &mut app.tray_icon {
        let data = app.data.read();
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

    if app.update_check_time.is_none_or(|t| t.elapsed().as_secs() > 86400) {
        request_update_check(app, &ctx);
    }
    let check_result = app.update_check_result_share.lock().take();
    if let Some(result) = check_result {
        app.update_check_pending = false;
        match result {
            Ok(info) => {
                app.update_check_status = Some(if info.update_available {
                    format!("Update {} is available", info.latest_version)
                } else {
                    format!("Update check completed for version {}", info.current_version)
                });
                app.show_update_notification = info.update_available;
                *app.update_info_share.lock() = Some(info);
            }
            Err(error) => {
                app.update_check_status = Some(error.clone());
                app.update_error = Some(error);
            }
        }
    }

    // A resumed instance (relaunched after the update helper ran) reports its
    // install outcome exactly once through the same banner path.
    if let Some(outcome) = updater::take_install_outcome() {
        let mut share = app.update_result_share.lock();
        if share.is_none() {
            *share = Some(Ok(outcome));
        }
    }

    let installer_result = app.update_result_share.lock().take();
    if let Some(result) = installer_result {
        app.update_downloading = false;
        match result {
            Ok(crate::updater::InstallOutcome::Launched) => {
                // Installer launched successfully — hide banner.
                app.show_update_notification = false;
                app.update_error = None;
            }
            Ok(crate::updater::InstallOutcome::Canceled) => {
                app.show_update_notification = false;
                app.update_error = None;
            }
            Ok(outcome) => {
                app.show_update_notification = false;
                app.update_error = Some(format!("Update installation ended: {outcome:?}"));
            }
            Err(msg) => {
                app.update_error = Some(msg);
            }
        }
    }

    // Mirror details selection into shared state so the monitor thread computes details
    {
        let mut d = app.data.write();
        if d.selected_process_pid != app.details_pid {
            d.selected_process_pid = app.details_pid;
            d.selected_process_details = None;
        }
    }

    // Clone a point-in-time snapshot of SystemData so the lock is released instantly.
    // This completely eliminates reader-writer lock contention and deadlocks between the UI
    // thread and background monitoring / worker threads.
    let data = app.data.read().clone();

    // Handle process kill actions
    if let Some(pid) = app.selected_process_pid.take() {
        app.queue_action(app::commands::ActionCommand::KillProcess(pid));
    }

    // Handle process tree kill actions (background thread; tree walk + kills can take seconds)
    if let Some(root) = app.kill_tree_pid.take() {
        app.queue_action(app::commands::ActionCommand::KillProcessTree(root));
    }

    // Handle process suspend actions
    if let Some(pid) = app.suspend_process_pid.take() {
        app.queue_action(app::commands::ActionCommand::SuspendProcess(pid));
    }

    // Handle process resume actions
    if let Some(pid) = app.resume_process_pid.take() {
        app.queue_action(app::commands::ActionCommand::ResumeProcess(pid));
    }

    // Handle process priority changes
    if let Some((identity, priority)) = app.priority_change.take() {
        app.queue_action(app::commands::ActionCommand::SetPriority { identity, priority });
    }

    // Handle process CPU affinity changes
    if let Some((identity, preset)) = app.affinity_change.take() {
        app.queue_action(app::commands::ActionCommand::SetAffinity { identity, preset });
    }

    // Automatic cleanup uses the same bounded action worker and audit path as manual cleanup.
    if app.ram_cleaner_state.auto_clean_enabled
        && !app.ram_cleaner_state.is_cleaning
        && !app.action_pending
        && app.pending_action_plan.is_none()
        && !data.monitoring_paused
        && data.memory_percentage >= app.ram_cleaner_state.auto_clean_threshold
        && app
            .ram_cleaner_state
            .last_cleaned
            .is_none_or(|last| last.elapsed().as_secs() >= app.ram_cleaner_state.auto_clean_interval)
    {
        let policy = &app.ram_cleaner_state;
        let command = app::commands::ActionCommand::AutoCleanRam {
            exclusions: policy.auto_clean_exclusions.clone(),
            smart_only: policy.auto_clean_smart_only,
            budget_bytes: (policy.auto_clean_max_mb != 0).then(|| policy.auto_clean_max_mb.saturating_mul(1024 * 1024)),
            target_percent: policy.auto_clean_target,
            idle_only: policy.auto_clean_idle_only,
        };
        app.ram_cleaner_state.last_cleaned = Some(Instant::now());
        match app.app_channels.action_sender.send(command) {
            Ok(()) => {
                app.ram_cleaner_state.is_cleaning = true;
                app.action_pending = true;
            }
            Err(error) => app.action_status = Some(format!("Automatic cleanup could not start: {error}")),
        }
    }
    let demand = (app.session_recorder.is_recording(), app.show_process_manager);
    let demand_id = egui::Id::new("monitoring_consumer_demand");
    let previous = ctx.data(|data| data.get_temp::<(bool, bool)>(demand_id));
    if previous != Some(demand)
        && app
            .app_channels
            .monitoring_sender
            .send(app::commands::MonitoringCommand::SetConsumerDemand {
                recording: demand.0,
                process_manager: demand.1,
            })
            .is_ok()
    {
        ctx.data_mut(|data| data.insert_temp(demand_id, demand));
    }
    if !ctx.embed_viewports() {
        crate::ui::hud::show_hud(app, &ctx, &data);
    }
    ctx.data_mut(|state| state.insert_temp(egui::Id::new("render_snapshot"), std::sync::Arc::new(data)));
}

pub(crate) fn ui_shell(app: &mut SystemMonitorApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let data = ctx
        .data(|state| state.get_temp::<std::sync::Arc<SystemData>>(egui::Id::new("render_snapshot")))
        .unwrap_or_else(|| std::sync::Arc::new(app.data.read().clone()));
    // Show update notification banner
    let update_info_opt = app.update_info_share.lock().clone();
    if let Some(update_info) = update_info_opt
        && update_info.update_available
        && app.show_update_notification
    {
        let mut frame = egui::Frame::NONE.fill(ThemePalette::BG_SURFACE);
        frame.inner_margin = egui::Margin::symmetric(16, 12);

        egui::Panel::top("update_notification").frame(frame).show(ui, |ui| {
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
                if let Some(err) = &app.update_error {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 70), format!("⚠ {}", err));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !app.update_downloading {
                        if ui.button("Dismiss").clicked() {
                            app.show_update_notification = false;
                            app.update_error = None;
                        }
                        ui.add_space(8.0);
                    }
                    if app.update_downloading {
                        ui.add_enabled(
                            false,
                            egui::Button::new(egui::RichText::new("⏳ Downloading…").strong()),
                        );
                    } else if ui.button(egui::RichText::new("Install Update").strong()).clicked() {
                        let download_url = update_info.download_url.clone();
                        let checksum_url = update_info.checksum_url.clone();
                        let result_share = app.update_result_share.clone();
                        app.update_downloading = true;
                        app.update_error = None;
                        thread::Builder::new()
                            .name("updater_downloader".to_string())
                            .stack_size(8 * 1024 * 1024)
                            .spawn(move || {
                                let result =
                                    updater::Updater::new().download_and_install_update(&download_url, &checksum_url);
                                *result_share.lock() = Some(result);
                            })
                            .expect("failed to spawn updater downloader thread");
                    }
                });
            });
        });
    }

    // Keyboard shortcuts
    ctx.input(|i| {
        if i.key_pressed(egui::Key::F5) {
            // Refresh (reset statistics)
            {
                let mut data = app.data.write();
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

            if let Some(tab) = new_tab
                && (tab != Tab::CpuCores || app.settings.show_cpu_cores)
            {
                app.selected_tab = tab;
            }
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::E) {
            // Ctrl+E = Export
            app.show_export = true;
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::B) {
            // Ctrl+B = Toggle Sidebar
            app.settings.sidebar_collapsed = !app.settings.sidebar_collapsed;
            crate::ui::pages::settings::commit_settings(app);
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::M) {
            // Ctrl+M = Toggle Mini-Widget / HUD
            app.widget_open = !app.widget_open;
            app.settings.show_widget = app.widget_open;
            crate::ui::pages::settings::commit_settings(app);
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::Comma) {
            // Ctrl+, = Settings
            app.show_settings = true;
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::U) {
            request_update_check(app, &ctx);
        }
    });

    // CSV Export window
    let mut show_export_csv = app.show_export_csv;
    if show_export_csv {
        let csv_result = SystemMonitorApp::export_to_csv(&data);
        egui::Window::new("Export to CSV")
            .open(&mut show_export_csv)
            .resizable(true)
            .default_width(500.0)
            .show(ui, |ui| {
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
                                ui.ctx().copy_text(csv_data.clone());
                            }
                            if ui.button("💾 Save to File...").clicked() {
                                let date_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
                                if let Some(path) = FileDialog::new()
                                    .set_file_name(format!("sysmon_export_{}.csv", date_str))
                                    .add_filter("CSV File", &["csv"])
                                    .save_file()
                                    && std::fs::write(&path, &csv_data).is_ok()
                                {
                                    #[cfg(target_os = "windows")]
                                    play_success_sound();
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
    app.show_export_csv = show_export_csv;

    // JSON Export window
    let mut show_export = app.show_export;
    if show_export {
        let json_result = app.export_data_to_json(&data);
        egui::Window::new("Export Data")
            .open(&mut show_export)
            .resizable(true)
            .default_width(500.0)
            .show(ui, |ui| {
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
                                ui.ctx().copy_text(json_data.clone());
                            }
                            if ui.button("💾 Save to File...").clicked() {
                                let date_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
                                if let Some(path) = FileDialog::new()
                                    .set_file_name(format!("sysmon_export_{}.json", date_str))
                                    .add_filter("JSON File", &["json"])
                                    .save_file()
                                    && std::fs::write(&path, &json_data).is_ok()
                                {
                                    #[cfg(target_os = "windows")]
                                    play_success_sound();
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
    app.show_export = show_export;

    // Alerts window
    let mut show_alerts = app.show_alerts;
    let mut clear_alerts = false;
    if show_alerts {
        egui::Window::new("System Alerts")
            .open(&mut show_alerts)
            .resizable(true)
            .default_width(600.0)
            .show(ui, |ui| {
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
    app.show_alerts = show_alerts;
    if clear_alerts {
        app.data.write().alerts.clear();
    }

    let is_dark = ThemePalette::is_dark_mode(app.settings.theme);

    let is_collapsed = app.settings.sidebar_collapsed;
    let sidebar_width = if is_collapsed { 52.0 } else { 200.0 };
    let sidebar_frame = egui::Frame::NONE
        .fill(ThemePalette::bg_surface(is_dark))
        .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)));

    egui::Panel::left("sidebar_panel")
        .resizable(false)
        .exact_size(sidebar_width)
        .frame(sidebar_frame)
        .show(ui, |ui| {
            // Reserve the utility dock before allocating the scrollable navigation.
            // Neither region can paint or receive input over the other.
            egui::Panel::bottom("sidebar_utilities")
                .exact_size(132.0)
                .show_separator_line(true)
                .show(ui, |ui| {
                    let status = if data.monitoring_paused {
                        "Paused"
                    } else {
                        "Last sample"
                    };
                    ui.label(egui::RichText::new(status).small());
                    ui.label(egui::RichText::new(&data.last_update).small())
                        .on_hover_text("Timestamp of the last collected sample; not a health assessment");
                    for (icon, name) in [("⚙", "Settings"), ("⌨", "Shortcuts"), ("ℹ", "About")] {
                        let text = if is_collapsed {
                            icon.to_owned()
                        } else {
                            format!("{icon}  {name}")
                        };
                        let response = ui.add_sized([ui.available_width(), 24.0], egui::Button::new(text));
                        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
                        if response.on_hover_text(name).clicked() {
                            match name {
                                "Settings" => app.show_settings = true,
                                "Shortcuts" => app.show_shortcuts = true,
                                _ => app.selected_tab = Tab::About,
                            }
                        }
                    }
                });
            ui.horizontal(|ui| {
                if !is_collapsed {
                    ui.label(
                        egui::RichText::new("SysMon")
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                }
                let label = if is_collapsed {
                    "Expand sidebar"
                } else {
                    "Collapse sidebar"
                };
                let response = ui.button(if is_collapsed { "▶" } else { "◀" });
                response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
                if response.on_hover_text(format!("{label} (Ctrl+B)")).clicked() {
                    app.settings.sidebar_collapsed = !is_collapsed;
                    crate::ui::pages::settings::commit_settings(app);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("sidebar_navigation")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (group, tab, icon, name) in [
                        ("TELEMETRY", Tab::Overview, "📊", "Overview"),
                        ("", Tab::Performance, "📈", "Performance"),
                        ("", Tab::CpuCores, "⚡", "CPU Cores"),
                        ("", Tab::Storage, "💾", "Storage"),
                        ("", Tab::Network, "🌐", "Network"),
                        ("SYSTEM CONTROL", Tab::Processes, "📋", "Processes"),
                        ("", Tab::Services, "⚙", "Services"),
                        ("", Tab::StartupManager, "🚀", "Startup Apps"),
                        ("", Tab::RamCleaner, "🧹", "RAM Cleaner"),
                        ("DIAGNOSTICS & HEALTH", Tab::Diagnostics, "🩺", "Diagnostics"),
                        ("", Tab::Timeline, "🕒", "Timeline"),
                        ("", Tab::SystemInfo, "💻", "System Info"),
                        ("", Tab::Alerts, "🔔", "Alerts"),
                    ] {
                        if tab == Tab::CpuCores && !app.settings.show_cpu_cores {
                            continue;
                        }
                        if !group.is_empty() {
                            ui.add_space(6.0);
                            if !is_collapsed {
                                ui.label(
                                    egui::RichText::new(group)
                                        .small()
                                        .color(ThemePalette::text_secondary(is_dark)),
                                );
                            }
                        }
                        let selected = app.selected_tab == tab;
                        let text = if is_collapsed {
                            icon.to_owned()
                        } else {
                            format!("{icon}  {name}")
                        };
                        let response =
                            ui.add_sized([ui.available_width(), 30.0], egui::Button::selectable(selected, text));
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, name)
                        });
                        if response.on_hover_text(name).clicked() {
                            app.selected_tab = tab;
                        }
                    }
                });
        });

    // Process Manager window
    if app.show_process_manager {
        crate::ui::windows::process_manager::show(app, &ctx, &data);
    }

    // Keyboard Shortcuts dialog
    crate::ui::dialogs::render_shortcuts_dialog(app, &ctx, is_dark);

    // Settings window
    if app.show_settings {
        let mut show_settings = app.show_settings;
        egui::Window::new("Settings")
            .open(&mut show_settings)
            .resizable(true)
            .default_width(600.0)
            .default_height(500.0)
            .show(ui, |ui| {
                crate::ui::pages::settings::show(app, ui);
            });
        app.show_settings = show_settings;
    }

    // Desktop mini-widget: a compact always-visible telemetry window
    if ctx.embed_viewports() {
        crate::ui::hud::show_hud(app, &ctx, &data);
    }

    // Global always-visible status bar header
    let status_bar_frame = egui::Frame::NONE
        .fill(ThemePalette::bg_deepest(is_dark))
        .inner_margin(egui::Margin::symmetric(14, 0))
        .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)));

    egui::Panel::top("global_status_bar")
        .exact_size(42.0)
        .frame(status_bar_frame)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if is_collapsed {
                    let expand_btn =
                        egui::Button::new(egui::RichText::new("☰").size(13.0).color(ThemePalette::ACCENT_PRIMARY))
                            .fill(ThemePalette::bg_surface(is_dark))
                            .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
                            .corner_radius(egui::CornerRadius::same(4));
                    if ui.add(expand_btn).on_hover_text("Expand Sidebar (Ctrl+B)").clicked() {
                        app.settings.sidebar_collapsed = false;
                        crate::ui::pages::settings::commit_settings(app);
                    }
                    ui.add_space(4.0);
                }

                let avail_w = ui.available_width();
                let show_gpu = app.settings.show_gpu && avail_w >= 760.0;
                let show_net = avail_w >= 960.0;

                // Telemetry Ribbon: CPU & RAM always visible; GPU & NET responsive
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

                if show_gpu && ui.available_width() >= 380.0 {
                    ui.add_space(3.0);
                    if let Some(gpu) = data.gpu_info.first() {
                        let utilization = gpu.utilization;
                        let gpu_c = utilization
                            .map(get_usage_color)
                            .unwrap_or(ThemePalette::text_dimmed(is_dark));
                        paint_telemetry_chip(
                            ui,
                            "GPU",
                            &utilization
                                .map(|value| format!("{value:.1}%"))
                                .unwrap_or_else(|| "N/A".into()),
                            utilization.map(|value| value / 100.0),
                            gpu_c,
                            is_dark,
                        );
                    } else {
                        paint_telemetry_chip(ui, "GPU", "N/A", None, ThemePalette::text_dimmed(is_dark), is_dark);
                    }
                }

                if show_net && ui.available_width() >= 380.0 {
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
                }

                // Right side Quick Action Hub
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(2.0);

                    // Alerts badge / button
                    if !data.alerts.is_empty() {
                        let alert_count = data.alerts.len();
                        let label = if avail_w >= 780.0 {
                            format!("⚠ {alert_count} ALERTS")
                        } else {
                            format!("⚠ {alert_count}")
                        };
                        let alert_btn = egui::Button::new(
                            egui::RichText::new(label)
                                .size(10.5)
                                .strong()
                                .color(ThemePalette::STATUS_WARNING),
                        )
                        .fill(ThemePalette::STATUS_WARNING.gamma_multiply(if is_dark { 0.18 } else { 0.12 }))
                        .stroke(egui::Stroke::new(1.0, ThemePalette::STATUS_WARNING.gamma_multiply(0.5)))
                        .corner_radius(egui::CornerRadius::same(4));

                        if ui.add(alert_btn).on_hover_text("View active system alerts").clicked() {
                            app.selected_tab = Tab::Alerts;
                        }
                    } else {
                        let label = if avail_w >= 780.0 { "✓ NORMAL" } else { "✓" };
                        let nominal_btn = egui::Button::new(
                            egui::RichText::new(label)
                                .size(10.0)
                                .strong()
                                .color(ThemePalette::STATUS_HEALTHY),
                        )
                        .fill(ThemePalette::bg_surface(is_dark))
                        .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
                        .corner_radius(egui::CornerRadius::same(4));

                        if ui
                            .add(nominal_btn)
                            .on_hover_text("All metric thresholds nominal")
                            .clicked()
                        {
                            app.selected_tab = Tab::Alerts;
                        }
                    }

                    ui.add_space(3.0);

                    // Diagnostic Session Record toggle button
                    let is_recording = app.session_recorder.is_recording();
                    let (rec_text, rec_color) = if is_recording {
                        (
                            format!("⏹ ({})", app.session_recorder.sample_count()),
                            ThemePalette::STATUS_CRITICAL,
                        )
                    } else if avail_w >= 820.0 {
                        ("⏺ Record".to_string(), ThemePalette::text_secondary(is_dark))
                    } else {
                        ("⏺ Rec".to_string(), ThemePalette::text_secondary(is_dark))
                    };

                    let rec_btn =
                        egui::Button::new(egui::RichText::new(&rec_text).size(10.5).strong().color(rec_color))
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
                            .corner_radius(egui::CornerRadius::same(4));

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
                        app.session_status = Some(match app.session_recorder.toggle() {
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

                    ui.add_space(3.0);

                    // Clean RAM button
                    let is_cleaning = app.ram_cleaner_state.is_cleaning;
                    let clean_text = if is_cleaning {
                        "🧹 ..."
                    } else if avail_w >= 820.0 {
                        "🧹 Free RAM"
                    } else {
                        "🧹 RAM"
                    };
                    let clean_btn = egui::Button::new(egui::RichText::new(clean_text).size(10.5).strong().color(
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
                    .corner_radius(egui::CornerRadius::same(4));

                    ui.add_enabled_ui(!is_cleaning, |ui| {
                        if ui
                            .add(clean_btn)
                            .on_hover_text("Free working sets of running processes")
                            .clicked()
                        {
                            app.start_ram_clean(&ctx);
                        }
                    });

                    ui.add_space(3.0);

                    // Mini-Widget / HUD Toggle button
                    let hud_open = app.widget_open;
                    let hud_label = if hud_open {
                        if avail_w >= 820.0 { "◰ HUD ON" } else { "◰ ON" }
                    } else {
                        "◰ HUD"
                    };
                    let hud_btn =
                        egui::Button::new(egui::RichText::new(hud_label).size(10.5).strong().color(if hud_open {
                            ThemePalette::ACCENT_PRIMARY
                        } else {
                            ThemePalette::text_secondary(is_dark)
                        }))
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
                        .corner_radius(egui::CornerRadius::same(4));

                    if ui
                        .add(hud_btn)
                        .on_hover_text("Toggle Desktop Floating Mini-HUD (Ctrl+M)")
                        .clicked()
                    {
                        app.widget_open = !app.widget_open;
                        app.settings.show_widget = app.widget_open;
                        crate::ui::pages::settings::commit_settings(app);
                    }

                    ui.add_space(3.0);

                    // Window Target Picker crosshair tool
                    let picker_active = app.window_picker_active;
                    let picker_label = if picker_active {
                        "🎯 Drag to Window..."
                    } else if avail_w >= 820.0 {
                        "🎯 Find Window"
                    } else {
                        "🎯 Target"
                    };
                    let picker_color = if picker_active {
                        ThemePalette::STATUS_WARNING
                    } else {
                        ThemePalette::ACCENT_PRIMARY
                    };
                    let picker_btn = egui::Button::new(
                        egui::RichText::new(picker_label)
                            .size(10.5)
                            .strong()
                            .color(picker_color),
                    )
                    .fill(if picker_active {
                        ThemePalette::STATUS_WARNING.gamma_multiply(if is_dark { 0.20 } else { 0.12 })
                    } else {
                        ThemePalette::bg_surface(is_dark)
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if picker_active {
                            ThemePalette::STATUS_WARNING.gamma_multiply(0.6)
                        } else {
                            ThemePalette::ACCENT_PRIMARY.gamma_multiply(0.4)
                        },
                    ))
                    .corner_radius(egui::CornerRadius::same(4));

                    let picker_resp = ui
                        .add(picker_btn.sense(egui::Sense::drag()))
                        .on_hover_text("Drag crosshair over any desktop window to inspect its process");

                    if picker_resp.drag_started() {
                        crate::ui::windows::process_manager::begin_window_picker(app, &ctx);
                    }
                });
            });
        });
    // Main content area. Pages emit intents; the application shell owns side effects.
    let is_elevated = app.selected_tab == Tab::Services && privilege::is_app_elevated();
    let mut ui_intents = Vec::new();
    egui::CentralPanel::default().show(ui, |ui| match app.selected_tab {
        Tab::Overview => crate::ui::pages::overview::show(app, ui, &data),
        Tab::Performance => crate::ui::pages::performance::show(app, ui, &data),
        Tab::Processes => crate::ui::pages::processes::show(app, ui, &data),
        Tab::CpuCores => crate::ui::pages::cpu_cores::show(app, ui, &data),
        Tab::Storage => crate::ui::pages::storage::show(app, ui, &data),
        Tab::Network => crate::ui::pages::network::show(app, ui, &data),
        Tab::SystemInfo => crate::ui::pages::system_info::show(app, ui, &data),
        Tab::Alerts => crate::ui::pages::alerts::show(app, ui, &data),
        Tab::RamCleaner => crate::ui::pages::ram_cleaner::show(app, ui, &data),
        Tab::StartupManager => crate::ui::pages::startup_manager::show(app, ui),
        Tab::Services => ui_intents.extend(crate::ui::pages::services::show(app, ui, &data, is_elevated)),
        Tab::Diagnostics => crate::ui::pages::diagnostics::show(app, ui, &data),
        Tab::Timeline => crate::ui::pages::timeline::show(app, ui),
        Tab::About => crate::ui::pages::about::show(app, ui, &data),
    });
    for intent in ui_intents {
        app.handle_ui_intent(intent);
    }
    crate::ui::dialogs::render_action_confirmation(app, &ctx);
    crate::ui::dialogs::render_action_history(app, &ctx);
}
