//! Desktop floating mini-widget telemetry HUD.

use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::{SystemData, SystemMonitorApp};
use eframe::egui;

/// Render only the HUD in its own native window, never the full application shell.
/// The HUD has no minimize button: it stays visible until explicitly closed.
pub(crate) fn show_hud(app: &mut SystemMonitorApp, ctx: &egui::Context, data: &SystemData) {
    if !app.widget_open {
        return;
    }
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("sysmon.desktop_hud"),
        egui::ViewportBuilder::default()
            .with_title("SysMon HUD")
            .with_inner_size([272.0, 320.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_minimize_button(false)
            .with_always_on_top()
            .with_active(false)
            .with_taskbar(true),
        |ui, class| {
            // Embedded/headless backends already supply a Window and its Ui.
            // Do not enter run_ui or invoke the shell recursively.
            if class != egui::ViewportClass::EmbeddedWindow {
                if ui.ctx().input(|input| input.viewport().close_requested()) {
                    close_hud(app);
                    return;
                }
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            }
            egui::Frame::new()
                .fill(ui.visuals().panel_fill)
                .inner_margin(10.0)
                .show(ui, |ui| render_hud(app, ui, data));
            ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
        },
    );
}

fn close_hud(app: &mut SystemMonitorApp) {
    app.settings.show_widget = false;
    crate::ui::pages::settings::commit_settings(app);
}

fn metric_fresh(data: &SystemData, key: &str, max_age: std::time::Duration) -> bool {
    !data.monitoring_paused
        && data.metric_status.get(key).is_some_and(|status| {
            status.is_fresh()
                && status
                    .observed_at
                    .and_then(|time| time.elapsed().ok())
                    .is_some_and(|age| age <= max_age)
        })
}

pub(crate) fn render_hud(app: &mut SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    let max_age = std::time::Duration::from_secs(app.settings.refresh_interval.saturating_mul(3).max(15));
    let fresh = |key: &str| metric_fresh(data, key, max_age);
    ui.spacing_mut().item_spacing = egui::vec2(6.0, 5.0);
    ui.set_width(240.0);

    // Header
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("◰ HUD TELEMETRY")
                .size(11.0)
                .monospace()
                .strong()
                .color(ThemePalette::ACCENT_PRIMARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("✕").on_hover_text("Close HUD (Ctrl+M)").clicked() {
                close_hud(app);
            }
        });
    });

    ui.add_space(2.0);

    if fresh("cpu.global_usage") {
        // CPU Metric
        let cpu_color = get_usage_color(data.cpu_usage);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("CPU")
                    .size(11.0)
                    .monospace()
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(temp) = data.cpu_temperature {
                    ui.label(
                        egui::RichText::new(format!("{temp:.0}°C"))
                            .size(10.5)
                            .monospace()
                            .color(ThemePalette::text_dimmed(is_dark)),
                    );
                    ui.add_space(4.0);
                }
                ui.label(
                    egui::RichText::new(format!("{:.1}%", data.cpu_usage))
                        .size(11.5)
                        .monospace()
                        .strong()
                        .color(cpu_color),
                );
            });
        });
        paint_progress_bar(ui, data.cpu_usage / 100.0, cpu_color, 4.0, is_dark);
    } else {
        ui.label("CPU  N/A");
    }

    // RAM Metric
    if fresh("memory.used") {
        let mem_color = get_usage_color(data.memory_percentage);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("RAM")
                    .size(11.0)
                    .monospace()
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let used_gb = data.memory_used as f64 / 1024.0 / 1024.0 / 1024.0;
                let total_gb = data.memory_total as f64 / 1024.0 / 1024.0 / 1024.0;
                ui.label(
                    egui::RichText::new(format!("{used_gb:.1}/{total_gb:.1}G"))
                        .size(10.5)
                        .monospace()
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("{:.1}%", data.memory_percentage))
                        .size(11.5)
                        .monospace()
                        .strong()
                        .color(mem_color),
                );
            });
        });
        paint_progress_bar(ui, data.memory_percentage / 100.0, mem_color, 4.0, is_dark);
    } else {
        ui.label("RAM  N/A");
    }

    // GPU Metric (if available)
    if app.settings.show_gpu {
        if let Some(gpu) = data.gpu_info.first() {
            let usage = gpu.utilization.filter(|_| fresh("gpu"));
            let gpu_color = usage.map(get_usage_color).unwrap_or(ThemePalette::text_dimmed(is_dark));
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("GPU")
                        .size(11.0)
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(temp) = gpu.temperature.filter(|_| fresh("gpu")) {
                        ui.label(
                            egui::RichText::new(format!("{temp:.0}°C"))
                                .size(10.5)
                                .monospace()
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                        ui.add_space(4.0);
                    }
                    ui.label(
                        egui::RichText::new(
                            usage
                                .map(|value| format!("{value:.1}%"))
                                .unwrap_or_else(|| "N/A".into()),
                        )
                        .size(11.5)
                        .monospace()
                        .strong()
                        .color(gpu_color),
                    );
                });
            });
            if let Some(usage) = usage {
                paint_progress_bar(ui, usage / 100.0, gpu_color, 4.0, is_dark);
            }
        } else {
            ui.label("GPU  N/A");
        }
    }

    // Network I/O
    let network_fresh = fresh("network") && !data.network_info.is_empty();
    let dl = network_fresh.then(|| data.network_info.iter().map(|n| n.received_rate).sum::<f64>());
    let ul = network_fresh.then(|| data.network_info.iter().map(|n| n.transmitted_rate).sum::<f64>());
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "Down {}",
                dl.map(rates_to_human).unwrap_or_else(|| "N/A".into())
            ))
            .size(10.5)
            .monospace()
            .color(ThemePalette::ACCENT_PRIMARY),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!("Up {}", ul.map(rates_to_human).unwrap_or_else(|| "N/A".into())))
                .size(10.5)
                .monospace()
                .color(ThemePalette::ACCENT_ACTIVE),
        );
    });

    ui.horizontal(|ui| {
        let available = fresh("disk");
        let read = if available {
            rates_to_human(data.disk_read_rate)
        } else {
            "N/A".into()
        };
        let write = if available {
            rates_to_human(data.disk_write_rate)
        } else {
            "N/A".into()
        };
        ui.label(
            egui::RichText::new(format!("Disk R {read}  W {write}"))
                .size(10.5)
                .monospace(),
        );
    });
    let age = (data.sampled_at != std::time::SystemTime::UNIX_EPOCH)
        .then(|| data.sampled_at.elapsed().ok().map(|elapsed| elapsed.as_secs()))
        .flatten();
    let state = if data.monitoring_paused {
        "Paused"
    } else if fresh("cpu.global_usage") && fresh("memory.used") {
        "Live"
    } else {
        "Stale / unavailable"
    };
    ui.label(
        egui::RichText::new(format!(
            "{state} · sample age {}",
            age.map(|seconds| format!("{seconds}s"))
                .unwrap_or_else(|| "unknown".into())
        ))
        .size(10.5)
        .monospace(),
    );

    ui.separator();

    // Footer Actions
    ui.horizontal(|ui| {
        let is_cleaning = app.ram_cleaner_state.is_cleaning;
        let clean_label = if is_cleaning { "..." } else { "⚡ Clean RAM" };
        if ui
            .add_enabled(
                !is_cleaning,
                egui::Button::new(egui::RichText::new(clean_label).size(10.5)),
            )
            .clicked()
        {
            app.queue_action(crate::app::commands::ActionCommand::CleanRam);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(&data.last_update)
                    .size(10.0)
                    .monospace()
                    .color(ThemePalette::text_dimmed(is_dark)),
            );
        });
    });
}
