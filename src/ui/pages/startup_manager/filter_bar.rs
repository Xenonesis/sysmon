use crate::startup::{ImpactTier, StartupSortColumn};
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use eframe::egui;

pub(crate) fn paint_filter_bar(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, is_dark: bool) {
    card_frame(is_dark).show(ui, |ui| {
        ui.horizontal(|ui| {
            // Search Input with integrated Clear button
            ui.label(
                egui::RichText::new("Search:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.startup_search)
                    .hint_text("Search name, command, publisher...")
                    .desired_width(240.0),
            );
            if !app.startup_search.is_empty() && ui.small_button("×").on_hover_text("Clear search filter").clicked() {
                app.startup_search.clear();
            }

            ui.add_space(8.0);

            // Impact filter
            ui.label(
                egui::RichText::new("Impact:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            egui::ComboBox::from_id_source("startup_impact_filter")
                .selected_text(match &app.startup_filter_impact {
                    Some(ImpactTier::High) => "High",
                    Some(ImpactTier::Medium) => "Medium",
                    Some(ImpactTier::Low) => "Low",
                    _ => "All",
                })
                .show_ui(ui, |ui: &mut egui::Ui| {
                    if ui
                        .selectable_label(app.startup_filter_impact.is_none(), "All")
                        .clicked()
                    {
                        app.startup_filter_impact = None;
                    }
                    if ui
                        .selectable_label(app.startup_filter_impact == Some(ImpactTier::High), "High")
                        .clicked()
                    {
                        app.startup_filter_impact = Some(ImpactTier::High);
                    }
                    if ui
                        .selectable_label(app.startup_filter_impact == Some(ImpactTier::Medium), "Medium")
                        .clicked()
                    {
                        app.startup_filter_impact = Some(ImpactTier::Medium);
                    }
                    if ui
                        .selectable_label(app.startup_filter_impact == Some(ImpactTier::Low), "Low")
                        .clicked()
                    {
                        app.startup_filter_impact = Some(ImpactTier::Low);
                    }
                });

            ui.add_space(8.0);

            // Signed filter
            ui.label(
                egui::RichText::new("Publisher:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );
            egui::ComboBox::from_id_source("startup_signed_filter")
                .selected_text(match app.startup_filter_signed {
                    Some(true) => "Signed",
                    Some(false) => "Unsigned",
                    None => "All",
                })
                .show_ui(ui, |ui: &mut egui::Ui| {
                    if ui
                        .selectable_label(app.startup_filter_signed.is_none(), "All")
                        .clicked()
                    {
                        app.startup_filter_signed = None;
                    }
                    if ui
                        .selectable_label(app.startup_filter_signed == Some(true), "Signed")
                        .clicked()
                    {
                        app.startup_filter_signed = Some(true);
                    }
                    if ui
                        .selectable_label(app.startup_filter_signed == Some(false), "Unsigned")
                        .clicked()
                    {
                        app.startup_filter_signed = Some(false);
                    }
                });

            ui.add_space(8.0);
            ui.checkbox(&mut app.startup_filter_broken, "Broken only");

            let has_active_filters = !app.startup_search.is_empty()
                || app.startup_filter_impact.is_some()
                || app.startup_filter_signed.is_some()
                || app.startup_filter_broken;

            if has_active_filters
                && ui
                    .small_button("Reset")
                    .on_hover_text("Reset all search and filtering")
                    .clicked()
            {
                app.startup_search.clear();
                app.startup_filter_impact = None;
                app.startup_filter_signed = None;
                app.startup_filter_broken = false;
            }
        });

        ui.add_space(4.0);

        // Sort controls
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Sort by:")
                    .strong()
                    .color(ThemePalette::text_secondary(is_dark)),
            );

            let sorts = [
                (StartupSortColumn::Impact, "Impact"),
                (StartupSortColumn::Name, "Name"),
                (StartupSortColumn::Source, "Source"),
                (StartupSortColumn::Publisher, "Publisher"),
            ];
            for (col, label) in &sorts {
                let is_active = app.startup_sort == *col;
                let text = if is_active {
                    let arrow = if app.startup_sort_ascending { " ▲" } else { " ▼" };
                    format!("{}{}", label, arrow)
                } else {
                    label.to_string()
                };
                let text_color = if is_active {
                    ThemePalette::ACCENT_PRIMARY
                } else {
                    ThemePalette::text_primary(is_dark)
                };
                if ui
                    .button(egui::RichText::new(text).small().strong().color(text_color))
                    .clicked()
                {
                    if is_active {
                        app.startup_sort_ascending = !app.startup_sort_ascending;
                    } else {
                        app.startup_sort = *col;
                        app.startup_sort_ascending = true;
                    }
                }
            }
        });
    });
}
