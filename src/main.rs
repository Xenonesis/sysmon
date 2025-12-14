use chrono::Local;
use colored::*;
use crossterm::{
    cursor,
    execute,
    terminal::{Clear, ClearType},
};
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use sysinfo::System;

#[cfg(target_os = "windows")]
use nvml_wrapper::Nvml;

struct SystemMonitor {
    sys: System,
    #[cfg(target_os = "windows")]
    nvml: Option<Nvml>,
}

impl SystemMonitor {
    fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        #[cfg(target_os = "windows")]
        let nvml = Nvml::init().ok();

        #[cfg(not(target_os = "windows"))]
        let nvml = None;

        SystemMonitor { sys, nvml }
    }

    fn refresh(&mut self) {
        self.sys.refresh_all();
    }

    fn get_memory_info(&self) -> (u64, u64, f32) {
        let total = self.sys.total_memory();
        let used = self.sys.used_memory();
        let percentage = (used as f64 / total as f64) * 100.0;
        (total, used, percentage as f32)
    }

    fn get_cpu_usage(&mut self) -> f32 {
        self.sys.global_cpu_info().cpu_usage()
    }

    fn get_top_processes(&self, count: usize) -> Vec<ProcessInfo> {
        let mut processes: Vec<_> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: *pid,
                name: process.name().to_string(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            })
            .collect();

        processes.sort_by(|a, b| b.memory.cmp(&a.memory));
        processes.truncate(count);
        processes
    }

    #[cfg(target_os = "windows")]
    fn get_gpu_info(&self) -> Option<GpuInfo> {
        if let Some(ref nvml) = self.nvml {
            if let Ok(device_count) = nvml.device_count() {
                if device_count > 0 {
                    if let Ok(device) = nvml.device_by_index(0) {
                        let name = device.name().unwrap_or_else(|_| "Unknown GPU".to_string());
                        let utilization = device
                            .utilization_rates()
                            .map(|u| u.gpu)
                            .unwrap_or(0);
                        let memory = device.memory_info().ok();
                        let temperature = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu).ok();

                        return Some(GpuInfo {
                            name,
                            utilization: utilization as f32,
                            memory_used: memory.as_ref().map(|m| m.used),
                            memory_total: memory.as_ref().map(|m| m.total),
                            temperature,
                        });
                    }
                }
            }
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    fn get_gpu_info(&self) -> Option<GpuInfo> {
        None
    }
}

#[derive(Debug)]
struct ProcessInfo {
    pid: sysinfo::Pid,
    name: String,
    cpu_usage: f32,
    memory: u64,
}

#[derive(Debug)]
struct GpuInfo {
    name: String,
    utilization: f32,
    memory_used: Option<u64>,
    memory_total: Option<u64>,
    temperature: Option<u32>,
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

fn get_usage_color(percentage: f32) -> Color {
    if percentage < 50.0 {
        Color::Green
    } else if percentage < 75.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn draw_progress_bar(percentage: f32, width: usize) -> String {
    let filled = ((percentage / 100.0) * width as f32) as usize;
    let empty = width - filled;
    let color = get_usage_color(percentage);
    
    format!(
        "{}{}",
        "█".repeat(filled).color(color),
        "░".repeat(empty).dimmed()
    )
}

fn display_system_info(monitor: &mut SystemMonitor) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    
    // Clear screen and move cursor to top
    execute!(
        io::stdout(),
        Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    ).unwrap();

    println!("{}", "═══════════════════════════════════════════════════════════════════════".bright_cyan().bold());
    println!("{} {}", "🖥️  SYSTEM MONITOR".bright_cyan().bold(), format!("[{}]", timestamp).dimmed());
    println!("{}", "═══════════════════════════════════════════════════════════════════════".bright_cyan().bold());
    println!();

    // Memory Information
    let (total_mem, used_mem, mem_percentage) = monitor.get_memory_info();
    println!("{}", "💾 MEMORY USAGE".bright_yellow().bold());
    println!("   Total: {:.2} GB", bytes_to_gb(total_mem));
    println!("   Used:  {:.2} GB ({:.1}%)", bytes_to_gb(used_mem), mem_percentage);
    println!("   Free:  {:.2} GB", bytes_to_gb(total_mem - used_mem));
    println!("   {}", draw_progress_bar(mem_percentage, 60));
    println!();

    // CPU Information
    let cpu_usage = monitor.get_cpu_usage();
    println!("{}", "⚡ CPU USAGE".bright_yellow().bold());
    println!("   Usage: {:.1}%", cpu_usage);
    println!("   {}", draw_progress_bar(cpu_usage, 60));
    println!();

    // GPU Information
    if let Some(gpu_info) = monitor.get_gpu_info() {
        println!("{}", "🎮 GPU USAGE".bright_yellow().bold());
        println!("   Name: {}", gpu_info.name);
        println!("   Utilization: {:.1}%", gpu_info.utilization);
        println!("   {}", draw_progress_bar(gpu_info.utilization, 60));
        
        if let (Some(used), Some(total)) = (gpu_info.memory_used, gpu_info.memory_total) {
            let gpu_mem_percentage = (used as f64 / total as f64) * 100.0;
            println!("   Memory: {:.0} MB / {:.0} MB ({:.1}%)", 
                bytes_to_mb(used), bytes_to_mb(total), gpu_mem_percentage);
        }
        
        if let Some(temp) = gpu_info.temperature {
            let temp_color = if temp < 70 {
                Color::Green
            } else if temp < 85 {
                Color::Yellow
            } else {
                Color::Red
            };
            println!("   Temperature: {}°C", format!("{}", temp).color(temp_color).bold());
        }
        println!();
    }

    // Top Processes
    println!("{}", "📊 TOP 15 PROCESSES BY MEMORY".bright_yellow().bold());
    println!("{}", "───────────────────────────────────────────────────────────────────────".dimmed());
    println!(
        "{:<8} {:<30} {:<12} {:<12}",
        "PID".bold(),
        "NAME".bold(),
        "MEMORY".bold(),
        "CPU %".bold()
    );
    println!("{}", "───────────────────────────────────────────────────────────────────────".dimmed());

    let top_processes = monitor.get_top_processes(15);
    for process in top_processes {
        let memory_mb = bytes_to_mb(process.memory);
        let memory_color = if memory_mb > 500.0 {
            Color::Red
        } else if memory_mb > 200.0 {
            Color::Yellow
        } else {
            Color::Green
        };

        println!(
            "{:<8} {:<30} {:<12} {:<12}",
            process.pid.to_string().dimmed(),
            if process.name.len() > 28 {
                format!("{}...", &process.name[..27])
            } else {
                process.name.clone()
            },
            format!("{:.2} MB", memory_mb).color(memory_color),
            format!("{:.1}%", process.cpu_usage).dimmed()
        );
    }

    println!();
    println!("{}", "═══════════════════════════════════════════════════════════════════════".bright_cyan().bold());
    println!("{}", "Press Ctrl+C to exit | Refreshing every 2 seconds...".dimmed());
    
    io::stdout().flush().unwrap();
}

fn main() {
    println!("{}", "Initializing System Monitor...".bright_green().bold());
    println!("{}", "Loading system information...".dimmed());
    
    let mut monitor = SystemMonitor::new();
    
    // Initial refresh
    thread::sleep(Duration::from_millis(500));
    monitor.refresh();

    loop {
        display_system_info(&mut monitor);
        thread::sleep(Duration::from_secs(2));
        monitor.refresh();
    }
}
