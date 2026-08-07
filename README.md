# System Monitor - Professional System Monitoring for Windows

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows-blue.svg)](https://www.microsoft.com/windows)
[![GitHub stars](https://img.shields.io/github/stars/Xenonesis/sysmon?style=social)](https://github.com/Xenonesis/sysmon/stargazers)
[![Open Source](https://img.shields.io/badge/Type-Open%20Source-success.svg)](https://github.com/Xenonesis/sysmon)

> 🌟 **Open Source** - This project is open source and free to use. Star the repo to show your support!

A comprehensive, professional system monitoring application built with Rust featuring a beautiful native Windows GUI. Monitor CPU, RAM, GPU, storage, network, and processes in real-time with historical performance graphs.

![System Monitor](https://img.shields.io/badge/version-2.2.0-brightgreen)

---

## 🚀 Quick Download

**Visit our website for easy installation:** [systemmonitor.xenonesis.dev](https://systemmonitor.xenonesis.dev)

Or download directly from [GitHub Releases](https://github.com/Xenonesis/sysmon/releases)

### ⚡ Installer Download (Recommended)

- **Proper Windows install** - Installs to Program Files with Start Menu entry
- **Searchable** - Find it from Windows search / Start Menu
- **Clean uninstall** - Listed in Settings → Apps
- **Auto-updates** - App automatically checks for updates every 24 hours
- **Size:** ~5.4 MB - Lightweight and fast

Prefer portable? Use `system-monitor-<version>-windows-x64.exe` - no installation needed.

---

## ✨ Screenshots

> **Overview Tab** - Complete system dashboard
>
> **Performance Tab** - Historical graphs with 2 minutes of data
>
> **Storage Tab** - All drives with capacity and usage
>
> **Network Tab** - Real-time network rates and statistics

---

## Features

### 🎨 Modern GUI Interface

- ✅ **Professional multi-tab layout** with sidebar navigation
- ✅ **Menu bar** with View and Help options
- ✅ **Resizable window** (1100x800 default, minimum 900x600)
- ✅ **Quick Stats panel** in sidebar with color-coded metrics
- ✅ **Multiple views**: Overview, Performance, Processes, About

### 📊 Real-Time Monitoring

- ✅ **CPU usage** monitoring with visual progress bars
- ✅ **Memory (RAM)** usage with detailed breakdown
- ✅ **GPU monitoring** (NVIDIA GPUs via NVML)
  - GPU utilization percentage
  - GPU memory (VRAM) usage
  - GPU temperature with color-coded warnings (Green/Yellow/Red)
- ✅ **Process monitoring** - Top 15 processes by memory consumption
- ✅ **Auto-refresh** every 2 seconds

### 📈 Performance Graphs

- ✅ **Historical graphs** - Last 2 minutes (60 data points)
- ✅ **CPU usage history** - Real-time line chart
- ✅ **Memory usage history** - Visual trend tracking
- ✅ **GPU usage history** - Performance over time
- ✅ **Smooth animations** and updates

### 🎛️ Customization

- ✅ **Toggle graphs** on/off via View menu
- ✅ **Show/hide GPU section** based on preference
- ✅ **Show/hide process list** to simplify view
- ✅ **Color-coded indicators** (Green < 50%, Yellow 50-75%, Red > 75%)
- ✅ **Professional styling** with proper spacing

### 💻 Windows Integration

- ✅ **Standalone executable** - No dependencies after build
- ✅ **Desktop shortcut** - One-click installation
- ✅ **Start Menu entry** - Searchable and accessible
- ✅ **Native Windows app** - Full OS integration
- ✅ **Auto-update system** - Automatic update checking and installation
- ✅ **Direct .exe downloads** - No ZIP extraction needed

## 🎯 Key Features at a Glance

| Feature                   | Description                                                            |
| ------------------------- | ---------------------------------------------------------------------- |
| 💻 **7 Monitoring Tabs**  | Overview, Performance, Processes, Storage, Network, System Info, About |
| 📊 **Historical Graphs**  | 2 minutes of CPU/Memory/GPU history with smooth animations             |
| ⚙️ **Full Customization** | Settings panel with persistent configuration                           |
| 🎨 **Dual Themes**        | Dark mode and Light mode support                                       |
| 🚀 **High Performance**   | < 1% CPU usage, ~35-40 MB RAM                                          |
| 💾 **Storage Monitor**    | All drives with capacity, usage, and progress bars                     |
| 🌐 **Network Monitor**    | Real-time download/upload rates per interface                          |
| 💻 **System Info**        | Complete system specifications and uptime                              |
| 🎮 **GPU Support**        | NVIDIA GPU monitoring (utilization, temp, VRAM)                        |
| 📈 **Real-time Updates**  | Configurable refresh interval (1-10 seconds)                           |
| 🔄 **Auto-Update**        | Automatic update checking every 24 hours with one-click install        |
| ⬇️ **Direct Downloads**   | Website serves .exe directly - no installation hassle                  |

## 📋 Prerequisites

- **Windows 10/11** (64-bit)
- **Rust 1.70+** (for building from source)
- **NVIDIA GPU** (optional, for GPU monitoring)
- **NVIDIA Drivers** (if you have NVIDIA GPU)

## Installation

### Option 1: Installer (Recommended)

Download `SystemMonitor-<version>-setup.exe` from the [Releases page](https://github.com/Xenonesis/sysmon/releases).

The installer:

- Installs to `C:\Program Files\System Monitor`
- Adds a Start Menu entry (searchable from Windows search)
- Optionally creates a Desktop shortcut
- Registers the app in Settings → Apps for clean uninstall
- Requires administrator privileges (the app needs elevated rights for RAM cleaning, process management, and startup optimization)

### Option 2: Portable

Prefer running without installing? Download `system-monitor-<version>-windows-x64.exe` and run it directly. Settings are still stored per-user in `%APPDATA%\Xenonesis\SystemMonitor`.

### Building from source

1. Install [Rust](https://rustup.rs) 1.70+
2. `cargo build --release`
3. The executable is at `target\release\system-monitor.exe`

## Usage

Simply run the application and it will open a GUI window displaying:

- Real-time system statistics with auto-refresh
- Memory usage with visual progress bars
- CPU utilization with color-coded bars
- GPU stats (if NVIDIA GPU detected)
- Top 15 memory-consuming processes in a scrollable table

### Auto-Update Feature

- App automatically checks for updates every 24 hours
- Green notification banner appears when update available
- Click "Download & Install" for one-click update
- Or press `Ctrl+U` to manually check for updates

Close the window to exit.

## Color Coding

- 🟢 **Green**: < 50% usage (healthy)
- 🟡 **Yellow**: 50-75% usage (moderate)
- 🔴 **Red**: > 75% usage (high)

## Building Standalone Executable

To create a standalone executable:

```powershell
cargo build --release
```

The executable will be at: `target/release/system-monitor.exe`

## Dependencies

- `sysinfo` - Cross-platform system information
- `nvml-wrapper` - NVIDIA GPU monitoring
- `chrono` - Timestamp formatting
- `eframe` - GUI framework (egui backend)
- `egui` - Immediate mode GUI library
- `egui_plot` - Plotting widgets for egui

## Troubleshooting

**GPU stats not showing?**

- Ensure you have an NVIDIA GPU
- Make sure NVIDIA drivers are installed
- The app will gracefully fall back if GPU monitoring isn't available

**High CPU usage from monitor itself?**

- This is normal for real-time monitoring
- Adjust refresh rate in code if needed (change `Duration::from_secs(2)`)

## Recent Updates

- ✅ **Auto-update system** - Automatic update checking and one-click installation
- ✅ **Direct .exe downloads** - Website now serves executables directly
- ✅ **Downloads folder** - Build artifacts automatically saved for distribution
- ✅ **Smart download fallback** - Local-first, then GitHub releases
- ✅ **Update notifications** - In-app banner when new version available

## Future Enhancements

- [ ] Export logs to file
- [ ] Alert system for high usage
- [ ] Disk I/O statistics
- [ ] Process management (kill/suspend processes)
- [ ] System tray icon with notifications
- [ ] Code signing certificate for executable

## License

MIT License - Feel free to modify and use as needed!
