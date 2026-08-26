use crate::app::models::AppTheme;
use eframe::egui;

pub(crate) struct ThemePalette;
#[allow(dead_code)]
impl ThemePalette {
    pub(crate) fn is_dark_mode(theme: AppTheme) -> bool {
        match theme {
            AppTheme::Dark => true,
            AppTheme::Light => false,
            AppTheme::System => is_windows_dark_mode(),
        }
    }

    pub(crate) fn is_dark(theme: AppTheme) -> bool {
        Self::is_dark_mode(theme)
    }

    pub(crate) fn bg_deepest(is_dark: bool) -> egui::Color32 {
        if is_dark {
            egui::Color32::from_rgb(9, 9, 11) // #09090B Terminal Base
        } else {
            egui::Color32::from_rgb(244, 244, 245) // #F4F4F5 Clean Slate
        }
    }

    pub(crate) fn bg_surface(is_dark: bool) -> egui::Color32 {
        if is_dark {
            egui::Color32::from_rgb(24, 24, 27) // #18181B Deep Surface
        } else {
            egui::Color32::from_rgb(255, 255, 255) // Pure White
        }
    }

    pub(crate) fn bg_card(is_dark: bool) -> egui::Color32 {
        if is_dark {
            egui::Color32::from_rgb(24, 24, 27)
        } else {
            egui::Color32::from_rgb(255, 255, 255)
        }
    }

    pub(crate) fn bg_track(is_dark: bool) -> egui::Color32 {
        if is_dark {
            egui::Color32::from_rgb(39, 39, 42) // Zinc-800
        } else {
            egui::Color32::from_rgb(228, 228, 231) // Zinc-200
        }
    }

    pub(crate) fn border(is_dark: bool) -> egui::Color32 {
        if is_dark {
            egui::Color32::from_rgba_premultiplied(25, 25, 25, 25)
        } else {
            egui::Color32::from_rgb(228, 228, 231) // Zinc-200
        }
    }

    pub(crate) fn text_primary(is_dark: bool) -> egui::Color32 {
        if is_dark {
            egui::Color32::from_rgb(244, 244, 245) // #F4F4F5 Primary Ink
        } else {
            egui::Color32::from_rgb(9, 9, 11) // #09090B Ink Black
        }
    }

    pub(crate) fn text_secondary(is_dark: bool) -> egui::Color32 {
        if is_dark {
            egui::Color32::from_rgb(161, 161, 170) // #A1A1AA Muted Steel
        } else {
            egui::Color32::from_rgb(113, 113, 122) // Zinc-500
        }
    }

    pub(crate) fn text_dimmed(is_dark: bool) -> egui::Color32 {
        if is_dark {
            egui::Color32::from_rgb(82, 82, 91) // Zinc-600
        } else {
            egui::Color32::from_rgb(161, 161, 170) // Zinc-400
        }
    }

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

#[cfg(target_os = "windows")]
pub fn is_windows_dark_mode() -> bool {
    use winreg::RegKey;
    use winreg::enums::*;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        && let Ok(val) = key.get_value::<u32, _>("AppsUseLightTheme")
    {
        return val == 0;
    }
    true // default to dark if unreadable
}

#[cfg(not(target_os = "windows"))]
pub fn is_windows_dark_mode() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::models::AppTheme;

    #[test]
    fn theme_resolution_works() {
        assert!(ThemePalette::is_dark_mode(AppTheme::Dark));
        assert!(!ThemePalette::is_dark_mode(AppTheme::Light));
        assert!(ThemePalette::is_dark(AppTheme::Dark));
        assert!(!ThemePalette::is_dark(AppTheme::Light));
        // System mode resolves cleanly without panicking
        let _ = ThemePalette::is_dark_mode(AppTheme::System);
    }

    #[test]
    fn dynamic_palette_contrast() {
        let dark_bg = ThemePalette::bg_deepest(true);
        let dark_text = ThemePalette::text_primary(true);
        assert_ne!(dark_bg, dark_text);

        let light_bg = ThemePalette::bg_deepest(false);
        let light_text = ThemePalette::text_primary(false);
        assert_ne!(light_bg, light_text);

        let dark_surface = ThemePalette::bg_surface(true);
        let dark_card = ThemePalette::bg_card(true);
        assert_eq!(dark_surface, dark_card);

        let light_surface = ThemePalette::bg_surface(false);
        let light_card = ThemePalette::bg_card(false);
        assert_eq!(light_surface, light_card);

        let dark_track = ThemePalette::bg_track(true);
        let light_track = ThemePalette::bg_track(false);
        assert_ne!(dark_track, light_track);

        let dark_border = ThemePalette::border(true);
        let light_border = ThemePalette::border(false);
        assert_ne!(dark_border, light_border);

        let dark_sec = ThemePalette::text_secondary(true);
        let light_sec = ThemePalette::text_secondary(false);
        assert_ne!(dark_sec, light_sec);

        let dark_dim = ThemePalette::text_dimmed(true);
        let light_dim = ThemePalette::text_dimmed(false);
        assert_ne!(dark_dim, light_dim);
    }
}
