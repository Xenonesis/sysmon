use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(super) fn paint_disk_perf_card(ui: &mut egui::Ui, disk_perf: &[crate::storage::DiskPerfStats], is_dark: bool) {
    if disk_perf.is_empty() {
        return;
    }
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("DISK LATENCY & QUEUE DEPTH (LIVE)")
                    .size(11.0)
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                status_pill(ui, "~5s REFRESH", ThemePalette::ACCENT_PRIMARY, is_dark);
            });
        });

        ui.add_space(8.0);

        for perf in disk_perf {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Physical Disk {}", perf.name))
                            .size(12.0)
                            .strong()
                            .color(ThemePalette::text_primary(is_dark)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let busy = perf.disk_time_pct.is_some_and(|pct| pct >= 90.0);
                        let label = match perf.disk_time_pct {
                            Some(pct) => format!("{}% ACTIVE", pct.min(999.0) as u32),
                            None => "N/A".to_string(),
                        };
                        status_pill(
                            ui,
                            &label,
                            if busy {
                                ThemePalette::STATUS_WARNING
                            } else {
                                ThemePalette::STATUS_HEALTHY
                            },
                            is_dark,
                        );
                    });
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let metric = |ui: &mut egui::Ui, label: &str, value: String, warn: bool| {
                        ui.label(
                            egui::RichText::new(format!("{label}: {value}"))
                                .monospace()
                                .size(11.0)
                                .color(if warn {
                                    ThemePalette::STATUS_WARNING
                                } else {
                                    ThemePalette::text_secondary(is_dark)
                                }),
                        );
                        ui.add_space(14.0);
                    };
                    metric(
                        ui,
                        "Read Lat",
                        perf.read_latency_ms
                            .map(|ms| format!("{ms:.0} ms"))
                            .unwrap_or_else(|| "N/A".into()),
                        perf.read_latency_ms.is_some_and(|ms| ms >= 50.0),
                    );
                    metric(
                        ui,
                        "Write Lat",
                        perf.write_latency_ms
                            .map(|ms| format!("{ms:.0} ms"))
                            .unwrap_or_else(|| "N/A".into()),
                        perf.write_latency_ms.is_some_and(|ms| ms >= 50.0),
                    );
                    metric(
                        ui,
                        "Queue",
                        perf.queue_depth
                            .map(|depth| format!("{depth:.0}"))
                            .unwrap_or_else(|| "N/A".into()),
                        perf.queue_depth.is_some_and(|depth| depth >= 2.0),
                    );
                    metric(
                        ui,
                        "Read IOPS",
                        perf.read_iops
                            .map(|iops| format!("{iops:.0}"))
                            .unwrap_or_else(|| "N/A".into()),
                        false,
                    );
                    metric(
                        ui,
                        "Write IOPS",
                        perf.write_iops
                            .map(|iops| format!("{iops:.0}"))
                            .unwrap_or_else(|| "N/A".into()),
                        false,
                    );
                });
            });
            ui.add_space(4.0);
        }
    });
}
