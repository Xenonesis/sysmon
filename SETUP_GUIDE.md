# System Monitor - Complete Setup Guide

## Quick Start Guide for Windows

### Prerequisites
This application requires proper build tools to compile. Choose **ONE** of the following options:

---

## Option 1: Using MSVC Toolchain (Recommended for Windows)

### Step 1: Install Visual Studio Build Tools
1. Download **Visual Studio Build Tools** from: https://visualstudio.microsoft.com/downloads/
2. Run the installer
3. Select "Desktop development with C++"
4. Install (requires ~7GB)

### Step 2: Configure Rust
```powershell
rustup default stable-x86_64-pc-windows-msvc
```

### Step 3: Build the Application
```powershell
cd "c:\Users\addy\Pictures\rust app"
cargo build --release
```

---

## Option 2: Using MSYS2/MinGW64 (Alternative)

### Step 1: Install MSYS2
1. Download MSYS2 from: https://www.msys2.org/
2. Run the installer (install to C:\msys64)
3. Open "MSYS2 MinGW 64-bit" terminal
4. Update MSYS2:
```bash
pacman -Syu
```
5. Install MinGW64 toolchain:
```bash
pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-toolchain
```

### Step 2: Add MSYS2 to Windows PATH
Add this to your PATH environment variable:
```
C:\msys64\mingw64\bin
```

### Step 3: Configure Rust
```powershell
rustup default stable-x86_64-pc-windows-gnu
```

### Step 4: Build the Application
```powershell
cd "c:\Users\addy\Pictures\rust app"
cargo build --release
```

---

## Option 3: Download Pre-Built Executable (Easiest)

If you don't want to build from source:

1. Download the latest release from the GitHub releases page
2. Extract `system-monitor.exe`
3. Run the installation script

---

## After Building Successfully

### Running the Application
```powershell
.\target\release\system-monitor.exe
```

### Installing the Application
```powershell
.\install.ps1
```

This will:
- Copy the executable to a permanent location
- Create desktop and Start Menu shortcuts
- Set up the application for easy access

### Uninstalling
```powershell
.\uninstall.ps1
```

---

## Features

✅ **Real-time Monitoring**
- CPU usage (overall and per-core)
- Memory usage
- GPU usage and temperature (NVIDIA cards)
- Disk space
- Network activity

✅ **Performance Graphs**
- Historical data visualization
- 2-minute rolling window
- Color-coded indicators

✅ **Process Management**
- View running processes
- Search and filter
- Memory and CPU usage per process
- Kill processes (use with caution)

✅ **System Information**
- OS details
- Hardware information
- Uptime tracking

✅ **Alerts & Notifications**
- Configurable thresholds
- High CPU/Memory alerts
- GPU temperature warnings
- Low disk space alerts

✅ **Data Export**
- Export to JSON
- Export to CSV
- Save reports to file

✅ **Customization**
- Dark/Light themes
- Auto-start with Windows
- Configurable refresh intervals
- Always-on-top mode

---

## Keyboard Shortcuts

- `F5` - Reset statistics
- `Ctrl+E` - Export data
- `Ctrl+,` - Open settings

---

## Troubleshooting

### Build Errors

**Error: "linker `link.exe` not found"**
- Solution: Install Visual Studio Build Tools (Option 1 above)

**Error: "dlltool.exe not found"**
- Solution: Install MSYS2/MinGW64 (Option 2 above)

**Error: "Access is denied" when building**
- Solution: Close any running instances of system-monitor.exe

### Runtime Errors

**GPU information not showing**
- This is normal if you don't have an NVIDIA GPU
- GPU monitoring only works with NVIDIA cards

**High CPU usage by the app**
- Increase the refresh interval in Settings
- Recommended: 2-5 seconds

**Permission errors when killing processes**
- Run the application as Administrator for full process control

---

## System Requirements

- **Operating System**: Windows 10 or later (64-bit)
- **RAM**: 100 MB minimum
- **Disk Space**: 20 MB
- **Optional**: NVIDIA GPU for GPU monitoring

---

## Technical Details

- **Framework**: egui (Immediate Mode GUI)
- **Language**: Rust
- **System Info**: sysinfo library
- **GPU**: NVML wrapper (NVIDIA Management Library)
- **Update Rate**: Configurable (default 2 seconds)
- **History**: 60 data points (2 minutes at 2-second intervals)

---

## License

MIT License - Free and open source

---

## Support

For issues, bug reports, or feature requests, please create an issue on GitHub.

---

## Version

Current Version: 1.3.0
Last Updated: February 2026
