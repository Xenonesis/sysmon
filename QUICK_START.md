# System Monitor - Quick Start Guide

## 🚀 For First-Time Users

### Step 1: Build the Application
Open PowerShell in this folder and run:
```powershell
.\build.ps1
```

This will:
- Compile the application with optimizations
- Create the executable at `target\release\system-monitor.exe`
- Offer to run it immediately

### Step 2: Install (Optional)
To install as a Windows application:
```powershell
.\install.ps1
```

This will:
- Copy the executable to a permanent location
- Create a Desktop shortcut
- Create a Start Menu shortcut
- Allow you to launch it like any other Windows app

### Step 3: Run the Application
After installation, you can run System Monitor by:
- Double-clicking the desktop shortcut
- Searching "System Monitor" in Windows Start Menu
- Running `.\target\release\system-monitor.exe` directly

---

## 📊 What You'll See

The GUI window displays:

### Memory Section
- **Total RAM**: Your system's total memory
- **Used RAM**: Currently used memory with percentage
- **Free RAM**: Available memory
- **Progress Bar**: Color-coded (Green < 50%, Yellow 50-75%, Red > 75%)

### CPU Section
- **Usage Percentage**: Real-time CPU utilization
- **Progress Bar**: Color-coded by usage level

### GPU Section (NVIDIA only)
- **GPU Name**: Your graphics card model
- **Utilization**: GPU usage percentage
- **Memory**: VRAM usage
- **Temperature**: Current GPU temperature with color coding

### Process Table
- **Top 15 Processes**: Sorted by memory consumption
- **Columns**: PID, Name, Memory (MB), CPU %
- **Scrollable**: Navigate through the list
- **Color-Coded Memory**: Red > 500MB, Yellow > 200MB, Green < 200MB

---

## 🎨 GUI Features

### Window Controls
- **Resizable**: Drag corners to resize (minimum 700x600)
- **Auto-Refresh**: Updates every 2 seconds automatically
- **Timestamp**: Shows last update time
- **Close Button**: Standard window close (X)

### Visual Indicators
- 🟢 **Green**: Healthy (< 50% usage)
- 🟡 **Yellow**: Moderate (50-75% usage)
- 🔴 **Red**: High (> 75% usage)

---

## 💡 Tips

### Pin to Taskbar
1. Right-click the desktop shortcut
2. Select "Pin to taskbar"
3. Quick access from your taskbar!

### Run at Startup (Optional)
1. Press `Win + R`
2. Type: `shell:startup`
3. Copy the shortcut to this folder

### Create Multiple Instances
You can run multiple windows if needed - just launch it again!

### Minimal CPU Usage
The monitor uses minimal resources and won't slow down your system.

---

## 🔧 Troubleshooting

### Build Fails
- Ensure Rust is installed: https://rustup.rs/
- Run: `rustup update`
- Try again: `.\build.ps1`

### GPU Stats Not Showing
- Only NVIDIA GPUs are supported
- Ensure NVIDIA drivers are installed
- AMD/Intel GPUs will simply not show GPU section

### Window Too Small/Large
- Resize the window by dragging corners
- Minimum size: 700x600 pixels
- Window size is not saved between sessions (future enhancement)

### Application Won't Start
- Check if another instance is running
- Try running from command line to see error messages:
  ```powershell
  .\target\release\system-monitor.exe
  ```

---

## 📁 File Structure

```
system-monitor/
├── src/
│   └── main.rs          # Main GUI application code
├── target/
│   └── release/
│       └── system-monitor.exe  # Compiled executable
├── Cargo.toml           # Rust dependencies
├── build.ps1            # Build script
├── install.ps1          # Installation script
├── README.md            # Full documentation
└── QUICK_START.md       # This file
```

---

## 🎯 Next Steps

After getting familiar with the application:
1. ✅ Monitor your system during gaming or heavy workloads
2. ✅ Identify memory-hungry processes
3. ✅ Keep an eye on GPU temperature during intensive tasks
4. ✅ Share the executable with friends (it's standalone!)

---

## 🆘 Need Help?

- Check the full README.md for detailed information
- The application is open source - feel free to modify!
- Report issues or suggest features

---

**Enjoy monitoring your system!** 🖥️✨
