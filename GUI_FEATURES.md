# System Monitor - GUI Features & User Guide

## 🎨 Modern GUI Interface

The System Monitor now features a **fully functional graphical user interface** built with the modern `egui` framework, providing a native Windows application experience.

---

## 🖥️ Main Window Layout

### Header Section
- **Title**: 🖥️ System Monitor
- **Last Update**: Real-time timestamp showing when data was last refreshed
- **Auto-refresh**: Updates every 2 seconds automatically

### Memory Usage Panel (💾)
```
┌─────────────────────────────────────┐
│ 💾 Memory Usage                     │
├─────────────────────────────────────┤
│ Total: 15.70 GB                     │
│ Used:  13.45 GB (85.7%)             │
│ Free:  2.25 GB                      │
│ [████████████████████░░] 85.7%      │
└─────────────────────────────────────┘
```
- Shows total, used, and free RAM
- Color-coded progress bar
- Real-time percentage

### CPU Usage Panel (⚡)
```
┌─────────────────────────────────────┐
│ ⚡ CPU Usage                         │
├─────────────────────────────────────┤
│ Usage: 12.3%                        │
│ [████░░░░░░░░░░░░░░░░] 12.3%        │
└─────────────────────────────────────┘
```
- Real-time CPU utilization
- Visual progress bar with color coding

### GPU Usage Panel (🎮) - NVIDIA Only
```
┌─────────────────────────────────────┐
│ 🎮 GPU Usage                         │
├─────────────────────────────────────┤
│ Name: NVIDIA GeForce RTX 3060       │
│ Utilization: 45.2%                  │
│ [█████████░░░░░░░░░░░] 45.2%        │
│ Memory: 2048 MB / 4096 MB (50.0%)   │
│ Temperature: 65°C                   │
└─────────────────────────────────────┘
```
- GPU model name
- Utilization percentage
- VRAM usage
- Temperature with color warnings

### Process Table (📊)
```
┌─────────────────────────────────────────────────────────┐
│ 📊 Top 15 Processes by Memory                           │
├─────────┬──────────────────────┬───────────┬───────────┤
│ PID     │ Name                 │ Memory    │ CPU %     │
├─────────┼──────────────────────┼───────────┼───────────┤
│ 12345   │ chrome.exe           │ 1234.56 MB│ 5.2%      │
│ 67890   │ Discord.exe          │ 856.34 MB │ 2.1%      │
│ ...     │ ...                  │ ...       │ ...       │
└─────────┴──────────────────────┴───────────┴───────────┘
```
- Scrollable table
- Top 15 memory consumers
- Real-time CPU usage per process
- Color-coded memory values

---

## 🎨 Color Coding System

### Usage Levels
| Color | Percentage | Status | Visual |
|-------|-----------|--------|--------|
| 🟢 **Green** | 0-49% | Healthy | Low usage, system running smoothly |
| 🟡 **Yellow** | 50-74% | Moderate | Moderate usage, monitor carefully |
| 🔴 **Red** | 75-100% | High | High usage, may impact performance |

### Temperature Colors (GPU)
| Color | Temperature | Status |
|-------|------------|--------|
| 🟢 Green | < 70°C | Normal operating temperature |
| 🟡 Yellow | 70-84°C | Warm, but acceptable |
| 🔴 Red | ≥ 85°C | Hot, check cooling |

### Memory Colors (Process Table)
| Color | Memory | Status |
|-------|--------|--------|
| 🟢 Green | < 200 MB | Low memory usage |
| 🟡 Yellow | 200-500 MB | Moderate memory usage |
| 🔴 Red | > 500 MB | High memory usage |

---

## 🖱️ Window Controls

### Resizing
- **Default Size**: 900x800 pixels
- **Minimum Size**: 700x600 pixels
- **Resizable**: Drag any corner or edge
- **Adaptive**: Content adjusts to window size

### Movement
- Drag the title bar to move the window
- Standard Windows window controls

### Closing
- Click the X button (top-right)
- Press `Alt + F4`
- All standard Windows shortcuts work

---

## ⚡ Performance

### Resource Usage
- **CPU Impact**: < 1% on modern systems
- **Memory Usage**: ~20-30 MB
- **Update Frequency**: 2 seconds (configurable in code)
- **Background Thread**: Separate monitoring thread for smooth UI

### Optimization
- Compiled with full optimizations (`--release`)
- Link-time optimization (LTO) enabled
- No unnecessary allocations during updates
- Efficient GUI rendering with `egui`

---

## 🔧 Technical Details

### GUI Framework
- **Library**: egui (Immediate Mode GUI)
- **Backend**: eframe (native window support)
- **Rendering**: Hardware-accelerated (wgpu/OpenGL)
- **Cross-platform**: Works on Windows, macOS, Linux

### System Monitoring
- **Library**: sysinfo (cross-platform)
- **GPU Support**: nvml-wrapper (NVIDIA only)
- **Update Model**: Background thread + shared state

### Architecture
```
┌─────────────────────────────────────┐
│     GUI Thread (Main)               │
│     - Renders UI at 60 FPS          │
│     - Displays data                 │
└──────────┬──────────────────────────┘
           │ Arc<Mutex<SystemData>>
┌──────────▼──────────────────────────┐
│     Monitoring Thread               │
│     - Collects system data          │
│     - Updates every 2 seconds       │
└─────────────────────────────────────┘
```

---

## 📊 Data Display

### Memory Information
- **Precision**: 2 decimal places for GB
- **Update**: Real-time
- **Calculation**: Used / Total × 100%

### CPU Information
- **Precision**: 1 decimal place
- **Measurement**: Global CPU usage (all cores)
- **Average**: Averaged over update interval

### GPU Information
- **Support**: NVIDIA GPUs only (via NVML)
- **Metrics**: Utilization, Memory, Temperature
- **Fallback**: Section hidden if no NVIDIA GPU

### Process Information
- **Count**: Top 15 by memory
- **Sorting**: Descending by memory usage
- **Update**: Refreshed every cycle
- **Name Length**: Truncated to 35 characters

---

## 🎯 Use Cases

### 1. Gaming Performance Monitoring
Monitor GPU temperature and memory while gaming to ensure optimal performance.

### 2. Development Work
Track memory usage when running multiple development tools and IDEs.

### 3. Troubleshooting
Identify which processes are consuming the most resources.

### 4. System Health Check
Quick overview of system status at a glance.

### 5. Background Monitoring
Run in a corner of your screen while working.

---

## 🆚 Comparison: GUI vs Terminal Version

| Feature | GUI Version | Terminal Version |
|---------|------------|------------------|
| Interface | Modern GUI | Text-based TUI |
| Installation | Installable app | Script only |
| Shortcuts | Desktop + Start Menu | Command line |
| Visual Appeal | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Resource Usage | ~20-30 MB | ~10-15 MB |
| Resizable | Yes | Terminal size |
| Colors | Rich colors | Terminal colors |
| Scrolling | Native scrolling | Terminal scroll |
| Multiple Windows | Yes | Multiple terminals |
| User Friendly | Very easy | Requires PowerShell |

---

## 🚀 Advanced Features

### Future Enhancements (Planned)

1. **Historical Graphs**
   - Time-series charts for CPU/Memory/GPU
   - Last 60 seconds of data
   - Interactive zoom/pan

2. **Dark/Light Theme**
   - Theme switcher
   - Follows Windows theme
   - Custom color schemes

3. **System Tray Icon**
   - Minimize to tray
   - Quick stats in tooltip
   - Right-click menu

4. **Alerts & Notifications**
   - Set threshold alerts
   - Windows notifications
   - Custom alert rules

5. **Export/Logging**
   - Export stats to CSV
   - Continuous logging
   - Scheduled reports

6. **Process Management**
   - Kill processes from GUI
   - Suspend/resume
   - Process details

7. **Settings Panel**
   - Configurable refresh rate
   - Choose metrics to display
   - Save preferences

8. **Multiple Monitor Support**
   - Track specific GPUs
   - Multi-core CPU breakdown
   - Per-process details

---

## 💡 Tips & Tricks

### Performance Tips
1. **Increase refresh interval** for lower CPU usage (edit code)
2. **Close when not needed** - no background service
3. **Single instance** recommended for normal use

### Workflow Integration
1. **Pin to taskbar** for one-click access
2. **Keep on second monitor** if available
3. **Run before heavy tasks** to establish baseline

### Keyboard Shortcuts
- `Alt + F4`: Close application
- `Win + D`: Minimize all (show desktop)
- Window snapping works (Win + Arrow keys)

---

## 🔍 Understanding the Metrics

### Memory Percentage
- **Formula**: (Used Memory / Total Memory) × 100
- **High Usage**: > 75% may cause slowdowns
- **Healthy**: < 50% is ideal for multitasking

### CPU Usage
- **Measurement**: Percentage of CPU capacity in use
- **Spikes**: Normal during intense operations
- **Sustained High**: May indicate background processes

### GPU Temperature
- **Idle**: 30-50°C is normal
- **Load**: 60-80°C is typical for gaming
- **Critical**: > 90°C requires immediate attention

---

## 📝 Notes

- **Windows Only**: Designed and tested for Windows 10/11
- **NVIDIA GPU**: GPU monitoring requires NVIDIA hardware
- **Rust Required**: Only for building from source
- **No Admin**: Runs with normal user privileges
- **Standalone**: Single executable, no dependencies after build

---

**Enjoy your modern GUI system monitor!** 🎉
