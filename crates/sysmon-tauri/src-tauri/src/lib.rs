use parking_lot::Mutex;
use sysmon_core::app::models::SystemMonitor;
use tauri::State;

struct SysState(Mutex<SystemMonitor>);

#[tauri::command]
fn get_system_stats(state: State<SysState>) -> Result<(f32, u64, usize), String> {
    let mut monitor = state.0.lock();
    
    // Use the shared engine from sysmon-core
    monitor.sys.refresh_cpu_all();
    monitor.sys.refresh_memory();
    monitor.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    
    let cpu = monitor.sys.global_cpu_usage();
    let mem = monitor.sys.used_memory() / 1024 / 1024;
    let procs = monitor.sys.processes().len();
    
    Ok((cpu, mem, procs))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut monitor = SystemMonitor::new();
    monitor.sys.refresh_cpu_all();
    monitor.sys.refresh_memory();
    
    tauri::Builder::default()
        .manage(SysState(Mutex::new(monitor)))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_system_stats])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
