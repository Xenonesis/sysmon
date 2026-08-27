use super::*;
use crate::services::ServiceSortColumn;
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

#[test]
fn sort_header_indicators_are_preserved() {
    assert_eq!(
        sort_header_label(
            "Display Name",
            ServiceSortColumn::DisplayName,
            ServiceSortColumn::DisplayName,
            true,
        ),
        "Display Name ▲"
    );
    assert_eq!(
        sort_header_label(
            "Display Name",
            ServiceSortColumn::DisplayName,
            ServiceSortColumn::DisplayName,
            false,
        ),
        "Display Name ▼"
    );
    assert_eq!(
        sort_header_label("State", ServiceSortColumn::State, ServiceSortColumn::DisplayName, true,),
        "State"
    );
}

#[test]
fn services_page_renders_empty_populated_filtered_and_selected_states() {
    let mut app = crate::SystemMonitorApp::test_app();
    let mut data = SystemData::default();
    let context = egui::Context::default();

    render(&context, &mut app, &data, false);

    data.services = vec![
        service("ADBCSvc", "Acer Display Backlight Control Service", "Running"),
        service("BITS", "Background Intelligent Transfer Service", "Running"),
        service("AppIDSvc", "Application Identity", "Stopped"),
    ];
    render(&context, &mut app, &data, false);

    app.service_page.search = "backlight".to_string();
    render(&context, &mut app, &data, false);

    app.service_page.state_filter = Some("Stopped".to_string());
    render(&context, &mut app, &data, false);

    app.service_page.selected_name = Some("BITS".to_string());
    render(&context, &mut app, &data, false);
    render(&context, &mut app, &data, true);
}

fn service(name: &str, display_name: &str, state: &str) -> crate::services::ServiceInfo {
    crate::services::ServiceInfo {
        name: name.to_string(),
        display_name: display_name.to_string(),
        state: state.to_string(),
    }
}

fn render(context: &egui::Context, app: &mut crate::SystemMonitorApp, data: &SystemData, elevated: bool) {
    context
        .run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let intents = show(app, ui, data, elevated);
                assert!(intents.is_empty());
            });
        })
        .textures_delta
        .clear();
}
