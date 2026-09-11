//! Process monitoring: models and pure logic (no OS-tied execution).

use serde::Serialize;
use std::collections::HashMap;
use sysinfo::{Pid, System};

// ─── Data Models ─────────────────────────────────────────────

/// Windows process lifetime token. Creation time is native FILETIME ticks, not Unix seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, serde::Deserialize)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub creation_time: u64,
}

impl std::fmt::Display for ProcessIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (created {})", self.pid, self.creation_time)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AffinityPreset {
    All,
    First,
    Second,
    FirstHalf,
}

pub fn affinity_mask(allowed: usize, preset: AffinityPreset) -> Result<usize, String> {
    if allowed == 0 {
        return Err("No supported processor affinity mask is available".into());
    }
    let count = match preset {
        AffinityPreset::All => return Ok(allowed),
        AffinityPreset::First => 1,
        AffinityPreset::Second => {
            let rest = allowed & (allowed - 1);
            return (rest != 0)
                .then(|| rest & rest.wrapping_neg())
                .ok_or_else(|| "A second allowed processor is unavailable".into());
        }
        AffinityPreset::FirstHalf => (allowed.count_ones() / 2).max(1),
    };
    let mut rest = allowed;
    let mut mask = 0;
    for _ in 0..count {
        let bit = rest & rest.wrapping_neg();
        mask |= bit;
        rest &= !bit;
    }
    Ok(mask)
}

pub(crate) fn validate_process_identity(expected: ProcessIdentity, actual: ProcessIdentity) -> Result<(), String> {
    if expected != actual || expected.creation_time == 0 {
        return Err(format!("Stale process target {expected}; current identity is {actual}"));
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) struct ProcessHandle(pub windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
pub(crate) fn handle_identity(handle: &ProcessHandle, pid: u32) -> Result<ProcessIdentity, String> {
    use windows_sys::Win32::{Foundation::FILETIME, System::Threading::GetProcessTimes};
    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    if unsafe { GetProcessTimes(handle.0, &mut created, &mut exited, &mut kernel, &mut user) } == 0 {
        return Err(format!("GetProcessTimes({pid}): {}", std::io::Error::last_os_error()));
    }
    Ok(ProcessIdentity {
        pid,
        creation_time: ((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64,
    })
}

#[cfg(windows)]
pub fn process_identity(pid: u32) -> Result<ProcessIdentity, String> {
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if raw.is_null() {
        return Err(format!("OpenProcess({pid}): {}", std::io::Error::last_os_error()));
    }
    handle_identity(&ProcessHandle(raw), pid)
}

#[cfg(not(windows))]
pub fn process_identity(_pid: u32) -> Result<ProcessIdentity, String> {
    Err("Native process identity is unavailable on this platform".into())
}

pub(crate) fn guard_process_target(identity: ProcessIdentity, own_pid: u32, critical: bool) -> Result<(), String> {
    if identity.pid == own_pid || identity.pid <= 4 || critical {
        return Err(format!("Refusing mutation of self or critical process {identity}"));
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn open_process_for_action(identity: ProcessIdentity, access: u32) -> Result<ProcessHandle, String> {
    use windows_sys::Win32::System::Threading::{IsProcessCritical, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    guard_process_target(identity, std::process::id(), false)?;
    let raw = unsafe { OpenProcess(access | PROCESS_QUERY_LIMITED_INFORMATION, 0, identity.pid) };
    if raw.is_null() {
        return Err(format!("OpenProcess({identity}): {}", std::io::Error::last_os_error()));
    }
    let handle = ProcessHandle(raw);
    validate_process_identity(identity, handle_identity(&handle, identity.pid)?)?;
    let mut critical = 0;
    if unsafe { IsProcessCritical(handle.0, &mut critical) } == 0 {
        return Err(format!(
            "IsProcessCritical({identity}): {}",
            std::io::Error::last_os_error()
        ));
    }
    guard_process_target(identity, std::process::id(), critical != 0)?;
    Ok(handle)
}

/// Build a lifetime-validated deepest-first snapshot. No PID-only targets escape this boundary.
pub fn process_tree_targets(
    root: ProcessIdentity,
    snapshot: &[(ProcessIdentity, Option<u32>)],
) -> Result<Vec<ProcessIdentity>, String> {
    let identities: HashMap<_, _> = snapshot.iter().map(|(identity, _)| (identity.pid, *identity)).collect();
    let current = identities
        .get(&root.pid)
        .ok_or_else(|| "Root process is no longer available".to_string())?;
    validate_process_identity(root, *current)?;
    let parents = snapshot
        .iter()
        .filter_map(|(child, parent)| {
            let parent = identities.get(&(*parent)?)?;
            (parent.creation_time < child.creation_time).then_some((child.pid, parent.pid))
        })
        .collect();
    Ok(kill_order(&build_tree(&parents), root.pid)
        .into_iter()
        .filter_map(|pid| identities.get(&pid).copied())
        .collect())
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub identity: Option<ProcessIdentity>,
    pub start_time: u64,
    pub name: String,
    pub parent_pid: Option<u32>,
    pub cpu_usage: f32,
    pub memory: u64,
    pub vram_bytes: Option<u64>,
    pub status: String,
    pub disk_read_bytes: u64,
    pub disk_written_bytes: u64,
    pub disk_read_bytes_per_second: Option<f64>,
    pub disk_written_bytes_per_second: Option<f64>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum ProcessSortColumn {
    Pid,
    Name,
    Memory,
    Vram,
    Cpu,
    Disk,
}

#[derive(Clone)]
pub struct ProcessDetails {
    pub exe_path: Option<String>,
    pub command_line: String,
    pub cwd: Option<String>,
    pub start_time: u64,
    pub run_time: u64,
    pub parent_pid: Option<u32>,
    pub parent_name: Option<String>,
}

// ─── Pure Logic ──────────────────────────────────────────────

/// Case-insensitive substring filter on name or PID. Empty query returns all.
pub fn filter_processes<'a>(items: &'a [ProcessInfo], query: &str) -> Vec<&'a ProcessInfo> {
    if query.is_empty() {
        return items.iter().collect();
    }
    let q = query.to_lowercase();
    items
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&q) || p.pid.to_string().contains(&q))
        .collect()
}

/// In-place sort of references. `Status` falls back to memory (no status ordering defined).
pub fn sort_processes_refs(items: &mut [&ProcessInfo], column: ProcessSortColumn, ascending: bool) {
    fn ord(o: std::cmp::Ordering, ascending: bool) -> std::cmp::Ordering {
        if ascending { o } else { o.reverse() }
    }

    items.sort_by(|a, b| match column {
        ProcessSortColumn::Pid => ord(a.pid.cmp(&b.pid), ascending),
        ProcessSortColumn::Name => ord(a.name.to_lowercase().cmp(&b.name.to_lowercase()), ascending),
        ProcessSortColumn::Cpu => ord(
            a.cpu_usage
                .partial_cmp(&b.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal),
            ascending,
        ),
        ProcessSortColumn::Memory => ord(a.memory.cmp(&b.memory), ascending),
        ProcessSortColumn::Vram => ord(a.vram_bytes.unwrap_or(0).cmp(&b.vram_bytes.unwrap_or(0)), ascending),
        ProcessSortColumn::Disk => ord(
            (a.disk_read_bytes + a.disk_written_bytes).cmp(&(b.disk_read_bytes + b.disk_written_bytes)),
            ascending,
        ),
    });
}
/// Build pid -> [child pids] adjacency from a pid -> parent_pid map.
pub fn build_tree(parent_map: &HashMap<u32, u32>) -> HashMap<u32, Vec<u32>> {
    let mut tree: HashMap<u32, Vec<u32>> = HashMap::new();
    for (&pid, &parent) in parent_map {
        tree.entry(parent).or_default().push(pid);
    }
    for children in tree.values_mut() {
        children.sort_unstable();
    }
    tree
}
/// Deepest-first kill order (children before parents), cycle-safe, orphan-safe.
pub fn kill_order(tree: &HashMap<u32, Vec<u32>>, root_pid: u32) -> Vec<u32> {
    fn visit(
        pid: u32,
        tree: &HashMap<u32, Vec<u32>>,
        visited: &mut std::collections::HashSet<u32>,
        order: &mut Vec<u32>,
    ) {
        if !visited.insert(pid) {
            return;
        }
        if let Some(children) = tree.get(&pid) {
            for &child in children {
                visit(child, tree, visited, order);
            }
        }
        order.push(pid);
    }
    let mut order = Vec::new();
    let mut visited = std::collections::HashSet::new();
    visit(root_pid, tree, &mut visited, &mut order);
    order
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessTreeRow {
    pub process: ProcessInfo,
    pub depth: usize,
    pub has_children: bool,
    pub prefix: String,
}

/// Build ordered hierarchical tree rows with indentation and visual branches.
pub fn build_tree_rows(items: &[ProcessInfo], tree: &HashMap<u32, Vec<u32>>, query: &str) -> Vec<ProcessTreeRow> {
    let item_map: HashMap<u32, &ProcessInfo> = items.iter().map(|p| (p.pid, p)).collect();
    let mut rows = Vec::new();
    let mut visited = std::collections::HashSet::new();

    let mut root_pids: Vec<u32> = items
        .iter()
        .map(|p| p.pid)
        .filter(|&pid| {
            !items.iter().any(|parent| {
                if let Some(children) = tree.get(&parent.pid) {
                    children.contains(&pid)
                } else {
                    false
                }
            })
        })
        .collect();
    root_pids.sort_unstable();

    #[allow(clippy::too_many_arguments)]
    fn traverse(
        pid: u32,
        depth: usize,
        prefix: &str,
        is_last: bool,
        item_map: &HashMap<u32, &ProcessInfo>,
        tree: &HashMap<u32, Vec<u32>>,
        visited: &mut std::collections::HashSet<u32>,
        rows: &mut Vec<ProcessTreeRow>,
    ) {
        if !visited.insert(pid) {
            return;
        }
        let Some(&proc) = item_map.get(&pid) else {
            return;
        };

        let branch = if depth == 0 {
            String::new()
        } else if is_last {
            format!("{prefix}└─ ")
        } else {
            format!("{prefix}├─ ")
        };

        let children = tree.get(&pid);
        let has_children = children.is_some_and(|c| !c.is_empty());

        rows.push(ProcessTreeRow {
            process: proc.clone(),
            depth,
            has_children,
            prefix: branch,
        });

        if let Some(child_list) = children {
            let next_prefix = if depth == 0 {
                ""
            } else if is_last {
                &format!("{prefix}   ")
            } else {
                &format!("{prefix}│  ")
            };

            for (i, &child_pid) in child_list.iter().enumerate() {
                let child_is_last = i == child_list.len() - 1;
                traverse(
                    child_pid,
                    depth + 1,
                    next_prefix,
                    child_is_last,
                    item_map,
                    tree,
                    visited,
                    rows,
                );
            }
        }
    }

    for root in root_pids {
        traverse(root, 0, "", true, &item_map, tree, &mut visited, &mut rows);
    }

    if !query.is_empty() {
        let q = query.to_lowercase();
        let parent_by_child: HashMap<u32, u32> = tree
            .iter()
            .flat_map(|(parent, children)| children.iter().map(|child| (*child, *parent)))
            .collect();
        let mut included: std::collections::HashSet<u32> = items
            .iter()
            .filter(|process| process.name.to_lowercase().contains(&q) || process.pid.to_string().contains(&q))
            .map(|process| process.pid)
            .collect();
        for pid in included.clone() {
            let mut current = pid;
            let mut chain = std::collections::HashSet::new();
            while chain.insert(current) {
                let Some(parent) = parent_by_child.get(&current).copied() else {
                    break;
                };
                if item_map.contains_key(&parent) {
                    included.insert(parent);
                }
                current = parent;
            }
        }
        rows.retain(|row| included.contains(&row.process.pid));
    }

    rows
}

/// Resolve a preset only within a single native processor group and its allowed mask.
#[cfg(windows)]
pub fn set_process_affinity(identity: ProcessIdentity, preset: AffinityPreset) -> Result<(), String> {
    use windows_sys::Win32::System::Threading::{
        GetProcessAffinityMask, GetProcessGroupAffinity, PROCESS_QUERY_INFORMATION, PROCESS_SET_INFORMATION,
        SetProcessAffinityMask,
    };
    let handle = open_process_for_action(identity, PROCESS_QUERY_INFORMATION | PROCESS_SET_INFORMATION)?;
    let mut groups = [0u16; 64];
    let mut count = groups.len() as u16;
    if unsafe { GetProcessGroupAffinity(handle.0, &mut count, groups.as_mut_ptr()) } == 0 {
        return Err(format!("GetProcessGroupAffinity: {}", std::io::Error::last_os_error()));
    }
    if count != 1 {
        return Err("Affinity presets are unavailable for multi-group processes".into());
    }
    let mut process_mask = 0;
    let mut system_mask = 0;
    if unsafe { GetProcessAffinityMask(handle.0, &mut process_mask, &mut system_mask) } == 0 {
        return Err(format!("GetProcessAffinityMask: {}", std::io::Error::last_os_error()));
    }
    let mask = affinity_mask(process_mask & system_mask, preset)?;
    if unsafe { SetProcessAffinityMask(handle.0, mask) } == 0 {
        return Err(format!("SetProcessAffinityMask: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_process_affinity(_identity: ProcessIdentity, _preset: AffinityPreset) -> Result<(), String> {
    Err("Process affinity is only supported on Windows".into())
}

/// Look up detailed information for one PID from a sysinfo snapshot.
pub fn lookup_details(sys: &System, pid: u32) -> Option<ProcessDetails> {
    let process = sys.process(Pid::from_u32(pid))?;
    let parent_name = process
        .parent()
        .and_then(|pp| sys.process(pp))
        .map(|pp| pp.name().to_string_lossy().into_owned());
    Some(ProcessDetails {
        exe_path: process.exe().map(|p| p.to_string_lossy().to_string()),
        command_line: process
            .cmd()
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" "),
        cwd: process.cwd().map(|p| p.to_string_lossy().to_string()),
        start_time: process.start_time(),
        run_time: process.run_time(),
        parent_pid: process.parent().map(|p| p.as_u32()),
        parent_name,
    })
}

/// Query the owner PID of whichever top-level or child window resides at screen coordinates (x, y).
#[cfg(target_os = "windows")]
pub fn get_process_id_from_screen_point(x: i32, y: i32) -> Option<u32> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GA_ROOT, GetAncestor, GetWindowThreadProcessId, WindowFromPoint,
    };

    let pt = POINT { x, y };
    unsafe {
        let hwnd = WindowFromPoint(pt);
        if hwnd.is_null() {
            return None;
        }
        let root_hwnd = GetAncestor(hwnd, GA_ROOT);
        let target_hwnd = if root_hwnd.is_null() { hwnd } else { root_hwnd };

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(target_hwnd, &mut pid);
        if pid != 0 { Some(pid) } else { None }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_process_id_from_screen_point(_x: i32, _y: i32) -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, name: &str, cpu: f32, mem: u64, status: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            identity: None,
            start_time: 0,
            name: name.to_string(),
            parent_pid: None,
            cpu_usage: cpu,
            memory: mem,
            vram_bytes: None,
            status: status.to_string(),
            disk_read_bytes: 0,
            disk_written_bytes: 0,
            disk_read_bytes_per_second: None,
            disk_written_bytes_per_second: None,
        }
    }

    #[test]
    fn filter_matches_name_case_insensitive() {
        let items = vec![
            p(1, "explorer.exe", 1.0, 100, "Running"),
            p(2, "System", 0.5, 10, "Running"),
        ];
        let out = filter_processes(&items, "EXPLORER");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].pid, 1);
    }

    #[test]
    fn filter_matches_pid() {
        let items = vec![
            p(42, "svchost.exe", 1.0, 100, "Running"),
            p(7, "dwm.exe", 2.0, 50, "Running"),
        ];
        let out = filter_processes(&items, "42");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "svchost.exe");
    }

    #[test]
    fn filter_empty_query_returns_all() {
        let items = vec![p(1, "a.exe", 1.0, 1, "Running"), p(2, "b.exe", 1.0, 1, "Running")];
        assert_eq!(filter_processes(&items, "").len(), 2);
    }

    #[test]
    fn sort_memory_descending_default() {
        let items = [
            p(1, "a", 0.0, 100, "Running"),
            p(2, "b", 0.0, 900, "Running"),
            p(3, "c", 0.0, 400, "Running"),
        ];
        let mut refs: Vec<_> = items.iter().collect();
        sort_processes_refs(&mut refs, ProcessSortColumn::Memory, false);
        let pids: Vec<u32> = refs.iter().map(|x| x.pid).collect();
        assert_eq!(pids, vec![2, 3, 1]);
    }

    #[test]
    fn sort_cpu_descending() {
        let items = [
            p(1, "a", 3.0, 0, "Running"),
            p(2, "b", 1.0, 0, "Running"),
            p(3, "c", 7.0, 0, "Running"),
        ];
        let mut refs: Vec<_> = items.iter().collect();
        sort_processes_refs(&mut refs, ProcessSortColumn::Cpu, false);
        let pids: Vec<u32> = refs.iter().map(|x| x.pid).collect();
        assert_eq!(pids, vec![3, 1, 2]);
    }

    #[test]
    fn sort_name_ascending() {
        let items = [
            p(1, "zeta", 0.0, 0, "Running"),
            p(2, "alpha", 0.0, 0, "Running"),
            p(3, "Beta", 0.0, 0, "Running"),
        ];
        let mut refs: Vec<_> = items.iter().collect();
        sort_processes_refs(&mut refs, ProcessSortColumn::Name, true);
        let names: Vec<&str> = refs.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "Beta", "zeta"]);
    }

    #[test]
    fn sort_disk_descending() {
        let mut p1 = p(1, "a", 0.0, 0, "Running");
        p1.disk_read_bytes = 1000;
        let mut p2 = p(2, "b", 0.0, 0, "Running");
        p2.disk_written_bytes = 5000;
        let p3 = p(3, "c", 0.0, 0, "Running");
        let items = [p1, p2, p3];
        let mut refs: Vec<_> = items.iter().collect();
        sort_processes_refs(&mut refs, ProcessSortColumn::Disk, false);
        let pids: Vec<u32> = refs.iter().map(|x| x.pid).collect();
        assert_eq!(pids, vec![2, 1, 3]);
    }

    #[test]
    fn test_process_sort_by_vram() {
        let p1 = ProcessInfo {
            pid: 100,
            identity: None,
            start_time: 0,
            name: "Game.exe".into(),
            parent_pid: None,
            cpu_usage: 5.0,
            memory: 1000,
            vram_bytes: Some(4 * 1024 * 1024 * 1024), // 4 GB
            status: "Running".into(),
            disk_read_bytes: 0,
            disk_written_bytes: 0,
            disk_read_bytes_per_second: None,
            disk_written_bytes_per_second: None,
        };
        let p2 = ProcessInfo {
            pid: 200,
            identity: None,
            start_time: 0,
            name: "Browser.exe".into(),
            parent_pid: None,
            cpu_usage: 1.0,
            memory: 500,
            vram_bytes: Some(512 * 1024 * 1024), // 512 MB
            status: "Running".into(),
            disk_read_bytes: 0,
            disk_written_bytes: 0,
            disk_read_bytes_per_second: None,
            disk_written_bytes_per_second: None,
        };

        let mut items = vec![&p2, &p1];
        sort_processes_refs(&mut items, ProcessSortColumn::Vram, false);
        // Descending sort by VRAM: Game.exe (4GB) should come first
        assert_eq!(items[0].pid, 100);
    }

    #[test]
    fn test_process_sort_by_vram_ascending_and_none() {
        let p1 = ProcessInfo {
            pid: 100,
            identity: None,
            start_time: 0,
            name: "Game.exe".into(),
            parent_pid: None,
            cpu_usage: 5.0,
            memory: 1000,
            vram_bytes: Some(4 * 1024 * 1024 * 1024),
            status: "Running".into(),
            disk_read_bytes: 0,
            disk_written_bytes: 0,
            disk_read_bytes_per_second: None,
            disk_written_bytes_per_second: None,
        };
        let p2 = ProcessInfo {
            pid: 200,
            identity: None,
            start_time: 0,
            name: "Browser.exe".into(),
            parent_pid: None,
            cpu_usage: 1.0,
            memory: 500,
            vram_bytes: Some(512 * 1024 * 1024),
            status: "Running".into(),
            disk_read_bytes: 0,
            disk_written_bytes: 0,
            disk_read_bytes_per_second: None,
            disk_written_bytes_per_second: None,
        };
        let p3 = ProcessInfo {
            pid: 300,
            identity: None,
            start_time: 0,
            name: "Idle.exe".into(),
            parent_pid: None,
            cpu_usage: 0.0,
            memory: 100,
            vram_bytes: None,
            status: "Running".into(),
            disk_read_bytes: 0,
            disk_written_bytes: 0,
            disk_read_bytes_per_second: None,
            disk_written_bytes_per_second: None,
        };

        let mut items = vec![&p1, &p2, &p3];
        sort_processes_refs(&mut items, ProcessSortColumn::Vram, true);
        // Ascending: None (0), 512MB, 4GB
        assert_eq!(items[0].pid, 300);
        assert_eq!(items[1].pid, 200);
        assert_eq!(items[2].pid, 100);

        sort_processes_refs(&mut items, ProcessSortColumn::Vram, false);
        // Descending: 4GB, 512MB, None (0)
        assert_eq!(items[0].pid, 100);
        assert_eq!(items[1].pid, 200);
        assert_eq!(items[2].pid, 300);
    }

    #[test]
    fn reused_identity_rejects_before_mutation() {
        let selected = ProcessIdentity {
            pid: 99,
            creation_time: 100,
        };
        let reused = ProcessIdentity {
            creation_time: 101,
            ..selected
        };
        let mut mutations = 0;
        let result = validate_process_identity(selected, reused).map(|_| mutations += 1);
        assert!(result.is_err());
        assert_eq!(mutations, 0);
        assert!(guard_process_target(selected, 99, false).is_err());
        assert!(guard_process_target(selected, 1, true).is_err());
    }

    #[test]
    fn tree_excludes_child_older_than_reused_parent() {
        let root = ProcessIdentity {
            pid: 10,
            creation_time: 100,
        };
        let old_child = ProcessIdentity {
            pid: 11,
            creation_time: 90,
        };
        let child = ProcessIdentity {
            pid: 12,
            creation_time: 110,
        };
        assert_eq!(
            process_tree_targets(root, &[(root, None), (old_child, Some(10)), (child, Some(10))]).unwrap(),
            vec![child, root]
        );
        assert!(
            process_tree_targets(
                ProcessIdentity {
                    creation_time: 99,
                    ..root
                },
                &[(root, None)]
            )
            .is_err()
        );
    }

    #[test]
    fn affinity_handles_sparse_and_full_width_masks_without_shifts() {
        assert_eq!(affinity_mask(0b10100, AffinityPreset::First).unwrap(), 0b100);
        assert_eq!(affinity_mask(0b10100, AffinityPreset::Second).unwrap(), 0b10000);
        assert_eq!(affinity_mask(usize::MAX, AffinityPreset::All).unwrap(), usize::MAX);
        assert_eq!(
            affinity_mask(usize::MAX, AffinityPreset::FirstHalf)
                .unwrap()
                .count_ones(),
            usize::BITS / 2
        );
        assert!(affinity_mask(0, AffinityPreset::All).is_err());
        assert!(affinity_mask(1, AffinityPreset::Second).is_err());
    }

    #[test]
    fn build_tree_maps_parents() {
        let parents: HashMap<u32, u32> = [(2, 1), (3, 1), (4, 2)].into_iter().collect();
        let tree = build_tree(&parents);
        assert_eq!(tree.get(&1), Some(&vec![2, 3]));
        assert_eq!(tree.get(&2), Some(&vec![4]));
    }

    #[test]
    fn kill_order_deepest_first() {
        let parents: HashMap<u32, u32> = [(2, 1), (3, 2)].into_iter().collect();
        let tree = build_tree(&parents);
        assert_eq!(kill_order(&tree, 1), vec![3, 2, 1]);
    }

    #[test]
    fn kill_order_cycle_safe() {
        let parents: HashMap<u32, u32> = [(1, 2), (2, 1), (3, 2)].into_iter().collect();
        let tree = build_tree(&parents);
        let order = kill_order(&tree, 1);
        assert_eq!(order.len(), 3);
        assert_eq!(order[order.len() - 1], 1); // root killed last
    }

    #[test]
    fn kill_order_root_not_in_tree() {
        let tree: HashMap<u32, Vec<u32>> = HashMap::new();
        assert_eq!(kill_order(&tree, 999), vec![999]);
    }

    #[test]
    fn build_tree_rows_constructs_hierarchy() {
        let items = vec![
            p(1, "system.exe", 1.0, 100, "Running"),
            p(2, "smss.exe", 0.5, 50, "Running"),
            p(3, "csrss.exe", 0.8, 80, "Running"),
        ];
        let parents: HashMap<u32, u32> = [(2, 1), (3, 2)].into_iter().collect();
        let tree = build_tree(&parents);
        let rows = build_tree_rows(&items, &tree, "");

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].process.pid, 1);
        assert_eq!(rows[0].depth, 0);
        assert!(rows[0].has_children);

        assert_eq!(rows[1].process.pid, 2);
        assert_eq!(rows[1].depth, 1);
        assert!(rows[1].has_children);
        assert_eq!(rows[1].prefix, "└─ ");

        assert_eq!(rows[2].process.pid, 3);
        assert_eq!(rows[2].depth, 2);
        assert!(!rows[2].has_children);
        assert_eq!(rows[2].prefix, "   └─ ");
    }

    #[test]
    fn build_tree_rows_filters_query() {
        let items = vec![
            p(1, "system.exe", 1.0, 100, "Running"),
            p(2, "smss.exe", 0.5, 50, "Running"),
            p(3, "csrss.exe", 0.8, 80, "Running"),
        ];
        let tree = HashMap::new();
        let rows = build_tree_rows(&items, &tree, "csrss");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].process.pid, 3);
    }

    #[test]
    fn build_tree_rows_search_keeps_ancestors() {
        let items = vec![
            p(1, "system.exe", 1.0, 100, "Running"),
            p(2, "service-host.exe", 0.5, 50, "Running"),
            p(3, "target.exe", 0.8, 80, "Running"),
        ];
        let parents: HashMap<u32, u32> = [(2, 1), (3, 2)].into_iter().collect();
        let rows = build_tree_rows(&items, &build_tree(&parents), "target");
        assert_eq!(
            rows.iter().map(|row| row.process.pid).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }
}

#[cfg(test)]
mod window_picker_tests {
    use super::*;

    #[test]
    fn test_point_outside_valid_screen_returns_none_or_desktop() {
        // Offscreen / unreachable coordinates should handle gracefully without crashing or panicking
        let pid = get_process_id_from_screen_point(-99999, -99999);
        if let Some(p) = pid {
            assert!(p < 10000000);
        }
    }

    #[test]
    fn test_point_at_origin_does_not_panic() {
        let pid = get_process_id_from_screen_point(0, 0);
        if let Some(p) = pid {
            assert!(p > 0);
        }
    }
}
