use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

/// Renders the Power Schemes and active battery status overview card.
pub(crate) fn paint_power_schemes_card(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    data: &SystemData,
    is_dark: bool,
) {
    let battery = &data.battery_health;
    let power_plans = &data.power_plans;

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
            for plan in power_plans {
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
}

/// Renders the detailed Battery Health & Power Management diagnostics card if a battery is present.
pub(crate) fn paint_battery_diagnostics_card(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    if let Some(bat) = &data.battery_info
        && bat.present
    {
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
