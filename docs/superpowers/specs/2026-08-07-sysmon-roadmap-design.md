# SysMon Pro Max - Master Roadmap & Design Spec

## 1. Context & Goal
SysMon is already a highly optimized, native Rust system monitor featuring CPU, RAM, GPU, and process management. The goal of this roadmap is to expand SysMon from a basic monitor into the "Ultimate Windows Utility," catering to Gamers, Power Users, and Casual Users. Due to the massive scope of requested features, development is broken down into four isolated, sequential milestones to maintain system stability and the low-resource footprint of the application.

## 2. Four-Phase Architecture Breakdown

### Phase 1: The "Deep Observer" Update (Data Expansion)
**Goal:** Expand core telemetry collection.
*   **Per-App Network & Disk I/O:**
    *   *Implementation:* Hook into `GetExtendedTcpTable` (network) and `NtQuerySystemInformation` (process I/O counters) via `windows-rs`.
    *   *UI:* Add "Net (Kbps)" and "Disk (MB/s)" columns to the Processes tab.
*   **Windows Services Manager:**
    *   *Implementation:* Utilize the `windows-service` crate. Spawn a dedicated background thread to poll service states without blocking the UI.
    *   *UI:* New "Services" tab (Start/Stop/Restart toggles).
*   **Battery & Power Health:**
    *   *Implementation:* Query `Win32_Battery` via WMI (or use the `battery` crate) to retrieve design capacity, full charge capacity, and discharge rate.

### Phase 2: The "Widget & Overlay" Update (UI Expansion)
**Goal:** Expose data outside the main window.
*   **Desktop Mini-Widget (Always on Top):**
    *   *Implementation:* Create a secondary `eframe::Window` configured with `always_on_top(true)`, `decorated(false)`, and transparent rendering.
    *   *UI:* A minimal overlay displaying essential CPU/RAM/Net metrics.
*   **Power Plan Toggle in Tray:**
    *   *Implementation:* Map standard Windows Power GUIDs (Balanced, High Performance, Power Saver). Use `PowerSetActiveScheme` to toggle.
    *   *UI:* Add sub-menus to the existing system tray context menu.

### Phase 3: The "Controller" Update (Actionable Tools)
**Goal:** Provide system intervention tools.
*   **Game Booster Mode:**
    *   *Implementation:* A configurable JSON profile defining non-essential background tasks. When activated, loops through matching PIDs to invoke `NtSuspendProcess`. Deactivation invokes `NtResumeProcess`.
*   **Startup Delayer:**
    *   *Implementation:* Migrate targets from the `HKCU\...\Run` registry key into Windows Task Scheduler (`schtasks` or COM API) configured with a boot delay.
*   **Deep Process Inspection:**
    *   *Implementation:* Process property modal. Use `EnumProcessModules` and `NtQueryInformationProcess` to list loaded DLLs and open file handles.

### Phase 4: The "Time-Machine" Update (Historical Logging)
**Goal:** Retain data for analytics and diagnostics.
*   **24-Hour Historical Logging:**
    *   *Implementation:* Integrate `rusqlite`. A background worker flushes aggregated telemetry (CPU, RAM, Temp averages) to a local SQLite database (`sysmon_history.db`) every 5 minutes to minimize disk I/O.
*   **PC Health Score:**
    *   *Implementation:* An algorithm analyzing disk free space, RAM headroom, startup impact count, and historical thermal throttling to generate a 0-100 score and actionable recommendations.

## 3. Data Flow & Performance Constraints
*   **Zero-UI Blocking:** All new data collection (Services, WMI, SQLite) MUST run in background threads and communicate via `Arc<Mutex<State>>`.
*   **Low Overhead:** The core tenet of SysMon is minimal resource usage. The SQLite database writes must be batched. The Game Booster mode must not rely on constant polling.

## 4. Ambiguity Resolution & Scope
*   **In-Game OSD vs Desktop Widget:** Due to the complexity and anti-cheat risks of injecting DirectX overlays, the visual overlay will be restricted to a borderless, always-on-top desktop widget (safe for windowed/borderless gaming) rather than true DirectX injection.
*   **Milestone Execution:** Each phase is treated as a separate project and requires its own implementation plan before coding begins. Phase 1 is the immediate next priority.