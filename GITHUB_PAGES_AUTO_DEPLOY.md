# GitHub Pages Auto-Deploy Guide

## ✅ Perfect Solution Implemented!

Your application now **automatically deploys .exe files to GitHub Pages** on every commit!

---

## 🎯 How It Works

### Complete Workflow:

```
1. Code Change → Commit → Push
           ↓
2. GitHub Actions Triggered
           ↓
3. Build Rust Application
           ↓
4. Copy .exe to docs/downloads/
           ↓
5. Auto-commit to repository
           ↓
6. GitHub Pages deploys
           ↓
7. Website serves .exe directly! 🎉
```

---

## 🚀 What's Been Configured

### 1. **Build Workflow** ✅
- Builds application on every commit
- Extracts version from `Cargo.toml`
- Creates `SystemMonitor-v1.0.0.exe`
- Creates `SystemMonitor-latest.exe`

### 2. **Auto-Deploy to docs/** ✅
- Copies `.exe` files to `docs/downloads/`
- Auto-commits changes to repository
- Triggers GitHub Pages deployment

### 3. **Website Integration** ✅
- Website checks `downloads/` folder first
- Works on both local and GitHub Pages
- No redirect to releases needed
- Direct .exe download!

---

## 📁 File Structure After Deploy

```
docs/
├── downloads/
│   ├── SystemMonitor-v1.0.0.exe    ← Auto-deployed
│   ├── SystemMonitor-latest.exe     ← Auto-deployed
│   └── SystemMonitor-v1.0.0.zip     ← Auto-deployed
├── index.html
├── script.js
└── styles.css
```

---

## 🔄 Automatic Workflow Steps

### Step 1: GitHub Actions Build
```yaml
- Build Rust app
- Create SystemMonitor-v1.0.0.exe
- Create SystemMonitor-latest.exe
- Copy to docs/downloads/
```

### Step 2: Auto-Commit
```yaml
- git add docs/downloads/*.exe
- git commit -m "🚀 Auto-deploy: Update binaries to v1.0.0"
- git push
```

### Step 3: GitHub Pages Deploy
```yaml
- Trigger on docs/ changes
- Upload docs folder
- Deploy to GitHub Pages
- Available at: https://username.github.io/repo/
```

---

## 🌐 Download URLs

### GitHub Pages (Primary):
```
https://xenonesis.github.io/sysmon/downloads/SystemMonitor-v1.0.0.exe
https://xenonesis.github.io/sysmon/downloads/SystemMonitor-latest.exe
```

### GitHub Releases (Backup):
```
https://github.com/Xenonesis/sysmon/releases/download/v1.0.0/SystemMonitor-v1.0.0.exe
https://github.com/Xenonesis/sysmon/releases/download/v1.0.0/SystemMonitor-latest.exe
```

---

## 🎨 Website Behavior

### On GitHub Pages:
```javascript
// Tries: downloads/SystemMonitor-latest.exe
fetch('downloads/SystemMonitor-latest.exe')
  .then(response => {
    if (response.ok) {
      // ✅ Download directly from GitHub Pages!
      return 'downloads/SystemMonitor-latest.exe';
    }
  })
  .catch(() => {
    // ❌ Fallback to GitHub Releases
    return 'https://github.com/.../releases/download/...';
  });
```

### Priority Order:
1. ✅ `downloads/SystemMonitor-latest.exe` (GitHub Pages)
2. ✅ `downloads/SystemMonitor-v1.0.0.exe` (GitHub Pages)
3. ⏭️ GitHub Releases (fallback)

---

## 📊 Version Management

### Current Version:
```toml
# Cargo.toml
[package]
version = "1.0.0"
```

### To Release New Version:

1. **Update Version:**
```toml
# Cargo.toml
version = "1.0.1"
```

2. **Commit and Push:**
```bash
git add Cargo.toml src/
git commit -m "Bump to v1.0.1"
git push origin main
```

3. **Automatic Process:**
- GitHub Actions builds
- Creates `SystemMonitor-v1.0.1.exe`
- Updates `SystemMonitor-latest.exe`
- Commits to `docs/downloads/`
- Deploys to GitHub Pages
- Creates GitHub Release

4. **Result:**
- Website serves new version automatically!
- Users download latest .exe
- No manual deployment needed!

---

## 🧪 Testing

### Test 1: Check Current Files
```bash
# View what's deployed
curl -I https://xenonesis.github.io/sysmon/downloads/SystemMonitor-latest.exe

# Should return: 200 OK
```

### Test 2: Make a Change
```bash
# Edit code
echo "// Test" >> src/main.rs

# Commit
git add src/main.rs
git commit -m "Test auto-deploy"
git push origin main

# Wait 5-10 minutes
# Check GitHub Actions progress
```

### Test 3: Verify Deployment
```bash
# Visit website
https://xenonesis.github.io/sysmon/

# Click "Download Now"
# Should download from downloads/ folder

# Check browser console:
# "Local download found: downloads/SystemMonitor-latest.exe"
```

---

## 🔍 Monitoring

### Check Build Progress:
1. Go to: `https://github.com/Xenonesis/sysmon/actions`
2. See "Build and Release on Push" running
3. Click to view logs

### Check Auto-Commit:
1. Go to repository commits
2. Look for: "🚀 Auto-deploy: Update binaries to v1.0.0"
3. Verify files in `docs/downloads/`

### Check GitHub Pages:
1. Repository → Settings → Pages
2. See deployment status
3. Click "Visit site"

---

## 📝 Workflow Files

### `.github/workflows/release-build.yml`
```yaml
# Builds app
# Copies to docs/downloads/
# Commits changes
# Creates GitHub Release
```

### `.github/workflows/pages-deploy.yml`
```yaml
# Triggered by docs/ changes
# Uploads docs folder
# Deploys to GitHub Pages
```

---

## 🎉 Benefits

✅ **Zero Manual Work**
- Commit code → Everything automatic

✅ **Always Up-to-Date**
- Website always serves latest build

✅ **Fast Downloads**
- Direct from GitHub Pages CDN

✅ **Dual Availability**
- GitHub Pages (primary)
- GitHub Releases (backup)

✅ **Version Control**
- Both versioned and "latest" available

---

## 🐛 Troubleshooting

### Issue: "Files not appearing in docs/downloads/"
**Check:** GitHub Actions logs for copy step

**Solution:**
```bash
# Check workflow logs
# Verify build completed successfully
# Check if commit step ran
```

### Issue: "Auto-commit failed"
**Cause:** Permission issue

**Solution:**
```yaml
# Already configured with:
env:
  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Issue: "Website shows 404"
**Cause:** Files not committed yet

**Solution:**
```bash
# Wait for GitHub Actions to complete
# Check repository for auto-commit
# Refresh GitHub Pages (takes 1-2 min)
```

### Issue: "Old version still downloading"
**Cause:** Browser cache

**Solution:**
```bash
# Hard refresh: Ctrl + F5
# Or clear browser cache
# Or open in incognito mode
```

---

## 🚀 Complete Deployment Flow

```
Developer:
├─ Edit src/main.rs
├─ Update Cargo.toml version
├─ git commit -m "New feature"
└─ git push origin main
       ↓
GitHub Actions:
├─ Trigger: release-build.yml
├─ Build: cargo build --release
├─ Copy: system-monitor.exe → docs/downloads/
├─ Rename: SystemMonitor-v1.0.1.exe
├─ Copy: SystemMonitor-latest.exe
├─ Commit: "🚀 Auto-deploy: Update binaries"
└─ Push: Changes to repository
       ↓
GitHub Pages:
├─ Trigger: pages-deploy.yml
├─ Upload: docs/ folder
└─ Deploy: https://username.github.io/repo/
       ↓
User:
├─ Visit: Website
├─ Click: "Download Now"
├─ Download: SystemMonitor-latest.exe
└─ Run: Application! 🎉
```

---

## 📋 Summary

### What You Get:

1. **Automatic Builds**
   - Every commit triggers build

2. **Automatic Deployment**
   - .exe copied to docs/downloads/
   - Auto-committed to repository

3. **GitHub Pages Hosting**
   - Direct .exe downloads
   - Fast CDN delivery

4. **GitHub Releases**
   - Backup download location
   - Version history

5. **Zero Manual Work**
   - Push code → Everything automatic!

---

## 🎊 You're All Set!

Your system now:
✅ Builds automatically on commit
✅ Deploys .exe to GitHub Pages
✅ Serves downloads directly
✅ Creates GitHub Releases
✅ Updates website automatically

**Just commit and push - everything else is automatic!** 🚀

---

## 🔥 Quick Commands

```bash
# Make changes
code src/main.rs

# Update version (optional)
code Cargo.toml
# version = "1.0.1"

# Commit and push
git add .
git commit -m "New feature"
git push origin main

# Wait 5-10 minutes
# Visit: https://xenonesis.github.io/sysmon/
# Click: "Download Now"
# Download: SystemMonitor-latest.exe
# Done! 🎉
```
