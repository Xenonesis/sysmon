//! Process monitoring: models and pure logic (no OS-tied execution).

use serde::Serialize;
use std::collections::HashMap;
use sysinfo::{Pid, System};

// ─── Data Models ─────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub start_time: u64,
    pub name: String,
    pub parent_pid: Option<u32>,
    pub cpu_usage: f32,
    pub memory: u64,
    pub status: String,
    pub disk_read_bytes: u64,
    pub disk_written_bytes: u64,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ProcessSortColumn {
    Pid,
    Name,
    Memory,
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
        if ascending {
            o
        } else {
            o.reverse()
        }
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

/// Derive the parent map from a sysinfo `System` snapshot.
pub fn build_process_tree(sys: &System) -> HashMap<u32, Vec<u32>> {
    let parents: HashMap<u32, u32> = sys
        .processes()
        .iter()
        .filter_map(|(pid, proc)| proc.parent().map(|p| (pid.as_u32(), p.as_u32())))
        .collect();
    build_tree(&parents)
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

/// Set CPU core affinity mask for a process by PID on Windows.
#[cfg(target_os = "windows")]
pub fn set_process_affinity(pid: u32, mask: usize) -> Result<(), String> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, SetProcessAffinityMask, PROCESS_QUERY_INFORMATION, PROCESS_SET_INFORMATION,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_SET_INFORMATION | PROCESS_QUERY_INFORMATION, 0, pid);
        if handle.is_null() {
            return Err(format!("Failed to open process {pid} (error code {})", GetLastError()));
        }
        let result = SetProcessAffinityMask(handle, mask);
        CloseHandle(handle);
        if result == 0 {
            return Err(format!(
                "Failed to set affinity mask {mask:#x} for PID {pid} (error code {})",
                GetLastError()
            ));
        }
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_process_affinity(_pid: u32, _mask: usize) -> Result<(), String> {
    Err("Process affinity is only supported on Windows".into())
}

/// Look up detailed information for one PID from a sysinfo snapshot.
pub fn lookup_details(sys: &System, pid: u32) -> Option<ProcessDetails> {
    let process = sys.process(Pid::from_u32(pid))?;
    let parent_name = process
        .parent()
        .and_then(|pp| sys.process(pp))
        .map(|pp| pp.name().to_string());
    Some(ProcessDetails {
        exe_path: process.exe().map(|p| p.to_string_lossy().to_string()),
        command_line: process.cmd().join(" "),
        cwd: process.cwd().map(|p| p.to_string_lossy().to_string()),
        start_time: process.start_time(),
        run_time: process.run_time(),
        parent_pid: process.parent().map(|p| p.as_u32()),
        parent_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, name: &str, cpu: f32, mem: u64, status: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            start_time: 0,
            name: name.to_string(),
            parent_pid: None,
            cpu_usage: cpu,
            memory: mem,
            status: status.to_string(),
            disk_read_bytes: 0,
            disk_written_bytes: 0,
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
