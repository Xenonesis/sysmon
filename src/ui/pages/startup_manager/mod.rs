mod action_handler;
mod filter_bar;
mod item_card;
mod page;
mod summary_card;

use crate::startup::{ImpactTier, StartupSortColumn};
use crate::ui::theme::ThemePalette;
use eframe::egui;

/// Resolves badge text and high-contrast semantic color for startup impact tiers.
pub(crate) fn impact_tier_badge_color(tier: &ImpactTier, is_dark: bool) -> (&'static str, egui::Color32) {
    match tier {
        ImpactTier::High => ("HIGH", ThemePalette::STATUS_CRITICAL),
        ImpactTier::Medium => ("MED", ThemePalette::STATUS_WARNING),
        ImpactTier::Low => ("LOW", ThemePalette::STATUS_HEALTHY),
        ImpactTier::Unknown => ("UNKNOWN", ThemePalette::text_dimmed(is_dark)),
    }
}

pub(crate) fn filter_and_sort_indices(app: &crate::SystemMonitorApp) -> Vec<usize> {
    let search_lower = app.startup_search.to_lowercase();
    let mut filtered_indices: Vec<usize> = app
        .startup_items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            // Search filter
            if !search_lower.is_empty() {
                let matches = item.name.to_lowercase().contains(&search_lower)
                    || item.command.to_lowercase().contains(&search_lower)
                    || item
                        .publisher
                        .as_ref()
                        .map(|p| p.to_lowercase().contains(&search_lower))
                        .unwrap_or(false);
                if !matches {
                    return false;
                }
            }
            // Impact filter
            if let Some(filter) = &app.startup_filter_impact
                && item.impact_tier != *filter
            {
                return false;
            }
            // Signed filter
            if let Some(filter_signed) = app.startup_filter_signed
                && item.is_signed != Some(filter_signed)
            {
                return false;
            }
            // Broken filter
            if app.startup_filter_broken && item.exe_exists {
                return false;
            }
            true
        })
        .map(|(i, _)| i)
        .collect();

    // Sort the filtered view
    let items_ref = &app.startup_items;
    let sort_col = app.startup_sort;
    let ascending = app.startup_sort_ascending;
    filtered_indices.sort_by(|a, b| {
        let ia = &items_ref[*a];
        let ib = &items_ref[*b];
        let cmp = match sort_col {
            StartupSortColumn::Name => ia.name.to_lowercase().cmp(&ib.name.to_lowercase()),
            StartupSortColumn::Impact => ia.impact_tier.sort_key().cmp(&ib.impact_tier.sort_key()),
            StartupSortColumn::Source => ia.source.cmp(&ib.source),
            StartupSortColumn::Publisher => {
                let pa = ia.publisher.as_deref().unwrap_or("zzz").to_lowercase();
                let pb = ib.publisher.as_deref().unwrap_or("zzz").to_lowercase();
                pa.cmp(&pb)
            }
        };
        if ascending { cmp } else { cmp.reverse() }
    });

    filtered_indices
}

pub(crate) use page::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::engine::SystemMonitorApp;
    use crate::startup::Recommendation;

    #[test]
    fn test_impact_tier_badge_colors() {
        let (label_high, color_high) = impact_tier_badge_color(&ImpactTier::High, true);
        assert_eq!(label_high, "HIGH");
        assert_eq!(color_high, ThemePalette::STATUS_CRITICAL);

        let (label_med, color_med) = impact_tier_badge_color(&ImpactTier::Medium, true);
        assert_eq!(label_med, "MED");
        assert_eq!(color_med, ThemePalette::STATUS_WARNING);

        let (label_low, color_low) = impact_tier_badge_color(&ImpactTier::Low, true);
        assert_eq!(label_low, "LOW");
        assert_eq!(color_low, ThemePalette::STATUS_HEALTHY);
    }

    #[test]
    fn test_startup_manager_render_all_states() {
        let mut app = SystemMonitorApp::test_app();
        app.startup_items_loaded = true;
        app.startup_items_loading = false;
        app.startup_items = vec![
            crate::startup::StartupItem {
                name: "Test App Normal".into(),
                command: r#""C:\Program Files\Test\app.exe" --silent"#.into(),
                enabled: true,
                source: "Registry (HKCU)".into(),
                locator: Default::default(),
                exe_path: Some(r#"C:\Program Files\Test\app.exe"#.into()),
                exe_exists: true,
                publisher: Some("Test Corp".into()),
                is_signed: Some(true),
                impact_tier: ImpactTier::High,
                recommendation: Recommendation::Keep,
                reason: "Test reason".into(),
            },
            crate::startup::StartupItem {
                name: "Broken App <Unicode> 日本語".into(),
                command: r#"C:\NonExistent\broken.exe"#.into(),
                enabled: false,
                source: "Task Scheduler".into(),
                locator: Default::default(),
                exe_path: Some(r#"C:\NonExistent\broken.exe"#.into()),
                exe_exists: false,
                publisher: None,
                is_signed: Some(false),
                impact_tier: ImpactTier::High,
                recommendation: Recommendation::Cleanup,
                reason: "File not found".into(),
            },
            crate::startup::StartupItem {
                name: "Unsigned App with Null\0Byte".into(),
                command: "C:\\App\\app.exe\0--arg".into(),
                enabled: true,
                source: "Startup Folder (User)".into(),
                locator: Default::default(),
                exe_path: None,
                exe_exists: false,
                publisher: Some("Unknown Pub".into()),
                is_signed: None,
                impact_tier: ImpactTier::Medium,
                recommendation: Recommendation::Review,
                reason: "Review for necessity".into(),
            },
        ];

        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                show(&mut app, ui);
            });
        })
        .textures_delta
        .clear();

        // Test with confirmation dialog open
        app.startup_show_confirm = Some("Test App Normal".into());
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                show(&mut app, ui);
            });
        })
        .textures_delta
        .clear();

        // Test with empty items and filters active
        app.startup_search = "NonExistentFilter999".into();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                show(&mut app, ui);
            });
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn test_filter_and_sort_indices() {
        let mut app = SystemMonitorApp::test_app();
        app.startup_items = vec![
            crate::startup::StartupItem {
                name: "Alpha App".into(),
                command: "alpha.exe".into(),
                enabled: true,
                source: "Registry (HKCU)".into(),
                locator: Default::default(),
                exe_path: None,
                exe_exists: true,
                publisher: Some("Corp A".into()),
                is_signed: Some(true),
                impact_tier: ImpactTier::Low,
                recommendation: Recommendation::Keep,
                reason: "Low impact".into(),
            },
            crate::startup::StartupItem {
                name: "Beta App".into(),
                command: "beta.exe".into(),
                enabled: true,
                source: "Registry (HKLM)".into(),
                locator: Default::default(),
                exe_path: None,
                exe_exists: true,
                publisher: Some("Corp B".into()),
                is_signed: Some(false),
                impact_tier: ImpactTier::High,
                recommendation: Recommendation::Review,
                reason: "High impact".into(),
            },
        ];

        app.startup_sort = StartupSortColumn::Impact;
        app.startup_sort_ascending = true;
        let indices = filter_and_sort_indices(&app);
        assert_eq!(indices.len(), 2);

        // Filter by impact High
        app.startup_filter_impact = Some(ImpactTier::High);
        let indices = filter_and_sort_indices(&app);
        assert_eq!(indices, vec![1]);

        // Filter by search
        app.startup_filter_impact = None;
        app.startup_search = "Alpha".into();
        let indices = filter_and_sort_indices(&app);
        assert_eq!(indices, vec![0]);
    }
}
