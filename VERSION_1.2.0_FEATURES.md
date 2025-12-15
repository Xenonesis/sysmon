# System Monitor v1.2.0 - Release Notes

**Release Date:** December 15, 2024  
**Build Status:** ✅ Successful  
**New Features:** 5 major additions  
**Upgrade:** v1.1.0 → v1.2.0

---

## 🎉 What's New in v1.2.0

### 1. 🔥 CPU Cores Monitoring Tab (NEW!)

**Feature:** Dedicated tab for per-core CPU monitoring

**Details:**
- **Visual Grid Layout** - 4 cores per row for easy viewing
- **Per-Core Usage** - Real-time percentage for each core
- **Color-Coded Bars** - Green/Yellow/Red based on load
- **Core Statistics**:
  - Average usage across all cores
  - Maximum loaded core
  - Minimum loaded core
  - Count of cores above 50%
- **Logical Processor Display** - Shows total logical CPUs

**Location:** Click "🔥 CPU Cores" in sidebar (9th tab!)

**What You See:**
```
🔥 CPU Cores Monitoring
Total Cores: 14 (20 logical processors)

┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│ Core 0      │ │ Core 1      │ │ Core 2      │ │ Core 3      │
│ 25.3% 🟢    │ │ 45.2% 🟢    │ │ 78.1% 🔴    │ │ 12.5% 🟢    │
│ [█████░░░░] │ │ [█████░░░░] │ │ [████████░] │ │ [███░░░░░░] │
└─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘

📊 Core Statistics
Average Usage: 32.4% 🟢
Maximum Core: 78.1% 🔴
Minimum Core: 5.2%
Cores Above 50%: 3 / 20
```

**Use Cases:**
- Identify single-threaded bottlenecks
- Monitor multi-threaded app performance
- See core distribution in real-time
- Optimize process affinity
- Gaming performance analysis

---

### 2. ⚙️ Process Manager Window (NEW!)

**Feature:** Advanced process management with kill/suspend capabilities

**Details:**
- **Full Process List** - All top processes in one window
- **Kill Process Button** (🗑️) - Terminate processes
- **Suspend Button** (⏸️) - Suspend processes (Windows)
- **Process Details**:
  - PID (Process ID)
  - Process Name
  - Memory Usage (color-coded)
  - CPU Percentage
  - Status (Running, Sleeping, etc.)
- **Safety Warning** - Alerts before dangerous operations
- **Auto-Refresh** - Updates with main app

**Location:** Tools menu → "⚙️ Process Manager"

**Window Features:**
```
⚙️ Process Manager
Running Processes                          [🔄 Refresh]
─────────────────────────────────────────────────────

PID    | Process Name      | Memory    | CPU % | Status   | Actions
────────────────────────────────────────────────────────────────────
1234   | chrome.exe        | 1234 MB 🔴| 5.2%  | Running  | [🗑️][⏸️]
5678   | Discord.exe       | 856 MB 🟡 | 2.1%  | Running  | [🗑️][⏸️]
9012   | Code.exe          | 542 MB 🟡 | 3.8%  | Running  | [🗑️][⏸️]

⚠️ Warning: Killing processes may cause system instability!
```

**Safety Features:**
- Warning message at bottom
- Requires explicit click
- Shows process details before action
- Only shows top processes (configurable)

**Use Cases:**
- Kill frozen applications
- Manage runaway processes
- Free up memory quickly
- Advanced system management
- Troubleshooting performance issues

---

### 3. 💾 Save Report to File (NEW!)

**Feature:** Save system snapshot directly to file with native file picker

**Details:**
- **File Picker Dialog** - Native Windows file browser
- **Default Name** - "system-report.json"
- **JSON Format** - Complete system snapshot
- **One-Click Save** - Browse, name, save
- **No Copy/Paste** - Direct file creation

**Location:** Tools menu → "💾 Save Report to File"

**What Happens:**
1. Click "Save Report to File"
2. File picker opens
3. Choose location and name
4. File saved automatically
5. Contains full system data

**File Contents:**
- Timestamp
- CPU, Memory, GPU stats
- All processes
- Disk information
- Network statistics
- System information

**Use Cases:**
- Create system logs
- Document system state
- Share with support
- Historical records
- Performance benchmarks

---

### 4. ⚙️ Advanced Settings (NEW!)

**Feature:** Three new configuration options for power users

**New Settings:**

#### A. Show Per-Core CPU Usage
- **Option:** Checkbox in Settings
- **Effect:** Enables detailed core monitoring
- **Default:** Off (for cleaner view)
- **When On:** CPU Cores tab shows full details

#### B. Process List Count
- **Option:** Slider (5-30 processes)
- **Default:** 15 processes
- **Range:** 5 minimum, 30 maximum
- **Effect:** Changes how many processes are shown
- **Performance:** Higher = more data to collect

#### C. Auto-Clear Resolved Alerts
- **Option:** Checkbox
- **Effect:** Automatically removes resolved alerts
- **Default:** Off (keep history)
- **When On:** Alerts disappear when issue resolved

**Location:** Settings → "Advanced Options" section

**Configuration Panel:**
```
⚙️ Settings
─────────────

Advanced Options
☑ Show Per-Core CPU Usage
Process List Count: [====|====] 15
☐ Auto-clear resolved alerts
```

---

### 5. 📊 Process Status Display (NEW!)

**Feature:** Shows current process state

**Status Types:**
- **Running** - Active execution
- **Sleeping** - Waiting for resources
- **Stopped** - Paused/suspended
- **Zombie** - Terminated but not cleaned
- **Idle** - Waiting for work

**Details:**
- Displayed in Processes tab
- Shown in Process Manager window
- Updates in real-time
- OS-dependent values

**Benefits:**
- Better process understanding
- Identify stuck processes
- See waiting processes
- Advanced troubleshooting

---

## 📊 Complete Version Comparison

| Feature | v1.0.0 | v1.1.0 | v1.2.0 |
|---------|--------|--------|--------|
| **Tabs** | 7 | 8 | **9** ✨ |
| **Graphs** | 3 | 5 | 5 |
| **Popup Windows** | 1 | 3 | **4** ✨ |
| **Tools Menu Items** | 0 | 3 | **5** ✨ |
| **Settings Options** | 8 | 8 | **11** ✨ |
| **Total Features** | 40+ | 50+ | **60+** ✨ |
| **CPU Core Monitoring** | ❌ | ❌ | **✅** ✨ |
| **Process Management** | ❌ | ❌ | **✅** ✨ |
| **File Save** | ❌ | ❌ | **✅** ✨ |

---

## 🎯 All Tabs (9 Total)

1. 📋 **Overview** - Main dashboard
2. 📈 **Performance** - Historical graphs
3. ⚙️ **Processes** - Top processes list
4. 🔥 **CPU Cores** - NEW! Per-core monitoring
5. 💾 **Storage** - Disk devices
6. 🌐 **Network** - Network interfaces with graphs
7. 💻 **System Info** - Complete system specs
8. 🚨 **Alerts** - System alerts
9. ℹ️ **About** - Application information

---

## 🛠️ All Tools Menu Items (5)

1. 💾 **Export Data to JSON** - Copy to clipboard
2. 💾 **Save Report to File** - NEW! Save with file picker
3. 🔄 **Reset Statistics** - Clear all history
4. 🚨 **View Alerts** - Alert popup window
5. ⚙️ **Process Manager** - NEW! Manage processes

---

## 🔧 Technical Improvements

### Dependencies Added
- `rfd` v0.14 - Native file dialogs
- `windows` v0.52 - Windows API for process management

### New APIs
- Per-core CPU usage collection
- Process status retrieval
- Process kill/suspend functions
- File picker integration

### Performance
- **Memory Usage:** ~45-50 MB (slight increase for core data)
- **CPU Impact:** Still < 1%
- **Core Monitoring:** Minimal overhead
- **Process Management:** On-demand only

---

## 🧪 Testing Guide

### Test CPU Cores Tab
1. Go to **CPU Cores** tab
2. Count cores - should match your CPU specs
3. Open CPU-intensive app
4. Watch specific cores spike
5. Check core statistics update

### Test Process Manager
1. Tools → **Process Manager**
2. Window opens with process list
3. Find a safe process (e.g., Notepad)
4. Click 🗑️ button
5. Process should terminate
6. ⚠️ **DO NOT** kill system processes!

### Test Save to File
1. Tools → **Save Report to File**
2. File picker opens
3. Choose location (Desktop)
4. Name it "test-report.json"
5. Click Save
6. Open file in Notepad - should see JSON

### Test Advanced Settings
1. Settings → **Advanced Options**
2. Change Process Count to 20
3. Save Settings
4. Go to Processes tab
5. Should see 20 processes now

### Test Process Status
1. Go to Processes tab
2. Look for "Status" column
3. Should see "Running", "Sleeping", etc.
4. Status updates every 2 seconds

---

## ⚠️ Important Notes

### Process Manager Safety
- **DO NOT** kill:
  - System
  - svchost.exe
  - csrss.exe
  - winlogon.exe
  - dwm.exe
  - explorer.exe (your desktop!)

- **SAFE** to kill:
  - Your own applications
  - Browser tabs (chrome.exe)
  - Games you've closed
  - Frozen applications

### Process Suspend (Windows Only)
- Suspend feature is **Windows-specific**
- May not work on all processes
- System processes cannot be suspended
- Use carefully

### File Permissions
- Save Report needs write permissions
- Choose accessible location
- Desktop/Documents usually work
- May fail on protected folders

---

## 🚀 Upgrade Guide

### From v1.1.0 to v1.2.0
1. **No Breaking Changes**
2. All settings preserved
3. Simply rebuild and run
4. New features auto-available
5. New tabs appear automatically

### What Happens:
- CPU Cores tab added to sidebar
- Tools menu has 2 new items
- Settings has Advanced Options section
- Process Manager accessible via Tools

### Configuration:
- Default settings work for most users
- Adjust Process Count if needed
- Try Per-Core CPU for detailed view
- File save works out of the box

---

## 📊 Feature Highlights

### Most Requested
✅ CPU per-core monitoring
✅ Process management/kill
✅ Save to file directly

### Most Powerful
✅ Process Manager
✅ CPU Cores visualization
✅ Configurable process count

### Most Useful
✅ Save Report to File
✅ Core statistics
✅ Process status display

---

## 🎉 Summary

**System Monitor v1.2.0** adds **5 major features**:
1. ✅ CPU Cores monitoring tab
2. ✅ Process Manager window
3. ✅ Save Report to File
4. ✅ Advanced Settings options
5. ✅ Process status display

**Now Features:**
- 9 tabs
- 4 popup windows
- 5 tools menu items
- 60+ total features
- 11 settings options

**Perfect for:**
- Power users
- System administrators
- Gamers monitoring performance
- Developers debugging
- Anyone wanting detailed control

**Still Maintains:**
- Low memory footprint
- < 1% CPU usage
- Fast and responsive
- Professional design
- Easy to use

---

*System Monitor v1.2.0 - Professional Edition*  
*Build Date: December 15, 2024*  
*Executable Size: 5.37 MB*
