use eframe::egui;

pub(crate) struct ThemePalette;
impl ThemePalette {
    // Single Diagnostic Accent
    pub(crate) const ACCENT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(16, 185, 129); // #10B981 Diagnostic Emerald
    pub(crate) const ACCENT_ACTIVE: egui::Color32 = egui::Color32::from_rgb(5, 150, 105); // Emerald-600

    // Cockpit Dense Backgrounds
    pub(crate) const BG_DEEPEST: egui::Color32 = egui::Color32::from_rgb(9, 9, 11); // #09090B Terminal Base
    pub(crate) const BG_DEEP: egui::Color32 = egui::Color32::from_rgb(9, 9, 11);
    pub(crate) const BG_SURFACE: egui::Color32 = egui::Color32::from_rgb(24, 24, 27); // #18181B Deep Surface
    pub(crate) const BG_CARD: egui::Color32 = egui::Color32::from_rgb(24, 24, 27);
    pub(crate) const BG_TRACK: egui::Color32 = egui::Color32::from_rgb(39, 39, 42); // Zinc-800

    // Component states
    pub(crate) const WIDGET_INACTIVE: egui::Color32 = egui::Color32::from_rgb(39, 39, 42);
    pub(crate) const WIDGET_HOVERED: egui::Color32 = egui::Color32::from_rgb(63, 63, 70); // Zinc-700
    pub(crate) const BORDER: egui::Color32 = egui::Color32::from_rgba_premultiplied(25, 25, 25, 25); // 10% white equivalent
    pub(crate) const BORDER_LIGHT: egui::Color32 = egui::Color32::from_rgba_premultiplied(12, 12, 12, 12);

    // Semantic Status
    pub(crate) const STATUS_HEALTHY: egui::Color32 = egui::Color32::from_rgb(16, 185, 129); // #10B981
    pub(crate) const STATUS_WARNING: egui::Color32 = egui::Color32::from_rgb(245, 158, 11); // Amber-500
    pub(crate) const STATUS_CRITICAL: egui::Color32 = egui::Color32::from_rgb(239, 68, 68); // #EF4444

    // Typography (Zinc Scale)
    pub(crate) const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(244, 244, 245); // #F4F4F5 Primary Ink
    pub(crate) const TEXT_SELECTED: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
    pub(crate) const TEXT_FEATURE: egui::Color32 = egui::Color32::from_rgb(228, 228, 231); // Zinc-200
    pub(crate) const TEXT_SUBTITLE: egui::Color32 = egui::Color32::from_rgb(161, 161, 170); // #A1A1AA Muted Steel
    pub(crate) const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(161, 161, 170);
    pub(crate) const TEXT_LABEL: egui::Color32 = egui::Color32::from_rgb(113, 113, 122); // Zinc-500
    pub(crate) const TEXT_LABEL_SUB: egui::Color32 = egui::Color32::from_rgb(113, 113, 122);
    pub(crate) const TEXT_TERTIARY: egui::Color32 = egui::Color32::from_rgb(82, 82, 91); // Zinc-600
    pub(crate) const TEXT_DIMMED: egui::Color32 = egui::Color32::from_rgb(82, 82, 91);

    pub(crate) const GPU_UNAVAILABLE: egui::Color32 = egui::Color32::from_rgb(82, 82, 91);

    // Legacy colors remapped to strict palette to avoid compilation errors
    pub(crate) const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(161, 161, 170); // Remapped to Muted Steel
    pub(crate) const ACCENT_CYAN: egui::Color32 = egui::Color32::from_rgb(228, 228, 231); // Remapped to Zinc-200
}
