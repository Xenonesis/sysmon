use crate::timeline::TimelineEventKind;
use crate::ui::components::card_frame;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_event_rail(
    app: &mut crate::SystemMonitorApp,
    ui: &mut egui::Ui,
    window: &crate::timeline::TimelineWindow,
    is_dark: bool,
) {
    card_frame(is_dark).show(ui, |ui| {
        ui.label(
            egui::RichText::new("EVENT RAIL")
                .strong()
                .size(11.0)
                .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.label(format!(
            "Events {}–{} of {} (500 per page). Exports include this page; omissions are declared in summary.json.",
            if window.events.is_empty() {
                0
            } else {
                window.events_offset + 1
            },
            window.events_offset + window.events.len() as u64,
            window.total_events
        ));
        ui.horizontal(|ui| {
            let mut offset = window.events_offset;
            if ui.add_enabled(offset > 0, egui::Button::new("Newer events")).clicked() {
                offset = offset.saturating_sub(500);
            }
            if ui
                .add_enabled(
                    offset + (window.events.len() as u64) < window.total_events,
                    egui::Button::new("Older events"),
                )
                .clicked()
            {
                offset = offset.saturating_add(500);
            }
            if offset != window.events_offset {
                app.timeline_ui.events_offset = offset;
                app.timeline_ui.selected_timestamp_ms = None;
                app.timeline_ui.selected_event_id = None;
                app.timeline_ui.range_end_ms = Some(window.query.end_ms);
                app.timeline.request_window_page(window.query, offset);
            }
        });
        if window.events.is_empty() {
            ui.label("No alert, action, provider, or power events in this range.");
        } else {
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for event in &window.events {
                    let selected = app.timeline_ui.selected_event_id == event.id
                        && app.timeline_ui.selected_timestamp_ms == Some(event.timestamp_ms);
                    let label = format!("{}  {}", event_time(event.timestamp_ms), event.summary);
                    if ui.selectable_label(selected, label).clicked() {
                        app.timeline_ui.selected_timestamp_ms = Some(event.timestamp_ms);
                        app.timeline_ui.selected_event_id = event.id;
                    }
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {} · {}",
                            event_kind_label(event.kind),
                            event.source,
                            event.evidence
                        ))
                        .size(10.5)
                        .color(ThemePalette::text_dimmed(is_dark)),
                    );
                    ui.separator();
                }
            });
        }
    });
}

pub(super) fn event_time(timestamp_ms: i64) -> String {
    chrono::DateTime::<chrono::Local>::from(
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(timestamp_ms)
            .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH),
    )
    .format("%m-%d %H:%M:%S")
    .to_string()
}

pub(super) fn event_kind_label(kind: TimelineEventKind) -> &'static str {
    match kind {
        TimelineEventKind::AlertTriggered => "Alert triggered",
        TimelineEventKind::AlertResolved => "Alert resolved",
        TimelineEventKind::ActionSucceeded => "Action succeeded",
        TimelineEventKind::ActionFailed => "Action failed",
        TimelineEventKind::ProviderUnavailable => "Provider unavailable",
        TimelineEventKind::ProviderRecovered => "Provider recovered",
        TimelineEventKind::MonitoringPaused => "Monitoring paused",
        TimelineEventKind::MonitoringResumed => "Monitoring resumed",
        TimelineEventKind::PowerChanged => "Power changed",
        TimelineEventKind::ServiceChanged => "Service changed",
        TimelineEventKind::StartupChanged => "Startup changed",
        TimelineEventKind::System => "System event",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_kind_label() {
        assert_eq!(event_kind_label(TimelineEventKind::AlertTriggered), "Alert triggered");
        assert_eq!(event_kind_label(TimelineEventKind::PowerChanged), "Power changed");
    }

    #[test]
    fn test_event_time_formatting() {
        let formatted = event_time(0);
        assert!(!formatted.is_empty());
    }
}
