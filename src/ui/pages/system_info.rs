use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "System Information & Hardware Specs", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── 1. Operating System & Platform ──
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("OPERATING SYSTEM & PLATFORM")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(8.0);

            egui::Grid::new("sysinfo_os_grid")
                .num_columns(4)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("OS Name:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(&data.system_info.os_name)
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.label(
                        egui::RichText::new("OS Version:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(&data.system_info.os_version)
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Kernel Version:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(&data.system_info.kernel_version)
                            .monospace()
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.label(
                        egui::RichText::new("Hostname:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(&data.system_info.hostname)
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.end_row();

                    let days = data.system_info.uptime / 86400;
                    let hours = (data.system_info.uptime % 86400) / 3600;
                    let minutes = (data.system_info.uptime % 3600) / 60;

                    ui.label(
                        egui::RichText::new("System Uptime:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{}d {}h {}m", days, hours, minutes))
                            .monospace()
                            .strong()
                            .color(ThemePalette::STATUS_HEALTHY),
                    );

                    if let Some(build) = &data.system_info.os_build {
                        ui.label(
                            egui::RichText::new("OS Build:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(build)
                                .monospace()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                    } else {
                        ui.label("");
                        ui.label("");
                    }
                    ui.end_row();

                    if let Some(mb) = &data.system_info.motherboard {
                        ui.label(
                            egui::RichText::new("Motherboard:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(mb)
                                .monospace()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                    }
                    if let Some(bios) = &data.system_info.bios_version {
                        ui.label(
                            egui::RichText::new("BIOS Version:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(bios)
                                .monospace()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();
                    } else if data.system_info.motherboard.is_some() {
                        ui.end_row();
                    }
                });
        });

        ui.add_space(10.0);

        // ── Power Management & Battery Diagnostics ──
        let battery = crate::power::get_battery_health();
        let power_plans = crate::power::get_power_plans();

        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("POWER SCHEMES & BATTERY HEALTH")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if battery.has_battery {
                        let batt_color = if battery.is_charging {
                            ThemePalette::STATUS_HEALTHY
                        } else if battery.percentage < 20.0 {
                            ThemePalette::STATUS_CRITICAL
                        } else {
                            ThemePalette::ACCENT_PRIMARY
                        };
                        let charge_str = if battery.is_charging {
                            "⚡ Charging"
                        } else if battery.ac_online {
                            "🔌 AC Online"
                        } else {
                            "🔋 On Battery"
                        };
                        status_pill(
                            ui,
                            &format!("{charge_str} · {:.0}%", battery.percentage),
                            batt_color,
                            is_dark,
                        );
                    } else {
                        status_pill(ui, "DESKTOP / AC POWER", ThemePalette::STATUS_HEALTHY, is_dark);
                    }
                });
            });

            ui.add_space(8.0);

            // Active Power Plan Switcher
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Windows Power Scheme:")
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                for plan in &power_plans {
                    let is_active = plan.is_active;
                    let btn = egui::Button::new(
                        egui::RichText::new(if is_active {
                            format!("✓ {}", plan.name)
                        } else {
                            plan.name.clone()
                        })
                        .size(11.0)
                        .strong()
                        .color(if is_active {
                            ThemePalette::STATUS_HEALTHY
                        } else {
                            ThemePalette::text_secondary(is_dark)
                        }),
                    )
                    .fill(if is_active {
                        ThemePalette::STATUS_HEALTHY.gamma_multiply(if is_dark { 0.15 } else { 0.10 })
                    } else {
                        ThemePalette::bg_track(is_dark)
                    })
                    .stroke(egui::Stroke::new(
                        1.0,
                        if is_active {
                            ThemePalette::STATUS_HEALTHY.gamma_multiply(0.4)
                        } else {
                            ThemePalette::border(is_dark)
                        },
                    ))
                    .rounding(egui::Rounding::same(4.0));

                    if ui
                        .add(btn)
                        .on_hover_text(format!("Switch to {} scheme", plan.name))
                        .clicked()
                        && !is_active
                    {
                        app.queue_action(crate::app::commands::ActionCommand::SetPowerPlan(plan.guid.clone()));
                    }
                }
            });
        });

        ui.add_space(10.0);

        // ── 2. Processor (CPU) ──
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("PROCESSOR (CPU)")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(8.0);

            egui::Grid::new("sysinfo_cpu_grid")
                .num_columns(4)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("CPU Model:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(&data.system_info.cpu_brand)
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.label(
                        egui::RichText::new("Core Count:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} logical cores ({} physical)",
                            data.cpu_cores.len(),
                            data.system_info.cpu_count
                        ))
                        .monospace()
                        .strong()
                        .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Global Utilization:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    let cpu_color = get_usage_color(data.cpu_usage);
                    ui.label(
                        egui::RichText::new(format!("{:.1}%", data.cpu_usage))
                            .monospace()
                            .strong()
                            .color(cpu_color),
                    );

                    ui.label(
                        egui::RichText::new("CPU Temperature:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    if let Some(temp) = data.cpu_temperature {
                        let temp_color = if temp < 70.0 {
                            ThemePalette::STATUS_HEALTHY
                        } else if temp < 85.0 {
                            ThemePalette::STATUS_WARNING
                        } else {
                            ThemePalette::STATUS_CRITICAL
                        };
                        ui.label(
                            egui::RichText::new(format!("{:.1} °C", temp))
                                .monospace()
                                .strong()
                                .color(temp_color),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("N/A")
                                .monospace()
                                .color(ThemePalette::text_dimmed(is_dark)),
                        );
                    }
                    ui.end_row();
                });

            ui.add_space(6.0);
            paint_progress_bar(
                ui,
                data.cpu_usage / 100.0,
                get_usage_color(data.cpu_usage),
                6.0,
                is_dark,
            );
        });

        ui.add_space(10.0);

        // ── 3. Memory & Virtual Memory (RAM / Page File) ──
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("SYSTEM MEMORY & PAGE FILE")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add_space(8.0);

            egui::Grid::new("sysinfo_mem_grid")
                .num_columns(4)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Total Physical RAM:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.2} GB", bytes_to_gb(data.memory_total)))
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.label(
                        egui::RichText::new("Used RAM:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.2} GB", bytes_to_gb(data.memory_used)))
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Available RAM:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.2} GB",
                            bytes_to_gb(data.memory_total.saturating_sub(data.memory_used))
                        ))
                        .monospace()
                        .strong()
                        .color(ThemePalette::STATUS_HEALTHY),
                    );

                    ui.label(
                        egui::RichText::new("RAM Utilization:")
                            .size(11.5)
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    let mem_color = get_usage_color(data.memory_percentage);
                    ui.label(
                        egui::RichText::new(format!("{:.1}%", data.memory_percentage))
                            .monospace()
                            .strong()
                            .color(mem_color),
                    );
                    ui.end_row();
                });

            ui.add_space(6.0);
            paint_progress_bar(
                ui,
                data.memory_percentage / 100.0,
                get_usage_color(data.memory_percentage),
                6.0,
                is_dark,
            );

            if data.swap_info.total > 0 {
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                egui::Grid::new("sysinfo_swap_grid")
                    .num_columns(4)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Total Page File:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2} GB", bytes_to_gb(data.swap_info.total)))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );

                        ui.label(
                            egui::RichText::new("Used Page File:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2} GB", bytes_to_gb(data.swap_info.used)))
                                .monospace()
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.end_row();

                        ui.label(
                            egui::RichText::new("Page File Load:")
                                .size(11.5)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        let swap_color = get_usage_color(data.swap_info.percentage);
                        ui.label(
                            egui::RichText::new(format!("{:.1}%", data.swap_info.percentage))
                                .monospace()
                                .strong()
                                .color(swap_color),
                        );
                        ui.end_row();
                    });

                ui.add_space(4.0);
                paint_progress_bar(
                    ui,
                    data.swap_info.percentage / 100.0,
                    get_usage_color(data.swap_info.percentage),
                    4.0,
                    is_dark,
                );
            }
        });

        ui.add_space(10.0);

        // ── 4. Graphics Hardware (GPU) ──
        if data.gpu_info.is_empty() {
            card_frame(is_dark).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("GRAPHICS PROCESSING UNIT (GPU)")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("No dedicated GPU detected via NVML or generic graphics adapter.")
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
            });
        } else {
            for (idx, gpu_info) in data.gpu_info.iter().enumerate() {
                card_frame(is_dark).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("GPU #{}: {}", idx, gpu_info.name))
                                .size(13.0)
                                .strong()
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let gpu_color = get_usage_color(gpu_info.utilization);
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", gpu_info.utilization))
                                    .monospace()
                                    .strong()
                                    .color(gpu_color),
                            );
                            status_pill(
                                ui,
                                if gpu_info.utilization >= 90.0 {
                                    "HIGH LOAD"
                                } else {
                                    "ONLINE"
                                },
                                gpu_color,
                                is_dark,
                            );
                        });
                    });

                    ui.add_space(8.0);

                    egui::Grid::new(format!("sysinfo_gpu_grid_{}", idx))
                        .num_columns(4)
                        .spacing([24.0, 6.0])
                        .show(ui, |ui| {
                            if let (Some(used), Some(total)) = (gpu_info.memory_used, gpu_info.memory_total) {
                                ui.label(
                                    egui::RichText::new("VRAM Usage:")
                                        .size(11.5)
                                        .color(ThemePalette::text_secondary(is_dark)),
                                );
                                let used_mb = bytes_to_mb(used);
                                let total_mb = bytes_to_mb(total);
                                let vram_str = if total_mb >= 1024.0 {
                                    format!("{:.2} / {:.2} GB", used_mb / 1024.0, total_mb / 1024.0)
                                } else {
                                    format!("{:.0} / {:.0} MB", used_mb, total_mb)
                                };
                                ui.label(
                                    egui::RichText::new(vram_str)
                                        .monospace()
                                        .strong()
                                        .color(ThemePalette::text_primary(is_dark)),
                                );
                            }

                            if let Some(temp) = gpu_info.temperature {
                                ui.label(
                                    egui::RichText::new("GPU Temperature:")
                                        .size(11.5)
                                        .color(ThemePalette::text_secondary(is_dark)),
                                );
                                let temp_color = if temp < 70 {
                                    ThemePalette::STATUS_HEALTHY
                                } else if temp < 85 {
                                    ThemePalette::STATUS_WARNING
                                } else {
                                    ThemePalette::STATUS_CRITICAL
                                };
                                ui.label(
                                    egui::RichText::new(format!("{} °C", temp))
                                        .monospace()
                                        .strong()
                                        .color(temp_color),
                                );
                            }
                            ui.end_row();

                            if let Some(drv) = &data.system_info.gpu_driver {
                                ui.label(
                                    egui::RichText::new("Driver Version:")
                                        .size(11.5)
                                        .color(ThemePalette::text_secondary(is_dark)),
                                );
                                ui.label(
                                    egui::RichText::new(drv)
                                        .monospace()
                                        .color(ThemePalette::text_primary(is_dark)),
                                );
                            }

                            if let Some(clock) = gpu_info.clock_mhz {
                                ui.label(
                                    egui::RichText::new("Clock Speed:")
                                        .size(11.5)
                                        .color(ThemePalette::text_secondary(is_dark)),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{} MHz", clock))
                                        .monospace()
                                        .color(ThemePalette::text_primary(is_dark)),
                                );
                            }
                            ui.end_row();
                        });

                    ui.add_space(6.0);
                    paint_progress_bar(
                        ui,
                        gpu_info.utilization / 100.0,
                        get_usage_color(gpu_info.utilization),
                        6.0,
                        is_dark,
                    );
                });
                ui.add_space(8.0);
            }
        }

        ui.add_space(10.0);

        // ── 5. Battery Health (if present) ──
        if let Some(bat) = &data.battery_info {
            if bat.present {
                card_frame(is_dark).show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("BATTERY HEALTH & POWER MANAGEMENT")
                            .size(11.0)
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.add_space(8.0);

                    egui::Grid::new("sysinfo_battery_grid")
                        .num_columns(4)
                        .spacing([24.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Design Capacity:")
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new(format!("{} mWh", bat.design_capacity))
                                    .monospace()
                                    .strong()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );

                            ui.label(
                                egui::RichText::new("Full Charge Capacity:")
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new(format!("{} mWh", bat.full_charge_capacity))
                                    .monospace()
                                    .strong()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                            ui.end_row();

                            let wear = if bat.design_capacity > 0 {
                                100.0 - ((bat.full_charge_capacity as f32 / bat.design_capacity as f32) * 100.0)
                            } else {
                                0.0
                            };

                            ui.label(
                                egui::RichText::new("Battery Wear Level:")
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            let wear_color = if wear < 15.0 {
                                ThemePalette::STATUS_HEALTHY
                            } else if wear < 30.0 {
                                ThemePalette::STATUS_WARNING
                            } else {
                                ThemePalette::STATUS_CRITICAL
                            };
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", wear))
                                    .monospace()
                                    .strong()
                                    .color(wear_color),
                            );

                            ui.label(
                                egui::RichText::new("Power State:")
                                    .size(11.5)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new(bat.discharge_state.as_deref().unwrap_or("N/A"))
                                    .monospace()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                            ui.end_row();
                        });
                });
            }
        }
    });
}
