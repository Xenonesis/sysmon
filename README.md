# SysMon

**Refined System Intelligence for Windows**

A comprehensive, professional system monitoring application built with Rust featuring a native, high-performance GUI. SysMon delivers real-time telemetry across CPU, memory, GPU, storage, network, and active processes with historical performance tracking.

[![License: MIT](https://img.shields.io/badge/License-MIT-gray.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-gray.svg?style=flat-square)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows-gray.svg?style=flat-square)](https://www.microsoft.com/windows)
[![Version](https://img.shields.io/badge/Version-2.4.0-gray.svg?style=flat-square)](https://github.com/Xenonesis/sysmon/releases/latest)

---

## Download & Installation

The recommended distribution is our standalone Windows installer, providing automatic updates and seamless system integration.

**[Download SysMon Installer (v2.4.0)](https://systemmonitor.xenonesis.dev)**

**Installer Features:**
* **Native Integration:** Installs to `Program Files` with a searchable Start Menu entry.
* **Maintenance:** Clean uninstallation via Windows Settings → Apps.
* **Auto-updates:** Silent, background update checking every 24 hours.
* **Footprint:** Extremely lightweight (~5 MB).

---

## Core Capabilities

### Modern GUI Interface
* **Premium Typography & Spacing:** Designed using an 8dp spacing rhythm and clear visual hierarchies.
* **Dual Themes:** Polished 'Terminal Noir' dark mode and an Apple-inspired minimal light mode.
* **Adaptive Layout:** Resizable interface with a quick-stats sidebar and multi-tab structure.
* **Visual States:** Color-coded usage indicators and smooth transition animations.

### Real-Time Monitoring
* **Processor (CPU):** Usage monitoring with per-core analysis and thermal tracking.
* **Memory (RAM):** Comprehensive breakdown with threshold-based auto-cleaning.
* **Graphics (GPU):** Full NVIDIA NVML integration (Utilization, VRAM, Temp, Clock Speed, Power Draw, Fan Speed).
* **Network & Storage:** Live interface telemetry, capacities, and read/write rates.

### Process Intelligence (Process Pro)
* **Deep Process Details:** Executable paths, command-line arguments, start times, and parent process lineage.
* **Task Management:** Granular control to Suspend, Resume, Change Priority, and Kill tasks.
* **Kill Tree:** Graceful termination of a process and its entire descendant tree.
* **Real-time Filtering:** Sort by memory/CPU and substring search by name or PID.

### System Integration
* **Tray Quick Actions:** Clean RAM, Open Process Manager, or Pause Monitoring directly from the system tray.
* **Power Plan Toggle:** Switch between installed Windows power plans straight from the tray menu.
* **Desktop Mini-Widget:** A compact always-visible overlay showing live CPU, RAM, GPU, network, and thermal telemetry.
* **System Information:** WMI-enriched motherboard, BIOS, GPU driver, and OS build telemetry.
* **Data Export:** Snapshot current system state to CSV or JSON formats for analysis.
* **Notifications:** Windows-native alerts for high CPU, memory, GPU temperatures, or heavy startup impact.

---

## Prerequisites & Building from Source

**Requirements:**
* Windows 10/11 (64-bit)
* Rust 1.70+ (for source compilation)
* NVIDIA Drivers (optional, for GPU telemetry)

**Build Instructions:**
```powershell
cargo build --release
```
The compiled executable will be located at: `target/release/system-monitor.exe`. 
*Note: The application requires administrator privileges for advanced process management and RAM cleaning.*

---

## Changelog

### [2.4.0] — Feature Expansion
* **Process Pro:** Added an exhaustive details panel (path, command line, threads, start time) and a Deep Kill Tree action.
* **Tray Quick Actions:** Upgraded system tray menu with actions to Clean RAM, pause monitoring, and open the process manager.
* **Power Plan Toggle:** Enumerate and switch active Windows power schemes from the tray menu.
* **Desktop Mini-Widget:** Floating telemetry overlay (CPU, RAM, GPU, network, thermals), toggled from Settings.
* **Advanced GPU Metrics:** Added real-time Clock Speed, Power Draw, and Fan Speed metrics via NVML.
* **System Info Enrichment:** Deeper system insights using WMI (Motherboard, BIOS, GPU Driver, OS Build).
* **UI/UX Polish:** Refined application spacing, corner radiuses, and shadow elevations across themes for a premium feel.

### [2.3.0] — Deployment & Quality
* **Installer-First Distribution:** Fully transitioned to automated, installer-based updates.
* **Silent Updates:** Background installer routines for seamless upgrades.
* **Notification Enhancements:** In-app banners for available updates.

---

## Roadmap

* [ ] Code signing certificate for trusted execution.

---

## License

Released under the [MIT License](LICENSE).
