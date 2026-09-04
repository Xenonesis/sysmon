use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::*;
use eframe::egui;

pub(super) fn paint_status_headline(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    if data.alerts.is_empty() {
        status_pill(ui, "ALL SYSTEMS NOMINAL", ThemePalette::STATUS_HEALTHY, is_dark);
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Zero active threshold violations · Telemetry operating within safety boundaries.")
                .size(12.5)
                .color(ThemePalette::text_primary(is_dark)),
        );
    } else {
        let count = data.alerts.len();
        status_pill(
            ui,
            &format!("⚠️ {} ACTIVE INCIDENT{}", count, if count > 1 { "S" } else { "" }),
            ThemePalette::STATUS_WARNING,
            is_dark,
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Metric threshold violations detected · Immediate review and mitigation recommended.")
                .size(12.5)
                .strong()
                .color(ThemePalette::text_primary(is_dark)),
        );
    }
}

pub(super) fn paint_control_hub_actions(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    data: &SystemData,
    is_dark: bool,
    trigger_test_alert: &mut bool,
    clear_all_alerts: &mut bool,
) {
    // Rendered right-to-left
    if super::buttons::secondary_button(
        ui,
        "⚙ Thresholds",
        "Configure alert trigger thresholds in Settings (Ctrl+,)",
        is_dark,
    )
    .clicked()
    {
        app.show_settings = true;
    }

    if super::buttons::accent_button(
        ui,
        "🧪 Test Alert",
        "Simulate a test hardware alert to verify audio chimes & visual indicators",
        is_dark,
    )
    .clicked()
    {
        *trigger_test_alert = true;
    }

    let sound_on = app.settings.enable_alert_sound && app.settings.enable_sounds;
    let sound_label = if sound_on { "🔊 Sound: ON" } else { "🔇 Sound: OFF" };
    if super::buttons::toggle_button(
        ui,
        sound_label,
        sound_on,
        "Toggle alert notification audio chime on/off",
        is_dark,
    )
    .clicked()
    {
        app.settings.enable_alert_sound = !app.settings.enable_alert_sound;
        let _ = app.settings.save();
    }

    let toast_on = app.settings.show_notifications;
    let toast_label = if toast_on { "🔔 Toast: ON" } else { "🔕 Toast: OFF" };
    if super::buttons::toggle_button(
        ui,
        toast_label,
        toast_on,
        "Toggle Windows desktop notification popups on/off",
        is_dark,
    )
    .clicked()
    {
        app.settings.show_notifications = !app.settings.show_notifications;
        let _ = app.settings.save();
    }

    if !data.alerts.is_empty() {
        let clear_btn = egui::Button::new(
            egui::RichText::new("Clear All Alerts")
                .strong()
                .size(11.5)
                .color(ThemePalette::STATUS_CRITICAL),
        )
        .fill(ThemePalette::STATUS_CRITICAL.gamma_multiply(if is_dark { 0.15 } else { 0.10 }))
        .stroke(egui::Stroke::new(
            1.0,
            ThemePalette::STATUS_CRITICAL.gamma_multiply(0.45),
        ))
        .corner_radius(egui::CornerRadius::same(4));

        if ui
            .add(clear_btn)
            .on_hover_text("Dismiss all active system alerts")
            .clicked()
        {
            *clear_all_alerts = true;
        }
    }
}
