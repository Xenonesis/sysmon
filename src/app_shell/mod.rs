//! Application shell: per-frame UI body extracted verbatim from
//! `eframe::App::ui` in `main.rs`. Original section order preserved.

use crate::app;
use crate::app::models::*;
use crate::monitoring::engine::{SystemMonitorApp, Tab};
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::{persistence, privilege, startup, updater};
use chrono::Local;
use eframe::egui;
use rfd::FileDialog;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use tracing::warn;
#[cfg(target_os = "windows")]
use tray_icon::menu::MenuEvent;

pub(crate) fn ui_shell(app: &mut SystemMonitorApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let ctx_clone = ctx.clone();
    if app.data.read().last_activity.elapsed().as_secs() > 0 {
        app.data.write().last_activity = Instant::now();
    }
    while let Ok(event) = app.app_channels.event_receiver.try_recv() {
        match event {
            app::events::AppEvent::Snapshot(snapshot) => {
                if let Err(error) = app.session_recorder.record(&snapshot) {
                    app.session_status = Some(format!("Session recording failed: {error}"));
                }
                let snapshot = *snapshot;
                app.timeline.record_snapshot(snapshot.clone());
                app.latest_snapshot = Some(snapshot);
            }
            app::events::AppEvent::AuditRecorded(record) => {
                app.action_history
                    .push(app::actions::ActionHistoryEntry { record, undo: None });
            }
            app::events::AppEvent::ActionCompleted { command, record, undo } => {
                app.timeline
                    .record_event(crate::timeline::TimelineEvent::from_audit(&record));
                app.action_pending = false;
                app.action_status = Some(record.message.clone());
                if matches!(&command, app::commands::ActionCommand::CleanRam) {
                    app.ram_cleaner_state.is_cleaning = false;
                    app.ram_cleaner_state.last_cleaned = Some(Instant::now());
                    app.ram_cleaner_state.last_cleaned_display = Local::now().format("%H:%M:%S").to_string();
                    app.ram_cleaner_state.clean_count += 1;
                    if let Some(bytes) = record
                        .message
                        .strip_prefix("Freed ")
                        .and_then(|value| value.split_whitespace().next())
                        .and_then(|value| value.parse::<u64>().ok())
                    {
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
                    app::commands::ActionCommand::RestoreStartup { quarantine_id, .. } => {
                        for entry in &mut app.action_history {
                            if matches!(
                                &entry.undo,
                                Some(app::commands::ActionCommand::RestoreStartup {
                                    quarantine_id: existing,
                                    ..
                                }) if existing == quarantine_id
                            ) {
                                entry.undo = None;
                            }
                        }
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
                if matches!(&command, app::commands::ActionCommand::CleanRam) {
                    app.ram_cleaner_state.is_cleaning = false;
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
                app.timeline_ui.window = Some(window);
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
        let services = app.data.read().services.clone();
        if !services.is_empty() {
            let current: std::collections::HashMap<_, _> = services
                .into_iter()
                .map(|service| (service.name, service.state))
                .collect();
            let mut changes = Vec::new();
            if let Some(previous) = &app.timeline_ui.service_states {
                for (name, state) in current.iter().take(50) {
                    match previous.get(name) {
                        Some(old_state) if old_state != state => changes.push(crate::timeline::TimelineEvent::new(
                            crate::timeline::TimelineEventKind::ServiceChanged,
                            "services",
                            "info",
                            format!("Service {name} changed state"),
                            format!("State changed from {old_state} to {state}"),
                        )),
                        None => changes.push(crate::timeline::TimelineEvent::new(
                            crate::timeline::TimelineEventKind::ServiceChanged,
                            "services",
                            "info",
                            format!("Service {name} detected"),
                            format!("Current state: {state}"),
                        )),
                        _ => {}
                    }
                }
                for name in previous.keys().filter(|name| !current.contains_key(*name)).take(50) {
                    changes.push(crate::timeline::TimelineEvent::new(
                        crate::timeline::TimelineEventKind::ServiceChanged,
                        "services",
                        "info",
                        format!("Service {name} no longer reported"),
                        "The latest successful service inventory no longer contains this service.",
                    ));
                }
            }
            app.timeline_ui.service_states = Some(current);
            for event in changes {
                app.timeline.record_event(event);
            }
        }

        if app.startup_items_loaded {
            let current: std::collections::HashMap<_, _> = app
                .startup_items
                .iter()
                .map(|item| (item.name.clone(), item.enabled))
                .collect();
            let mut changes = Vec::new();
            if let Some(previous) = &app.timeline_ui.startup_states {
                for (name, enabled) in current.iter().take(50) {
                    match previous.get(name) {
                        Some(was_enabled) if was_enabled != enabled => {
                            changes.push(crate::timeline::TimelineEvent::new(
                                crate::timeline::TimelineEventKind::StartupChanged,
                                "startup",
                                "info",
                                format!("Startup item {name} changed"),
                                format!("Enabled changed from {was_enabled} to {enabled}"),
                            ));
                        }
                        None => changes.push(crate::timeline::TimelineEvent::new(
                            crate::timeline::TimelineEventKind::StartupChanged,
                            "startup",
                            "info",
                            format!("Startup item {name} detected"),
                            format!("Enabled: {enabled}"),
                        )),
                        _ => {}
                    }
                }
                for name in previous.keys().filter(|name| !current.contains_key(*name)).take(50) {
                    changes.push(crate::timeline::TimelineEvent::new(
                        crate::timeline::TimelineEventKind::StartupChanged,
                        "startup",
                        "info",
                        format!("Startup item {name} no longer reported"),
                        "The latest successful startup inventory no longer contains this item.",
                    ));
                }
            }
            app.timeline_ui.startup_states = Some(current);
            for event in changes {
                app.timeline.record_event(event);
            }
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
                app.is_hidden = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
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
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if Some(&event.id) == app.tray_menu_show_id.as_ref() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            app.is_hidden = false;
        } else if Some(&event.id) == app.tray_menu_clean_id.as_ref() {
            app.queue_action(app::commands::ActionCommand::CleanRam);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            app.is_hidden = false;
        } else if Some(&event.id) == app.tray_menu_procman_id.as_ref() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            app.is_hidden = false;
            let _ = app
                .app_channels
                .monitoring_sender
                .send(app::commands::MonitoringCommand::SetHidden(false));
            app.show_process_manager = true;
        } else if Some(&event.id) == app.tray_menu_pause_id.as_ref() {
            let paused = {
                let mut d = app.data.write();
                d.monitoring_paused = !d.monitoring_paused;
                d.monitoring_paused
            };
            let _ = app
                .app_channels
                .monitoring_sender
                .send(app::commands::MonitoringCommand::SetPaused(paused));
            if let Some(item) = &app.tray_menu_pause_item {
                item.set_checked(paused);
            }
        } else if let Some(plan_guid) = app.tray_menu_power_guids.get(&event.id) {
            let plan_guid = plan_guid.clone();
            app.queue_action(app::commands::ActionCommand::SetPowerPlan(plan_guid));
        }
    }

    if ctx.input(|i| i.viewport().close_requested()) && app.settings.minimize_to_tray {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        app.is_hidden = true;
        let _ = app
            .app_channels
            .monitoring_sender
            .send(app::commands::MonitoringCommand::SetHidden(true));
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

    // Check for updates automatically (once every 24 hours)
    if app.update_check_time.is_none_or(|t| t.elapsed().as_secs() > 86400) {
        let mut updater = app.updater.clone();
        let update_info_share = app.update_info_share.clone();
        thread::Builder::new()
            .name("auto_updater_check".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                if let Ok(update_info) = updater.check_for_updates() {
                    *update_info_share.lock() = Some(update_info.clone());
                }
            })
            .expect("failed to spawn auto updater check thread");
        app.update_check_time = Some(Instant::now());
    }

    // Poll background installer result each frame.
    let installer_result = app.update_result_share.lock().take();
    if let Some(result) = installer_result {
        app.update_downloading = false;
        match result {
            Ok(()) => {
                // Installer launched successfully — hide banner.
                app.show_update_notification = false;
                app.update_error = None;
            }
            Err(msg) => {
                app.update_error = Some(msg);
            }
        }
    }

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
            let _ = app.settings.save();
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::M) {
            // Ctrl+M = Toggle Mini-Widget / HUD
            app.widget_open = !app.widget_open;
            app.settings.show_widget = app.widget_open;
            let _ = app.settings.save();
            {
                let mut shared = app.shared_settings.lock();
                *shared = app.settings.clone();
            }
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::Comma) {
            // Ctrl+, = Settings
            app.show_settings = true;
        }
        if i.modifiers.ctrl && i.key_pressed(egui::Key::U) {
            // Ctrl+U = Check for updates manually
            let mut updater = app.updater.clone();
            let update_info_share = app.update_info_share.clone();
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
    if let Some((pid, priority)) = app.priority_change.take() {
        app.queue_action(app::commands::ActionCommand::SetPriority { pid, priority });
    }

    // Handle process CPU affinity changes
    if let Some((pid, mask)) = app.affinity_change.take() {
        app.queue_action(app::commands::ActionCommand::SetAffinity { pid, mask });
    }

    // Auto RAM cleaning
    if app.ram_cleaner_state.auto_clean_enabled && !app.ram_cleaner_state.is_cleaning {
        let idle_ok = {
            let d = app.data.read();
            !app.ram_cleaner_state.auto_clean_idle_only || d.last_activity.elapsed().as_secs() > 120
        };
        let should_clean = if let Some(last) = app.ram_cleaner_state.last_cleaned {
            last.elapsed().as_secs() >= app.ram_cleaner_state.auto_clean_interval
                && data.memory_percentage >= app.ram_cleaner_state.auto_clean_threshold
        } else {
            data.memory_percentage >= app.ram_cleaner_state.auto_clean_threshold
        };
        if should_clean && idle_ok {
            app.ram_cleaner_state.is_cleaning = true;
            app.ram_cleaner_state.last_cleaned = Some(Instant::now());
            app.ram_cleaner_state.last_cleaned_display = Local::now().format("%H:%M:%S").to_string();
            app.ram_cleaner_state.clean_count += 1;
            let data_arc = Arc::clone(&app.data);
            let repaint_ctx = ctx_clone.clone();
            let enable_sounds = app.settings.enable_sounds;
            let target = app.ram_cleaner_state.auto_clean_target;
            let max_mb = app.ram_cleaner_state.auto_clean_max_mb;
            let notify = app.ram_cleaner_state.auto_clean_notify;
            let exclusions = app.ram_cleaner_state.auto_clean_exclusions.clone();
            let smart_only = app.ram_cleaner_state.auto_clean_smart_only;
            let total_ram = data.memory_total;
            let auto_event_sender = app.app_channels.event_sender.clone();
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
            app.data.write().ram_clean_is_cleaning = true;
        }
    }
    // Sync back from shared data
    {
        let d = app.data.read();
        if !d.ram_clean_is_cleaning && app.ram_cleaner_state.is_cleaning {
            app.ram_cleaner_state.is_cleaning = false;
        }
        app.ram_cleaner_state.bytes_freed = d.ram_clean_freed_bytes;
    }

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

    // Modern sleek SidePanel for navigation
    egui::Panel::left("sidebar_panel")
        .resizable(false)
        .exact_size(sidebar_width)
        .frame(sidebar_frame)
        .show(ui, |ui| {
            ui.add_space(12.0);

            if !is_collapsed {
                // Brand Header (Expanded)
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.add(
                        egui::Image::new(egui::include_image!("../../assets/icon.png"))
                            .max_width(20.0)
                            .max_height(20.0),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("Sys")
                            .size(17.5)
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new("Mon")
                            .size(17.5)
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        let (btn_rect, btn_resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
                        let is_btn_hovered = btn_resp.hovered();
                        if is_btn_hovered {
                            let hover_fill = if is_dark {
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 14)
                            } else {
                                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 10)
                            };
                            ui.painter()
                                .rect_filled(btn_rect, egui::CornerRadius::same(4), hover_fill);
                        }
                        ui.painter().text(
                            btn_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "◀",
                            egui::FontId::proportional(11.0),
                            if is_btn_hovered {
                                ThemePalette::text_primary(is_dark)
                            } else {
                                ThemePalette::text_secondary(is_dark)
                            },
                        );
                        if btn_resp.on_hover_text("Collapse Sidebar (Ctrl+B)").clicked() {
                            app.settings.sidebar_collapsed = true;
                            let _ = app.settings.save();
                        }
                    });
                });
            } else {
                // Brand Header (Collapsed)
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::click());
                let is_hovered = response.hovered();
                if response.on_hover_text("Expand Sidebar (Ctrl+B)").clicked() {
                    app.settings.sidebar_collapsed = false;
                    let _ = app.settings.save();
                }
                if is_hovered {
                    let hover_fill = if is_dark {
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 14)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 10)
                    };
                    let pill_rect = rect.shrink2(egui::vec2(6.0, 2.0));
                    ui.painter()
                        .rect_filled(pill_rect, egui::CornerRadius::same(4), hover_fill);
                }
                let logo_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(22.0, 22.0));
                egui::Image::new(egui::include_image!("../../assets/icon.png")).paint_at(ui, logo_rect);
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

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
                        if app.settings.show_cpu_cores {
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
                            tab: Tab::Timeline,
                            label: "Timeline",
                            icon: "🕒",
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
                    if g_idx == 0 {
                        ui.add_space(4.0);
                    } else {
                        ui.add_space(10.0);
                    }
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new(group.title)
                                .size(10.0)
                                .strong()
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                    });
                    ui.add_space(3.0);
                } else if g_idx > 0 {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                }

                ui.spacing_mut().item_spacing.y = 2.0;
                for item in &group.items {
                    let is_selected = app.selected_tab == item.tab;
                    let item_h = if is_collapsed { 30.0 } else { 28.0 };
                    let (raw_rect, response) =
                        ui.allocate_exact_size(egui::vec2(ui.available_width(), item_h), egui::Sense::click());
                    let pill_rect = raw_rect.shrink2(egui::vec2(6.0, 1.0));

                    let tooltip_text = if item.tab == Tab::Alerts && !data.alerts.is_empty() {
                        format!("{} ({} active)", item.label, data.alerts.len())
                    } else {
                        item.label.to_string()
                    };

                    let is_hovered = response.hovered();
                    if response.on_hover_text(tooltip_text).clicked() {
                        app.selected_tab = item.tab;
                    }

                    if is_selected {
                        let fill = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(16, 185, 129, 28)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(16, 185, 129, 35)
                        };
                        let border_color = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(16, 185, 129, 65)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(16, 185, 129, 85)
                        };
                        ui.painter().rect_filled(pill_rect, egui::CornerRadius::same(5), fill);
                        ui.painter().rect_stroke(
                            pill_rect,
                            egui::CornerRadius::same(5),
                            egui::Stroke::new(1.0, border_color),
                            egui::StrokeKind::Middle,
                        );

                        // Inset rounded 3px vertical left accent indicator
                        let edge_rect = egui::Rect::from_min_max(
                            egui::pos2(pill_rect.left() + 2.0, pill_rect.top() + 4.0),
                            egui::pos2(pill_rect.left() + 5.0, pill_rect.bottom() - 4.0),
                        );
                        ui.painter()
                            .rect_filled(edge_rect, egui::CornerRadius::same(2), ThemePalette::ACCENT_PRIMARY);
                    } else if is_hovered {
                        let hover_fill = if is_dark {
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 10)
                        };
                        ui.painter()
                            .rect_filled(pill_rect, egui::CornerRadius::same(5), hover_fill);
                    }

                    if !is_collapsed {
                        let text_color = if is_selected || is_hovered {
                            ThemePalette::text_primary(is_dark)
                        } else {
                            ThemePalette::text_secondary(is_dark)
                        };

                        let icon_center = egui::pos2(pill_rect.left() + 16.0, pill_rect.center().y);
                        ui.painter().text(
                            icon_center,
                            egui::Align2::CENTER_CENTER,
                            item.icon,
                            egui::FontId::proportional(12.5),
                            if is_selected {
                                ThemePalette::ACCENT_PRIMARY
                            } else {
                                text_color
                            },
                        );

                        let text_pos = egui::pos2(pill_rect.left() + 30.0, pill_rect.center().y);
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
                            let badge_bg = badge_color.gamma_multiply(if is_dark { 0.22 } else { 0.18 });
                            let badge_rect = egui::Rect::from_center_size(
                                egui::pos2(pill_rect.right() - 14.0, pill_rect.center().y),
                                egui::vec2(18.0, 15.0),
                            );
                            ui.painter()
                                .rect_filled(badge_rect, egui::CornerRadius::same(4), badge_bg);
                            ui.painter().rect_stroke(
                                badge_rect,
                                egui::CornerRadius::same(4),
                                egui::Stroke::new(1.0, badge_color.gamma_multiply(0.45)),
                                egui::StrokeKind::Middle,
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
                            pill_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            item.icon,
                            egui::FontId::proportional(13.5),
                            icon_color,
                        );

                        // Alert indicator dot on collapsed token
                        if item.tab == Tab::Alerts && !data.alerts.is_empty() {
                            let dot_pos = pill_rect.right_top() + egui::vec2(-4.0, 4.0);
                            ui.painter().circle_filled(dot_pos, 3.0, ThemePalette::STATUS_WARNING);
                        }
                    }
                }
            }

            // Pinned Bottom Utility Dock
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(8.0);

                if !is_collapsed {
                    // Live heartbeat label (Live • HH:MM:SS)
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let dot_color = ThemePalette::STATUS_HEALTHY;
                        let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 2.5, dot_color);
                        ui.add_space(4.0);
                        let time_str = data.last_update.split_whitespace().nth(1).unwrap_or(&data.last_update);
                        ui.label(
                            egui::RichText::new(format!("Live • {time_str}"))
                                .size(10.5)
                                .monospace()
                                .color(ThemePalette::text_secondary(is_dark)),
                        )
                        .on_hover_text(format!("Live Telemetry Engine · Full Timestamp: {}", data.last_update));
                    });
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Utility buttons (Expanded, consistent left alignment)
                    let draw_util_btn = |ui: &mut egui::Ui, icon: &str, text: &str, tooltip: &str| -> egui::Response {
                        let (raw_rect, response) =
                            ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), egui::Sense::click());
                        let pill_rect = raw_rect.shrink2(egui::vec2(6.0, 1.0));
                        let is_hovered = response.hovered();
                        if is_hovered {
                            let hover_fill = if is_dark {
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                            } else {
                                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 10)
                            };
                            ui.painter()
                                .rect_filled(pill_rect, egui::CornerRadius::same(5), hover_fill);
                        }

                        let text_color = if is_hovered {
                            ThemePalette::text_primary(is_dark)
                        } else {
                            ThemePalette::text_secondary(is_dark)
                        };

                        let icon_center = egui::pos2(pill_rect.left() + 16.0, pill_rect.center().y);
                        ui.painter().text(
                            icon_center,
                            egui::Align2::CENTER_CENTER,
                            icon,
                            egui::FontId::proportional(12.0),
                            text_color,
                        );

                        let text_pos = egui::pos2(pill_rect.left() + 30.0, pill_rect.center().y);
                        ui.painter().text(
                            text_pos,
                            egui::Align2::LEFT_CENTER,
                            text,
                            egui::FontId::proportional(12.0),
                            text_color,
                        );

                        response.on_hover_text(tooltip)
                    };

                    // Added in bottom_up order (Bottom to Top: About -> Shortcuts -> Settings)
                    if draw_util_btn(ui, "ℹ", "About", "About SysMon").clicked() {
                        app.selected_tab = Tab::About;
                    }
                    if draw_util_btn(ui, "⌨", "Shortcuts", "Keyboard shortcuts (Ctrl+B, F5...)").clicked() {
                        app.show_shortcuts = true;
                    }
                    if draw_util_btn(ui, "⚙", "Settings", "Application settings (Ctrl+,)").clicked() {
                        app.show_settings = true;
                    }

                    ui.add_space(4.0);
                    ui.separator();
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

                    let draw_util_compact_btn = |ui: &mut egui::Ui, icon: &str, tip: &str| -> egui::Response {
                        let (raw_rect, response) =
                            ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), egui::Sense::click());
                        let pill_rect = raw_rect.shrink2(egui::vec2(6.0, 1.0));
                        let is_hovered = response.hovered();
                        if is_hovered {
                            let hover_fill = if is_dark {
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                            } else {
                                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 10)
                            };
                            ui.painter()
                                .rect_filled(pill_rect, egui::CornerRadius::same(5), hover_fill);
                        }

                        let text_color = if is_hovered {
                            ThemePalette::text_primary(is_dark)
                        } else {
                            ThemePalette::text_secondary(is_dark)
                        };

                        ui.painter().text(
                            pill_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            icon,
                            egui::FontId::proportional(12.5),
                            text_color,
                        );

                        response.on_hover_text(tip)
                    };

                    if draw_util_compact_btn(ui, "ℹ", "About SysMon").clicked() {
                        app.selected_tab = Tab::About;
                    }
                    if draw_util_compact_btn(ui, "⌨", "Keyboard Shortcuts").clicked() {
                        app.show_shortcuts = true;
                    }
                    if draw_util_compact_btn(ui, "⚙", "Settings (Ctrl+,)").clicked() {
                        app.show_settings = true;
                    }

                    ui.add_space(4.0);
                    ui.separator();
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
    if app.widget_open {
        egui::Window::new("SysMon Widget")
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(0.0, 0.0))
            .resizable(false)
            .title_bar(true)
            .collapsible(false)
            .show(ui, |ui| {
                crate::ui::hud::render_hud(app, ui, &data);
            });
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
                        let _ = app.settings.save();
                    }
                    ui.add_space(4.0);
                }

                let avail_w = ui.available_width();
                let show_gpu = avail_w >= 760.0;
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
                        let _ = app.settings.save();
                        {
                            let mut shared = app.shared_settings.lock();
                            *shared = app.settings.clone();
                        }
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
                        .add(picker_btn)
                        .on_hover_text("Drag crosshair over any desktop window to inspect its process");

                    if picker_resp.drag_started() || picker_resp.clicked() {
                        app.window_picker_active = true;
                    }

                    if app.window_picker_active {
                        ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                        ctx.request_repaint();

                        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                            app.window_picker_active = false;
                        } else if picker_resp.drag_stopped()
                            || (ctx.input(|i| i.pointer.any_released()) && !picker_resp.clicked())
                        {
                            let (sx, sy) = crate::processes::get_current_cursor_screen_point();
                            if let Some(pid) = crate::processes::get_process_id_from_screen_point(sx, sy) {
                                app.selected_tab = Tab::Processes;
                                app.details_pid = Some(pid);
                                app.process_search = pid.to_string();
                                app.action_status = Some(format!("Targeted window at ({sx}, {sy}): PID {pid}"));
                            } else {
                                app.action_status = Some(format!("No window process found at ({sx}, {sy})"));
                            }
                            app.window_picker_active = false;
                        }
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
