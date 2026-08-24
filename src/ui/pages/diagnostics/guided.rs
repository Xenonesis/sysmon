use crate::persistence::session::{analyze_session_against_baseline, SessionDiagnosis};
use crate::ui::components::{card_frame, paint_progress_bar, status_pill};
use crate::ui::theme::ThemePalette;
use crate::{SystemMonitorApp, Tab};
use eframe::egui;

const BASELINE_TARGET_SAMPLES: u64 = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GuidedStage {
    Ready,
    Baseline,
    Reproduce,
    Review,
}

fn stage(recording: bool, samples: u64, has_session: bool) -> GuidedStage {
    if recording && samples < BASELINE_TARGET_SAMPLES {
        GuidedStage::Baseline
    } else if recording {
        GuidedStage::Reproduce
    } else if has_session {
        GuidedStage::Review
    } else {
        GuidedStage::Ready
    }
}

pub(super) fn show(app: &mut SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    let current_stage = stage(
        app.session_recorder.is_recording(),
        app.session_recorder.sample_count(),
        app.session_recorder.path().is_some(),
    );

    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("WHY IS MY PC SLOW?")
                    .strong()
                    .size(13.0)
                    .color(ThemePalette::text_primary(is_dark)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (label, color) = match current_stage {
                    GuidedStage::Ready => ("READY", ThemePalette::ACCENT_PRIMARY),
                    GuidedStage::Baseline => ("CAPTURING BASELINE", ThemePalette::STATUS_WARNING),
                    GuidedStage::Reproduce => ("REPRODUCE NOW", ThemePalette::STATUS_CRITICAL),
                    GuidedStage::Review => ("EVIDENCE READY", ThemePalette::STATUS_HEALTHY),
                };
                status_pill(ui, label, color, is_dark);
            });
        });
        ui.add_space(5.0);
        ui.label(
            egui::RichText::new(
                "Capture a quiet baseline, reproduce the slowdown, then compare the incident against your own machine.",
            )
            .size(12.0)
            .color(ThemePalette::text_secondary(is_dark)),
        );
        ui.add_space(10.0);

        match current_stage {
            GuidedStage::Ready => show_ready(app, ui, is_dark),
            GuidedStage::Baseline => show_baseline(app, ui, is_dark),
            GuidedStage::Reproduce => show_reproduce(app, ui, is_dark),
            GuidedStage::Review => show_review(app, ui, is_dark),
        }
    });
}

fn show_ready(app: &mut SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    step(
        ui,
        "1",
        "Capture baseline",
        "Leave the machine in its normal state for about 15 seconds.",
        is_dark,
    );
    if ui
        .button(
            egui::RichText::new("Start guided diagnosis")
                .strong()
                .color(ThemePalette::ACCENT_PRIMARY),
        )
        .clicked()
    {
        app.session_status = Some(match app.session_recorder.start() {
            Ok(path) => format!("Guided capture started at {}", path.display()),
            Err(error) => format!("Could not start guided capture: {error}"),
        });
    }
}

fn show_baseline(app: &mut SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    let samples = app.session_recorder.sample_count();
    step(
        ui,
        "1",
        "Building a personal baseline",
        "Keep your usual applications open, but do not reproduce the slowdown yet.",
        is_dark,
    );
    ui.add_space(7.0);
    paint_progress_bar(
        ui,
        samples as f32 / BASELINE_TARGET_SAMPLES as f32,
        ThemePalette::STATUS_WARNING,
        6.0,
        is_dark,
    );
    ui.label(
        egui::RichText::new(format!("{samples}/{BASELINE_TARGET_SAMPLES} baseline samples"))
            .monospace()
            .size(11.0)
            .color(ThemePalette::text_secondary(is_dark)),
    );
}

fn show_reproduce(app: &mut SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    step(
        ui,
        "2",
        "Reproduce the slowdown",
        "Run the workload that feels slow. Finish when the problem has appeared.",
        is_dark,
    );
    ui.add_space(8.0);
    if ui
        .button(
            egui::RichText::new("Finish capture and analyze")
                .strong()
                .color(ThemePalette::STATUS_CRITICAL),
        )
        .clicked()
    {
        app.session_status = Some(match app.session_recorder.stop() {
            Ok(Some(path)) => format!("Evidence captured at {}", path.display()),
            Ok(None) => "Guided capture stopped".into(),
            Err(error) => format!("Could not finish guided capture: {error}"),
        });
    }
}

fn show_review(app: &mut SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    let diagnosis = app
        .session_recorder
        .path()
        .and_then(|path| analyze_session_against_baseline(path).ok());

    if let Some(diagnosis) = diagnosis {
        show_diagnosis(app, ui, is_dark, &diagnosis);
    } else {
        step(
            ui,
            "3",
            "More evidence required",
            "A useful comparison needs at least 3 baseline and 3 incident samples. Run the guide again and capture longer.",
            is_dark,
        );
    }

    ui.add_space(9.0);
    if ui.button("Run guided diagnosis again").clicked() {
        app.session_status = Some(match app.session_recorder.start() {
            Ok(path) => format!("New guided capture started at {}", path.display()),
            Err(error) => format!("Could not start guided capture: {error}"),
        });
    }
}

fn show_diagnosis(app: &mut SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool, diagnosis: &SessionDiagnosis) {
    step(
        ui,
        "3",
        &format!("Primary change: {}", diagnosis.primary_signal),
        &diagnosis.summary,
        is_dark,
    );
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        status_pill(
            ui,
            &format!("{} CONFIDENCE", diagnosis.confidence.to_uppercase()),
            ThemePalette::STATUS_HEALTHY,
            is_dark,
        );
        ui.label(
            egui::RichText::new(format!(
                "{} samples · {} baseline · incident #{}",
                diagnosis.sample_count, diagnosis.baseline_samples, diagnosis.incident_sample
            ))
            .monospace()
            .size(11.0)
            .color(ThemePalette::text_secondary(is_dark)),
        );
    });
    if let Some(contributor) = &diagnosis.contributor {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(format!("Likely contributor: {contributor}"))
                .monospace()
                .color(ThemePalette::text_primary(is_dark)),
        );
    }
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(&diagnosis.recommendation)
            .strong()
            .color(ThemePalette::text_primary(is_dark)),
    );
    ui.add_space(7.0);
    if ui
        .button(format!("Inspect {} evidence", diagnosis.primary_signal))
        .clicked()
    {
        app.selected_tab = match diagnosis.primary_signal.as_str() {
            "CPU" | "Memory" => Tab::Processes,
            "Disk I/O" => Tab::Storage,
            "Network" => Tab::Network,
            _ => Tab::Timeline,
        };
    }
}

fn step(ui: &mut egui::Ui, number: &str, title: &str, detail: &str, is_dark: bool) {
    ui.horizontal(|ui| {
        status_pill(ui, number, ThemePalette::ACCENT_PRIMARY, is_dark);
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .strong()
                    .color(ThemePalette::text_primary(is_dark)),
            );
            ui.label(
                egui::RichText::new(detail)
                    .size(11.5)
                    .color(ThemePalette::text_secondary(is_dark)),
            );
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guided_stage_transitions_are_deterministic() {
        assert_eq!(stage(false, 0, false), GuidedStage::Ready);
        assert_eq!(stage(true, 5, true), GuidedStage::Baseline);
        assert_eq!(stage(true, BASELINE_TARGET_SAMPLES, true), GuidedStage::Reproduce);
        assert_eq!(stage(false, 20, true), GuidedStage::Review);
    }
}
