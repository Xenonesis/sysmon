# System Monitor - Complete Installation Guide

## 📋 Table of Contents
1. [System Requirements](#system-requirements)
2. [Quick Installation (3 Steps)](#quick-installation)
3. [Detailed Installation](#detailed-installation)
4. [Post-Installation](#post-installation)
5. [Uninstallation](#uninstallation)
6. [Troubleshooting](#troubleshooting)

---

## 🖥️ System Requirements

### Minimum Requirements
- **OS**: Windows 10 or Windows 11
- **RAM**: 4 GB (recommended: 8 GB+)
- **Storage**: 50 MB free space
- **Display**: 1280x720 or higher

### For Building from Source
- **Rust**: Latest stable version (1.70+)
- **Internet**: Required for downloading dependencies
- **Build Time**: 2-5 minutes (first build)

### For GPU Monitoring (Optional)
- **GPU**: NVIDIA graphics card
- **Drivers**: Latest NVIDIA drivers installed

---

## ⚡ Quick Installation (3 Steps)

### Step 1: Open PowerShell
Right-click in the folder containing the System Monitor files and select:
```
"Open in Terminal" or "Open PowerShell window here"
```

### Step 2: Build the Application
Copy and paste this command:
```powershell
.\build.ps1
```
Press **Enter** and wait for compilation to complete (2-5 minutes first time).

### Step 3: Install
Copy and paste this command:
```powershell
.\install.ps1
```
Press **Enter** and follow the prompts.

**Done!** Look for the System Monitor icon on your desktop! 🎉

---

## 📖 Detailed Installation

### Option A: Automated Installation (Recommended)

#### 1️⃣ Build the Application
```powershell
.\build.ps1
```

**What happens:**
- Downloads Rust dependencies (~350 packages)
- Compiles with optimizations
- Creates executable at `target\release\system-monitor.exe`
- Prompts to run immediately

**Expected Output:**
```
Building System Monitor GUI Application...

Building release version (optimized)...
[Compilation output...]

Build successful!

Executable location: target\release\system-monitor.exe

Would you like to run the application now? (Y/N)
```

#### 2️⃣ Install as Windows Application
```powershell
.\install.ps1
```

**What happens:**
- Creates installation directory: `%LOCALAPPDATA%\Programs\SystemMonitor`
- Copies executable to permanent location
- Creates Desktop shortcut
- Creates Start Menu entry
- Prompts to launch

**Expected Output:**
```
===================================
  System Monitor - Installation
===================================

Creating installation directory...
Installing System Monitor to: C:\Users\[YourName]\AppData\Local\Programs\SystemMonitor
Creating desktop shortcut...
Creating Start Menu shortcut...

Installation complete!

Would you like to launch System Monitor now? (Y/N)
```

### Option B: Manual Installation

#### 1️⃣ Build Manually
```powershell
cargo build --release
```

#### 2️⃣ Copy Executable
```powershell
# Choose a location
$destination = "$env:LOCALAPPDATA\Programs\SystemMonitor"
New-Item -ItemType Directory -Path $destination -Force
Copy-Item "target\release\system-monitor.exe" "$destination\system-monitor.exe"
```

#### 3️⃣ Create Shortcuts Manually
1. Right-click `system-monitor.exe`
2. Select "Create shortcut"
3. Move shortcut to Desktop or Start Menu

---

## 🎯 Post-Installation

### Verify Installation

#### Check Desktop Shortcut
Look for **"System Monitor"** icon on your desktop.

#### Check Start Menu
1. Press **Windows Key**
2. Type **"System Monitor"**
3. You should see the application

#### Test Run
1. Double-click the desktop shortcut
2. Application window should open
3. You should see real-time system stats

### Expected First Run
```
┌─────────────────────────────────────┐
│ 🖥️ System Monitor                   │
│ Last Update: 2024-12-15 02:50:15   │
├─────────────────────────────────────┤
│ 💾 Memory Usage                     │
│ Total: XX.XX GB                     │
│ Used:  XX.XX GB (XX.X%)             │
│ Free:  XX.XX GB                     │
│ [Progress Bar]                      │
└─────────────────────────────────────┘
[... more sections ...]
```

### Pin to Taskbar (Optional)
1. Right-click desktop shortcut
2. Select **"Pin to taskbar"**
3. Now accessible from taskbar!

### Set to Run at Startup (Optional)
1. Press `Win + R`
2. Type: `shell:startup`
3. Copy the shortcut to this folder

---

## 🗑️ Uninstallation

### Automated Uninstallation
```powershell
.\uninstall.ps1
```

Follow the prompts. This will:
- Remove installation directory
- Delete desktop shortcut
- Delete Start Menu entry

### Manual Uninstallation

#### 1. Delete Installation
```powershell
Remove-Item -Path "$env:LOCALAPPDATA\Programs\SystemMonitor" -Recurse -Force
```

#### 2. Delete Desktop Shortcut
```powershell
Remove-Item -Path "$env:USERPROFILE\Desktop\System Monitor.lnk"
```

#### 3. Delete Start Menu Entry
```powershell
Remove-Item -Path "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\System Monitor.lnk"
```

#### 4. Unpin from Taskbar (if pinned)
Right-click taskbar icon → Unpin

---

## 🔧 Troubleshooting

### Issue: "Rust not found" or "cargo not found"

**Solution:**
1. Install Rust from: https://rustup.rs/
2. Download and run `rustup-init.exe`
3. Restart PowerShell
4. Try again: `.\build.ps1`

### Issue: "Build failed" or compilation errors

**Solution:**
```powershell
# Update Rust to latest version
rustup update

# Clean previous build
cargo clean

# Try building again
.\build.ps1
```

### Issue: "Cannot run scripts" (PowerShell execution policy)

**Solution:**
```powershell
# Allow scripts for current user
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned

# Try again
.\build.ps1
```

### Issue: GPU stats not showing

**Expected Behavior:**
- Only NVIDIA GPUs are supported
- AMD/Intel GPUs will not show GPU section
- App works fine without GPU monitoring

**Solution:**
- Ensure NVIDIA drivers are installed
- Check Device Manager for GPU
- No fix needed if you don't have NVIDIA GPU

### Issue: Application won't start after installation

**Solutions:**

1. **Check if already running:**
   ```powershell
   Get-Process system-monitor -ErrorAction SilentlyContinue
   ```

2. **Run from command line to see errors:**
   ```powershell
   .\target\release\system-monitor.exe
   ```

3. **Rebuild:**
   ```powershell
   cargo clean
   .\build.ps1
   ```

### Issue: High memory usage from System Monitor itself

**Normal Behavior:**
- ~20-30 MB is normal
- Brief spikes during refresh are normal

**If excessive (>100 MB):**
- Close and restart the application
- Check for multiple instances running

### Issue: Window too small/large on first run

**Solution:**
- Resize by dragging corners
- Minimum size: 700x600 pixels
- Window size resets each launch (future: save preferences)

### Issue: Can't find executable after build

**Check location:**
```powershell
# Should be here:
Test-Path "target\release\system-monitor.exe"

# If True, run:
.\target\release\system-monitor.exe

# If False, rebuild:
.\build.ps1
```

### Issue: Slow compilation/build times

**Normal:**
- First build: 2-5 minutes (downloads dependencies)
- Subsequent builds: 30-60 seconds

**To speed up:**
```powershell
# Use all CPU cores
$env:CARGO_BUILD_JOBS = [Environment]::ProcessorCount
cargo build --release
```

---

## 📝 File Locations Reference

### Source Files
```
[Project Root]/
├── src/main.rs           ← Application source code
├── Cargo.toml            ← Dependencies configuration
├── build.ps1             ← Build script
├── install.ps1           ← Installation script
└── uninstall.ps1         ← Uninstallation script
```

### Built Executable
```
[Project Root]/target/release/system-monitor.exe
```

### Installation Location
```
%LOCALAPPDATA%\Programs\SystemMonitor\system-monitor.exe
(Usually: C:\Users\[YourName]\AppData\Local\Programs\SystemMonitor\)
```

### Shortcuts
```
Desktop: %USERPROFILE%\Desktop\System Monitor.lnk
Start Menu: %APPDATA%\Microsoft\Windows\Start Menu\Programs\System Monitor.lnk
```

---

## ✅ Installation Checklist

Use this checklist to ensure successful installation:

- [ ] Rust installed (or build completed with `.\build.ps1`)
- [ ] Build completed successfully
- [ ] Executable exists at `target\release\system-monitor.exe`
- [ ] Installation script run (`.\install.ps1`)
- [ ] Desktop shortcut created
- [ ] Start Menu entry created
- [ ] Application launches successfully
- [ ] System stats display correctly
- [ ] Window can be resized
- [ ] Application can be closed normally

---

## 🆘 Still Having Issues?

### Check Documentation
- **README.md** - Full feature documentation
- **QUICK_START.md** - Quick start guide
- **GUI_FEATURES.md** - Detailed GUI features
- **INSTRUCTIONS.md** - Quick reference

### System Requirements Check
```powershell
# Check Windows version
Get-ComputerInfo | Select-Object WindowsProductName, WindowsVersion

# Check available RAM
Get-CimInstance Win32_PhysicalMemory | Measure-Object -Property capacity -Sum | ForEach-Object {[math]::round($_.sum / 1GB, 2)}

# Check for NVIDIA GPU
Get-WmiObject Win32_VideoController | Select-Object Name
```

### Build Environment Check
```powershell
# Check Rust installation
cargo --version
rustc --version

# Check available disk space
Get-PSDrive C | Select-Object Free
```

---

## 💡 Tips for Best Experience

1. **First Time Users**: Follow Quick Installation steps above
2. **Developers**: Use manual build for customization
3. **Regular Users**: Use install script for clean setup
4. **Multiple PCs**: Copy `system-monitor.exe` (it's standalone!)
5. **Updates**: Rebuild with `.\build.ps1` to get latest changes

---

**Installation complete! Enjoy monitoring your system!** 🎉

For more information, see the other documentation files included with this application.
