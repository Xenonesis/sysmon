# System Monitor - Instructions

## 📦 What's in this folder:

### ✅ GUI Application (Main):
- **src/main.rs** - GUI application source code
- **build.ps1** - One-click build script
- **install.ps1** - Installation script for Windows
- **QUICK_START.md** - Step-by-step guide

### ⚡ Legacy Terminal Version:
- **SystemMonitor.ps1** - PowerShell terminal version (still works!)

### 📚 Documentation:
- **README.md** - Full documentation
- **Cargo.toml** - Rust project configuration

---

## 🚀 Quick Start - GUI Version (Recommended)

### Step 1: Build the Application
```powershell
.\build.ps1
```

### Step 2: Install (Optional but Recommended)
```powershell
.\install.ps1
```

### Step 3: Run
- Double-click desktop shortcut, OR
- Search "System Monitor" in Start Menu, OR
- Run directly: `.\target\release\system-monitor.exe`

**See QUICK_START.md for detailed instructions!**

---

## ⚡ Alternative - PowerShell Terminal Version

### Run the terminal version:
```powershell
.\SystemMonitor.ps1
```

### With custom settings:
```powershell
.\SystemMonitor.ps1 -RefreshInterval 5 -TopProcessCount 20
```

---

## 📊 What It Shows:

- ✅ Real-time RAM usage (Total, Used, Free, %)
- ✅ CPU utilization with visual progress bar
- ✅ GPU stats (NVIDIA only: utilization, memory, temperature)
- ✅ Top 15 memory-consuming processes
- ✅ Color-coded alerts (Green < 50%, Yellow 50-75%, Red > 75%)
- ✅ Auto-refresh every 2 seconds

---

## 🎨 Color Coding:

- 🟢 **Green** = Healthy (< 50% usage)
- 🟡 **Yellow** = Moderate (50-75% usage)
- 🔴 **Red** = High (> 75% usage)

---

## ⚙️ Troubleshooting:

### PowerShell version not running?
```powershell
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
```

### GPU stats not showing?
- Only works with NVIDIA GPUs
- Ensure NVIDIA drivers are installed
- Script will work fine without GPU monitoring

### Rust compilation fails?
- Wait for Windows SDK installation to complete
- Restart terminal after SDK installation
- Try: `rustup update` and rebuild

---

## 💡 Tips:

1. **Run in separate terminal** to monitor while you work
2. **Adjust refresh rate** for lower CPU usage: `-RefreshInterval 5`
3. **Show more processes**: `-TopProcessCount 20`
4. **Press Ctrl+C** to exit at any time

---

## 📈 Your Current PC Status:

Based on initial scan:
- **RAM**: ~86% used (13.5 GB / 15.7 GB) - ⚠️ HIGH
- **Top Memory User**: Antigravity (~1.4 GB)
- **CPU**: Normal (~12-16%)
- **GPU**: NVIDIA RTX 3060 Laptop (4GB)

---

## 🔧 Future Enhancements:

Want to add features? Edit the code to add:
- Alert notifications when usage is too high
- Export logs to file
- Historical graphs
- Kill high-memory processes
- Network and disk monitoring

Enjoy your system monitor! 🎉
