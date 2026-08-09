> **NOTE — SUPERSEDED (2026-08): This document describes an OBSOLETE process that
> distributed PORTABLE bare `.exe` and `.zip` files. The project now ships ONLY an
> installable Inno Setup installer (`SystemMonitor-<version>-setup.exe`). Portable
> builds are disabled and refused by the build scripts. See `docs/release-rule.md`,
> `release.ps1`, `build.ps1`, and `.github/workflows/windows-release.yml` for the
> current installable-only release pipeline. The guidance below is historical only.

# Automatic Release Workflow Guide

## ✅ Complete Implementation!

Your repository is now configured for **automatic releases on every commit** with direct `.exe` downloads!

---

## 🎯 What's Been Configured

### 1. **Automatic Release on Push** ✅
- Every commit to `main` branch triggers automatic build
- Only triggers when you change: `src/**`, `Cargo.toml`, or `Cargo.lock`
- Creates GitHub Release automatically
- Uploads `.exe` files and `.zip` package

### 2. **GitHub Pages Fix** ✅
- Website detects GitHub Pages and uses GitHub Releases
- No more CORS or file path issues
- Direct download from releases

### 3. **Versioned Naming** ✅
- `SystemMonitor-v1.0.0.exe` (from Cargo.toml version)
- `SystemMonitor-latest.exe` (always points to latest)
- `SystemMonitor-v1.0.0.zip` (with installer scripts)

---

## 🚀 How It Works

### Workflow Trigger:
```yaml
on:
  push:
    branches:
      - main
    paths:
      - 'src/**'        # Any Rust code changes
      - 'Cargo.toml'    # Version or dependency changes
      - 'Cargo.lock'    # Lock file changes
```

### What Happens Automatically:
1. **Commit pushed** → GitHub Actions triggered
2. **Build Rust app** → Creates `system-monitor.exe`
3. **Extract version** → From `Cargo.toml` (e.g., `1.0.0`)
4. **Create artifacts:**
   - `SystemMonitor-v1.0.0.exe`
   - `SystemMonitor-latest.exe`
   - `SystemMonitor-v1.0.0.zip`
5. **Create/Update Release** → Tag: `v1.0.0`
6. **Upload files** → To GitHub Releases
7. **Website serves** → From GitHub Releases URL

---

## 📁 Release Structure

Each release includes:

```
Release: v1.0.0
├── SystemMonitor-v1.0.0.exe    (Direct application)
├── SystemMonitor-latest.exe     (Always latest)
└── SystemMonitor-v1.0.0.zip     (Package with scripts)
```

### Download URLs:
```
https://github.com/Xenonesis/sysmon/releases/download/v1.0.0/SystemMonitor-v1.0.0.exe
https://github.com/Xenonesis/sysmon/releases/download/v1.0.0/SystemMonitor-latest.exe
https://github.com/Xenonesis/sysmon/releases/download/v1.0.0/SystemMonitor-v1.0.0.zip
```

---

## 🔄 Complete Workflow

### Step 1: Make Changes
```bash
# Edit your Rust code
code src/main.rs

# Or update version
code Cargo.toml
# Change: version = "1.0.1"
```

### Step 2: Commit and Push
```bash
git add .
git commit -m "Add new feature"
git push origin main
```

### Step 3: Automatic Build & Release
GitHub Actions will:
- ✅ Build Windows release
- ✅ Extract version from Cargo.toml
- ✅ Create release `v1.0.1`
- ✅ Upload `.exe` and `.zip` files
- ✅ Make available for download

### Step 4: Users Download
Website automatically:
- ✅ Detects GitHub Pages
- ✅ Uses GitHub Releases URL
- ✅ Downloads `SystemMonitor-v1.0.1.exe`

---

## 🌐 Website Behavior

### On GitHub Pages:
```javascript
// Detects GitHub Pages
const isGitHubPages = window.location.hostname.includes('github.io');

if (isGitHubPages) {
    // Skip local files
    // Go directly to GitHub Releases
    console.log('GitHub Pages detected - using GitHub Releases for downloads');
}
```

### On Local:
```javascript
// Tries local files first
for (const localPath of localCandidates) {
    // Try downloads/SystemMonitor-latest.exe
    // Then fallback to GitHub Releases
}
```

---

## 📊 Version Management

### Current Version (Cargo.toml):
```toml
[package]
name = "system-monitor"
version = "1.0.0"    # ← This is used for releases
```

### To Release New Version:

#### Option 1: Update Version Number
```toml
version = "1.0.1"    # Increment version
```

```bash
git add Cargo.toml
git commit -m "Bump version to 1.0.1"
git push origin main
```
**Result:** Release `v1.0.1` created automatically!

#### Option 2: Manual Workflow Trigger
1. Go to GitHub → Actions
2. Select "Build and Release on Push"
3. Click "Run workflow"
4. Enter custom version (optional)
5. Release created!

---

## 🧪 Testing the Workflow

### Test 1: Make a Code Change
```bash
# Edit any source file
echo "// Test comment" >> src/main.rs

git add src/main.rs
git commit -m "Test automatic release"
git push origin main
```

**Expected:**
- GitHub Actions runs
- Release created with current version
- Files uploaded to releases

### Test 2: Update Version
```bash
# Edit Cargo.toml
# Change version = "1.0.0" to "1.0.1"

git add Cargo.toml
git commit -m "Bump to v1.0.1"
git push origin main
```

**Expected:**
- Release `v1.0.1` created
- Files uploaded with new version name
- Website serves new version

### Test 3: Check GitHub Pages
```bash
# Visit your live site
https://xenonesis.github.io/sysmon/

# Click "Download Now"
# Should download from GitHub Releases
```

**Check browser console:**
```
GitHub Pages detected - using GitHub Releases for downloads
GitHub download found: https://github.com/.../SystemMonitor-latest.exe
```

---

## 🔍 Monitoring

### Check GitHub Actions:
1. Go to repository
2. Click "Actions" tab
3. See "Build and Release on Push" workflows
4. Click to view logs

### Check Releases:
1. Go to repository
2. Click "Releases" section (right sidebar)
3. See all versions listed
4. Each has `.exe` and `.zip` files

### Check Downloads:
```bash
# Test direct download URL
curl -I https://github.com/Xenonesis/sysmon/releases/download/v1.0.0/SystemMonitor-latest.exe

# Should return: 200 OK
```

---

## 🐛 Troubleshooting

### Issue: "Workflow not triggering"
**Cause:** Changes not in watched paths

**Solution:**
```yaml
# Workflow only triggers on changes to:
- src/**
- Cargo.toml
- Cargo.lock

# Make sure you're editing these files
```

### Issue: "Release already exists"
**Cause:** Same version number

**Solution:**
```bash
# Update version in Cargo.toml
version = "1.0.1"  # Increment

# Or delete old release on GitHub
```

### Issue: "Build failed"
**Cause:** Compilation errors

**Solution:**
```bash
# Test locally first
cargo build --release

# Fix any errors
# Then commit and push
```

### Issue: "Website still shows old version"
**Cause:** Browser cache

**Solution:**
```bash
# Hard refresh
Ctrl + F5  (Windows)
Cmd + Shift + R  (Mac)

# Or clear browser cache
```

### Issue: "Download link broken"
**Cause:** File not uploaded to release

**Solution:**
1. Check GitHub Actions logs
2. Verify files created in workflow
3. Check release page for files
4. Re-run workflow if needed

---

## 📝 Release Notes (Auto-Generated)

Each release includes formatted notes:

```markdown
## System Monitor v1.0.0

### 🚀 What's New
- Auto-built from commit abc123
- Direct .exe download available
- No installation required - just run!

### 📥 Download
- **SystemMonitor-v1.0.0.exe** - Direct application (recommended)
- **SystemMonitor-latest.exe** - Always points to latest version
- **SystemMonitor-v1.0.0.zip** - Package with installer scripts

### 🎯 Quick Start
1. Download `SystemMonitor-v1.0.0.exe`
2. Run the .exe file
3. Enjoy real-time system monitoring!

### 🔄 Auto-Update
The application includes built-in auto-update functionality that checks for new versions every 24 hours.
```

---

## 🎉 Success Metrics

### When Everything Works:

✅ **Commit pushed** → GitHub Actions runs  
✅ **Build completes** → ~5-10 minutes  
✅ **Release created** → With version tag  
✅ **Files uploaded** → `.exe` and `.zip`  
✅ **Website works** → Downloads from releases  
✅ **Users happy** → Direct .exe download!  

---

## 🚀 Next Steps

### 1. Push First Release:
```bash
# Make sure version is set
cat Cargo.toml | grep version

# Make a small change
echo "# Release v1.0.0" >> README.md

# Commit and push
git add .
git commit -m "Initial automatic release"
git push origin main
```

### 2. Watch GitHub Actions:
- Go to Actions tab
- Watch "Build and Release on Push" run
- Should complete in ~5-10 minutes

### 3. Verify Release:
- Go to Releases
- See `v1.0.0` release
- Download files to test

### 4. Test Website:
- Visit `https://xenonesis.github.io/sysmon/`
- Click "Download Now"
- Should download from GitHub Releases

---

## 🎊 You're Done!

Your repository now has:

✅ **Automatic builds** on every commit  
✅ **Automatic releases** with version tags  
✅ **Direct .exe downloads** from releases  
✅ **GitHub Pages** serving from releases  
✅ **Auto-update** checking in application  

**Workflow:**
1. Code → Commit → Push
2. GitHub Actions → Build → Release
3. Users → Download → Run
4. App → Auto-update → Latest version

**Zero manual work required!** 🎉
