mod analysis;
mod events;
mod plots;

use crate::timeline::{IncidentSelection, TimelineRange};
use crate::ui::components::{card_frame, paint_section_header};
use crate::ui::theme::ThemePalette;
use eframe::egui;
use std::time::Duration;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Diagnostic Timeline", is_dark);

    if !app.settings.timeline_enabled {
        card_frame(is_dark).show(ui, |ui| {
            ui.heading("Private history is off");
            ui.label("Enable the local timeline to correlate resource spikes with processes, alerts, power changes, and guarded actions.");
            let privacy_note = "Stored locally only. Command lines, paths, usernames, working directories, and remote IPs are excluded.";
            ui.label(egui::RichText::new(privacy_note).color(ThemePalette::text_secondary(is_dark)));
            if ui.button("Enable 7-day Timeline").clicked() {
                app.settings.timeline_enabled = true;
                app.settings.timeline_retention_days = 7;
                let saved = crate::ui::pages::settings::commit_settings(app);
                app.timeline_ui.window = None;
                let msg = match saved {
                    true => "Timeline recording enabled.",
                    false => "Timeline enabled for this run; settings could not be saved. Retry in Settings.",
                };
                app.timeline_ui.message = Some(msg.into());
            }
        });
        return;
    }

    let stale = app
        .timeline_ui
        .last_refresh
        .is_some_and(|t| t.elapsed() >= Duration::from_secs(10));
    let (querying, clearing) = (app.timeline.query_in_flight(), app.timeline.clear_in_flight());
    if (app.timeline_ui.window.is_none() || stale) && !querying && !clearing {
        app.timeline
            .request_window_page(app.timeline_ui.query(), app.timeline_ui.events_offset);
    }

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Range:");
            let mut range_changed = false;
            for range in TimelineRange::ALL {
                range_changed |= ui
                    .selectable_value(&mut app.timeline_ui.range, range, range.label())
                    .changed();
            }
            if range_changed {
                reset_selection_and_request(app);
            }
            if ui.button("Refresh").clicked() {
                app.timeline
                    .request_window_page(app.timeline_ui.query(), app.timeline_ui.events_offset);
            }
            let selection = app.timeline_ui.window.as_ref().map(|w| IncidentSelection {
                query: w.query,
                timestamp_ms: app
                    .timeline_ui
                    .selected_timestamp_ms
                    .unwrap_or_else(|| w.metrics.last().map_or(w.query.end_ms, |s| s.timestamp_ms)),
                event_id: app.timeline_ui.selected_event_id,
                events_offset: w.events_offset,
            });
            let busy = app.timeline.export_in_flight() || app.timeline.clear_in_flight();
            let export_btn = egui::Button::new("Export Selected Incident");
            if ui.add_enabled(selection.is_some() && !busy, export_btn).clicked()
                && let (Some(selection), Some(folder)) = (selection, rfd::FileDialog::new().pick_folder())
            {
                app.timeline.request_export(selection, folder);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let query = app.timeline_ui.query();
            let dur = app.timeline_ui.range.duration_ms();
            let oldest =
                crate::timeline::now_ms().saturating_sub(i64::from(app.settings.timeline_retention_days) * 86_400_000);
            let mut nav = false;
            if ui
                .add_enabled(query.start_ms > oldest, egui::Button::new("Earlier"))
                .clicked()
            {
                app.timeline_ui.range_end_ms = Some(query.start_ms.max(oldest.saturating_add(dur)));
                nav = true;
            }
            if ui
                .add_enabled(app.timeline_ui.range_end_ms.is_some(), egui::Button::new("Later"))
                .clicked()
            {
                let next = query.end_ms.saturating_add(dur);
                app.timeline_ui.range_end_ms = (next < crate::timeline::now_ms()).then_some(next);
                nav = true;
            }
            if ui.button("Now").clicked() {
                app.timeline_ui.range_end_ms = None;
                nav = true;
            }
            if nav {
                reset_selection_and_request(app);
            }
            ui.label(format!(
                "{} – {}",
                events::event_time(query.start_ms),
                events::event_time(query.end_ms)
            ));
        });
        let status = app.timeline.status();
        let mb = status.storage_bytes as f64 / 1_048_576.0;
        let text = format!(
            "Recording every 5 seconds · {}-day retention · {:.1} MB local storage",
            status.retention_days, mb
        );
        ui.label(
            egui::RichText::new(text)
                .size(11.0)
                .color(ThemePalette::text_dimmed(is_dark)),
        );
    });

    for (active, text) in [
        (app.timeline.query_in_flight(), "Loading timeline history…"),
        (app.timeline.export_in_flight(), "Writing sanitized incident export…"),
        (
            app.timeline.clear_in_flight(),
            "Clearing timeline; waiting for storage acknowledgement…",
        ),
    ] {
        if active {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(text);
            });
        }
    }
    if let Some(message) = &app.timeline_ui.message {
        ui.label(
            egui::RichText::new(message)
                .monospace()
                .color(ThemePalette::text_secondary(is_dark)),
        );
    }

    let Some(window) = app.timeline_ui.window.clone() else {
        return;
    };
    if window.metrics.is_empty() {
        card_frame(is_dark).show(ui, |ui| {
            ui.label("No samples have been recorded in this range yet.");
            ui.label("Leave SysMon running for at least ten seconds, then refresh.");
        });
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        plots::paint_utilization_plot(ui, &window, is_dark);
        ui.add_space(10.0);
        plots::paint_io_plot(ui, &window, is_dark);
        ui.add_space(10.0);
        ui.columns(2, |columns| {
            events::paint_event_rail(app, &mut columns[0], &window, is_dark);
            analysis::paint_analysis_card(app, &mut columns[1], &window, is_dark);
        });
    });
}

fn reset_selection_and_request(app: &mut crate::SystemMonitorApp) {
    app.timeline_ui.events_offset = 0;
    app.timeline_ui.selected_timestamp_ms = None;
    app.timeline_ui.selected_event_id = None;
    app.timeline_ui.window = None;
    app.timeline.request_window(app.timeline_ui.query());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_page_renders_when_disabled() {
        let mut app = crate::SystemMonitorApp::test_app();
        egui::Context::default()
            .run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui));
            })
            .textures_delta
            .clear();
    }
}
