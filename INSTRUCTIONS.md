# System Monitor - Instructions

## 📦 What's in this folder:

### ✅ Ready to Use NOW:
- **SystemMonitor.ps1** - PowerShell version (works immediately!)

### ⏳ Needs Compilation:
- **Cargo.toml** - Rust project configuration
- **src/main.rs** - Rust source code (more features, better performance)
- **README.md** - Full documentation

---

## 🚀 Quick Start - PowerShell Version

### Option 1: Run directly
Open PowerShell in this folder and run:
```powershell
.\SystemMonitor.ps1
```

### Option 2: With custom settings
```powershell
# Refresh every 5 seconds, show top 20 processes
.\SystemMonitor.ps1 -RefreshInterval 5 -TopProcessCount 20
```

### Option 3: Run in new window
```powershell
Start-Process powershell -ArgumentList "-NoExit", "-File", "`"$PWD\SystemMonitor.ps1`""
```

---

## 🦀 Rust Version (After SDK Installation Completes)

### Check if ready:
```powershell
cargo build --release
```

### If successful, run:
```powershell
.\target\release\system-monitor.exe
```

### Or directly with:
```powershell
cargo run --release
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
