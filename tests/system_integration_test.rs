#![allow(unused_imports)]

use system_monitor::providers::*;

#[test]
fn test_new_subsystems_headless_initialization() {
    // 1. Verify file lock finder works on current exe
    let exe = std::env::current_exe().expect("current exe");
    let lock_res = system_monitor::storage::file_locks::find_locking_processes(exe.to_str().unwrap());
    assert_eq!(lock_res.path, exe.to_str().unwrap());

    // 2. Verify cache scanner finds categories without error
    let cats = system_monitor::storage::reclaimer::scan_reclaimable_caches();
    assert_eq!(cats.len(), 4);
    for cat in &cats {
        assert!(!cat.id.is_empty());
        assert!(!cat.label.is_empty());
    }

    // 3. Verify minidump scanner executes safely
    let crashes = system_monitor::diagnostics::minidump::scan_recent_crashes();
    println!("Found {} crash reports", crashes.len());

    // 4. Verify screen point lookup handles origin safely
    let pid = system_monitor::processes::get_process_id_from_screen_point(0, 0);
    println!("Screen origin PID: {:?}", pid);
}
