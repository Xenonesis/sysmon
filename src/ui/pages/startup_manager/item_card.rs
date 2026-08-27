use crate::startup::{self, Recommendation, StartupItem};
use crate::ui::components::*;
use crate::ui::pages::startup_manager::impact_tier_badge_color;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(crate) struct StartupActionRequest {
    pub command: crate::app::commands::ActionCommand,
}

pub(crate) fn paint_startup_item_card(
    _show_confirm: &mut Option<String>,
    ui: &mut egui::Ui,
    item: &StartupItem,
    is_dark: bool,
    is_elevated: bool,
) -> Option<StartupActionRequest> {
    let mut action = None;

    card_frame(is_dark).show(ui, |ui| {
        // ── Row 1: High-Contrast Impact Badge + Signed Badge + Name + Source ──
        ui.horizontal(|ui| {
            // Impact tier badge
            let (badge_text, badge_color) = impact_tier_badge_color(&item.impact_tier, is_dark);
            status_pill(ui, badge_text, badge_color, is_dark);

            // Signed/Verified Publisher Badge
            match item.is_signed {
                Some(true) => {
                    status_pill(ui, "SIGNED", ThemePalette::STATUS_HEALTHY, is_dark);
                }
                Some(false) => {
                    status_pill(ui, "UNSIGNED", ThemePalette::STATUS_CRITICAL, is_dark);
                }
                None => {}
            }

            if !item.exe_exists && item.exe_path.is_some() {
                status_pill(ui, "FILE MISSING", ThemePalette::STATUS_CRITICAL, is_dark);
            }

            ui.add_space(4.0);

            let clean_name = item.name.replace('\0', "");
            if item.enabled {
                ui.strong(egui::RichText::new(&clean_name).color(ThemePalette::text_primary(is_dark)));
            } else {
                ui.label(
                    egui::RichText::new(&clean_name)
                        .strong()
                        .strikethrough()
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clean_source = item.source.replace('\0', "");
                ui.label(
                    egui::RichText::new(&clean_source)
                        .monospace()
                        .size(11.0)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
            });
        });

        ui.add_space(2.0);

        // ── Row 2: Command path (Monospace) ──
        let clean_cmd = item.command.replace('\0', "");
        let cmd_display = if clean_cmd.chars().count() > 90 {
            let truncated: String = clean_cmd.chars().take(87).collect();
            format!("{}...", truncated)
        } else {
            clean_cmd.clone()
        };
        ui.label(
            egui::RichText::new(cmd_display)
                .monospace()
                .size(11.0)
                .color(ThemePalette::text_secondary(is_dark)),
        )
        .on_hover_text(&clean_cmd);

        ui.add_space(2.0);

        // ── Row 3: Publisher + Recommendation Reason ──
        ui.horizontal(|ui| {
            if let Some(pub_name) = &item.publisher {
                let clean_pub = pub_name.replace('\0', "");
                ui.label(
                    egui::RichText::new(format!("Publisher: {}", clean_pub))
                        .size(11.5)
                        .color(ThemePalette::text_secondary(is_dark)),
                );
                ui.separator();
            }

            let rec_color = match item.recommendation {
                Recommendation::Keep => ThemePalette::STATUS_HEALTHY,
                Recommendation::Review => ThemePalette::STATUS_WARNING,
                Recommendation::Disable | Recommendation::Cleanup => ThemePalette::STATUS_CRITICAL,
            };
            ui.label(
                egui::RichText::new(format!("> {}", item.recommendation.label()))
                    .strong()
                    .size(11.5)
                    .color(rec_color),
            );
            let clean_reason = item.reason.replace('\0', "");
            ui.label(
                egui::RichText::new(format!("— {}", clean_reason))
                    .size(11.5)
                    .color(ThemePalette::text_dimmed(is_dark)),
            );
        });

        ui.add_space(4.0);

        // ── Row 4: Action Controls ──
        ui.horizontal(|ui| {
            let can_modify = !item.locator.requires_admin() || is_elevated;
            let is_keep = item.recommendation == Recommendation::Keep;

            // Disable/Enable button
            if item.enabled {
                ui.add_enabled_ui(can_modify && !is_keep, |ui| {
                    if ui
                        .button(egui::RichText::new("Disable").small())
                        .on_hover_text(if is_keep {
                            "System component — disabling not recommended"
                        } else if !can_modify {
                            "Requires Administrator privileges"
                        } else {
                            "Disable this startup item (reversible)"
                        })
                        .clicked()
                    {
                        action = Some(StartupActionRequest {
                            command: crate::app::commands::ActionCommand::DisableStartup {
                                item_name: item.name.clone(),
                                locator: item.locator.clone(),
                            },
                        });
                    }
                });
            } else {
                ui.add_enabled_ui(can_modify, |ui| {
                    if ui
                        .button(
                            egui::RichText::new("Enable")
                                .small()
                                .color(ThemePalette::STATUS_HEALTHY),
                        )
                        .on_hover_text(if !can_modify {
                            "Requires Administrator privileges"
                        } else {
                            "Re-enable this startup item"
                        })
                        .clicked()
                    {
                        action = Some(StartupActionRequest {
                            command: crate::app::commands::ActionCommand::EnableStartup {
                                item_name: item.name.clone(),
                                locator: item.locator.clone(),
                            },
                        });
                    }
                });
            }

            // Open location
            if let Some(path) = &item.exe_path
                && item.exe_exists
            {
                let path_clone = path.clone();
                if ui
                    .button(egui::RichText::new("Open").small())
                    .on_hover_text("Open file location in Explorer")
                    .clicked()
                {
                    startup::open_file_location(&path_clone);
                }
            }

            // Copy command
            if ui
                .button(egui::RichText::new("Copy").small())
                .on_hover_text("Copy full command to clipboard")
                .clicked()
            {
                ui.ctx().copy_text(item.command.clone());
            }

            // Search online
            let name_clone = item.name.clone();
            if ui
                .button(egui::RichText::new("Search Online").small())
                .on_hover_text("Search online for info about this item")
                .clicked()
            {
                startup::search_online(&name_clone);
            }

            // Quarantine keeps an exact local backup and exposes global Undo.
            if can_modify
                && !item.enabled
                && ui
                    .button(
                        egui::RichText::new("Quarantine")
                            .small()
                            .color(ThemePalette::STATUS_CRITICAL),
                    )
                    .on_hover_text("Back up this exact entry, then remove it from startup (reversible)")
                    .clicked()
            {
                action = Some(StartupActionRequest {
                    command: crate::app::commands::ActionCommand::QuarantineStartup {
                        item_name: item.name.clone(),
                        locator: item.locator.clone(),
                    },
                });
            }

            // Admin requirement notice
            if !can_modify {
                ui.label(
                    egui::RichText::new("(Requires Admin)")
                        .size(11.0)
                        .color(ThemePalette::text_dimmed(is_dark)),
                );
            }
        });
    });

    action
}
