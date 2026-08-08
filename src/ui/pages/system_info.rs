use crate::*;
use crate::ui::theme::ThemePalette;
use crate::ui::components::*;
use eframe::egui;
use egui_plot::*;

pub(crate) fn show(app: &crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
        paint_section_header(ui, "System Information");

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.heading("Operating System");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("OS Name:");
                    ui.strong(&data.system_info.os_name);
                });

                ui.horizontal(|ui| {
                    ui.label("OS Version:");
                    ui.strong(&data.system_info.os_version);
                });

                ui.horizontal(|ui| {
                    ui.label("Kernel Version:");
                    ui.strong(&data.system_info.kernel_version);
                });

                ui.horizontal(|ui| {
                    ui.label("Hostname:");
                    ui.strong(&data.system_info.hostname);
                });

                ui.horizontal(|ui| {
                    ui.label("Uptime:");
                    let days = data.system_info.uptime / 86400;
                    let hours = (data.system_info.uptime % 86400) / 3600;
                    let minutes = (data.system_info.uptime % 3600) / 60;
                    ui.strong(format!("{}d {}h {}m", days, hours, minutes));
                });

                    if let Some(mb) = &data.system_info.motherboard {
                        ui.horizontal(|ui| {
                            ui.label("Motherboard:");
                            ui.strong(mb);
                        });
                    }
                    if let Some(bios) = &data.system_info.bios_version {
                        ui.horizontal(|ui| {
                            ui.label("BIOS Version:");
                            ui.strong(bios);
                        });
                    }
                    if let Some(drv) = &data.system_info.gpu_driver {
                        ui.horizontal(|ui| {
                            ui.label("GPU Driver:");
                            ui.strong(drv);
                        });
                    }
                    if let Some(build) = &data.system_info.os_build {
                        ui.horizontal(|ui| {
                            ui.label("OS Build:");
                            ui.strong(build);
                        });
                    }
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("Processor");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("CPU Brand:");
                    ui.strong(&data.system_info.cpu_brand);
                });

                ui.horizontal(|ui| {
                    ui.label("CPU Cores:");
                    ui.strong(format!("{}", data.system_info.cpu_count));
                });

                ui.horizontal(|ui| {
                    ui.label("Current Usage:");
                    let color = get_usage_color(data.cpu_usage);
                    ui.colored_label(color, format!("{:.1}%", data.cpu_usage));
                });
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.heading("Memory");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Total RAM:");
                    ui.strong(format!("{:.2} GB", bytes_to_gb(data.memory_total)));
                });

                ui.horizontal(|ui| {
                    ui.label("Used RAM:");
                    ui.strong(format!("{:.2} GB", bytes_to_gb(data.memory_used)));
                });

                ui.horizontal(|ui| {
                    ui.label("Free RAM:");
                    ui.strong(format!("{:.2} GB", bytes_to_gb(data.memory_total - data.memory_used)));
                });

                ui.horizontal(|ui| {
                    ui.label("Usage:");
                    let color = get_usage_color(data.memory_percentage);
                    ui.colored_label(color, format!("{:.1}%", data.memory_percentage));
                });
            });

            ui.add_space(10.0);

            if data.gpu_info.is_empty() {
                ui.label(
                    egui::RichText::new("No supported GPU detected.")
                        .italics()
                        .color(ThemePalette::TEXT_DIMMED),
                );
            } else {
                for gpu_info in &data.gpu_info {
                    ui.group(|ui| {
                        ui.heading("Graphics Card");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label("GPU:");
                            ui.strong(&gpu_info.name);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Utilization:");
                            let color = get_usage_color(gpu_info.utilization);
                            ui.colored_label(color, format!("{:.1}%", gpu_info.utilization));
                        });

                        if let (Some(used), Some(total)) = (gpu_info.memory_used, gpu_info.memory_total) {
                            ui.horizontal(|ui| {
                                ui.label("VRAM:");
                                let used_mb = bytes_to_mb(used);
                                let total_mb = bytes_to_mb(total);
                                if total_mb >= 1024.0 {
                                    ui.strong(format!("{:.1} / {:.1} GB", used_mb / 1024.0, total_mb / 1024.0));
                                } else {
                                    ui.strong(format!("{:.0} / {:.0} MB", used_mb, total_mb));
                                }
                            });
                        }

                        if let Some(temp) = gpu_info.temperature {
                            ui.horizontal(|ui| {
                                ui.label("Temperature:");
                                let temp_color = if temp < 70 {
                                    ThemePalette::STATUS_HEALTHY
                                } else if temp < 85 {
                                    ThemePalette::STATUS_WARNING
                                } else {
                                    ThemePalette::STATUS_CRITICAL
                                };
                                ui.colored_label(temp_color, format!("🌡️ {}°C", temp));
                            });
                        }
                    });
                }
            }

            ui.add_space(10.0);

            // Swap / Page File info
            if data.swap_info.total > 0 {
                ui.group(|ui| {
                    ui.heading("Swap / Page File");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Total Swap:");
                        ui.strong(format!("{:.2} GB", bytes_to_gb(data.swap_info.total)));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Used Swap:");
                        ui.strong(format!("{:.2} GB", bytes_to_gb(data.swap_info.used)));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Usage:");
                        let color = get_usage_color(data.swap_info.percentage);
                        ui.colored_label(color, format!("{:.1}%", data.swap_info.percentage));
                    });

                    let color = get_usage_color(data.swap_info.percentage);
                    paint_progress_bar(ui, data.swap_info.percentage / 100.0, color, 5.0);
                });
            }

            ui.add_space(10.0);

            if let Some(bat) = &data.battery_info {
                if bat.present {
                    ui.group(|ui| {
                        ui.heading("Battery Health");
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Design Capacity:");
                            ui.strong(format!("{} mWh", bat.design_capacity));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Full Charge Capacity:");
                            ui.strong(format!("{} mWh", bat.full_charge_capacity));
                        });
                        let wear = if bat.design_capacity > 0 {
                            100.0 - ((bat.full_charge_capacity as f32 / bat.design_capacity as f32) * 100.0)
                        } else { 0.0 };
                        ui.horizontal(|ui| {
                            ui.label("Battery Wear Level:");
                            ui.strong(format!("{:.1}%", wear));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Discharge/Charge State:");
                            ui.strong(bat.discharge_state.as_deref().unwrap_or("N/A").to_string());
                        });
                        ui.horizontal(|ui| {
                            ui.label("Discharge/Charge State:");
                            ui.strong(bat.discharge_state.as_deref().unwrap_or("N/A").to_string());
                        });
                    });
                    ui.add_space(12.0);
                }
            }
        });
    }
