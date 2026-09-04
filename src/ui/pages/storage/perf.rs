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
                        let busy = perf.active_pct >= 90.0;
                        status_pill(
                            ui,
                            &format!("{}% ACTIVE", perf.active_pct as u32),
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
                        format!("{:.0} ms", perf.read_latency_ms),
                        perf.read_latency_ms >= 50.0,
                    );
                    metric(
                        ui,
                        "Write Lat",
                        format!("{:.0} ms", perf.write_latency_ms),
                        perf.write_latency_ms >= 50.0,
                    );
                    metric(ui, "Queue", format!("{:.0}", perf.queue_depth), perf.queue_depth >= 2.0);
                    metric(ui, "Read IOPS", perf.read_iops.to_string(), false);
                    metric(ui, "Write IOPS", perf.write_iops.to_string(), false);
                });
            });
            ui.add_space(4.0);
        }
    });
}
