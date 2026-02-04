# Testing Guide - Auto-Update and Download System

## 🧪 Test Checklist

### Test 1: Build Script with Downloads Folder
```powershell
# Run the build script
.\build.ps1

# Expected output:
# ✓ Build successful
# ✓ Build saved to downloads folder:
#   • downloads\system-monitor-1.0.0.exe
#   • downloads\system-monitor-latest.exe (latest)

# Verify files exist:
dir downloads\
```

### Test 2: Installer Creation
```powershell
# Create distribution package
.\create-installer.ps1

# Expected output:
# ✓ Build successful
# ✓ Installer saved to downloads folder
#   • downloads\SystemMonitor-v1.0.0.zip
#   • downloads\SystemMonitor-latest.zip (latest)

# Verify files exist:
dir downloads\
```

### Test 3: Website Local Downloads
```powershell
# Start a local web server
python -m http.server 8000
# OR
php -S localhost:8000

# Open browser to http://localhost:8000
# Click "Download Now" button
# Expected: Downloads from local downloads/ folder
# Check browser console for: "Local download found: downloads/SystemMonitor-latest.zip"
```

### Test 4: Auto-Update Check (Manual)
```powershell
# Build and run the application
.\build.ps1
.\target\release\system-monitor.exe

# In the application:
# 1. Press Ctrl+U (manual update check)
# 2. Watch console output for update check
# 3. If update available, banner should appear at top
```

### Test 5: Update Notification Banner
To test the update banner without a real update:

1. Temporarily modify `Cargo.toml`:
```toml
version = "0.5.0"  # Lower version to trigger update
```

2. Rebuild and run:
```powershell
.\build.ps1
.\target\release\system-monitor.exe
```

3. Wait for update check or press `Ctrl+U`
4. Green banner should appear with "New version 1.0.0 is available!"

### Test 6: Complete Update Flow
```powershell
# 1. Create a "fake" new version in downloads folder
Copy-Item downloads\SystemMonitor-latest.zip downloads\SystemMonitor-v2.0.0.zip

# 2. Modify your local version to 1.0.0 in Cargo.toml
# 3. Build and run application
# 4. Press Ctrl+U to check for updates
# 5. Click "Download & Install" when banner appears
# 6. Verify update downloads and installer runs
```

## 🔍 Verification Steps

### After Build:
- [ ] `downloads/` folder exists
- [ ] `system-monitor-1.0.0.exe` file exists
- [ ] `system-monitor-latest.exe` file exists
- [ ] Both files are identical (same size)

### After Create Installer:
- [ ] `SystemMonitor-v1.0.0.zip` exists in downloads/
- [ ] `SystemMonitor-latest.zip` exists in downloads/
- [ ] ZIP contains: installer.ps1, setup.bat, system-monitor.exe, README.md

### Website Download Test:
- [ ] Download button finds local file first
- [ ] Browser console shows "Local download found"
- [ ] File downloads correctly
- [ ] Falls back to GitHub if local not found

### Auto-Update Test:
- [ ] App checks for updates on startup
- [ ] Update check runs every 24 hours
- [ ] Ctrl+U triggers manual check
- [ ] Update banner appears when update available
- [ ] Download & Install button works
- [ ] App exits and installer runs

## 🐛 Troubleshooting

### Issue: "Rust/Cargo not found"
**Solution:** Install Rust from https://rustup.rs/

### Issue: Downloads folder not created
**Solution:** Run build.ps1 at least once to create folder

### Issue: Website can't find local downloads
**Solution:** 
1. Ensure web server is running from project root
2. Check downloads/ folder has files
3. Check browser console for CORS errors

### Issue: Update check fails
**Solution:**
1. Check internet connection
2. Verify GitHub API is accessible
3. Check Windows Firewall settings

### Issue: Update download fails
**Solution:**
1. Check download URL in error message
2. Verify file exists on GitHub Releases
3. Check temp directory permissions

## 📊 Expected Behavior Summary

| Action | Expected Result |
|--------|----------------|
| Run `build.ps1` | Saves `.exe` to downloads/ |
| Run `create-installer.ps1` | Saves `.zip` to downloads/ |
| Click website download | Downloads from downloads/ folder |
| App startup | Checks for updates (if 24h passed) |
| Press `Ctrl+U` | Manually checks for updates |
| Update available | Green banner appears at top |
| Click "Download & Install" | Downloads and installs update |
| No internet | App works normally, updates disabled |

## ✅ All Tests Passed?

If all tests pass, your auto-update and download system is working perfectly! 🎉

Users will now:
- ✅ Get downloads from local folder (faster)
- ✅ Receive automatic update notifications
- ✅ Update with one click
- ✅ Always have the latest version
