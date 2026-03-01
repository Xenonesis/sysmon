# GitHub Pages Deployment Guide

## ✅ Configuration Complete!

Your website is now configured to work with GitHub Pages and serve direct `.exe` downloads with versioned naming (e.g., `SystemMonitor-v2.0.0.exe`).

---

## 🎯 What's Been Fixed

### 1. **Versioned Naming** ✅

- Files now named as: `SystemMonitor-v2.0.0.exe` (instead of `system-monitor-2.0.0.exe`)
- Version automatically extracted from `Cargo.toml`
- Both versioned and "latest" copies created

### 2. **GitHub Pages Support** ✅

- Files saved to both `downloads/` and `docs/downloads/`
- `docs/` folder is served by GitHub Pages
- `.gitignore` configured to allow `.exe` files in `docs/downloads/`

### 3. **Website Updates** ✅

- Root `script.js` and `index.html` updated
- `docs/script.js` and `docs/index.html` updated
- Download button now says "Download Application"
- Prioritizes `.exe` files over `.zip`

---

## 🚀 Deployment Steps

### Step 1: Build the Application

```powershell
.\build.ps1
```

**This will create:**

```
downloads/
├── SystemMonitor-v2.0.0.exe
└── SystemMonitor-latest.exe

docs/downloads/
├── SystemMonitor-v2.0.0.exe
└── SystemMonitor-latest.exe
```

### Step 2: Commit and Push

```bash
git add .
git commit -m "Add direct .exe downloads with versioned naming"
git push origin main
```

### Step 3: Verify GitHub Actions

1. Go to your repository on GitHub
2. Click "Actions" tab
3. Wait for "Deploy GitHub Pages" workflow to complete
4. Should deploy automatically (triggered by changes in `docs/` folder)

### Step 4: Test Live Website

Visit your GitHub Pages URL:

- **Your URL:** `https://xenonesis.github.io/sysmon/`
- Click "Download Now" button
- Should download: `SystemMonitor-v2.0.0.exe`

---

## 📁 File Structure

```
project/
├── downloads/                          # Root downloads (not deployed)
│   ├── SystemMonitor-v2.0.0.exe
│   └── SystemMonitor-latest.exe
│
├── docs/                               # GitHub Pages source
│   ├── downloads/                      # Deployed downloads
│   │   ├── .gitkeep
│   │   ├── SystemMonitor-v2.0.0.exe   ✅ Deployed to Pages
│   │   └── SystemMonitor-latest.exe   ✅ Deployed to Pages
│   ├── index.html                      # Main page
│   ├── script.js                       # Download logic
│   ├── styles.css
│   └── _config.yml
│
├── build.ps1                           # Saves to both locations
└── .gitignore                          # Allows docs/downloads/*.exe
```

---

## 🔍 How It Works

### Local Development:

1. Website checks `downloads/SystemMonitor-latest.exe` (local)
2. If found → Direct download
3. If not found → Checks GitHub Releases

### GitHub Pages (Live):

1. Website checks `downloads/SystemMonitor-latest.exe` (relative path)
2. Serves from `docs/downloads/SystemMonitor-latest.exe`
3. Direct download to user
4. Fallback to GitHub Releases if not found

---

## 🎨 Download Priority Order

```javascript
// Priority 1: Local .exe files (versioned)
'downloads/SystemMonitor-latest.exe'
'downloads/SystemMonitor-v2.0.0.exe'

// Priority 2: Local legacy names (backward compatibility)
'downloads/system-monitor-latest.exe'
'downloads/system-monitor-2.0.0.exe'

// Priority 3: Local ZIP files
'downloads/SystemMonitor-latest.zip'
'downloads/SystemMonitor-v2.0.0.zip'

// Priority 4: GitHub Releases (remote fallback)
'SystemMonitor-latest.exe'
'SystemMonitor-v2.0.0.exe'
'system-monitor.exe'
...
```

---

## 📊 Version Management

### Current Version:

Located in `Cargo.toml`:

```toml
[package]
version = "2.0.0"
```

### To Update Version:

1. Edit `Cargo.toml` → Change version to `"1.1.0"`
2. Run `.\build.ps1`
3. Files created:
   - `SystemMonitor-v1.1.0.exe`
   - `SystemMonitor-latest.exe` (updated)
4. Commit and push
5. Website automatically serves new version

---

## 🧪 Testing Checklist

### Before Deployment:

- [ ] Run `.\build.ps1` successfully
- [ ] Verify `docs/downloads/SystemMonitor-v2.0.0.exe` exists
- [ ] Verify `docs/downloads/SystemMonitor-latest.exe` exists
- [ ] Check file size (~5-6 MB expected)

### After Deployment:

- [ ] Visit `https://xenonesis.github.io/sysmon/`
- [ ] Click "Download Now"
- [ ] Verify `SystemMonitor-v2.0.0.exe` or `SystemMonitor-latest.exe` downloads
- [ ] Open browser console - check for "Local download found" message
- [ ] Run downloaded `.exe` - verify it works

### If Download Not Working:

- [ ] Check browser console for errors
- [ ] Verify file exists in `docs/downloads/` on GitHub
- [ ] Check if GitHub Pages workflow completed successfully
- [ ] Verify `.gitignore` allows `.exe` files in `docs/downloads/`
- [ ] Try hard refresh (Ctrl+F5) to clear cache

---

## 🔧 Troubleshooting

### Issue: "File not found on GitHub Pages"

**Cause:** `.exe` files not committed to `docs/downloads/`

**Solution:**

```powershell
# Build first
.\build.ps1

# Check if files exist
dir docs\downloads\

# Add to git (override .gitignore if needed)
git add docs/downloads/*.exe -f

# Commit and push
git commit -m "Add .exe files for download"
git push origin main
```

### Issue: "Download button shows GitHub Releases"

**Cause:** Files not found in `downloads/` folder on live site

**Solution:**

1. Ensure files committed to `docs/downloads/`
2. Wait for GitHub Actions deployment to complete
3. Hard refresh page (Ctrl+F5)
4. Check browser console for fetch errors

### Issue: "Wrong version being downloaded"

**Cause:** Multiple versions in folder or cache

**Solution:**

```powershell
# Clean old builds
Remove-Item docs\downloads\*.exe

# Rebuild
.\build.ps1

# Verify correct version
dir docs\downloads\

# Commit fresh files
git add docs/downloads/*.exe
git commit -m "Update to correct version"
git push origin main
```

### Issue: "CORS errors in browser console"

**Cause:** Cross-origin resource sharing blocked

**Solution:** This shouldn't happen with relative paths, but if it does:

1. Verify files are in `docs/downloads/` (same origin)
2. Check browser security settings
3. Test in incognito/private mode

---

## 🎉 Success Indicators

When everything is working correctly:

### Browser Console Shows:

```
Download initiated
Local download found: downloads/SystemMonitor-latest.exe (5.43 MB)
```

### Download Manager Shows:

```
SystemMonitor-latest.exe
5.43 MB
```

### File Runs Successfully:

- Double-click `.exe`
- App window opens
- No installation required
- Auto-update banner may appear (if new version available)

---

## 📝 Maintenance

### Regular Updates:

1. Update version in `Cargo.toml`
2. Run `.\build.ps1`
3. Commit and push
4. GitHub Pages auto-deploys
5. Users get new version automatically

### Keeping Clean:

```powershell
# Remove old versions (keep latest only)
Remove-Item docs\downloads\SystemMonitor-v*.exe -Exclude "*latest*"

# Or keep last 3 versions
# Manually delete older ones
```

### Monitoring:

- Check GitHub Actions logs for deployment errors
- Test download periodically
- Monitor GitHub repository size (`.exe` files add up)

---

## 🚀 Live Deployment Checklist

- [x] Build script updated with versioned naming
- [x] Docs folder structure created
- [x] Website scripts updated
- [x] `.gitignore` configured
- [ ] **Run `.\build.ps1`** ← DO THIS NOW
- [ ] **Commit and push** ← THEN DO THIS
- [ ] **Wait for GitHub Actions** ← WATCH DEPLOYMENT
- [ ] **Test live website** ← VERIFY IT WORKS

---

## 🎊 You're All Set!

Once you complete the deployment steps:

1. Build → `.\build.ps1`
2. Commit → `git add . && git commit -m "Deploy"`
3. Push → `git push origin main`
4. Visit → `https://xenonesis.github.io/sysmon/`

Your users will be able to download `SystemMonitor-v2.0.0.exe` directly from your website! 🎉
