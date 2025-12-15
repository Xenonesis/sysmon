# System Monitor v1.2.0 - Professional Edition

**Release Date:** December 15, 2024  
**Download:** [system-monitor.exe](https://github.com/Xenonesis/sysmon/releases/download/v1.2.0/system-monitor.exe) (5.37 MB)

---

## 🎉 What's New in v1.2.0

This is a **major update** adding professional-grade features for power users and system administrators!

### 🔥 NEW: CPU Cores Monitoring Tab

Monitor individual CPU core usage in real-time!

- **Visual Grid Layout** - See all cores at once (4 per row)
- **Per-Core Usage** - Real-time percentage for each core
- **Color-Coded Bars** - Green/Yellow/Red based on load
- **Core Statistics** - Average, maximum, minimum usage
- **Perfect for**: Gaming, rendering, multi-threaded workloads

![CPU Cores Tab](screenshots/cpu-cores.png)

### ⚙️ NEW: Process Manager

Advanced process management with kill/suspend capabilities!

- **Kill Processes** - One-click termination (🗑️ button)
- **Suspend Processes** - Pause processes on Windows (⏸️ button)
- **Full Details** - PID, Name, Memory, CPU, Status
- **Safety Warnings** - Prevents accidental system crashes
- **Perfect for**: Troubleshooting, freeing memory, managing apps

![Process Manager](screenshots/process-manager.png)

### 💾 NEW: Save Report to File

Save system snapshots directly to disk!

- **Native File Picker** - Windows file dialog
- **JSON Format** - Complete system state
- **One-Click Save** - No copy/paste needed
- **Perfect for**: Documentation, bug reports, logging

### ⚙️ NEW: Advanced Settings

More control for power users!

- **Per-Core CPU Display** - Toggle detailed core view
- **Process Count** - Configure 5-30 processes (default: 15)
- **Auto-Clear Alerts** - Automatically remove resolved alerts
- **Perfect for**: Customizing your monitoring experience

### 📊 NEW: Process Status Display

See process states in real-time!

- Shows: Running, Sleeping, Stopped, Zombie, Idle
- Visible in Processes tab and Process Manager
- Updates every 2 seconds

---

## 📦 Download & Install

### Quick Install (Recommended)
1. Download [system-monitor.exe](https://github.com/Xenonesis/sysmon/releases/download/v1.2.0/system-monitor.exe)
2. Run the executable
3. Pin to taskbar for easy access

### Install with Scripts
```powershell
# Clone the repository
git clone https://github.com/Xenonesis/sysmon.git
cd sysmon

# Build and install
.\build.ps1
.\install.ps1
```

---

## ✨ Complete Feature List

### 9 Tabs
1. 📋 **Overview** - System dashboard
2. 📈 **Performance** - Historical graphs (CPU, Memory, GPU)
3. ⚙️ **Processes** - Top processes by memory
4. 🔥 **CPU Cores** - NEW! Per-core monitoring
5. 💾 **Storage** - All drives with usage
6. 🌐 **Network** - Interfaces with download/upload graphs
7. 💻 **System Info** - Complete system specifications
8. 🚨 **Alerts** - System alerts and warnings
9. ℹ️ **About** - Application information

### 5 Graph Types
- CPU usage history (2 minutes)
- Memory usage history (2 minutes)
- GPU usage history (2 minutes)
- Network download rate (2 minutes)
- Network upload rate (2 minutes)

### 4 Popup Windows
- ⚙️ Settings
- 💾 Export Data
- 🚨 View Alerts
- ⚙️ Process Manager (NEW!)

### Tools Menu
- Export Data to JSON
- Save Report to File (NEW!)
- Reset Statistics
- View Alerts
- Process Manager (NEW!)

---

## 📊 System Requirements

### Minimum
- **OS:** Windows 10 (64-bit)
- **RAM:** 512 MB available
- **Disk:** 50 MB free space
- **Display:** 1280x720

### Recommended
- **OS:** Windows 11 (64-bit)
- **RAM:** 1 GB available
- **Disk:** 100 MB free space
- **Display:** 1920x1080
- **GPU:** NVIDIA (for GPU monitoring)

---

## 🚀 Performance

- **Memory Usage:** ~45-50 MB
- **CPU Impact:** < 1%
- **Startup Time:** < 1 second
- **Update Frequency:** 2 seconds (configurable 1-10s)

---

## 🆚 Version History

| Version | Release Date | Key Features |
|---------|-------------|--------------|
| **v1.2.0** | Dec 15, 2024 | CPU Cores, Process Manager, Save to File |
| v1.1.0 | Dec 15, 2024 | Network Graphs, Alerts System, Export JSON |
| v1.0.0 | Dec 15, 2024 | Initial GUI release with 7 tabs |

---

## 🐛 Known Issues

- GPU monitoring requires NVIDIA hardware
- Process suspend feature is Windows-specific
- File save requires write permissions to target folder

---

## 📝 Changelog

### Added
- CPU Cores monitoring tab with grid layout
- Process Manager window with kill/suspend
- Save Report to File with native file picker
- Advanced Settings (3 new options)
- Process status display
- Enhanced process information

### Improved
- Better process identification
- More granular control options
- Enhanced customization

### Technical
- Added `rfd` library for file dialogs
- Added `windows` API for process management
- Per-core CPU data collection
- Process status retrieval

---

## 🙏 Acknowledgments

Built with:
- **Rust** - Safe, fast, concurrent programming
- **egui** - Immediate mode GUI framework
- **sysinfo** - Cross-platform system information
- **nvml-wrapper** - NVIDIA GPU monitoring

---

## 📄 License

MIT License - Free and open source

---

## 🔗 Links

- **Repository:** https://github.com/Xenonesis/sysmon
- **Issues:** https://github.com/Xenonesis/sysmon/issues
- **Documentation:** See repository README.md

---

## 💡 Usage Tips

### CPU Cores Tab
- Watch for uneven core distribution (single-threaded bottleneck)
- Monitor during gaming to see core utilization
- Identify thermal throttling patterns

### Process Manager
- **SAFE:** Kill your own applications, frozen apps
- **DANGEROUS:** Avoid killing system processes
- Test with Notepad first
- Use for freeing stuck memory

### Save Report
- Create logs before/after changes
- Include in bug reports
- Document system states
- Compare performance over time

---

## 🆘 Support

For questions or issues:
1. Check documentation in repository
2. Search existing issues
3. Create new issue with details
4. Include system information

---

**Download Now:** [system-monitor.exe](https://github.com/Xenonesis/sysmon/releases/download/v1.2.0/system-monitor.exe)

**Happy Monitoring!** 🎉
