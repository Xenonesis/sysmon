# 🚀 START HERE - System Monitor Installation Guide

Welcome! This is your **quick start guide** to installing and running System Monitor on your Windows computer.

---

## 🎯 What Do You Want to Do?

### ⚡ I Just Want to Use It (Recommended for Most Users)

**Double-click this file:**
```
INSTALL.bat
```

This will:
- Check if everything is ready
- Help you install build tools if needed
- Build and install the application automatically
- Create shortcuts for easy access

**OR** manually run in PowerShell:
```powershell
.\one-click-install.ps1
```

---

### 🛠️ I Want to Build It Myself

1. **First, check if you're ready:**
   ```powershell
   .\setup-build-environment.ps1 -CheckOnly
   ```

2. **If you need build tools, get them:**
   - **Easiest**: Install Visual Studio Build Tools
     - Download: https://visualstudio.microsoft.com/downloads/
     - Select "Desktop development with C++"
   
   - **Alternative**: Install MSYS2
     - Download: https://www.msys2.org/
     - Follow the MSYS2 setup instructions

3. **Build the application:**
   ```powershell
   .\build.ps1
   ```
   OR double-click: `BUILD.bat`

4. **Install it:**
   ```powershell
   .\install.ps1
   ```

---

### 📚 I Want to Read the Full Documentation First

Choose what you need:

- **[COMPLETE_INSTALLATION.md](COMPLETE_INSTALLATION.md)** ← **START HERE for detailed step-by-step**
- **[SETUP_GUIDE.md](SETUP_GUIDE.md)** ← Technical setup details
- **[README.md](README.md)** ← Project overview and features
- **[USER_GUIDE.md](USER_GUIDE.md)** ← How to use the application

---

## 🎬 Quick Start (3 Steps)

### Step 1: Install Rust (if you haven't already)
```
1. Go to: https://rustup.rs/
2. Download and run rustup-init.exe
3. Follow the installer (choose default options)
4. Restart PowerShell
```

### Step 2: Install Build Tools (choose ONE)

**Option A: Visual Studio Build Tools** (Recommended, ~7GB)
- Download: https://visualstudio.microsoft.com/downloads/
- Install "Desktop development with C++"
- Then run: `rustup default stable-x86_64-pc-windows-msvc`

**Option B: MSYS2** (Smaller, ~1GB)
- Download: https://www.msys2.org/
- Install, then open "MSYS2 MinGW 64-bit"
- Run: `pacman -Syu`
- Run: `pacman -S mingw-w64-x86_64-toolchain`
- Add to PATH: `C:\msys64\mingw64\bin`
- Then run: `rustup default stable-x86_64-pc-windows-gnu`

### Step 3: Build and Install

**Easiest way** - Double-click:
```
INSTALL.bat
```

**Or manually**:
```powershell
.\build.ps1
.\install.ps1
```

---

## ❓ Common Questions

### Q: Do I need to install Rust?
**A:** Yes, to build from source. Download from https://rustup.rs/

### Q: Which build tools should I use?
**A:** Visual Studio Build Tools (MSVC) is recommended. It's larger but more compatible.

### Q: How long does building take?
**A:** First build: 5-10 minutes. Subsequent builds: 30 seconds to 2 minutes.

### Q: Can I just download a pre-built version?
**A:** Not yet. You need to build it yourself. We've made scripts to make it easy!

### Q: Will it work on Windows 11?
**A:** Yes! Windows 10 and 11 are both supported (64-bit).

### Q: Do I need an NVIDIA GPU?
**A:** No. GPU monitoring is optional and only works with NVIDIA cards.

---

## 🆘 Something Went Wrong?

### Error: "Rust not found" or "cargo not found"
→ Install Rust from https://rustup.rs/ and restart PowerShell

### Error: "linker `link.exe` not found"
→ Install Visual Studio Build Tools

### Error: "dlltool not found"
→ Install MSYS2 and the 64-bit toolchain

### Error: "Access is denied"
→ Close any running System Monitor windows and try again

### Error: Build takes forever or crashes
→ Make sure you have 2GB free RAM, close other programs

### Still stuck?
→ Read [COMPLETE_INSTALLATION.md](COMPLETE_INSTALLATION.md) for detailed troubleshooting

---

## ✅ Installation Scripts Available

| File | Purpose | When to Use |
|------|---------|-------------|
| `INSTALL.bat` | Double-click installer | **Use this first!** |
| `BUILD.bat` | Double-click builder | If you just want to build |
| `one-click-install.ps1` | PowerShell installer | Advanced users |
| `build.ps1` | Build script | Manual building |
| `install.ps1` | Installation script | After building |
| `setup-build-environment.ps1` | Environment setup | When build tools are missing |
| `uninstall.ps1` | Uninstaller | To remove the app |

---

## 📋 What You'll Get

After successful installation:

✅ **Desktop Shortcut** - "System Monitor" icon on your desktop  
✅ **Start Menu Entry** - Search "System Monitor" in Windows  
✅ **Installed Application** - At `%LOCALAPPDATA%\Programs\SystemMonitor`  
✅ **Real-time Monitoring** - CPU, RAM, GPU, Disk, Network  
✅ **Beautiful Graphs** - Historical performance data  
✅ **Process Manager** - View and manage running programs  
✅ **System Information** - Detailed hardware and OS info  
✅ **Customizable** - Dark/light theme, settings, alerts  
✅ **Low Resource Usage** - Only ~50-100 MB RAM, <1% CPU  

---

## 🎯 Recommended Path for New Users

1. **Read this file** (you're here! ✓)
2. **Double-click `INSTALL.bat`**
3. **Follow the on-screen prompts**
4. **Launch System Monitor from desktop**
5. **Read [USER_GUIDE.md](USER_GUIDE.md)** to learn how to use it

---

## 🤝 Need More Help?

- **Detailed Instructions**: [COMPLETE_INSTALLATION.md](COMPLETE_INSTALLATION.md)
- **Technical Guide**: [SETUP_GUIDE.md](SETUP_GUIDE.md)
- **Feature List**: [README.md](README.md)
- **Usage Guide**: [USER_GUIDE.md](USER_GUIDE.md)

---

## 🎉 Ready?

**Let's get started!**

### Windows Users:
Double-click **`INSTALL.bat`** to begin

### PowerShell Users:
```powershell
.\one-click-install.ps1
```

---

**System Monitor v1.3.0**  
Built with ❤️ using Rust  
License: MIT

---

*This guide was created to make installation as easy as possible. If you have suggestions for improvement, please let us know!*
