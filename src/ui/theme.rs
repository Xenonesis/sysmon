use eframe::egui;

pub(crate) struct ThemePalette;
impl ThemePalette {
    // Primary Vibrant Accents -> Muted Minimalist Primary
    pub(crate) const ACCENT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(198, 198, 199); // #c6c6c7
    pub(crate) const ACCENT_ACTIVE: egui::Color32 = egui::Color32::from_rgb(226, 226, 226); // #e2e2e2

    // Sleek Dark Backgrounds -> Graphite Core
    pub(crate) const BG_DEEPEST: egui::Color32 = egui::Color32::from_rgb(14, 14, 14); // #0e0e0e
    pub(crate) const BG_DEEP: egui::Color32 = egui::Color32::from_rgb(14, 14, 14); 
    pub(crate) const BG_SURFACE: egui::Color32 = egui::Color32::from_rgb(19, 19, 19); // #131313
    pub(crate) const BG_CARD: egui::Color32 = egui::Color32::from_rgb(19, 19, 19);
    pub(crate) const BG_TRACK: egui::Color32 = egui::Color32::from_rgb(31, 32, 32); // #1f2020

    // Component states
    pub(crate) const WIDGET_INACTIVE: egui::Color32 = egui::Color32::from_rgb(31, 32, 32); // #1f2020
    pub(crate) const WIDGET_HOVERED: egui::Color32 = egui::Color32::from_rgb(37, 38, 38); // #252626
    pub(crate) const BORDER: egui::Color32 = egui::Color32::from_rgb(19, 19, 19); // Hidden in #131313
    pub(crate) const BORDER_LIGHT: egui::Color32 = egui::Color32::from_rgb(31, 32, 32); // Just slight edge

    // Modern Status Colors -> Minimalist Status
    pub(crate) const STATUS_HEALTHY: egui::Color32 = egui::Color32::from_rgb(230, 255, 244); // #e6fff4
    pub(crate) const STATUS_WARNING: egui::Color32 = egui::Color32::from_rgb(192, 191, 191); // Soft grey
    pub(crate) const STATUS_CRITICAL: egui::Color32 = egui::Color32::from_rgb(238, 125, 119); // #ee7d77

    // Gorgeous Typography hierarchy -> Crisp and Stark
    pub(crate) const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(255, 255, 255); // Stark white
    pub(crate) const TEXT_SELECTED: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
    pub(crate) const TEXT_FEATURE: egui::Color32 = egui::Color32::from_rgb(231, 229, 229); // #e7e5e5
    pub(crate) const TEXT_SUBTITLE: egui::Color32 = egui::Color32::from_rgb(172, 171, 170); // #acabaa
    pub(crate) const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(172, 171, 170); 
    pub(crate) const TEXT_LABEL: egui::Color32 = egui::Color32::from_rgb(118, 117, 117); // #767575
    pub(crate) const TEXT_LABEL_SUB: egui::Color32 = egui::Color32::from_rgb(118, 117, 117); 
    pub(crate) const TEXT_TERTIARY: egui::Color32 = egui::Color32::from_rgb(86, 85, 85); // #565555
    pub(crate) const TEXT_DIMMED: egui::Color32 = egui::Color32::from_rgb(86, 85, 85);

    pub(crate) const GPU_UNAVAILABLE: egui::Color32 = egui::Color32::from_rgb(86, 85, 85);
    pub(crate) const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(198, 198, 199); // Map purple to primary grey
    pub(crate) const ACCENT_CYAN: egui::Color32 = egui::Color32::from_rgb(198, 198, 199); // Map cyan to primary grey
}
