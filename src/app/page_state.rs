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

use crate::storage::file_locks::{FileLockResult, find_locking_processes};
use crate::storage::reclaimer::{ReclaimCategory, ReviewedCleanup, scan_reclaimable_caches};
use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

#[derive(Debug)]
struct StorageJob<T> {
    generation: u64,
    cancel: Arc<AtomicBool>,
    receiver: mpsc::Receiver<T>,
}
impl<T> Drop for StorageJob<T> {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Debug)]
pub(crate) struct StoragePageState {
    pub(crate) lock_path: String,
    pub(crate) lock_result: Option<FileLockResult>,
    pub(crate) lock_status: Option<String>,
    pub(crate) reclaimer_categories: Vec<ReclaimCategory>,
    pub(crate) reclaimer_selected: HashSet<String>,
    pub(crate) reclaimer_scanned: bool,
    pub(crate) reclaimer_status: Option<String>,
    lock_generation: u64,
    scan_generation: u64,
    lock_job: Option<StorageJob<FileLockResult>>,
    scan_job: Option<StorageJob<Vec<ReclaimCategory>>>,
}

impl Default for StoragePageState {
    fn default() -> Self {
        Self {
            lock_path: String::new(),
            lock_result: None,
            lock_status: None,
            reclaimer_categories: Vec::new(),
            reclaimer_selected: ["shader_cache", "user_temp", "crash_dumps"]
                .into_iter()
                .map(String::from)
                .collect(),
            reclaimer_scanned: false,
            reclaimer_status: None,
            lock_generation: 0,
            scan_generation: 0,
            lock_job: None,
            scan_job: None,
        }
    }
}

impl StoragePageState {
    pub(crate) fn scan_busy(&self) -> bool {
        self.scan_job.is_some()
    }
    pub(crate) fn lock_busy(&self) -> bool {
        self.lock_job.is_some()
    }

    pub(crate) fn scan_caches(&mut self) {
        if self.scan_job.is_some() {
            return;
        }
        self.scan_generation = self.scan_generation.wrapping_add(1);
        self.reclaimer_scanned = true;
        self.reclaimer_categories.clear();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let (sender, receiver) = mpsc::sync_channel(1);
        match std::thread::Builder::new()
            .name("storage-preview".into())
            .spawn(move || {
                let _ = sender.send(scan_reclaimable_caches(&worker_cancel));
            }) {
            Ok(_) => {
                self.scan_job = Some(StorageJob {
                    generation: self.scan_generation,
                    cancel,
                    receiver,
                });
                self.reclaimer_status
                    .get_or_insert_with(|| "Scanning eligible files in the background...".into());
            }
            Err(error) => self.reclaimer_status = Some(format!("Cannot start scan: {error}")),
        }
    }

    pub(crate) fn inspect_locks(&mut self) {
        if self.lock_job.is_some() {
            return;
        }
        let path = self.lock_path.trim().to_string();
        if path.is_empty() {
            self.lock_status = Some("Please provide an absolute file, folder, or volume path.".into());
            self.lock_result = None;
            return;
        }
        self.lock_generation = self.lock_generation.wrapping_add(1);
        self.lock_result = None;
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let (sender, receiver) = mpsc::sync_channel(1);
        match std::thread::Builder::new().name("storage-locks".into()).spawn(move || {
            let _ = sender.send(find_locking_processes(&path, &worker_cancel));
        }) {
            Ok(_) => {
                self.lock_job = Some(StorageJob {
                    generation: self.lock_generation,
                    cancel,
                    receiver,
                });
                self.lock_status =
                    Some("Inspecting in background; cancellation takes effect between native calls.".into());
            }
            Err(error) => self.lock_status = Some(format!("Cannot start inspection: {error}")),
        }
    }

    pub(crate) fn cancel_scan(&mut self) {
        if let Some(job) = &self.scan_job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.scan_generation = self.scan_generation.wrapping_add(1);
        self.reclaimer_categories.clear();
        self.reclaimer_status = Some("Scan cancelled; no cleanup was performed. Rescan to review files.".into());
    }
    pub(crate) fn cancel_inspection(&mut self) {
        if let Some(job) = &self.lock_job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.lock_generation = self.lock_generation.wrapping_add(1);
        self.lock_result = None;
        self.lock_status = Some("Inspection cancelled.".into());
    }

    pub(crate) fn poll_background(&mut self, ctx: &eframe::egui::Context) {
        if let Some(job) = &self.scan_job {
            match job.receiver.try_recv() {
                Ok(categories) => {
                    if job.generation == self.scan_generation {
                        self.reclaimer_categories = categories;
                        if self
                            .reclaimer_status
                            .as_deref()
                            .is_none_or(|s| s.starts_with("Scanning"))
                        {
                            self.reclaimer_status = Some(
                                "Scan complete. Review eligible counts and exclusions before permanent deletion."
                                    .into(),
                            );
                        }
                    }
                    self.scan_job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.scan_job = None;
                    self.reclaimer_status = Some("Scan worker stopped without a result.".into());
                }
                Err(mpsc::TryRecvError::Empty) => (),
            }
        }
        if let Some(job) = &self.lock_job {
            match job.receiver.try_recv() {
                Ok(result) => {
                    if job.generation == self.lock_generation && result.path == self.lock_path.trim() {
                        self.lock_status = result.error.clone();
                        self.lock_result = Some(result);
                    }
                    self.lock_job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.lock_job = None;
                    self.lock_status = Some("Inspection worker stopped without a result.".into());
                }
                Err(mpsc::TryRecvError::Empty) => (),
            }
        }
        if self.scan_busy() || self.lock_busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    pub(crate) fn cleanup_finished(&mut self, summary: String) {
        self.cancel_scan();
        self.reclaimer_scanned = false;
        self.reclaimer_status = Some(summary);
        self.locks_invalidated();
    }
    pub(crate) fn cleanup_cancelled(&mut self) {
        self.reclaimer_status = Some("Cleanup cancelled before execution; no files were removed.".into());
    }
    pub(crate) fn locks_invalidated(&mut self) {
        self.cancel_inspection();
        self.lock_status =
            Some("Previous lock results invalidated by an action. Inspect again for current users.".into());
    }
    pub(crate) fn toggle_category(&mut self, id: &str) {
        if !self.reclaimer_selected.remove(id) {
            self.reclaimer_selected.insert(id.to_string());
        }
    }
    pub(crate) fn reviewed_cleanup(&self) -> ReviewedCleanup {
        ReviewedCleanup::from_categories(&self.reclaimer_categories, &self.reclaimer_selected)
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
