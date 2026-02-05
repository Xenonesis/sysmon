# GitHub Pages Deployment Fix

## 🔴 Current Issue

The `.exe` files are NOT being committed to `docs/downloads/` because they're being ignored by git.

## 🎯 Root Cause

1. `.exe` files in `docs/downloads/` were being gitignored
2. Even with `.gitignore` changes, files need to be force-added
3. GitHub Actions needs to properly commit these files

## ✅ Solution Implemented

### 1. Fixed `.gitignore`
```gitignore
# Docs downloads folder - FORCE INCLUDE all files for GitHub Pages
!docs/downloads/
!docs/downloads/*
```

### 2. Manual Deployment Steps (For Now)

Since GitHub Actions will handle this automatically in the future, here's how to manually deploy:

#### Step 1: Build the Application
```powershell
.\build.ps1
```

#### Step 2: Copy to docs/downloads/
```powershell
# Get version from Cargo.toml
$cargoToml = Get-Content "Cargo.toml" -Raw
if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
    $version = $matches[1]
} else {
    $version = "1.0.0"
}

# Copy files
Copy-Item "target\release\system-monitor.exe" "docs\downloads\SystemMonitor-v$version.exe" -Force
Copy-Item "target\release\system-monitor.exe" "docs\downloads\SystemMonitor-latest.exe" -Force

Write-Host "✓ Files copied to docs/downloads/"
```

#### Step 3: Force Add to Git
```powershell
# Force add .exe files (override .gitignore)
git add docs/downloads/*.exe -f

# Verify
git status docs/downloads/
```

#### Step 4: Commit and Push
```bash
git commit -m "Add .exe files to docs/downloads/ for GitHub Pages"
git push origin main
```

#### Step 5: Verify on GitHub
1. Go to repository on GitHub
2. Navigate to `docs/downloads/`
3. Verify files are visible:
   - `SystemMonitor-v1.0.0.exe`
   - `SystemMonitor-latest.exe`

#### Step 6: Test Download
1. Wait 2-3 minutes for GitHub Pages to deploy
2. Visit: `https://xenonesis.github.io/sysmon/`
3. Open browser console (F12)
4. Click "Download Now"
5. Should see: "Local download found: downloads/SystemMonitor-latest.exe"

---

## 🚀 Automatic Deployment (GitHub Actions)

The GitHub Actions workflow is configured to do this automatically, but it requires:

1. **Build succeeds** on GitHub Actions runner
2. **Files are copied** to docs/downloads/
3. **Git commit** with force flag: `git add -f`
4. **Push** triggers Pages deployment

### Updated Workflow Steps

```yaml
- name: Prepare build artifacts
  run: |
    # Build app
    cargo build --release
    
    # Copy to docs/downloads/
    Copy-Item "target\release\system-monitor.exe" "docs\downloads\SystemMonitor-v1.0.0.exe" -Force
    Copy-Item "target\release\system-monitor.exe" "docs\downloads\SystemMonitor-latest.exe" -Force

- name: Commit and push to docs/downloads/
  run: |
    git config user.name "github-actions[bot]"
    git config user.email "github-actions[bot]@users.noreply.github.com"
    
    # Force add .exe files
    git add docs/downloads/*.exe -f
    
    # Commit if changes
    git diff --staged --quiet || git commit -m "🚀 Auto-deploy: Update binaries to v1.0.0"
    
    # Push
    git push
```

---

## 🧪 Testing

### Test 1: Check Files on GitHub
```bash
# Visit repository
https://github.com/Xenonesis/sysmon/tree/main/docs/downloads

# Should see:
# - SystemMonitor-v1.0.0.exe
# - SystemMonitor-latest.exe
```

### Test 2: Check Raw File URL
```bash
# Try accessing directly
https://xenonesis.github.io/sysmon/downloads/SystemMonitor-latest.exe

# Should download or show file
```

### Test 3: Check Website Download
```bash
# Visit
https://xenonesis.github.io/sysmon/

# Open console (F12)
# Click "Download Now"
# Check console output:
# "Local download found: downloads/SystemMonitor-latest.exe"
```

---

## 🔍 Troubleshooting

### Issue: "Files not in repository"
**Solution:**
```bash
# Force add
git add docs/downloads/*.exe -f

# Commit
git commit -m "Force add .exe files"

# Push
git push origin main
```

### Issue: "GitHub Pages 404"
**Solution:**
```bash
# Check GitHub Pages settings
Repository → Settings → Pages
Source: Deploy from branch
Branch: main
Folder: /docs

# Wait 2-3 minutes for deployment
```

### Issue: "Download still redirects to releases"
**Solution:**
```bash
# Clear browser cache
Ctrl + F5

# Or check if files actually exist:
https://github.com/Xenonesis/sysmon/tree/main/docs/downloads
```

---

## 📋 Current State Checklist

- [x] `.gitignore` updated to allow docs/downloads/*
- [x] GitHub Actions workflow configured
- [ ] **Manual Step: Build application** ← DO THIS
- [ ] **Manual Step: Copy to docs/downloads/** ← THEN THIS
- [ ] **Manual Step: Git add -f docs/downloads/*.exe** ← THEN THIS
- [ ] **Manual Step: Commit and push** ← FINALLY THIS
- [ ] Verify files on GitHub
- [ ] Test download from live site

---

## 🎯 Quick Fix Commands

Run these commands in order:

```powershell
# 1. Build (if not already built)
.\build.ps1

# 2. Get version and copy files
$cargoToml = Get-Content "Cargo.toml" -Raw
if ($cargoToml -match 'version\s*=\s*"([^"]+)"') { $version = $matches[1] } else { $version = "1.0.0" }
Copy-Item "target\release\system-monitor.exe" "docs\downloads\SystemMonitor-v$version.exe" -Force
Copy-Item "target\release\system-monitor.exe" "docs\downloads\SystemMonitor-latest.exe" -Force

# 3. Force add to git
git add docs/downloads/*.exe -f

# 4. Check what's being added
git status docs/downloads/

# 5. Commit
git commit -m "Deploy .exe files to GitHub Pages"

# 6. Push
git push origin main

# 7. Wait 2-3 minutes, then test:
# https://xenonesis.github.io/sysmon/
```

---

## 🎉 Expected Result

After following all steps:

1. ✅ Files visible in GitHub repository: `docs/downloads/*.exe`
2. ✅ GitHub Pages serves files: `https://...github.io/.../downloads/SystemMonitor-latest.exe`
3. ✅ Website downloads directly: No redirect to releases
4. ✅ Browser console shows: "Local download found: downloads/SystemMonitor-latest.exe"

---

## 📝 Summary

**Problem:** Files not being committed to docs/downloads/

**Solution:**
1. Update .gitignore to allow docs/downloads/*
2. Build application
3. Copy .exe to docs/downloads/
4. Force add with `git add -f`
5. Commit and push
6. Verify on GitHub and test download

**Once manual deployment works, GitHub Actions will handle it automatically!**
