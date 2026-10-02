use super::*;
use crate::ui::theme::ThemePalette;

#[test]
fn service_state_color_mapping_is_preserved() {
    assert_eq!(service_state_color("Running", true), ThemePalette::STATUS_HEALTHY);
    assert_eq!(service_state_color("RUNNING", false), ThemePalette::STATUS_HEALTHY);
    assert_eq!(service_state_color("Stopped", true), ThemePalette::text_dimmed(true));
    assert_eq!(service_state_color("Paused", true), ThemePalette::STATUS_WARNING);
    assert_eq!(service_state_color("Start Pending", true), ThemePalette::STATUS_WARNING);
    assert_eq!(service_state_color("Stop Pending", false), ThemePalette::STATUS_WARNING);
}
