use crate::timeline::{IncidentSelection, TimelineEventKind, TimelineRange, analyze_window};
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use std::time::Duration;

pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Diagnostic Timeline", is_dark);

    if !app.settings.timeline_enabled {
        card_frame(is_dark).show(ui, |ui| {
            ui.heading("Private history is off");
            ui.label(
                "Enable the local timeline to correlate resource spikes with processes, alerts, power changes, and guarded actions.",
            );
            ui.label(
                egui::RichText::new(
                    "Stored locally only. Command lines, paths, usernames, working directories, and remote IPs are excluded.",
                )
                .color(ThemePalette::text_secondary(is_dark)),
            );
            if ui.button("Enable 7-day Timeline").clicked() {
                app.settings.timeline_enabled = true;
                app.settings.timeline_retention_days = 7;
                let saved = crate::ui::pages::settings::commit_settings(app);
                app.timeline_ui.window = None;
                app.timeline_ui.message = Some(if saved {
                    "Timeline recording enabled.".into()
                } else {
                    "Timeline enabled for this run; settings could not be saved. Retry in Settings.".into()
                });
            }
        });
        return;
    }

    let needs_refresh = app.timeline_ui.window.is_none()
        || app
            .timeline_ui
            .last_refresh
            .is_some_and(|last| last.elapsed() >= Duration::from_secs(10));
    if needs_refresh && !app.timeline.query_in_flight() && !app.timeline.clear_in_flight() {
        app.timeline
            .request_window_page(app.timeline_ui.query(), app.timeline_ui.events_offset);
    }

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Range:");
            for range in TimelineRange::ALL {
                if ui
                    .selectable_value(&mut app.timeline_ui.range, range, range.label())
                    .changed()
                {
                    app.timeline_ui.window = None;
                    app.timeline_ui.selected_timestamp_ms = None;
                    app.timeline_ui.selected_event_id = None;
                    app.timeline_ui.events_offset = 0;
                    app.timeline.request_window(app.timeline_ui.query());
                }
            }
            if ui.button("Refresh").clicked() {
                app.timeline
                    .request_window_page(app.timeline_ui.query(), app.timeline_ui.events_offset);
            }
            let selection = app.timeline_ui.window.as_ref().map(|window| IncidentSelection {
                query: window.query,
                timestamp_ms: app.timeline_ui.selected_timestamp_ms.unwrap_or_else(|| {
                    window
                        .metrics
                        .last()
                        .map_or(window.query.end_ms, |sample| sample.timestamp_ms)
                }),
                event_id: app.timeline_ui.selected_event_id,
                events_offset: window.events_offset,
            });
            if ui
                .add_enabled(
                    selection.is_some() && !app.timeline.export_in_flight() && !app.timeline.clear_in_flight(),
                    egui::Button::new("Export Selected Incident"),
                )
                .clicked()
                && let Some(selection) = selection
                && let Some(folder) = rfd::FileDialog::new().pick_folder()
            {
                app.timeline.request_export(selection, folder);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let query = app.timeline_ui.query();
            let oldest =
                crate::timeline::now_ms().saturating_sub(i64::from(app.settings.timeline_retention_days) * 86_400_000);
            let mut navigate = false;
            if ui
                .add_enabled(query.start_ms > oldest, egui::Button::new("Earlier"))
                .clicked()
            {
                app.timeline_ui.range_end_ms = Some(
                    query
                        .start_ms
                        .max(oldest.saturating_add(app.timeline_ui.range.duration_ms())),
                );
                navigate = true;
            }
            if ui
                .add_enabled(app.timeline_ui.range_end_ms.is_some(), egui::Button::new("Later"))
                .clicked()
            {
                let next = query.end_ms.saturating_add(app.timeline_ui.range.duration_ms());
                app.timeline_ui.range_end_ms = (next < crate::timeline::now_ms()).then_some(next);
                navigate = true;
            }
            if ui.button("Now").clicked() {
                app.timeline_ui.range_end_ms = None;
                navigate = true;
            }
            if navigate {
                app.timeline_ui.events_offset = 0;
                app.timeline_ui.selected_timestamp_ms = None;
                app.timeline_ui.selected_event_id = None;
                app.timeline_ui.window = None;
                app.timeline.request_window(app.timeline_ui.query());
            }
            let query = app.timeline_ui.query();
            ui.label(format!("{} – {}", event_time(query.start_ms), event_time(query.end_ms)));
        });
        let status = app.timeline.status();
        ui.label(
            egui::RichText::new(format!(
                "Recording every 5 seconds · {}-day retention · {:.1} MB local storage",
                status.retention_days,
                status.storage_bytes as f64 / 1_048_576.0
            ))
            .size(11.0)
            .color(ThemePalette::text_dimmed(is_dark)),
        );
    });

    if app.timeline.query_in_flight() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Loading timeline history…");
        });
    }
    if app.timeline.export_in_flight() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Writing sanitized incident export…");
        });
    }
    if app.timeline.clear_in_flight() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Clearing timeline; waiting for storage acknowledgement…");
        });
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
        let origin = window.query.start_ms;
        // Preserve each bucket's extrema rather than dropping single-sample spikes.
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("UTILIZATION")
                    .strong()
                    .size(11.0)
                    .color(ThemePalette::ACCENT_PRIMARY),
            );
            let cpu: PlotPoints = envelope(&window.metrics, origin, |sample| Some(sample.cpu_pct)).into();
            let memory: PlotPoints = envelope(&window.metrics, origin, |sample| Some(sample.memory_pct)).into();
            let has_gpu = window.metrics.iter().any(|sample| sample.gpu_pct.is_some());
            let gpu: PlotPoints = envelope(&window.metrics, origin, |sample| sample.gpu_pct).into();
            Plot::new("timeline_utilization")
                .height(220.0)
                .legend(Legend::default())
                .allow_scroll(false)
                .include_y(0.0)
                .include_y(100.0)
                .x_axis_label("Seconds in selected range")
                .y_axis_label("Usage %")
                .show(ui, |plot| {
                    plot.line(Line::new("CPU", cpu).color(ThemePalette::ACCENT_PRIMARY));
                    plot.line(Line::new("Memory", memory).color(ThemePalette::STATUS_HEALTHY));
                    if has_gpu {
                        plot.line(Line::new("GPU", gpu).color(ThemePalette::STATUS_WARNING));
                    }
                });
        });

        ui.add_space(10.0);
        card_frame(is_dark).show(ui, |ui| {
            ui.label(
                egui::RichText::new("DISK & NETWORK THROUGHPUT")
                    .strong()
                    .size(11.0)
                    .color(ThemePalette::STATUS_WARNING),
            );
            let disk: PlotPoints = envelope(&window.metrics, origin, |sample| Some((sample.disk_read_bps + sample.disk_write_bps) / 1_048_576.0)).into();
            let network: PlotPoints = envelope(&window.metrics, origin, |sample| Some((sample.network_down_bps + sample.network_up_bps) / 1_048_576.0)).into();
            Plot::new("timeline_io")
                .height(190.0)
                .legend(Legend::default())
                .allow_scroll(false)
                .x_axis_label("Seconds in selected range")
                .y_axis_label("MB/s")
                .show(ui, |plot| {
                    plot.line(Line::new("Disk", disk).color(ThemePalette::STATUS_WARNING));
                    plot.line(Line::new("Network", network).color(ThemePalette::ACCENT_PRIMARY));
                });
        });

        ui.add_space(10.0);
        ui.columns(2, |columns| {
            card_frame(is_dark).show(&mut columns[0], |ui| {
                ui.label(
                    egui::RichText::new("EVENT RAIL")
                        .strong()
                        .size(11.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.label(format!("Events {}–{} of {} (500 per page). Exports include this page; omissions are declared in summary.json.",
                    if window.events.is_empty() { 0 } else { window.events_offset + 1 },
                    window.events_offset + window.events.len() as u64, window.total_events));
                ui.horizontal(|ui| {
                    let mut offset = window.events_offset;
                    if ui.add_enabled(offset > 0, egui::Button::new("Newer events")).clicked() {
                        offset = offset.saturating_sub(500);
                    }
                    if ui.add_enabled(offset + (window.events.len() as u64) < window.total_events, egui::Button::new("Older events")).clicked() {
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
                            let selected = app.timeline_ui.selected_event_id == event.id && app.timeline_ui.selected_timestamp_ms == Some(event.timestamp_ms);
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

            card_frame(is_dark).show(&mut columns[1], |ui| {
                ui.label(
                    egui::RichText::new("EVIDENCE-BASED ANALYSIS")
                        .strong()
                        .size(11.0)
                        .color(ThemePalette::ACCENT_PRIMARY),
                );
                let selected = app.timeline_ui.selected_timestamp_ms.unwrap_or_else(|| {
                    window
                        .metrics
                        .last()
                        .map_or(window.query.end_ms, |sample| sample.timestamp_ms)
                });
                let analysis = analyze_window(&window, selected);
                ui.heading(&analysis.title);
                ui.label(&analysis.summary);
                ui.label(
                    egui::RichText::new(format!("Confidence: {}", analysis.confidence))
                        .monospace()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.separator();
                for evidence in &analysis.evidence {
                    ui.label(format!("• {evidence}"));
                }
                if !analysis.contributors.is_empty() {
                    ui.separator();
                    ui.strong("Observed contributors");
                    for process in &analysis.contributors {
                        ui.label(format!(
                            "{} (PID {}) · CPU {:.1}% · RAM {:.1} MB · Disk {:.1} MB",
                            process.name,
                            process.pid,
                            process.cpu_pct,
                            process.memory_bytes as f64 / 1_048_576.0,
                            process.disk_bytes as f64 / 1_048_576.0
                        ));
                    }
                }
            });
        });
    });
}

fn envelope(
    samples: &[crate::timeline::TimelineMetricSample],
    origin: i64,
    value: impl Fn(&crate::timeline::TimelineMetricSample) -> Option<f64>,
) -> Vec<[f64; 2]> {
    let bucket_size = samples.len().div_ceil(1_000).max(1);
    let mut points = Vec::with_capacity(2_000);
    for bucket in samples.chunks(bucket_size) {
        let mut extrema: Option<([f64; 2], [f64; 2])> = None;
        for sample in bucket {
            let Some(y) = value(sample).filter(|value| value.is_finite()) else {
                continue;
            };
            let point = [seconds_from(origin, sample.timestamp_ms), y];
            match &mut extrema {
                None => extrema = Some((point, point)),
                Some((min, max)) => {
                    if y < min[1] {
                        *min = point;
                    }
                    if y > max[1] {
                        *max = point;
                    }
                }
            }
        }
        if let Some((a, b)) = extrema {
            if a[0] <= b[0] {
                points.extend([a, b]);
            } else {
                points.extend([b, a]);
            }
        }
    }
    points
}

fn seconds_from(origin_ms: i64, timestamp_ms: i64) -> f64 {
    timestamp_ms.saturating_sub(origin_ms) as f64 / 1_000.0
}

fn event_time(timestamp_ms: i64) -> String {
    chrono::DateTime::<chrono::Local>::from(
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(timestamp_ms)
            .unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH),
    )
    .format("%m-%d %H:%M:%S")
    .to_string()
}

fn event_kind_label(kind: TimelineEventKind) -> &'static str {
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
    fn timeline_page_renders_when_disabled() {
        let mut app = crate::SystemMonitorApp::test_app();
        let context = egui::Context::default();
        context
            .run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui));
            })
            .textures_delta
            .clear();
    }
}
