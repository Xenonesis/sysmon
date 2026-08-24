use crate::services::{self, ServiceInfo, ServiceSortColumn};

#[derive(Debug, Clone)]
pub(crate) struct ServicePageState {
    pub(crate) selected_name: Option<String>,
    pub(crate) search: String,
    pub(crate) state_filter: Option<String>,
    pub(crate) sort_column: ServiceSortColumn,
    pub(crate) sort_ascending: bool,
}

impl Default for ServicePageState {
    fn default() -> Self {
        Self {
            selected_name: None,
            search: String::new(),
            state_filter: None,
            sort_column: ServiceSortColumn::DisplayName,
            sort_ascending: true,
        }
    }
}

impl ServicePageState {
    pub(crate) fn visible_services<'a>(&self, items: &'a [ServiceInfo]) -> Vec<&'a ServiceInfo> {
        let query = self.search.to_lowercase();
        let mut visible: Vec<_> = items
            .iter()
            .filter(|service| {
                let name_matches = query.is_empty()
                    || service.name.to_lowercase().contains(&query)
                    || service.display_name.to_lowercase().contains(&query);
                let state_matches = self
                    .state_filter
                    .as_deref()
                    .is_none_or(|state| service.state.eq_ignore_ascii_case(state));
                name_matches && state_matches
            })
            .collect();
        services::sort_services_refs(&mut visible, self.sort_column, self.sort_ascending);
        visible
    }

    pub(crate) fn select_sort(&mut self, column: ServiceSortColumn) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
    }

    pub(crate) fn reset_filters(&mut self) {
        self.search.clear();
        self.state_filter = None;
    }

    pub(crate) fn toggle_selected(&mut self, name: &str) {
        if self.selected_name.as_deref() == Some(name) {
            self.selected_name = None;
        } else {
            self.selected_name = Some(name.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(name: &str, display_name: &str, state: &str) -> ServiceInfo {
        ServiceInfo {
            name: name.to_string(),
            display_name: display_name.to_string(),
            state: state.to_string(),
        }
    }

    #[test]
    fn service_page_filters_and_sorts_without_ui() {
        let items = vec![
            service("BITS", "Background Transfer", "Running"),
            service("AppIDSvc", "Application Identity", "Stopped"),
            service("AarSvc", "Agent Runtime", "Running"),
        ];
        let mut state = ServicePageState {
            search: "a".to_string(),
            state_filter: Some("Running".to_string()),
            ..Default::default()
        };

        let names: Vec<_> = state
            .visible_services(&items)
            .iter()
            .map(|item| item.name.as_str())
            .collect();
        assert_eq!(names, ["AarSvc", "BITS"]);

        state.select_sort(ServiceSortColumn::DisplayName);
        let names: Vec<_> = state
            .visible_services(&items)
            .iter()
            .map(|item| item.name.as_str())
            .collect();
        assert_eq!(names, ["BITS", "AarSvc"]);
    }

    #[test]
    fn service_page_selection_and_reset_are_deterministic() {
        let mut state = ServicePageState {
            search: "bits".to_string(),
            state_filter: Some("Running".to_string()),
            ..Default::default()
        };

        state.toggle_selected("BITS");
        assert_eq!(state.selected_name.as_deref(), Some("BITS"));
        state.toggle_selected("BITS");
        assert!(state.selected_name.is_none());

        state.reset_filters();
        assert!(state.search.is_empty());
        assert!(state.state_filter.is_none());
    }
}
