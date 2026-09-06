use crate::app::models::*;
use crate::monitoring::engine::SystemMonitorApp;
use crate::monitoring::engine::Tab;
use crate::{power, processes, services};
use chrono::Local;
#[cfg(target_os = "windows")]
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
#[cfg(target_os = "windows")]
use tray_icon::TrayIconBuilder;
#[cfg(target_os = "windows")]
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, Submenu};

impl SystemMonitorApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Install image loaders for showing the logo
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Load settings
        let settings = AppSettings::load();
        let timeline =
            crate::timeline::TimelineHandle::start(settings.timeline_enabled, settings.timeline_retention_days);

        // Load Windows system fonts at runtime to support all standard symbols and checkmarks
        #[cfg(target_os = "windows")]
        {
            let mut fonts = egui::FontDefinitions::default();
            let mut proportional_loaded = false;
            let mut monospace_loaded = false;

            // Load Segoe UI for standard proportional text
            let font_paths = ["C:\\Windows\\Fonts\\segoeui.ttf", "C:\\Windows\\Fonts\\SegoeUI.ttf"];
            for path in &font_paths {
                if let Ok(font_bytes) = std::fs::read(path) {
                    fonts
                        .font_data
                        .insert("segoe_ui".to_owned(), egui::FontData::from_owned(font_bytes).into());
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .insert(0, "segoe_ui".to_owned());
                    proportional_loaded = true;
                    break;
                }
            }

            // Load Segoe UI Symbol and Emoji fonts for crisp system symbols and icons
            let sym_paths = [
                ("segoe_symbol", "C:\\Windows\\Fonts\\seguisym.ttf"),
                ("segoe_symbol", "C:\\Windows\\Fonts\\SeguiSym.ttf"),
                ("segoe_emoji", "C:\\Windows\\Fonts\\seguiemj.ttf"),
                ("segoe_emoji", "C:\\Windows\\Fonts\\SeguiEmj.ttf"),
            ];
            for (key, path) in &sym_paths {
                if let Ok(font_bytes) = std::fs::read(path) {
                    fonts
                        .font_data
                        .insert(key.to_string(), egui::FontData::from_owned(font_bytes).into());
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .push(key.to_string());
                    fonts
                        .families
                        .entry(egui::FontFamily::Monospace)
                        .or_default()
                        .push(key.to_string());
                }
            }

            // Load Consolas for monospace text
            let mono_paths = ["C:\\Windows\\Fonts\\consola.ttf", "C:\\Windows\\Fonts\\Consola.ttf"];
            for path in &mono_paths {
                if let Ok(font_bytes) = std::fs::read(path) {
                    fonts
                        .font_data
                        .insert("consolas".to_owned(), egui::FontData::from_owned(font_bytes).into());
                    fonts
                        .families
                        .entry(egui::FontFamily::Monospace)
                        .or_default()
                        .insert(0, "consolas".to_owned());
                    monospace_loaded = true;
                    break;
                }
            }

            if proportional_loaded || monospace_loaded {
                cc.egui_ctx.set_fonts(fonts);
            }
        }

        // Configure fonts and style
        let mut style = (*cc.egui_ctx.style_of(cc.egui_ctx.theme())).clone();

        // Premium spacing
        style.spacing.item_spacing = egui::vec2(16.0, 12.0);
        style.spacing.button_padding = egui::vec2(16.0, 10.0);
        style.spacing.interact_size = egui::vec2(32.0, 32.0); // Touch target minimums
        style.spacing.window_margin = egui::Margin::same(20);
        style.spacing.menu_margin = egui::Margin::same(12);

        // Typographic hierarchy (slightly larger for premium feel)
        use egui::{FontFamily, FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Heading, FontId::new(24.0, FontFamily::Proportional)),
            (
                TextStyle::Name("Subheading".into()),
                FontId::new(18.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
            (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        ]
        .into();

        // Apply theme — custom "Terminal Noir" dark or clean light
        let is_dark = crate::ui::theme::ThemePalette::is_dark_mode(settings.theme);
        if is_dark {
            let mut visuals = egui::Visuals::dark();
            // Deep charcoal backgrounds
            visuals.panel_fill = crate::ui::theme::ThemePalette::BG_DEEP;
            visuals.window_fill = crate::ui::theme::ThemePalette::BG_SURFACE;
            visuals.extreme_bg_color = crate::ui::theme::ThemePalette::BG_DEEPEST;

            // Accent for selections and interactions
            visuals.selection.bg_fill = crate::ui::theme::ThemePalette::ACCENT_PRIMARY;
            visuals.selection.stroke = egui::Stroke::NONE;
            visuals.hyperlink_color = crate::ui::theme::ThemePalette::ACCENT_PRIMARY;

            // Subtle borders & widgets
            visuals.widgets.noninteractive.bg_fill = crate::ui::theme::ThemePalette::BG_CARD;
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::BORDER);
            visuals.widgets.noninteractive.fg_stroke =
                egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::TEXT_PRIMARY);

            // Inactive
            visuals.widgets.inactive.bg_fill = crate::ui::theme::ThemePalette::WIDGET_INACTIVE;
            visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::TEXT_SECONDARY);

            // Hovered
            visuals.widgets.hovered.bg_fill = crate::ui::theme::ThemePalette::WIDGET_HOVERED;
            visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::BORDER_LIGHT);
            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::TEXT_SELECTED);

            // Active
            visuals.widgets.active.bg_fill = crate::ui::theme::ThemePalette::ACCENT_ACTIVE;
            visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::TEXT_SELECTED);

            // Rounding (Terminal Noir Minimal 4px)
            visuals.window_corner_radius = egui::CornerRadius::same(4);
            visuals.menu_corner_radius = egui::CornerRadius::same(4);
            visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4);
            visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
            visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
            visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);

            // Window chrome and depth
            visuals.window_stroke = egui::Stroke::new(1.0, crate::ui::theme::ThemePalette::BORDER);
            visuals.window_shadow = egui::epaint::Shadow {
                offset: [0, 4],
                blur: 16,
                spread: 0,
                color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 40),
            };

            visuals.popup_shadow = egui::epaint::Shadow {
                offset: [0, 8],
                blur: 40,
                spread: 0,
                color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 20),
            };

            cc.egui_ctx.set_visuals(visuals);
        } else {
            let mut visuals = egui::Visuals::light();
            // Clean, Apple-like light theme backgrounds
            visuals.panel_fill = egui::Color32::from_rgb(245, 245, 247);
            visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
            visuals.extreme_bg_color = egui::Color32::from_rgb(235, 235, 240);

            // Accent overrides
            visuals.selection.bg_fill = crate::ui::theme::ThemePalette::ACCENT_PRIMARY;
            visuals.selection.stroke = egui::Stroke::NONE;

            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(250, 250, 250);
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 220, 225));
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 45));

            visuals.window_corner_radius = egui::CornerRadius::same(8);
            visuals.menu_corner_radius = egui::CornerRadius::same(8);

            cc.egui_ctx.set_visuals(visuals);
        }

        cc.egui_ctx.set_style_of(cc.egui_ctx.theme(), style);

        let data = Arc::new(RwLock::new(SystemData::default()));
        let data_clone = Arc::clone(&data);
        let shared_settings = Arc::new(Mutex::new(settings.clone()));
        let shared_settings_clone = Arc::clone(&shared_settings);
        let mut app_channels = crate::app::AppChannels::new();
        let monitoring_receiver = app_channels
            .monitoring_receiver
            .take()
            .expect("monitoring receiver missing");
        let action_receiver = app_channels.action_receiver.take().expect("action receiver missing");
        let action_events = app_channels.event_sender.clone();
        let monitoring_events = app_channels.event_sender.clone();

        thread::Builder::new()
            .name("actions".to_string())
            .spawn(move || crate::app::run_action_worker(action_receiver, action_events))
            .expect("failed to spawn action worker");

        let (mut telemetry_hub, mut telemetry_reader, telemetry_commands) = crate::telemetry::TelemetryHub::new();
        telemetry_hub.add_provider(Box::new(crate::providers::sysinfo_provider::SysinfoProvider::new()));
        telemetry_hub.add_provider(Box::new(crate::providers::nvml_provider::NvmlProvider::new()));
        telemetry_hub.add_provider(Box::new(crate::providers::wmi_provider::WmiProvider::new()));
        telemetry_hub.add_provider(Box::new(
            crate::providers::windows_gpu_provider::WindowsGpuProvider::new(),
        ));
        thread::Builder::new()
            .name("telemetry_hub".to_string())
            .spawn(move || telemetry_hub.run())
            .expect("failed to spawn telemetry hub");
        let telemetry_commands_for_monitor = telemetry_commands.clone();

        // Background thread for monitoring
        thread::Builder::new()
            .name("monitoring".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let mut monitor = SystemMonitor::new();

                // Get system info once (doesn't change)
                let system_info = monitor.get_system_info();
                let mut battery_check_counter: u32 = 0;
                let mut temperature_check_counter: u32 = 0;
                let mut service_check_counter: u32 = 0;
                let mut disk_smart_check_counter: u32 = 0;
                let mut disk_perf_check_counter: u32 = 0;
                let mut sockets_check_counter: u32 = 0;
                let mut power_plans_check_counter: u32 = 0;
                let mut last_alert_time: std::collections::HashMap<AlertType, Instant> =
                    std::collections::HashMap::new();
                let mut last_hidden_tick = Instant::now();
                let mut last_selected_tab = data_clone.read().selected_tab;
                let mut latest_telemetry = crate::telemetry::TelemetrySnapshot::default();

                loop {
                    let mut force_refresh = false;
                    while let Ok(command) = monitoring_receiver.try_recv() {
                        match command {
                            crate::app::commands::MonitoringCommand::SetSettings(new_settings) => {
                                *shared_settings_clone.lock() = *new_settings;
                            }
                            crate::app::commands::MonitoringCommand::SetPaused(paused) => {
                                data_clone.write().monitoring_paused = paused
                            }
                            crate::app::commands::MonitoringCommand::SetHidden(hidden) => {
                                data_clone.write().is_hidden = hidden;
                                let _ = telemetry_commands_for_monitor
                                    .try_send(crate::telemetry::HubCommand::SetBackgroundMode(hidden));
                            }
                            crate::app::commands::MonitoringCommand::RefreshNow => {
                                force_refresh = true;
                                let _ =
                                    telemetry_commands_for_monitor.try_send(crate::telemetry::HubCommand::ForceRefresh);
                            }
                            crate::app::commands::MonitoringCommand::Shutdown => {
                                let _ = telemetry_commands_for_monitor.try_send(crate::telemetry::HubCommand::Shutdown);
                                return;
                            }
                        }
                    }
                    if let Some(snapshot) = telemetry_reader.latest_if_updated() {
                        latest_telemetry = snapshot;
                    }
                    if !force_refresh {
                        thread::sleep(Duration::from_millis(500));
                    }

                    // Read hidden status, current tab, and pause state
                    let (is_hidden, selected_tab, paused) = {
                        let data = data_clone.read();
                        (data.is_hidden, data.selected_tab, data.monitoring_paused)
                    };

                    // Read current settings from shared state
                    let (refresh_interval, process_count, settings_snapshot) = {
                        let s = shared_settings_clone.lock();
                        (s.refresh_interval, s.process_count, s.clone())
                    };

                    let is_minimized_tick = is_hidden && last_hidden_tick.elapsed().as_secs() < 10;

                    if is_minimized_tick {
                        continue;
                    }

                    if paused {
                        continue;
                    }

                    if is_hidden {
                        last_hidden_tick = Instant::now();
                    }

                    // Rich process, disk and network details still use sysinfo's
                    // native structures; core CPU/RAM/GPU values come from the hub.
                    if !is_hidden {
                        monitor.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                        monitor.disks.refresh(true);
                        monitor.networks.refresh(true);
                    }

                    let fallback_memory = monitor.get_memory_info();
                    let total_mem = latest_telemetry
                        .metrics
                        .get("memory.total")
                        .copied()
                        .map(|value| value as u64)
                        .unwrap_or(fallback_memory.0);
                    let used_mem = latest_telemetry
                        .metrics
                        .get("memory.used")
                        .copied()
                        .map(|value| value as u64)
                        .unwrap_or(fallback_memory.1);
                    let mem_percentage = if total_mem == 0 {
                        0.0
                    } else {
                        used_mem as f32 / total_mem as f32 * 100.0
                    };
                    let cpu_usage = latest_telemetry
                        .metrics
                        .get("cpu.global_usage")
                        .copied()
                        .map(|value| value as f32)
                        .unwrap_or_else(|| monitor.get_cpu_usage());

                    // Optimized queries
                    let need_cpu_cores = !is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::CpuCores);
                    let need_cpu_temp = !is_hidden && selected_tab == Tab::Overview;
                    let need_gpu_wmi =
                        !is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::Performance);
                    let need_gpu_info = need_gpu_wmi
                        || settings_snapshot.show_notifications
                        || settings_snapshot.show_graphs
                        || settings_snapshot.timeline_enabled;
                    // Fetch processes for both Processes tab (all) and Overview tab (top N summary)
                    let need_processes =
                        !is_hidden && (selected_tab == Tab::Processes || selected_tab == Tab::Overview);
                    let need_disks = (!is_hidden && (selected_tab == Tab::Overview || selected_tab == Tab::Storage))
                        || settings_snapshot.show_notifications
                        || settings_snapshot.show_graphs
                        || settings_snapshot.timeline_enabled;
                    let need_network = settings_snapshot.timeline_enabled
                        || (!is_hidden
                            && (selected_tab == Tab::Overview
                                || selected_tab == Tab::Network
                                || selected_tab == Tab::Performance));

                    let cpu_cores = if need_cpu_cores {
                        let cores = cpu_cores_from_telemetry(&latest_telemetry);
                        if cores.is_empty() {
                            monitor.get_cpu_cores_info()
                        } else {
                            cores
                        }
                    } else {
                        Vec::new()
                    };

                    let cpu_temperature = if need_cpu_temp && temperature_check_counter.is_multiple_of(10) {
                        monitor.get_cpu_temperature_wmi()
                    } else {
                        None
                    };
                    temperature_check_counter = temperature_check_counter.wrapping_add(1);

                    let gpu_info = if need_gpu_info {
                        let hub_gpus = gpus_from_telemetry(&latest_telemetry);
                        if hub_gpus.is_empty() {
                            monitor.get_gpu_info(need_gpu_wmi)
                        } else {
                            hub_gpus
                        }
                    } else {
                        Vec::new()
                    };

                    let top_processes = if need_processes {
                        // On Processes tab, fetch ALL processes so search/sort works on the full list.
                        // On Overview tab, only fetch the top N by memory for the summary panel.
                        let fetch_count = if selected_tab == Tab::Processes {
                            usize::MAX
                        } else {
                            process_count
                        };
                        monitor.get_top_processes(fetch_count)
                    } else {
                        Vec::new()
                    };
                    let timeline_processes = if settings_snapshot.timeline_enabled {
                        monitor.get_timeline_processes(10)
                    } else {
                        Vec::new()
                    };

                    let disk_info = if need_disks {
                        monitor.get_disk_info()
                    } else {
                        Vec::new()
                    };

                    let network_info = if need_network {
                        monitor.get_network_info()
                    } else {
                        Vec::new()
                    };

                    let swap_total = latest_telemetry
                        .metrics
                        .get("memory.total_swap")
                        .copied()
                        .map(|v| v as u64);
                    let swap_used = latest_telemetry
                        .metrics
                        .get("memory.used_swap")
                        .copied()
                        .map(|v| v as u64);
                    let swap_info = match (swap_total, swap_used) {
                        (Some(total), Some(used)) => SwapInfo {
                            total,
                            used,
                            percentage: if total == 0 {
                                0.0
                            } else {
                                used as f32 / total as f32 * 100.0
                            },
                        },
                        _ => monitor.get_swap_info(),
                    };

                    let (disk_read_rate, disk_write_rate) = if !is_hidden || settings_snapshot.timeline_enabled {
                        monitor.get_disk_io(refresh_interval)
                    } else {
                        (0.0, 0.0)
                    };

                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

                    // Get battery info every 15 ticks (~7.5s) — retain previous value if unavailable
                    if battery_check_counter.is_multiple_of(15) {
                        let mut bi = None;
                        if let Some(wmi_con) = monitor.get_battery_wmi() {
                            bi = get_battery_info(&wmi_con);
                        }
                        if let Some(bi) = bi {
                            let mut data = data_clone.write();
                            data.battery_info = Some(bi);
                        }
                    }
                    battery_check_counter = battery_check_counter.wrapping_add(1);

                    // Poll services every 60 ticks (~30s) — WMI queries are expensive
                    if !is_hidden && selected_tab == Tab::Services {
                        let services_list = if last_selected_tab != Tab::Services
                            || data_clone.read().services.is_empty()
                            || service_check_counter.is_multiple_of(60)
                        {
                            services::get_services()
                        } else {
                            Vec::new()
                        };
                        if !services_list.is_empty() {
                            let mut data = data_clone.write();
                            data.services = services_list;
                        }
                    }
                    service_check_counter = service_check_counter.wrapping_add(1);

                    // Poll physical disk SMART health in background (~every 30s or when entering tab)
                    if !is_hidden
                        && (selected_tab == Tab::Storage || selected_tab == Tab::Overview)
                        && (disk_smart_check_counter.is_multiple_of(60) || data_clone.read().physical_disks.is_empty())
                    {
                        let drives = crate::storage::get_physical_disks();
                        if !drives.is_empty() {
                            let mut data = data_clone.write();
                            data.physical_disks = drives;
                        }
                    }
                    disk_smart_check_counter = disk_smart_check_counter.wrapping_add(1);

                    // Poll disk latency/queue counters in background (~every 5s on Storage tab)
                    if !is_hidden
                        && selected_tab == Tab::Storage
                        && (disk_perf_check_counter.is_multiple_of(10) || data_clone.read().disk_perf.is_empty())
                    {
                        let perf = crate::storage::get_disk_perf();
                        if !perf.is_empty() {
                            data_clone.write().disk_perf = perf;
                        }
                    }
                    disk_perf_check_counter = disk_perf_check_counter.wrapping_add(1);

                    // Poll active socket connections in background (~every 2s on Network tab)
                    if !is_hidden
                        && selected_tab == Tab::Network
                        && (sockets_check_counter.is_multiple_of(4) || data_clone.read().socket_connections.is_empty())
                    {
                        let process_map: std::collections::HashMap<u32, String> = monitor
                            .sys
                            .processes()
                            .iter()
                            .map(|(pid, p)| (pid.as_u32(), p.name().to_string_lossy().into_owned()))
                            .collect();
                        let conns = crate::network::get_active_connections(&process_map);
                        let mut data = data_clone.write();
                        data.socket_connections = conns;
                    }
                    sockets_check_counter = sockets_check_counter.wrapping_add(1);

                    // Poll power plans & battery health in background (~every 10s on SystemInfo tab or startup)
                    if !is_hidden
                        && (selected_tab == Tab::SystemInfo || selected_tab == Tab::Overview)
                        && (power_plans_check_counter.is_multiple_of(20) || data_clone.read().power_plans.is_empty())
                    {
                        let plans = crate::power::get_power_plans();
                        let bat_health = crate::power::get_battery_health();
                        let mut data = data_clone.write();
                        data.power_plans = plans;
                        data.battery_health = bat_health;
                    }
                    power_plans_check_counter = power_plans_check_counter.wrapping_add(1);

                    last_selected_tab = selected_tab;
                    // Calculate total network rates
                    let total_download_rate: f64 = network_info.iter().map(|n| n.received_rate).sum();
                    let total_upload_rate: f64 = network_info.iter().map(|n| n.transmitted_rate).sum();

                    {
                        let mut data = data_clone.write();
                        let elapsed = data.start_time.elapsed().as_secs_f64();

                        // Update current values
                        data.memory_total = total_mem;
                        data.memory_used = used_mem;
                        data.memory_percentage = mem_percentage;
                        data.cpu_usage = cpu_usage;
                        if need_cpu_cores {
                            data.cpu_cores = cpu_cores;
                        }
                        if need_cpu_temp {
                            data.cpu_temperature = cpu_temperature;
                        }
                        if need_gpu_info {
                            data.gpu_info = gpu_info.clone();
                        }
                        if need_processes {
                            data.top_processes = top_processes;
                        }
                        if settings_snapshot.timeline_enabled {
                            data.timeline_processes = timeline_processes;
                        } else {
                            data.timeline_processes.clear();
                        }
                        if need_disks {
                            data.disk_info = disk_info;
                        }
                        if need_network {
                            data.network_info = network_info;
                        }
                        data.system_info = system_info.clone();
                        data.last_update = timestamp;
                        data.swap_info = swap_info;
                        if !is_hidden || settings_snapshot.timeline_enabled {
                            data.disk_read_rate = disk_read_rate;
                            data.disk_write_rate = disk_write_rate;
                        }
                        data.network_sample_count += 1;
                        data.telemetry_history_stats = latest_telemetry.history_stats.clone();
                        data.provider_status = latest_telemetry.provider_status.clone();

                        // Check for alerts
                        let mut new_alerts = SystemMonitor::check_alerts(&settings_snapshot, &data);
                        let active_keys: std::collections::HashSet<String> =
                            data.alerts.iter().map(AlertInfo::key).collect();
                        new_alerts.retain(|alert| !active_keys.contains(&alert.key()));

                        if !new_alerts.is_empty()
                            && settings_snapshot.enable_alert_sound
                            && settings_snapshot.enable_sounds
                        {
                            play_alert_sound();
                        }

                        // Send desktop notifications for new alerts with a 5-minute cooldown
                        if settings_snapshot.show_notifications {
                            for alert in &new_alerts {
                                let now = Instant::now();
                                let should_notify = last_alert_time
                                    .get(&alert.alert_type)
                                    .is_none_or(|&last| now.saturating_duration_since(last).as_secs() > 300);

                                if should_notify {
                                    let _ = notify_rust::Notification::new()
                                        .summary("System Monitor Alert")
                                        .body(&alert.message)
                                        .timeout(notify_rust::Timeout::Milliseconds(5000))
                                        .show();
                                    last_alert_time.insert(alert.alert_type.clone(), now);
                                }
                            }
                        }

                        data.alerts.extend(new_alerts);

                        // Auto-clear resolved alerts
                        if settings_snapshot.auto_clear_alerts {
                            let temp_gpu_info = data.gpu_info.clone();
                            let disk_info = data.disk_info.clone();
                            let high_impact_count = data.high_impact_startup_count;
                            data.alerts.retain(|alert| match alert.alert_type {
                                AlertType::CpuHigh => cpu_usage > settings_snapshot.notification_cpu_threshold,
                                AlertType::MemoryHigh => {
                                    mem_percentage > settings_snapshot.notification_memory_threshold
                                }
                                AlertType::GpuTempHigh => match &alert.source {
                                    AlertSource::Gpu { index, name } => temp_gpu_info.get(*index).is_some_and(|gpu| {
                                        gpu.name == *name
                                            && gpu.temperature.is_some_and(|temperature| {
                                                temperature > settings_snapshot.notification_temp_threshold
                                            })
                                    }),
                                    _ => false,
                                },
                                AlertType::DiskSpaceLow => match &alert.source {
                                    AlertSource::Disk { mount_point, .. } => disk_info.iter().any(|disk| {
                                        disk.mount_point == *mount_point
                                            && disk.usage_percentage > settings_snapshot.notification_disk_threshold
                                    }),
                                    _ => false,
                                },
                                AlertType::StartupHighImpact => high_impact_count > 0,
                            });
                        }

                        // Keep only last 10 alerts
                        while data.alerts.len() > 10 {
                            data.alerts.remove(0);
                        }
                        // Update history (keep last 60 data points)
                        data.cpu_history.push(DataPoint {
                            time: elapsed,
                            value: cpu_usage as f64,
                        });
                        data.memory_history.push(DataPoint {
                            time: elapsed,
                            value: mem_percentage as f64,
                        });

                        if need_gpu_info {
                            let gpu_util = data.gpu_info.first().map(|gpu| gpu.utilization as f64);
                            if let Some(val) = gpu_util {
                                data.gpu_history.push(DataPoint {
                                    time: elapsed,
                                    value: val,
                                });
                            }
                        }

                        // Network history — skip first sample (inflated rates)
                        if need_network && data.network_sample_count > 1 {
                            data.network_download_history.push_back(DataPoint {
                                time: elapsed,
                                value: total_download_rate,
                            });
                            data.network_upload_history.push_back(DataPoint {
                                time: elapsed,
                                value: total_upload_rate,
                            });
                        }
                        if !is_hidden && data.network_sample_count > 1 {
                            data.disk_read_history.push_back(DataPoint {
                                time: elapsed,
                                value: disk_read_rate,
                            });
                            data.disk_write_history.push_back(DataPoint {
                                time: elapsed,
                                value: disk_write_rate,
                            });
                        }

                        // cpu_history capped at 60 by BoundedHistory; trim the rest
                        while data.network_download_history.len() > 60 {
                            data.network_download_history.pop_front();
                        }
                        while data.network_upload_history.len() > 60 {
                            data.network_upload_history.pop_front();
                        }
                        while data.disk_read_history.len() > 60 {
                            data.disk_read_history.pop_front();
                        }
                        while data.disk_write_history.len() > 60 {
                            data.disk_write_history.pop_front();
                        }
                    }

                    let snapshot = crate::snapshot_from_data(&data_clone.read());
                    let _ = monitoring_events.send(crate::app::events::AppEvent::Snapshot(Box::new(snapshot)));

                    // Process details for the selected row (recompute only when selection changed)
                    let selected_pid = {
                        let d = data_clone.read();
                        d.selected_process_pid
                    };
                    if let Some(pid) = selected_pid {
                        let cached = {
                            let d = data_clone.read();
                            d.selected_process_details.as_ref().map(|(p, _)| *p)
                        };
                        if cached != Some(pid)
                            && let Some(details) = processes::lookup_details(&monitor.sys, pid)
                        {
                            let mut d = data_clone.write();
                            d.selected_process_details = Some((pid, details));
                        }
                    }

                    if is_hidden {
                        // Minimized: sleep 10s
                        thread::sleep(Duration::from_millis(10000));
                    } else {
                        let sleep_ms = (refresh_interval * 1000).saturating_sub(500);
                        thread::sleep(Duration::from_millis(sleep_ms));
                    }
                }
            })
            .expect("failed to spawn monitoring thread");

        let mut tray_icon = None;
        let mut tray_menu_show_id = None;
        let mut tray_menu_quit_id = None;
        let mut tray_menu_clean_id = None;
        let mut tray_menu_procman_id = None;
        let mut tray_menu_pause_id = None;
        let mut tray_menu_pause_item = None;
        let mut tray_menu_handle = None;
        let mut tray_menu_power_item = None;
        let mut tray_menu_power_items: std::collections::HashMap<_, _> = Default::default();
        let mut tray_menu_power_guids: std::collections::HashMap<_, _> = Default::default();

        let mut _hotkey_manager = None;
        let mut clean_ram_hotkey = None;
        #[cfg(target_os = "windows")]
        {
            if let Ok(manager) = global_hotkey::GlobalHotKeyManager::new() {
                let hotkey = global_hotkey::hotkey::HotKey::new(
                    Some(global_hotkey::hotkey::Modifiers::CONTROL | global_hotkey::hotkey::Modifiers::ALT),
                    global_hotkey::hotkey::Code::KeyC,
                );
                if manager.register(hotkey).is_ok() {
                    _hotkey_manager = Some(manager);
                    clean_ram_hotkey = Some(hotkey);
                }
            }
        }

        #[cfg(target_os = "windows")]
        if let Some(icon) = crate::load_tray_icon() {
            let tray_menu = Menu::new();
            let show_i = MenuItem::new("Show Dashboard", true, None);
            let clean_i = MenuItem::new("Clean RAM Now", true, None);
            let procman_i = MenuItem::new("Open Process Manager", true, None);
            let pause_i = CheckMenuItem::new("Pause Monitoring", true, false, None);
            let quit_i = MenuItem::new("Quit System Monitor", true, None);

            tray_menu_show_id = Some(show_i.id().clone());
            tray_menu_clean_id = Some(clean_i.id().clone());
            tray_menu_procman_id = Some(procman_i.id().clone());
            tray_menu_pause_id = Some(pause_i.id().clone());
            tray_menu_quit_id = Some(quit_i.id().clone());
            let pause_item = pause_i.clone();
            let menu_handle = tray_menu.clone();

            let power_plans = power::get_power_plans();
            if !power_plans.is_empty() {
                let mut owned_power_items: Vec<CheckMenuItem> = Vec::new();
                for plan in &power_plans {
                    let item = CheckMenuItem::new(plan.name.clone(), true, plan.is_active, None);
                    tray_menu_power_guids.insert(item.id().clone(), plan.guid.clone());
                    owned_power_items.push(item);
                }
                let power_submenu = {
                    let refs: Vec<&dyn tray_icon::menu::IsMenuItem> = owned_power_items
                        .iter()
                        .map(|item| item as &dyn tray_icon::menu::IsMenuItem)
                        .collect();
                    Submenu::with_items("Power Plan", true, &refs).expect("failed to build power plan submenu")
                };
                tray_menu_power_item = Some(power_submenu);
                for item in owned_power_items {
                    tray_menu_power_items.insert(item.id().clone(), item);
                }
                let _ = tray_menu.append(tray_menu_power_item.as_ref().expect("power submenu built"));
            }

            let _ = tray_menu.append_items(&[&show_i, &clean_i, &procman_i, &pause_i, &quit_i]);

            if let Ok(tray) = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("System Monitor")
                .with_icon(icon)
                .build()
            {
                tray_icon = Some(tray);
            }
            tray_menu_pause_item = Some(pause_item);
            tray_menu_handle = Some(menu_handle);
        }

        let startup_items_share = Arc::new(Mutex::new(None));
        let boot_diagnostics_share = Arc::new(Mutex::new(None));

        let startup_share_clone = Arc::clone(&startup_items_share);
        let boot_share_clone = Arc::clone(&boot_diagnostics_share);
        let ctx_clone = cc.egui_ctx.clone();
        std::thread::Builder::new()
            .name("startup_loader".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(crate::startup::get_startup_data));
                match result {
                    Ok((items, diag)) => {
                        *startup_share_clone.lock() = Some(items);
                        *boot_share_clone.lock() = diag;
                    }
                    Err(_) => {
                        *startup_share_clone.lock() = Some(Vec::new());
                    }
                }
                ctx_clone.request_repaint();
            })
            .ok();

        Self {
            app_channels,
            latest_snapshot: None,
            action_pending: false,
            action_status: None,
            pending_action_plan: None,
            action_history: crate::persistence::action_log::load_recent(100)
                .into_iter()
                .map(|record| {
                    let undo = record
                        .quarantine_id
                        .as_ref()
                        .filter(|quarantine_id| crate::startup::quarantine_exists(quarantine_id))
                        .map(|quarantine_id| crate::app::commands::ActionCommand::RestoreStartup {
                            item_name: record.action.clone(),
                            quarantine_id: quarantine_id.clone(),
                        });
                    crate::app::actions::ActionHistoryEntry { record, undo }
                })
                .collect(),
            show_action_history: false,
            session_recorder: crate::persistence::session::SessionRecorder::default(),
            session_status: None,
            timeline,
            timeline_ui: crate::timeline::TimelineUiState::default(),
            telemetry_commands,
            data,
            settings: settings.clone(),
            shared_settings,
            selected_tab: Tab::Overview,
            show_settings: false,
            show_export: false,
            show_alerts: false,
            show_process_manager: false,
            selected_process_pid: None,
            details_pid: None,
            kill_tree_pid: None,
            process_search: String::new(),
            process_sort_column: crate::processes::ProcessSortColumn::Memory,
            process_sort_ascending: false,
            show_export_csv: false,
            updater: crate::updater::Updater::new(),
            update_info_share: Arc::new(Mutex::new(None)),
            show_update_notification: true,
            update_check_time: None,
            update_downloading: false,
            update_error: None,
            update_result_share: Arc::new(Mutex::new(None)),
            ram_cleaner_state: RamCleanerState {
                last_cleaned: None,
                last_cleaned_display: String::new(),
                bytes_freed: 0,
                auto_clean_enabled: settings.auto_ram_clean,
                auto_clean_threshold: settings.ram_clean_threshold,
                auto_clean_interval: settings.auto_clean_interval,
                auto_clean_target: settings.auto_clean_target,
                auto_clean_exclusions: settings.auto_clean_exclusions.clone(),
                auto_clean_idle_only: settings.auto_clean_idle_only,
                auto_clean_smart_only: settings.auto_clean_smart_only,
                auto_clean_notify: settings.auto_clean_notify,
                auto_clean_max_mb: settings.auto_clean_max_mb,
                is_cleaning: false,
                clean_count: 0,
            },
            startup_items: Vec::new(),
            startup_items_loaded: false,
            startup_items_loading: true,
            startup_items_share,
            startup_search: String::new(),
            startup_sort: crate::startup::StartupSortColumn::Impact,
            startup_sort_ascending: true,
            startup_filter_impact: None,
            startup_filter_signed: None,
            startup_filter_broken: false,
            startup_show_confirm: None,
            boot_diagnostics: None,
            boot_diagnostics_loaded: false,
            boot_diagnostics_share,
            show_shortcuts: false,
            suspend_process_pid: None,
            resume_process_pid: None,
            suspended_pids: std::collections::HashSet::new(),
            priority_change: None,
            process_tree_view: false,
            affinity_change: None,
            network_socket_search: String::new(),
            service_page: crate::app::page_state::ServicePageState::default(),
            storage_page: crate::app::page_state::StoragePageState::default(),
            crash_reports: None,
            window_picker_active: false,
            #[cfg(target_os = "windows")]
            tray_icon,
            #[cfg(target_os = "windows")]
            tray_menu_show_id,
            #[cfg(target_os = "windows")]
            tray_menu_quit_id,
            #[cfg(target_os = "windows")]
            tray_menu_clean_id,
            #[cfg(target_os = "windows")]
            tray_menu_procman_id,
            #[cfg(target_os = "windows")]
            tray_menu_pause_id,
            #[cfg(target_os = "windows")]
            tray_menu_pause_item,
            #[cfg(target_os = "windows")]
            tray_menu_handle,
            #[cfg(target_os = "windows")]
            tray_menu_power_item,
            #[cfg(target_os = "windows")]
            tray_menu_power_items,
            #[cfg(target_os = "windows")]
            tray_menu_power_guids,
            #[cfg(target_os = "windows")]
            _hotkey_manager,
            #[cfg(target_os = "windows")]
            clean_ram_hotkey,
            is_hidden: false,
            widget_open: settings.show_widget,
            start_minimized_applied: false,
        }
    }
    #[cfg(test)]
    pub(crate) fn test_app() -> Self {
        let settings = AppSettings::default();
        let data = Arc::new(parking_lot::RwLock::new(SystemData::default()));
        let shared_settings = Arc::new(Mutex::new(settings.clone()));
        let app_channels = crate::app::AppChannels::new();
        let (telemetry_commands, _) = std::sync::mpsc::sync_channel(16);
        let timeline = crate::timeline::TimelineHandle::start(false, 7);

        Self {
            app_channels,
            latest_snapshot: None,
            action_pending: false,
            action_status: None,
            pending_action_plan: None,
            action_history: Vec::new(),
            show_action_history: false,
            session_recorder: crate::persistence::session::SessionRecorder::default(),
            session_status: None,
            timeline,
            timeline_ui: crate::timeline::TimelineUiState::default(),
            telemetry_commands,
            data,
            settings: settings.clone(),
            shared_settings,
            selected_tab: Tab::Overview,
            show_settings: false,
            show_export: false,
            show_alerts: false,
            show_process_manager: false,
            selected_process_pid: None,
            details_pid: None,
            kill_tree_pid: None,
            process_search: String::new(),
            process_sort_column: crate::processes::ProcessSortColumn::Memory,
            process_sort_ascending: false,
            show_export_csv: false,
            updater: crate::updater::Updater::new(),
            update_info_share: Arc::new(Mutex::new(None)),
            show_update_notification: false,
            update_check_time: None,
            update_downloading: false,
            update_error: None,
            update_result_share: Arc::new(Mutex::new(None)),
            ram_cleaner_state: RamCleanerState {
                last_cleaned: None,
                last_cleaned_display: String::new(),
                bytes_freed: 0,
                auto_clean_enabled: false,
                auto_clean_threshold: 80.0,
                auto_clean_interval: 60,
                auto_clean_target: 60.0,
                auto_clean_exclusions: Vec::new(),
                auto_clean_idle_only: false,
                auto_clean_smart_only: true,
                auto_clean_notify: false,
                auto_clean_max_mb: 0,
                is_cleaning: false,
                clean_count: 0,
            },
            startup_items: Vec::new(),
            startup_items_loaded: false,
            startup_items_loading: false,
            startup_items_share: Arc::new(Mutex::new(None)),
            startup_search: String::new(),
            startup_sort: crate::startup::StartupSortColumn::Impact,
            startup_sort_ascending: true,
            startup_filter_impact: None,
            startup_filter_signed: None,
            startup_filter_broken: false,
            startup_show_confirm: None,
            boot_diagnostics: None,
            boot_diagnostics_loaded: false,
            boot_diagnostics_share: Arc::new(Mutex::new(None)),
            show_shortcuts: false,
            suspend_process_pid: None,
            resume_process_pid: None,
            suspended_pids: std::collections::HashSet::new(),
            priority_change: None,
            process_tree_view: false,
            affinity_change: None,
            network_socket_search: String::new(),
            service_page: crate::app::page_state::ServicePageState::default(),
            storage_page: crate::app::page_state::StoragePageState::default(),
            crash_reports: None,
            window_picker_active: false,
            #[cfg(target_os = "windows")]
            tray_icon: None,
            #[cfg(target_os = "windows")]
            tray_menu_show_id: None,
            #[cfg(target_os = "windows")]
            tray_menu_quit_id: None,
            #[cfg(target_os = "windows")]
            tray_menu_clean_id: None,
            #[cfg(target_os = "windows")]
            tray_menu_procman_id: None,
            #[cfg(target_os = "windows")]
            tray_menu_pause_id: None,
            #[cfg(target_os = "windows")]
            tray_menu_pause_item: None,
            #[cfg(target_os = "windows")]
            tray_menu_handle: None,
            #[cfg(target_os = "windows")]
            tray_menu_power_item: None,
            #[cfg(target_os = "windows")]
            tray_menu_power_items: std::collections::HashMap::new(),
            #[cfg(target_os = "windows")]
            tray_menu_power_guids: std::collections::HashMap::new(),
            #[cfg(target_os = "windows")]
            _hotkey_manager: None,
            #[cfg(target_os = "windows")]
            clean_ram_hotkey: None,
            is_hidden: false,
            widget_open: false,
            start_minimized_applied: true,
        }
    }
}
