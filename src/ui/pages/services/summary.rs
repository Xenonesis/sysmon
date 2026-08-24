use crate::app::commands::UiIntent;
use crate::services::ServiceInfo;
use crate::ui::components::{card_frame, status_pill};
use crate::ui::theme::ThemePalette;
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ServiceCounts {
    pub(super) total: usize,
    pub(super) running: usize,
    pub(super) stopped: usize,
    pub(super) other: usize,
    pub(super) running_percent: f32,
}

impl ServiceCounts {
    pub(super) fn from_services(services: &[ServiceInfo]) -> Self {
        let total = services.len();
        let running = services
            .iter()
            .filter(|service| service.state.eq_ignore_ascii_case("running"))
            .count();
        let stopped = services
            .iter()
            .filter(|service| service.state.eq_ignore_ascii_case("stopped"))
            .count();
        let other = total.saturating_sub(running + stopped);
        let running_percent = if total == 0 {
            0.0
        } else {
            (running as f32 / total as f32) * 100.0
        };
        Self {
            total,
            running,
            stopped,
            other,
            running_percent,
        }
    }
}

pub(super) fn paint(
    ui: &mut egui::Ui,
    counts: ServiceCounts,
    is_dark: bool,
    is_elevated: bool,
    intents: &mut Vec<UiIntent>,
) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            status_pill(
                ui,
                &format!("{} TOTAL SERVICES", counts.total),
                ThemePalette::ACCENT_PRIMARY,
                is_dark,
            );
            status_pill(
                ui,
                &format!("{} RUNNING ({:.0}%)", counts.running, counts.running_percent),
                ThemePalette::STATUS_HEALTHY,
                is_dark,
            );
            status_pill(
                ui,
                &format!("{} STOPPED", counts.stopped),
                ThemePalette::text_dimmed(is_dark),
                is_dark,
            );
            if counts.other > 0 {
                status_pill(
                    ui,
                    &format!("{} PENDING / PAUSED", counts.other),
                    ThemePalette::STATUS_WARNING,
                    is_dark,
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if is_elevated {
                    status_pill(ui, "ADMIN ELEVATED", ThemePalette::STATUS_HEALTHY, is_dark);
                } else {
                    #[cfg(target_os = "windows")]
                    if ui
                        .button(
                            egui::RichText::new("🛡 Elevate to Admin")
                                .size(11.0)
                                .strong()
                                .color(ThemePalette::STATUS_WARNING),
                        )
                        .on_hover_text(
                            "Relaunch System Monitor with Administrator privileges to enable service start/stop controls",
                        )
                        .clicked()
                    {
                        intents.push(UiIntent::RelaunchAsAdmin);
                    }

                    status_pill(
                        ui,
                        "ADMIN REQUIRED FOR CONTROL",
                        ThemePalette::STATUS_WARNING,
                        is_dark,
                    );
                }
            });
        });

        ui.add_space(8.0);
        let bar_h = 4.0;
        let bar_w = ui.available_width();
        let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
        ui.painter()
            .rect_filled(bar_rect, egui::Rounding::same(2.0), ThemePalette::bg_track(is_dark));

        if counts.total > 0 {
            let running_width = (bar_w * (counts.running as f32 / counts.total as f32)).max(2.0);
            let running_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(running_width, bar_h));
            ui.painter().rect_filled(
                running_rect,
                egui::Rounding::same(2.0),
                ThemePalette::STATUS_HEALTHY,
            );

            if counts.other > 0 {
                let other_width = bar_w * (counts.other as f32 / counts.total as f32);
                let other_rect = egui::Rect::from_min_size(
                    egui::pos2(bar_rect.min.x + running_width, bar_rect.min.y),
                    egui::vec2(other_width, bar_h),
                );
                ui.painter()
                    .rect_filled(other_rect, egui::Rounding::ZERO, ThemePalette::STATUS_WARNING);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_all_service_states() {
        let services = [
            ServiceInfo {
                name: "a".into(),
                display_name: "A".into(),
                state: "Running".into(),
            },
            ServiceInfo {
                name: "b".into(),
                display_name: "B".into(),
                state: "Stopped".into(),
            },
            ServiceInfo {
                name: "c".into(),
                display_name: "C".into(),
                state: "Start Pending".into(),
            },
        ];

        let counts = ServiceCounts::from_services(&services);
        assert_eq!(counts.total, 3);
        assert_eq!(counts.running, 1);
        assert_eq!(counts.stopped, 1);
        assert_eq!(counts.other, 1);
        assert!((counts.running_percent - 33.333_332).abs() < 0.001);
    }
}
