//! Process monitoring: models and pure logic (no OS-tied execution).

use serde::Serialize;
use std::collections::HashMap;
use sysinfo::{Pid, System};

// ─── Data Models ─────────────────────────────────────────────

#[derive(Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub status: String,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ProcessSortColumn {
    Pid,
    Name,
    Memory,
    Cpu,
    Status,
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
pub fn filter_processes(items: &[ProcessInfo], query: &str) -> Vec<ProcessInfo> {
    if query.is_empty() {
        return items.to_vec();
    }
    let q = query.to_lowercase();
    items
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&q) || p.pid.to_string().contains(&q))
        .cloned()
        .collect()
}

/// In-place sort. `Status` falls back to memory (no status ordering defined).
pub fn sort_processes(items: &mut [ProcessInfo], column: ProcessSortColumn, ascending: bool) {
    fn ord(o: std::cmp::Ordering, ascending: bool) -> std::cmp::Ordering {
        if ascending { o } else { o.reverse() }
    }
    match column {
        ProcessSortColumn::Pid => items.sort_by(|a, b| ord(a.pid.cmp(&b.pid), ascending)),
        ProcessSortColumn::Name => items
            .sort_by(|a, b| ord(a.name.to_lowercase().cmp(&b.name.to_lowercase()), ascending)),
        ProcessSortColumn::Memory => items.sort_by(|a, b| ord(a.memory.cmp(&b.memory), ascending)),
        ProcessSortColumn::Cpu => items.sort_by(|a, b| {
            ord(
                a.cpu_usage
                    .partial_cmp(&b.cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal),
                ascending,
            )
        }),
        ProcessSortColumn::Status => items.sort_by(|a, b| ord(a.memory.cmp(&b.memory), ascending)),
    }
}

/// Build pid -> [child pids] adjacency from a pid -> parent_pid map.
pub fn build_tree(parent_map: &HashMap<u32, u32>) -> HashMap<u32, Vec<u32>> {
    let mut tree: HashMap<u32, Vec<u32>> = HashMap::new();
    for (&pid, &parent) in parent_map {
        tree.entry(parent).or_default().push(pid);
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
            name: name.to_string(),
            cpu_usage: cpu,
            memory: mem,
            status: status.to_string(),
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
        let items = vec![
            p(1, "a.exe", 1.0, 1, "Running"),
            p(2, "b.exe", 1.0, 1, "Running"),
        ];
        assert_eq!(filter_processes(&items, "").len(), 2);
    }

    #[test]
    fn sort_memory_descending_default() {
        let mut items = vec![
            p(1, "a", 0.0, 100, "Running"),
            p(2, "b", 0.0, 900, "Running"),
            p(3, "c", 0.0, 400, "Running"),
        ];
        sort_processes(&mut items, ProcessSortColumn::Memory, false);
        let pids: Vec<u32> = items.iter().map(|x| x.pid).collect();
        assert_eq!(pids, vec![2, 3, 1]);
    }

    #[test]
    fn sort_cpu_descending() {
        let mut items = vec![
            p(1, "a", 3.0, 0, "Running"),
            p(2, "b", 1.0, 0, "Running"),
            p(3, "c", 7.0, 0, "Running"),
        ];
        sort_processes(&mut items, ProcessSortColumn::Cpu, false);
        let pids: Vec<u32> = items.iter().map(|x| x.pid).collect();
        assert_eq!(pids, vec![3, 1, 2]);
    }

    #[test]
    fn sort_name_ascending() {
        let mut items = vec![
            p(1, "zeta", 0.0, 0, "Running"),
            p(2, "alpha", 0.0, 0, "Running"),
            p(3, "Beta", 0.0, 0, "Running"),
        ];
        sort_processes(&mut items, ProcessSortColumn::Name, true);
        let names: Vec<&str> = items.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "Beta", "zeta"]);
    }

    #[test]
    fn sort_status_falls_back_to_memory() {
        let mut items = vec![
            p(1, "a", 0.0, 100, "Running"),
            p(2, "b", 0.0, 300, "Running"),
        ];
        sort_processes(&mut items, ProcessSortColumn::Status, false);
        assert_eq!(items[0].pid, 2);
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
}
