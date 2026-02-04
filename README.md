# System Monitor - Professional System Monitoring for Windows

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows-blue.svg)](https://www.microsoft.com/windows)

A comprehensive, professional system monitoring application built with Rust featuring a beautiful native Windows GUI. Monitor CPU, RAM, GPU, storage, network, and processes in real-time with historical performance graphs.

![System Monitor](https://img.shields.io/badge/version-1.0.0-brightgreen)

---

## 🚀 Quick Download

**Visit our website for easy installation:** [systemmonitor.xenonesis.dev](https://systemmonitor.xenonesis.dev)

Or download directly from [GitHub Releases](https://github.com/Xenonesis/sysmon/releases)

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

## 🎯 Key Features at a Glance

| Feature | Description |
|---------|-------------|
| 💻 **7 Monitoring Tabs** | Overview, Performance, Processes, Storage, Network, System Info, About |
| 📊 **Historical Graphs** | 2 minutes of CPU/Memory/GPU history with smooth animations |
| ⚙️ **Full Customization** | Settings panel with persistent configuration |
| 🎨 **Dual Themes** | Dark mode and Light mode support |
| 🚀 **High Performance** | < 1% CPU usage, ~35-40 MB RAM |
| 💾 **Storage Monitor** | All drives with capacity, usage, and progress bars |
| 🌐 **Network Monitor** | Real-time download/upload rates per interface |
| 💻 **System Info** | Complete system specifications and uptime |
| 🎮 **GPU Support** | NVIDIA GPU monitoring (utilization, temp, VRAM) |
| 📈 **Real-time Updates** | Configurable refresh interval (1-10 seconds) |

## 📋 Prerequisites

- **Windows 10/11** (64-bit)
- **Rust 1.70+** (for building from source)
- **NVIDIA GPU** (optional, for GPU monitoring)
- **NVIDIA Drivers** (if you have NVIDIA GPU)

## Installation

### Quick Build

Run the build script:
```powershell
.\build.ps1
```

### Manual Build

1. Build the project:
```powershell
cargo build --release
```

2. Run the monitor:
```powershell
.\target\release\system-monitor.exe
```

### Install as Windows Application

Copy the executable to your desired location:
```powershell
# Copy to a permanent location
Copy-Item "target\release\system-monitor.exe" "C:\Program Files\SystemMonitor\system-monitor.exe"

# Or copy to your user folder
Copy-Item "target\release\system-monitor.exe" "$env:USERPROFILE\Applications\system-monitor.exe"
```

Then create a desktop shortcut or pin to taskbar!

## Usage

Simply run the application and it will open a GUI window displaying:
- Real-time system statistics with auto-refresh
- Memory usage with visual progress bars
- CPU utilization with color-coded bars
- GPU stats (if NVIDIA GPU detected)
- Top 15 memory-consuming processes in a scrollable table

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

## Future Enhancements

- [ ] Historical graphs with time-series data
- [ ] Export logs to file
- [ ] Alert system for high usage
- [ ] Network usage monitoring
- [ ] Disk I/O statistics
- [ ] Process management (kill/suspend processes)
- [ ] System tray icon with notifications
- [ ] Multiple themes (dark/light mode)
- [ ] Customizable refresh intervals
- [ ] Save window position and size preferences

## License

MIT License - Feel free to modify and use as needed!
