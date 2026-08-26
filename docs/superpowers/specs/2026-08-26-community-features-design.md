# SysMon Community-Requested Features Design Specification

**Date:** 2026-08-26  
**Status:** Draft / Ready for Planning  
**Target Version:** 3.9.0  

---

## 1. Executive Summary & Context

SysMon is an established native Windows observability and diagnostics application written in Rust (v3.8.0), featuring an isolated `TelemetryHub`, decoupled UI rendering at 60 FPS, multi-resolution ring buffers, and guarded system actions.

Extensive research across technical user communities (Reddit `r/windows`, `r/Windows11`, `r/sysadmin`, `r/pcmasterrace`, `r/techsupport`, `r/pcgaming`, Hacker News, and GitHub issue trackers for `SystemInformer`, `PowerToys`, and `FanControl`) revealed several critical gaps in standard Windows tools:
1. **File and USB Drive Locks:** Windows gives vague errors (*"The action can't be completed because the file is open in another program"*, *"This device is currently in use"*) without identifying the culprit.
2. **Mystery Windows:** Users cannot identify which process created a rogue popup, background dialog, or invisible window stealing focus.
3. **Hidden Storage Waste:** Windows Disk Cleanup misses tens of gigabytes in shader caches, update downloads, and crash dumps.
4. **Crash Inscrutability:** When Windows crashes with a BSOD or a driver faults, users must resort to unmaintained third-party utilities (WhoCrashed, BlueScreenView) or post `.dmp` files online.
5. **VRAM Blindspots:** Web browsers, Discord, games, and local AI engines (Ollama, ComfyUI) exhaust GPU memory with no per-process VRAM visibility in standard task managers.
6. **Incomplete Network Telemetry:** Modern networks rely heavily on IPv6, but basic connection monitors only inspect IPv4.

This design introduces **six tightly-focused, zero-bloat features** solving these exact problems while maintaining SysMon's core principles:
* No external background services or daemons.
* All telemetry and diagnostics are computed locally.
* Destructive actions require explicit confirmation, risk disclosure, and local audit logging.

---

## 2. Architecture & Modules

The new capabilities map into existing architecture without disturbing the decoupled `TelemetryHub` or render thread:

```
src/
├── main.rs                      # Header target crosshair button + drag handler
├── app/
│   ├── commands.rs              # ActionCommand::KillLockingProcess, ReclaimStorageCaches
│   ├── models.rs                # State models for FileLocks, ReclaimableCache, CrashDump
│   └── worker.rs                # Async action worker execution for cache deletion
├── storage/
│   ├── mod.rs                   # Re-exports and storage models
│   ├── file_locks.rs            # NEW: Windows Restart Manager API integration
│   └── reclaimer.rs             # NEW: Safe system cache scanner and cleaner
├── diagnostics/
│   ├── mod.rs                   # Rules and findings
│   └── minidump.rs              # NEW: Native Windows 64-bit minidump parser & dictionary
├── processes.rs                 # Per-process VRAM query & Window-to-PID lookup
├── network.rs                   # IPv6 TCP/UDP socket tables support
└── ui/
    ├── pages/
    │   ├── storage.rs           # File Lock Inspector card + Storage Reclaimer card
    │   ├── diagnostics.rs       # BSOD & Minidump Crash History card
    │   ├── processes/table.rs   # VRAM column and sorting
    │   └── network/sockets.rs   # IPv6 address formatting and badge
    └── components.rs            # Target crosshair icon & button
```

---

## 3. Detailed Component Specifications

### 3.1 Feature 1: File & USB Lock Inspector (`src/storage/file_locks.rs`)

* **Problem:** Users cannot delete, rename, or eject a USB drive because a process is locking a handle, but Windows refuses to name the process.
* **API:** Windows Restart Manager API (`rstrtmgr.dll`):
  * `RmStartSession(&mut session_handle, 0, session_key)`
  * `RmRegisterResources(session_handle, file_count, file_paths, ...)`
  * `RmGetList(session_handle, ...)` returns an array of `RM_PROCESS_INFO` containing:
    * `Process.dwProcessId` (PID)
    * `strAppName` (UTF-16 executable/service name)
    * `ApplicationType` (StandAlone, Service, Explorer, etc.)
  * `RmEndSession(session_handle)`
* **Data Model:**
  ```rust
  #[derive(Clone, Debug, PartialEq, serde::Serialize)]
  pub struct FileLockEntry {
      pub target_path: String,
      pub locking_processes: Vec<LockingProcess>,
  }

  #[derive(Clone, Debug, PartialEq, serde::Serialize)]
  pub struct LockingProcess {
      pub pid: u32,
      pub name: String,
      pub app_type: String,
      pub is_service: bool,
  }
  ```
* **Guard Rails:**
  * Guarded action `ActionCommand::TerminateProcess(pid)` uses existing risk disclosures and audit logging.
  * System processes (PIDs ≤ 4 or critical services like `lsass.exe`, `csrss.exe`) are flagged as **CRITICAL RISK** with termination prohibited or double-confirmed.

### 3.2 Feature 2: Target Window Crosshair Picker (`src/processes.rs` & `src/main.rs`)

* **Problem:** Users encounter a mystery popup, invisible window stealing keyboard focus, or unwanted overlay and want to find which process owns it immediately.
* **API:**
  * Windows API: `WindowFromPoint(POINT)` to find the top-level or child window under the cursor.
  * `GetAncestor(hwnd, GA_ROOT)` to get the root application window.
  * `GetWindowThreadProcessId(hwnd, &mut pid)` to resolve the owner PID.
* **UX Flow:**
  1. User clicks and holds the `[ 🎯 Find Window ]` button in the top navigation bar.
  2. Cursor changes to a crosshair; SysMon captures mouse movement.
  3. When user releases the mouse over any desktop window, SysMon queries the PID under the cursor.
  4. SysMon automatically navigates to the **Processes** tab, sets the filter/selection to that PID, and reveals its executable path and command line.

### 3.3 Feature 3: Storage Space Reclaimer (`src/storage/reclaimer.rs`)

* **Problem:** Windows Disk Cleanup misses tens of gigabytes in shader caches, crash dumps, and Windows Update staging files.
* **Safe Locations Scanned:**
  1. **DirectX / GPU Shader Caches:** `%LOCALAPPDATA%\D3DSCache`, `%LOCALAPPDATA%\NVIDIA\DXCache`, `%LOCALAPPDATA%\AMD\DxCache`.
  2. **Windows Update Download Staging:** `C:\Windows\SoftwareDistribution\Download`.
  3. **Crash Dumps:** `C:\Windows\Minidump` and `%LOCALAPPDATA%\CrashDumps`.
  4. **User & System Temp:** `%TEMP%` and `C:\Windows\Temp` (files older than 24 hours).
* **Data Model:**
  ```rust
  #[derive(Clone, Debug, PartialEq, serde::Serialize)]
  pub struct ReclaimCategory {
      pub id: &'static str,
      pub label: &'static str,
      pub description: &'static str,
      pub paths: Vec<std::path::PathBuf>,
      pub size_bytes: u64,
      pub file_count: usize,
  }
  ```
* **Safety:**
  * Read-only scan on page open / refresh.
  * Cleanup runs on background `ActionWorker`.
  * Files that are locked or in-use are skipped without failing the entire batch.
  * Reclaimed bytes and skipped count are logged to the local audit trail.

### 3.4 Feature 4: BSOD & Crash Minidump Reader (`src/diagnostics/minidump.rs`)

* **Problem:** Users suffering crashes have no easy way to know why their system rebooted.
* **Parser Implementation:**
  * Checks `C:\Windows\Minidump\*.dmp`.
  * Native binary parsing of the Windows 64-bit minidump header (`MINIDUMP_HEADER` signature `0x504D444D` / "MDMP").
  * Locates the `SystemInfoStream`, `ExceptionStream`, and `ModuleListStream`.
  * Extracts:
    * BugCheck Code (e.g., `0x000000D1`, `0x00000116`, `0x00000124`, `0x0000003B`).
    * Crash Timestamp (converted to local date/time).
    * Faulting Driver / Module Name (e.g., `nvlddmkm.sys`, `amdkmdag.sys`, `ntoskrnl.exe`, `rtwlane.sys`).
* **Diagnostic Dictionary:**
  * Built-in static mapping of common bugcheck codes and hardware drivers to human explanations:
    * `0x116` (`VIDEO_TDR_ERROR`): "GPU driver failed to respond and Windows reset the display adapter."
    * `0x124` (`WHEA_UNCORRECTABLE_ERROR`): "A fatal hardware error occurred (CPU voltage, unstable overclock, or dying SSD)."
    * `0xD1` (`DRIVER_IRQL_NOT_LESS_OR_EQUAL`): "A kernel driver accessed memory at high IRQL. Usually caused by Wi-Fi or graphics drivers."
    * `0x3B` (`SYSTEM_SERVICE_EXCEPTION`): "Uncaught exception in kernel mode."

### 3.5 Feature 5: Per-Process Dedicated VRAM Observability (`src/processes.rs`)

* **Problem:** GPU VRAM exhaustion causes severe stuttering in modern workloads, but Task Manager only provides system-wide VRAM totals.
* **Implementation:**
  * NVML Provider: `nvmlDeviceGetGraphicsRunningProcesses` returns `nvmlProcessInfo_t` containing PID and used VRAM in bytes.
  * Windows D3DKMT fallback: `D3DKMTQueryStatistics` with `D3DKMT_QUERYSTATISTICS_PROCESS_VIDMM` (or WMI `Win32_PerfFormattedData_GPUPerformanceCounters_GPUMonitoring`).
  * `ProcessInfo` struct adds: `pub vram_bytes: Option<u64>`.
  * UI: New "VRAM" column in `Processes` table with formatted MB/GB and sorting support.

### 3.6 Feature 6: IPv6 Socket Connection Inspection (`src/network.rs`)

* **Problem:** Network monitoring ignores IPv6 connections even though major cloud services and CDNs default to IPv6.
* **Implementation:**
  * Call `GetExtendedTcpTable` and `GetExtendedUdpTable` with `ulAf = AF_INET6` (23).
  * Parse `MIB_TCP6ROW_OWNER_PID` and `MIB_UDP6ROW_OWNER_PID`.
  * Format IPv6 addresses cleanly with scope IDs and port separation (`[2001:db8::1]:443`).
  * Process name resolution matches the existing PID cache.

---

## 4. Safety & Security Verification

1. **Least Privilege:**
   * File lock queries and minidump reading operate without requiring Administrator rights (when dumps are accessible to users or during elevated runs).
   * Actions modifying system state (terminating processes, clearing system temp files) use SysMon's standard `privilege.rs` elevation check and write to the local audit log.
2. **Memory Safety & Error Isolation:**
   * All binary parsing (minidump headers) is strictly bounded with slice-length checks to prevent panic on truncated or corrupted files.
   * Restart Manager sessions always close with `RmEndSession` via RAII guards.
3. **Audit Trail:**
   * Reclaimed disk cache operations and process terminations log exact paths and PIDs to `action_audit.log`.

---

## 5. Non-Goals

* No background kernel driver (e.g. WinRing0, custom rootkit driver) — all features use documented Win32/NT APIs.
* No automatic scheduled deletion — all cleanup actions are strictly manual and user-confirmed.
* No telemetry phone-home — crash dump information is strictly analyzed on the local CPU and never transmitted over the internet.
