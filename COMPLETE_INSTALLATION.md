# Complete Installation Instructions for System Monitor

## 🎯 What You Need to Know

This application is built from source code using Rust. You have **3 ways** to get it running:

---

## ✅ Option 1: Automated Installation (EASIEST - Recommended)

### Step 1: Run the One-Click Installer

Open PowerShell in the application folder and run:

```powershell
.\one-click-install.ps1
```

This script will:
- Check if the application is already built
- Help you set up build tools if needed
- Build the application automatically
- Install it to a permanent location
- Create shortcuts on your desktop and Start Menu
- Launch the application

**Just follow the on-screen prompts!**

---

## ✅ Option 2: Manual Build and Install

### Step 1: Check if You Have Build Tools

Run this script to check your system:

```powershell
.\setup-build-environment.ps1 -CheckOnly
```

### Step 2: Install Build Tools (if needed)

**Choose ONE method:**

#### Method A: Visual Studio Build Tools (RECOMMENDED)

1. Download Visual Studio Build Tools: https://visualstudio.microsoft.com/downloads/
2. Run the installer
3. Select "Desktop development with C++"
4. Click Install (requires ~7GB, takes 15-30 minutes)
5. Run this command:
   ```powershell
   rustup default stable-x86_64-pc-windows-msvc
   ```

#### Method B: MSYS2/MinGW64

1. Download MSYS2: https://www.msys2.org/
2. Run the installer (default settings)
3. Open "MSYS2 MinGW 64-bit" from Start Menu
4. Run these commands:
   ```bash
   pacman -Syu
   pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-toolchain
   ```
5. Add to Windows PATH: `C:\msys64\mingw64\bin`
6. Run this command in PowerShell:
   ```powershell
   rustup default stable-x86_64-pc-windows-gnu
   ```

### Step 3: Build the Application

```powershell
.\build.ps1
```

This will compile the application. **First build takes 5-10 minutes.**

### Step 4: Install the Application

```powershell
.\install.ps1
```

This will:
- Copy the executable to `%LOCALAPPDATA%\Programs\SystemMonitor`
- Create desktop shortcut
- Create Start Menu shortcut

### Step 5: Launch

- Double-click the desktop shortcut **"System Monitor"**
- Or search "System Monitor" in Start Menu

---

## ✅ Option 3: Just Running Without Installing

If you don't want to install permanently:

1. Follow Steps 1-3 from Option 2
2. Run directly:
   ```powershell
   .\target\release\system-monitor.exe
   ```

---

## 🔧 Troubleshooting

### Problem: "Rust/Cargo not found"

**Solution**: Install Rust first
1. Go to: https://rustup.rs/
2. Download and run `rustup-init.exe`
3. Follow the installer prompts
4. Restart PowerShell
5. Verify: Run `cargo --version`

### Problem: "linker `link.exe` not found"

**Solution**: Install Visual Studio Build Tools (Method A above)

### Problem: "dlltool.exe not found" or "Invalid bfd target"

**Solution**: Install MSYS2 with 64-bit toolchain (Method B above)

Make sure you install the **64-bit** version:
```bash
pacman -S mingw-w64-x86_64-toolchain
```

NOT the 32-bit version!

### Problem: "Access is denied" when building

**Solution**:
1. Close any running System Monitor windows
2. Run this command:
   ```powershell
   Stop-Process -Name "system-monitor" -Force -ErrorAction SilentlyContinue
   ```
3. Try building again

### Problem: Build takes too long or crashes

**Solution**:
1. Make sure you have at least 2GB free RAM
2. Close other applications
3. Try building with fewer parallel jobs:
   ```powershell
   cargo build --release -j 2
   ```

### Problem: GPU information not showing

**Solution**: This is normal if you don't have an NVIDIA GPU
- GPU monitoring only works with NVIDIA graphics cards
- If you have NVIDIA GPU, make sure drivers are installed

---

## 📁 File Locations

### Application Folder
```
Your working directory (current folder)
```

### Installed Location  
```
C:\Users\<YourUsername>\AppData\Local\Programs\SystemMonitor\
```

### Shortcuts
- **Desktop**: `C:\Users\<YourUsername>\Desktop\System Monitor.lnk`
- **Start Menu**: `C:\Users\<YourUsername>\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\System Monitor.lnk`

### Configuration
```
C:\Users\<YourUsername>\AppData\Roaming\SystemMonitor\SystemMonitor\config\settings.json
```

---

## 🗑️ Uninstalling

To remove System Monitor:

```powershell
.\uninstall.ps1
```

This will:
- Delete the installed executable
- Remove desktop and Start Menu shortcuts
- Ask if you want to keep settings (optional)

---

## 🔄 Updating to a New Version

1. Get the new version (download or pull from Git)
2. Run the build script:
   ```powershell
   .\build.ps1
   ```
3. Run the install script:
   ```powershell
   .\install.ps1
   ```

The installer will overwrite the old version.

---

## 📊 What the Application Does

### Overview Tab
- CPU usage percentage
- Memory usage
- GPU usage (NVIDIA only)
- Top 5 memory-consuming processes

### Performance Tab
- CPU usage graph (2-minute history)
- Memory usage graph
- GPU usage graph

### Processes Tab
- All running processes
- Memory and CPU usage per process
- Search functionality
- Process management (kill, suspend)

### CPU Cores Tab
- Individual core usage
- Core statistics
- Color-coded visualization

### Storage Tab
- All disk drives
- Space usage and availability
- File system information
- Low space warnings

### Network Tab
- Network interfaces
- Upload/download rates
- Historical network activity graphs

### System Info Tab
- Operating system details
- CPU information
- Memory specifications
- GPU details
- System uptime

### Alerts Tab
- High CPU usage alerts
- High memory usage alerts
- High GPU temperature warnings
- Low disk space alerts

---

## ⚙️ Settings & Customization

Access Settings from the **View** menu:

- **Refresh Interval**: 1-10 seconds (default: 2)
- **Display Options**: Toggle graphs, GPU section, process list
- **Theme**: Dark mode / Light mode
- **Notifications**: Enable/disable alerts
- **Alert Thresholds**: Customize when alerts trigger
- **Auto-start**: Launch with Windows
- **Start Minimized**: Start in background

---

## ⌨️ Keyboard Shortcuts

- `F5` - Reset statistics and clear graphs
- `Ctrl+E` - Export data to JSON
- `Ctrl+,` - Open settings

---

## 💻 System Requirements

### Minimum
- Windows 10 (64-bit)
- 100 MB RAM
- 20 MB disk space
- Any CPU

### Recommended
- Windows 10/11 (64-bit)
- 200 MB RAM
- NVIDIA GPU (for GPU monitoring)
- Multi-core CPU

---

## 🆘 Getting Help

### Did something go wrong?

1. **Check the error message** - Most errors tell you what's wrong
2. **Run the setup script**:
   ```powershell
   .\setup-build-environment.ps1
   ```
3. **Read SETUP_GUIDE.md** - More detailed troubleshooting
4. **Check build tool installation** - Make sure Visual Studio Build Tools or MSYS2 is properly installed

### Still stuck?

- Check that Rust is installed: `cargo --version`
- Check that build tools are installed: `.\setup-build-environment.ps1 -CheckOnly`
- Try cleaning and rebuilding:
  ```powershell
  cargo clean
  .\build.ps1
  ```

---

## ✨ Quick Command Reference

```powershell
# Check if Rust is installed
cargo --version

# Check if build environment is ready
.\setup-build-environment.ps1 -CheckOnly

# Set up build environment automatically
.\setup-build-environment.ps1

# Build the application
.\build.ps1

# Install the application  
.\install.ps1

# One-click install (all-in-one)
.\one-click-install.ps1

# Run without installing
.\target\release\system-monitor.exe

# Uninstall
.\uninstall.ps1
```

---

## 🎉 Success Checklist

After installation, you should be able to:
- [ ] Find "System Monitor" shortcut on your desktop
- [ ] Find "System Monitor" in Start Menu search
- [ ] Launch the application by double-clicking
- [ ] See real-time CPU and memory usage
- [ ] Navigate between different tabs
- [ ] See graphs updating every 2 seconds
- [ ] Export data to JSON/CSV
- [ ] Change settings (theme, refresh rate, etc.)

If all of these work, **congratulations!** You've successfully installed System Monitor.

---

**Version**: 1.3.0  
**Last Updated**: February 2026  
**Platform**: Windows 10/11 64-bit  
**License**: MIT
