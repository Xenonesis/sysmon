#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
pub(crate) mod app;
mod app_paths;
mod app_shell;
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
mod timeline;
pub(crate) mod ui;
mod updater;
use eframe::egui;

use tracing::{error, info};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) use crate::app::models::*;
pub(crate) use crate::monitoring::engine::*;
#[cfg(target_os = "windows")]
impl SystemMonitorApp {
    fn handle_ui_intent(&mut self, intent: app::commands::UiIntent) {
        match intent {
            app::commands::UiIntent::OpenServicesConsole => {
                #[cfg(target_os = "windows")]
                if let Err(error) = std::process::Command::new("cmd")
                    .args(["/c", "start", "services.msc"])
                    .spawn()
                {
                    self.action_status = Some(format!("Could not open services.msc: {error}"));
                }
            }
            app::commands::UiIntent::RelaunchAsAdmin => {
                match crate::privilege::relaunch_as_admin() {
                    Ok(crate::privilege::ElevationOutcome::Ready) => {
                        // The elevated successor is running and confirmed readiness;
                        // this instance must yield so exactly one UI remains.
                        self.quit_requested = true;
                    }
                    Ok(crate::privilege::ElevationOutcome::Canceled) => {
                        self.action_status =
                            Some("Elevation canceled; continuing without administrator privileges".to_string());
                    }
                    Err(error) => {
                        self.action_status = Some(format!("Could not request administrator privileges: {error}"));
                    }
                }
            }
            app::commands::UiIntent::ControlService { name, action } => {
                self.queue_action(app::commands::ActionCommand::ControlService { name, action });
            }
            app::commands::UiIntent::CheckUpdates => {
                self.update_check_time = None;
            }
        }
    }
}

impl eframe::App for SystemMonitorApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::app_shell::logic_shell(self, ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::app_shell::ui_shell(self, ui);
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

    #[test]
    fn service_control_intent_keeps_confirmation_boundary() {
        let mut app = SystemMonitorApp::test_app();
        app.handle_ui_intent(app::commands::UiIntent::ControlService {
            name: "BITS".to_string(),
            action: services::ServiceControlAction::Stop,
        });

        let plan = app
            .pending_action_plan
            .as_ref()
            .expect("service intent should create an action plan");
        assert_eq!(plan.title, "Stop service BITS");
        assert!(plan.requires_admin);
        assert!(plan.reversible);
        assert!(matches!(
            &plan.command,
            app::commands::ActionCommand::ControlService {
                name,
                action: services::ServiceControlAction::Stop,
            } if name == "BITS"
        ));
    }

    #[test]
    fn test_window_picker_navigation_and_filtering() {
        let mut app = SystemMonitorApp::test_app();
        assert!(!app.window_picker_active);
        assert_ne!(app.selected_tab, Tab::Processes);

        // Simulate picker activation
        app.window_picker_active = true;

        // When a PID is resolved from screen coordinates:
        let test_pid = 4242;
        app.selected_tab = Tab::Processes;
        app.details_pid = None;
        app.process_search = test_pid.to_string();
        app.window_picker_active = false;

        assert_eq!(app.selected_tab, Tab::Processes);
        assert!(app.details_pid.is_none());
    }
}
fn main() {
    // ── 1. Single-Instance Enforcement & Privileged Handoff ────────────
    // A named mutex blocks duplicate instances; the same check also consumes
    // the authenticated handoff used by elevation and update successors.
    if let Err(error) = privilege::initialize_instance() {
        use windows::Win32::UI::WindowsAndMessaging::{MB_ICONINFORMATION, MB_OK, MessageBoxW};
        use windows::core::PCWSTR;

        let title: Vec<u16> = "System Monitor\0".encode_utf16().collect();
        let msg: Vec<u16> = format!("{error}\0").encode_utf16().collect();
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

    // ── 2. Crash Report Directory ───────────────────────────────────────
    let log_dir = crate::app_paths::data_local_dir().unwrap_or_else(|| std::env::temp_dir().join("SystemMonitor"));
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
            use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
            use windows::core::PCWSTR;

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

    // ── 4b. Privileged Handoff Startup Task ─────────────────────────────
    // A verified update helper performs the install here and exits; a resumed
    // GUI instance stores its install outcome for the first logic tick.
    match updater::run_startup_task() {
        Ok(true) => {
            info!("Update helper completed its startup task; exiting");
            std::process::exit(0);
        }
        Ok(false) => {}
        Err(error) => {
            error!("Startup task failed: {error}");
            #[cfg(target_os = "windows")]
            {
                use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
                use windows::core::PCWSTR;
                let title: Vec<u16> = "System Monitor — Startup Error\0".encode_utf16().collect();
                let msg: Vec<u16> = format!("The update helper could not complete.\n\n{error}\0")
                    .encode_utf16()
                    .collect();
                unsafe {
                    MessageBoxW(None, PCWSTR(msg.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR);
                }
            }
            std::process::exit(1);
        }
    }
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
                use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
                use windows::core::PCWSTR;

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
