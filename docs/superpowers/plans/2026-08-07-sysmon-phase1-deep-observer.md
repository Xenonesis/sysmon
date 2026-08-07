# Phase 1: Deep Observer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand core telemetry collection by implementing Battery/Power health monitoring, a Windows Services manager, and per-app Disk I/O tracking.

**Architecture:** Use WMI (`wmi` crate) for battery statistics, `windows-service` for enumerating and toggling background services, and native `sysinfo` capabilities for per-process disk read/write tracking. UI rendering uses `egui`. All blocking queries run in background threads, communicating via `Arc<Mutex<State>>`.

**Tech Stack:** Rust, `egui`, `wmi`, `windows-service`, `sysinfo`.

## Global Constraints
- Target platform: Windows 10/11 (64-bit) only.
- Zero UI blocking: All data collection must occur on background threads.
- No `TODO`s or placeholders in implementation.

---

### Task 1: Battery Telemetry Integration

**Files:**
- Modify: `src/main.rs:100-150` (SystemData struct)
- Modify: `src/main.rs:2000-2100` (Background WMI polling thread)
- Modify: `src/main.rs:3780-3850` (UI rendering for System Info)

**Interfaces:**
- Consumes: WMI connection in background thread.
- Produces: `BatteryInfo` struct in `SystemData`.

- [ ] **Step 1: Write the structs and test**

```rust
// Add to src/main.rs above SystemData
#[derive(Debug, Clone, Default)]
pub struct BatteryInfo {
    pub design_capacity: u32,
    pub full_charge_capacity: u32,
    pub status: u16,
    pub present: bool,
}

// In a test module (e.g. tests/battery_test.rs)
#[test]
fn test_battery_info_default() {
    let b = BatteryInfo::default();
    assert_eq!(b.design_capacity, 0);
    assert_eq!(b.present, false);
}
```

- [ ] **Step 2: Run test to verify**
Run: `cargo test test_battery_info_default`
Expected: PASS

- [ ] **Step 3: Write WMI query implementation**

```rust
// Modify SystemData in src/main.rs
pub struct SystemData {
    // ... existing fields ...
    pub battery_info: Option<BatteryInfo>,
}

// Inside the background monitor thread (src/main.rs)
// WMI query for Win32_Battery
use serde::Deserialize;
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Win32Battery {
    design_capacity: Option<u32>,
    full_charge_capacity: Option<u32>,
    battery_status: Option<u16>,
}

pub fn get_battery_info(wmi_con: &wmi::WMIConnection) -> Option<BatteryInfo> {
    let results: Result<Vec<Win32Battery>, _> = wmi_con.raw_query("SELECT DesignCapacity, FullChargeCapacity, BatteryStatus FROM Win32_Battery");
    if let Ok(mut bats) = results {
        if let Some(bat) = bats.pop() {
            return Some(BatteryInfo {
                design_capacity: bat.design_capacity.unwrap_or(0),
                full_charge_capacity: bat.full_charge_capacity.unwrap_or(0),
                status: bat.battery_status.unwrap_or(0),
                present: true,
            });
        }
    }
    None
}
```

- [ ] **Step 4: Wire UI in System Info Tab**

```rust
// Inside show_system_info_tab in src/main.rs
if let Some(bat) = &data.battery_info {
    if bat.present {
        ui.group(|ui| {
            ui.heading("Battery Health");
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Design Capacity:");
                ui.strong(format!("{} mWh", bat.design_capacity));
            });
            ui.horizontal(|ui| {
                ui.label("Full Charge Capacity:");
                ui.strong(format!("{} mWh", bat.full_charge_capacity));
            });
            let wear = if bat.design_capacity > 0 {
                100.0 - ((bat.full_charge_capacity as f32 / bat.design_capacity as f32) * 100.0)
            } else { 0.0 };
            ui.horizontal(|ui| {
                ui.label("Battery Wear Level:");
                ui.strong(format!("{:.1}%", wear));
            });
        });
        ui.add_space(12.0);
    }
}
```

- [ ] **Step 5: Commit**
```bash
git add src/main.rs
git commit -m "feat: add battery telemetry via WMI"
```

---

### Task 2: Windows Services Backend

**Files:**
- Modify: `Cargo.toml`
- Create: `src/services.rs`

**Interfaces:**
- Produces: `ServiceInfo` struct, `get_services()` function.

- [ ] **Step 1: Add dependencies**
```bash
cargo add windows-service
```

- [ ] **Step 2: Write Service definition & implementation**

```rust
// src/services.rs
use windows_service::service_manager::{ServiceManager, ServiceAccess};
use windows_service::service::ServiceState;

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: String,
}

pub fn get_services() -> Vec<ServiceInfo> {
    let mut result = Vec::new();
    if let Ok(manager) = ServiceManager::local_computer(None::<&str>, ServiceAccess::ENUMERATE_SERVICE) {
        if let Ok(services) = manager.enumerate_services_status() {
            for svc in services {
                let state_str = match svc.service_status.current_state {
                    ServiceState::Running => "Running",
                    ServiceState::Stopped => "Stopped",
                    ServiceState::Paused => "Paused",
                    _ => "Other",
                }.to_string();
                
                result.push(ServiceInfo {
                    name: svc.service_name.to_string_lossy(),
                    display_name: svc.display_name.to_string_lossy(),
                    state: state_str,
                });
            }
        }
    }
    result.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    result
}
```

- [ ] **Step 3: Compile to verify**
Run: `cargo check`
Expected: Passes without errors.

- [ ] **Step 4: Commit**
```bash
git add Cargo.toml src/services.rs
git commit -m "feat: add windows-service backend enumeration"
```

---

### Task 3: Services UI & Background Threading

**Files:**
- Modify: `src/main.rs` (Tab enum, Sidebar rendering, Tab rendering)

**Interfaces:**
- Consumes: `src/services.rs::get_services()`

- [ ] **Step 1: Add state and tab enum**

```rust
// Add to Tab enum in src/main.rs
#[derive(PartialEq)]
enum Tab {
    Overview,
    Processes,
    StartupApps,
    Services, // New
    Settings,
    About,
}

// Add to SystemData
pub struct SystemData {
    // ...
    pub services: Vec<crate::services::ServiceInfo>,
}
```

- [ ] **Step 2: Update background thread**

```rust
// Inside background thread loop in src/main.rs
let services_list = crate::services::get_services();
{
    let mut data = share.lock();
    data.services = services_list;
}
```

- [ ] **Step 3: Sidebar & Tab Render**

```rust
// In Sidebar render
if ui.add_sized([ui.available_width(), 32.0], egui::SelectableLabel::new(self.selected_tab == Tab::Services, "⚙ Services")).clicked() {
    self.selected_tab = Tab::Services;
}

// In main panel render match
Tab::Services => self.show_services_tab(ui, &data),

// Add method
impl SystemMonitorApp {
    fn show_services_tab(&mut self, ui: &mut egui::Ui, data: &SystemData) {
        crate::paint_section_header(ui, "Windows Services");
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("services_grid")
                .striped(true)
                .min_col_width(200.0)
                .show(ui, |ui| {
                    ui.strong("Display Name");
                    ui.strong("Service Name");
                    ui.strong("State");
                    ui.end_row();

                    for svc in &data.services {
                        ui.label(&svc.display_name);
                        ui.label(&svc.name);
                        let color = if svc.state == "Running" { egui::Color32::GREEN } else { egui::Color32::GRAY };
                        ui.colored_label(color, &svc.state);
                        ui.end_row();
                    }
                });
        });
    }
}
```

- [ ] **Step 4: Verify build**
Run: `cargo build`
Expected: Successful build.

- [ ] **Step 5: Commit**
```bash
git add src/main.rs
git commit -m "feat: add Services UI tab and background polling"
```

---

### Task 4: Per-App Disk I/O (sysinfo)

**Files:**
- Modify: `src/processes.rs`
- Modify: `src/main.rs` (Processes UI)

**Interfaces:**
- Consumes: `sysinfo::Process::disk_usage()`

- [ ] **Step 1: Add fields to ProcessInfo**

```rust
// src/processes.rs
pub struct ProcessInfo {
    // ...
    pub disk_read_bytes: u64,
    pub disk_written_bytes: u64,
}

// Inside update_process_list loop:
disk_read_bytes: process.disk_usage().read_bytes,
disk_written_bytes: process.disk_usage().written_bytes,
```

- [ ] **Step 2: Add UI Columns**

```rust
// src/main.rs -> Process List Grid Headers
ui.strong("CPU");
ui.strong("Memory");
ui.strong("Disk Read"); // New
ui.strong("Disk Write"); // New

// Rendering rows
ui.label(format!("{:.1}%", proc.cpu_usage));
ui.label(format!("{:.1} MB", proc.memory as f64 / 1_048_576.0));
ui.label(format!("{:.1} KB/s", proc.disk_read_bytes as f64 / 1024.0));
ui.label(format!("{:.1} KB/s", proc.disk_written_bytes as f64 / 1024.0));
```

- [ ] **Step 3: Build & Commit**
```bash
cargo check
git add src/processes.rs src/main.rs
git commit -m "feat: add per-process disk IO tracking"
```