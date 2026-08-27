mod guided;

use crate::diagnostics::{self, Severity};
use crate::ui::components::{card_frame, paint_progress_bar, paint_section_header, status_pill};
use crate::ui::theme::ThemePalette;
use crate::{SystemData, SystemMonitorApp, snapshot_from_data};
use eframe::egui;

pub(crate) fn show(app: &mut SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData) {
    let is_dark = ui.visuals().dark_mode;
    paint_section_header(ui, "Diagnostics & Session Recorder", is_dark);

    egui::ScrollArea::vertical().show(ui, |ui| {
        guided::show(app, ui, is_dark);
        ui.add_space(8.0);

        // ── 1. Session Recorder Banner ──
        card_frame(is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("SESSION RECORDER")
                        .size(11.0)
                        .strong()
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.session_recorder.is_recording() {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                        let pulse = ((ui.input(|i| i.time) * 4.0).sin().abs() as f32) * 0.4 + 0.6;
                        let rec_color = ThemePalette::STATUS_CRITICAL.gamma_multiply(pulse);
                        status_pill(ui, "RECORDING", rec_color, is_dark);
                    } else {
                        status_pill(ui, "IDLE", ThemePalette::text_dimmed(is_dark), is_dark);
                    }
                });
            });

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Captures 1 local JSONL snapshot per second for diagnostic reproduction. Zero external telemetry.",
                )
                .size(12.0)
                .color(ThemePalette::text_secondary(is_dark)),
            );

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if app.session_recorder.is_recording() {
                    let samples = app.session_recorder.sample_count();
                    let mins = samples / 60;
                    let secs = samples % 60;

                    if ui
                        .button(
                            egui::RichText::new("Stop Recording")
                                .strong()
                                .color(ThemePalette::STATUS_CRITICAL),
                        )
                        .clicked()
                    {
                        app.session_status = Some(match app.session_recorder.stop() {
                            Ok(Some(path)) => format!("Session saved to {}", path.display()),
                            Ok(None) => "Session stopped".into(),
                            Err(error) => format!("Could not stop recording: {error}"),
                        });
                    }

                    ui.label(
                        egui::RichText::new(format!("Elapsed: {:02}:{:02}  ·  {} samples", mins, secs, samples))
                            .monospace()
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                } else if ui
                    .button(
                        egui::RichText::new("Start Recording")
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    )
                    .clicked()
                {
                    app.session_status = Some(match app.session_recorder.start() {
                        Ok(path) => format!("Recording to {}", path.display()),
                        Err(error) => format!("Could not start recording: {error}"),
                    });
                }
            });

            if let Some(status) = &app.session_status {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(status)
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            } else if let Some(path) = app.session_recorder.path() {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("Last session: {}", path.display()))
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
            }
        });

        // ── 1b. Session History & Telemetry Exporter ──
        let recorded_sessions = crate::persistence::session::list_recorded_sessions();
        if !recorded_sessions.is_empty() {
            ui.add_space(8.0);
            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("RECORDED TELEMETRY SESSIONS")
                            .size(11.0)
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_pill(
                            ui,
                            &format!("{} SAVED SESSIONS", recorded_sessions.len()),
                            ThemePalette::ACCENT_PRIMARY,
                            is_dark,
                        );
                    });
                });

                ui.add_space(6.0);

                if let Some(latest) = recorded_sessions.first() {
                    if let Ok(summary) = crate::persistence::session::calculate_session_summary(latest) {
                        ui.horizontal(|ui| {
                            let file_name = latest.file_name().and_then(|n| n.to_str()).unwrap_or("session.jsonl");
                            ui.label(
                                egui::RichText::new(format!("Latest: {file_name}"))
                                    .monospace()
                                    .size(11.5)
                                    .strong()
                                    .color(ThemePalette::text_primary(is_dark)),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "({} samples · {}s)",
                                    summary.sample_count, summary.duration_secs
                                ))
                                .monospace()
                                .size(11.0)
                                .color(ThemePalette::text_dimmed(is_dark)),
                            );
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Avg CPU: {:.1}%", summary.avg_cpu))
                                    .monospace()
                                    .size(11.0)
                                    .color(ThemePalette::ACCENT_PRIMARY),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(format!("Peak CPU: {:.1}%", summary.max_cpu))
                                    .monospace()
                                    .size(11.0)
                                    .color(ThemePalette::STATUS_WARNING),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(format!("Avg RAM: {:.1}%", summary.avg_memory_pct))
                                    .monospace()
                                    .size(11.0)
                                    .color(ThemePalette::STATUS_HEALTHY),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "Net Total: {:.1} MB",
                                    summary.total_net_recv_mb + summary.total_net_sent_mb
                                ))
                                .monospace()
                                .size(11.0)
                                .color(ThemePalette::text_secondary(is_dark)),
                            );
                        });
                    }

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let csv_path = latest.with_extension("csv");
                        if ui
                            .button(egui::RichText::new("📊 Export Latest to CSV").strong())
                            .clicked()
                        {
                            match crate::persistence::session::export_session_to_csv(latest, &csv_path) {
                                Ok(count) => {
                                    app.session_status =
                                        Some(format!("Exported {count} telemetry rows to {}", csv_path.display()));
                                }
                                Err(err) => {
                                    app.session_status = Some(format!("Export error: {err}"));
                                }
                            }
                        }

                        if ui.button("📁 Open Sessions Folder").clicked()
                            && let Some(parent) = latest.parent()
                        {
                            let _ = std::process::Command::new("explorer").arg(parent).spawn();
                        }
                    });
                }
            });
        }

        ui.add_space(12.0);

        // ── 2. Diagnostic Analysis Findings ──
        let snapshot = app.latest_snapshot.clone().unwrap_or_else(|| snapshot_from_data(data));
        let report = diagnostics::analyze(&snapshot, &data.telemetry_history_stats);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("ANOMALY FINDINGS & DIAGNOSTIC REPORT")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{} finding(s)", report.findings.len()))
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
            });
        });

        ui.add_space(6.0);

        if report.findings.is_empty() {
            card_frame(is_dark).show(ui, |ui| {
                ui.horizontal(|ui| {
                    status_pill(ui, "OPTIMAL", ThemePalette::STATUS_HEALTHY, is_dark);
                    ui.label(
                        egui::RichText::new("No diagnostic anomalies or hardware bottlenecks detected.")
                            .size(13.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                });
            });
        } else {
            for finding in report.findings {
                card_frame(is_dark).show(ui, |ui| {
                    let color = match finding.severity {
                        Severity::Healthy => ThemePalette::STATUS_HEALTHY,
                        Severity::Info => ThemePalette::ACCENT_PRIMARY,
                        Severity::Warning => ThemePalette::STATUS_WARNING,
                        Severity::Critical => ThemePalette::STATUS_CRITICAL,
                    };

                    // Header row: badge, title, confidence
                    ui.horizontal(|ui| {
                        status_pill(ui, finding.severity.label(), color, is_dark);
                        ui.label(
                            egui::RichText::new(&finding.title)
                                .strong()
                                .size(14.0)
                                .color(ThemePalette::text_primary(is_dark)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{}% CONFIDENCE", finding.confidence))
                                    .monospace()
                                    .size(11.0)
                                    .color(ThemePalette::text_secondary(is_dark)),
                            );
                        });
                    });

                    // Confidence meter
                    ui.add_space(4.0);
                    paint_progress_bar(ui, finding.confidence as f32 / 100.0, color, 4.0, is_dark);

                    // Evidence
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("EVIDENCE")
                            .size(10.0)
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(&finding.evidence)
                            .monospace()
                            .size(12.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    // Recommendation
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("ACTION RECOMMENDATION")
                            .size(10.0)
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(&finding.recommendation)
                            .strong()
                            .size(12.5)
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                });
                ui.add_space(8.0);
            }
        }

        ui.add_space(10.0);

        // ── 3. BSOD & Crash Minidump History ──
        paint_bsod_history_card(app, ui, is_dark);
    });
}

fn paint_bsod_history_card(app: &mut SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    if app.crash_reports.is_none() {
        app.crash_reports = Some(crate::diagnostics::minidump::scan_recent_crashes());
    }
    let crashes = app.crash_reports.as_deref().unwrap_or(&[]);
    let mut rescan_requested = false;

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("BSOD & CRASH MINIDUMP HISTORY")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("⟳ Scan Dumps").clicked() {
                    rescan_requested = true;
                }
                if crashes.is_empty() {
                    status_pill(ui, "HEALTHY (0 CRASHES)", ThemePalette::STATUS_HEALTHY, is_dark);
                } else {
                    status_pill(
                        ui,
                        &format!("{} INCIDENT(S)", crashes.len()),
                        ThemePalette::STATUS_CRITICAL,
                        is_dark,
                    );
                }
            });
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Kernel BSOD minidumps (%SystemRoot%\\Minidump) and application crash dumps (%LOCALAPPDATA%\\CrashDumps) decoded via offline diagnostic dictionary.",
            )
            .size(12.0)
            .color(ThemePalette::text_secondary(is_dark)),
        );

        ui.add_space(8.0);
        if crashes.is_empty() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    status_pill(ui, "HEALTHY", ThemePalette::STATUS_HEALTHY, is_dark);
                    ui.label(
                        egui::RichText::new(
                            "No BSOD or application minidump crash files detected. System crash logs are clear.",
                        )
                        .size(12.5)
                        .color(ThemePalette::text_primary(is_dark)),
                    );
                });
            });
        } else {
            for crash in crashes {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        status_pill(
                            ui,
                            &format!("0x{:08X}", crash.bugcheck_code),
                            ThemePalette::STATUS_CRITICAL,
                            is_dark,
                        );
                        ui.label(
                            egui::RichText::new(&crash.bugcheck_name)
                                .strong()
                                .size(13.5)
                                .color(ThemePalette::text_primary(is_dark)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(&crash.timestamp)
                                    .monospace()
                                    .size(11.0)
                                    .color(ThemePalette::text_dimmed(is_dark)),
                            );
                        });
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Dump File: {}", crash.file_name))
                                .monospace()
                                .size(11.0)
                                .color(ThemePalette::text_secondary(is_dark)),
                        );
                        if let Some(module) = &crash.faulting_module {
                            status_pill(ui, &format!("Faulting: {module}"), ThemePalette::STATUS_WARNING, is_dark);
                        }
                    });

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("EXPLANATION")
                            .size(10.0)
                            .strong()
                            .color(ThemePalette::text_secondary(is_dark)),
                    );
                    ui.label(
                        egui::RichText::new(&crash.explanation)
                            .size(12.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("ACTIONABLE RECOMMENDATION")
                            .size(10.0)
                            .strong()
                            .color(ThemePalette::ACCENT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(&crash.recommendation)
                            .strong()
                            .size(12.0)
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                });
                ui.add_space(6.0);
            }
        }
    });

    if rescan_requested {
        app.crash_reports = Some(crate::diagnostics::minidump::scan_recent_crashes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn diagnostics_guided_flow_renders_ready_and_review_states() {
        let root = std::env::temp_dir().join(format!("sysmon-diagnostics-ui-{}", std::process::id()));
        crate::app_paths::with_test_data_local_dir(root.clone(), || {
            let mut app = crate::SystemMonitorApp::test_app();
            let data = SystemData::default();
            let ctx = egui::Context::default();

            ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
            })
            .textures_delta
            .clear();

            app.session_recorder.start().unwrap();
            for index in 0..20 {
                let snapshot = crate::monitoring::SystemSnapshot {
                    sampled_at: SystemTime::UNIX_EPOCH + Duration::from_secs(index),
                    cpu_usage: if index < 6 { 10.0 } else { 75.0 },
                    memory_percentage: 45.0,
                    ..Default::default()
                };
                app.session_recorder.record(&snapshot).unwrap();
            }
            app.session_recorder.stop().unwrap();

            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 700.0))),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
                },
            )
            .textures_delta
            .clear();
        });
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_diagnostics_renders_with_crashes() {
        let mut app = crate::SystemMonitorApp::test_app();
        let data = SystemData::default();
        app.crash_reports = Some(vec![
            crate::diagnostics::minidump::MinidumpCrashReport {
                file_name: "MEMORY.DMP".into(),
                timestamp: "2026-08-27 10:00:00 UTC".into(),
                bugcheck_code: 0x00000116,
                bugcheck_name: "VIDEO_TDR_ERROR".into(),
                explanation: "Display driver timed out".into(),
                faulting_module: Some("nvlddmkm.sys".into()),
                recommendation: "Reinstall GPU drivers".into(),
            },
            crate::diagnostics::minidump::MinidumpCrashReport {
                file_name: "CRASH.DMP".into(),
                timestamp: "2026-08-26 14:00:00 UTC".into(),
                bugcheck_code: 0x0000003B,
                bugcheck_name: "SYSTEM_SERVICE_EXCEPTION".into(),
                explanation: "System service routine error".into(),
                faulting_module: None,
                recommendation: "Check driver updates".into(),
            },
        ]);

        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| show(&mut app, ui, &data));
        })
        .textures_delta
        .clear();

        assert_eq!(app.crash_reports.as_ref().unwrap().len(), 2);
    }
}
