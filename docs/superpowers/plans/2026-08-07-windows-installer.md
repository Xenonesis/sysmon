# Windows Installer (Inno Setup) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a proper Windows installer (Inno Setup EXE) so System Monitor installs to Program Files, appears in Windows search / Start Menu, uninstalls cleanly from Settings → Apps, and the built-in updater silently installs over an installed app.

**Architecture:** One new Inno Setup script (`installer/installer.iss`) compiled by ISCC in CI after the existing release build; the updater prefers the `*-setup.exe` release asset and runs it silently when the app is installed (detected via exe path or uninstall registry key), falling back to the existing portable self-replace logic otherwise.

**Tech Stack:** Inno Setup 6 (ISCC), GitHub Actions (windows-latest, pwsh, choco), Rust 1.70+ (winreg crate already a dependency).

## Global Constraints

- Single version source: `Cargo.toml` `CARGO_PKG_VERSION` (2.2.0). CI passes it to ISCC as `/DAppVersion=` from the git tag (leading `v` stripped).
- Fixed installer `AppId`: `{{3F2A9C41-8E7D-4B6A-9C21-5D8E4F1A7B62}` (keeps updates installing over the same entry).
- App keeps `requireAdministrator` manifest — do NOT touch `build.rs` manifest.
- Release keeps BOTH assets: installer (`SystemMonitor-<ver>-setup.exe`) and portable exe.
- Settings already live in `%APPDATA%\Xenonesis\SystemMonitor\settings.json` — no change.
- `.gitignore` already contains `*.exe` — `installer/output/` needs no new rule.
- All paths in this plan are relative to the repo root `C:/Users/Acer/Desktop/sysmon` unless absolute.

---

### Task 1: Commit `assets/icon.ico`

**Files:**
- Create: `assets/icon.ico` (generated from `assets/icon.png` via the existing build.rs conversion)

**Interfaces:**
- Produces: `assets/icon.ico` — referenced by `SetupIconFile` in Task 2.

- [ ] **Step 1: Generate the icon**

Run (PowerShell, repo root):

```powershell
cargo build
$ico = Get-ChildItem -Path target\debug\build\system-monitor-*\out\icon.ico -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $ico) { throw "icon.ico not found in build output" }
Copy-Item $ico.FullName assets\icon.ico
```

Expected: `assets/icon.ico` exists and is non-empty.

- [ ] **Step 2: Verify it is a valid ICO**

Run: `git status --short`
Expected: `assets/icon.ico` listed as untracked (not ignored).

- [ ] **Step 3: Commit**

```bash
git add assets/icon.ico
git commit -m "build: add installer icon asset"
```

---

### Task 2: Create `installer/installer.iss`

**Files:**
- Create: `installer/installer.iss`

**Interfaces:**
- Produces: `installer/installer.iss` — compiled by `ISCC.exe` in Task 3 and Task 4. Sources `../target/release/system-monitor.exe` and `../assets/icon.ico`. Output lands in `installer/output/`.

- [ ] **Step 1: Write the script**

Create `installer/installer.iss` with exactly this content:

```ini
; System Monitor installer - Inno Setup 6
#ifndef AppVersion
  #define AppVersion "2.2.0"
#endif
#define MyAppName "System Monitor"
#define MyAppPublisher "Xenonesis"
#define MyAppExeName "system-monitor.exe"
#define MyAppId "{{3F2A9C41-8E7D-4B6A-9C21-5D8E4F1A7B62}"

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\System Monitor
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
OutputDir=output
OutputBaseFilename=SystemMonitor-{#AppVersion}-setup
SetupIconFile=..\assets\icon.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\system-monitor.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent
```

- [ ] **Step 2: Sanity-check the file**

Run: `git status --short`
Expected: `installer/installer.iss` untracked. No commit yet — Task 3 verifies it compiles first.

---

### Task 3: Compile the installer locally and verify output

**Files:**
- Modify: none (verification only)

**Interfaces:**
- Consumes: `installer/installer.iss` (Task 2), `target/release/system-monitor.exe` (this task).
- Produces: `installer/output/SystemMonitor-2.2.0-setup.exe` — the local artifact to inspect.

- [ ] **Step 1: Build the release exe**

```powershell
cargo build --release
```

Expected: `target\release\system-monitor.exe` exists. (Release build has LTO + panic=abort; can take several minutes.)

- [ ] **Step 2: Install Inno Setup**

```powershell
choco install innosetup -y --no-progress
```

Expected: `C:\Program Files (x86)\Inno Setup 6\ISCC.exe` exists.
If choco needs elevation and fails, verify the compile in CI instead (Task 4) and mark this step N/A.

- [ ] **Step 3: Compile the installer**

```powershell
& "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer\installer.iss
```

Expected: exit code 0, no errors. Default `AppVersion` 2.2.0 applies (no `/DAppVersion` locally).

- [ ] **Step 4: Inspect the output**

Run: `Get-ChildItem installer\output\SystemMonitor-*-setup.exe | Select-Object Name, Length`
Expected: `SystemMonitor-2.2.0-setup.exe`, non-trivial size (> 500 KB).

- [ ] **Step 5: Commit the script**

```bash
git add installer/installer.iss
git commit -m "build: add Inno Setup installer script"
```

---

### Task 4: Wire the installer into the GitHub Actions release workflow

**Files:**
- Modify: `.github/workflows/windows-release.yml`

**Interfaces:**
- Consumes: `installer/installer.iss`, `installer/output/SystemMonitor-<ver>-setup.exe` from Task 3's steps.
- Produces: two new CI steps that upload `SystemMonitor-<ver>-setup.exe` as a workflow artifact AND a GitHub Release asset (tag pushes only). Later tasks rely on the release asset being named `SystemMonitor-<version>-setup.exe`.

- [ ] **Step 1: Insert a version step**

After the existing `Build release` step, insert:

```yaml
      - name: Compute version from tag
        id: version
        shell: pwsh
        run: |
          $v = "${{ github.ref_name }}".TrimStart('v')
          echo "version=$v" | Out-File -FilePath $env:GITHUB_OUTPUT -Encoding utf8 -Append
```

- [ ] **Step 2: Insert installer build + artifact upload**

Directly after the Compute version step, insert:

```yaml
      - name: Install Inno Setup
        shell: pwsh
        run: choco install innosetup -y --no-progress

      - name: Build installer
        shell: pwsh
        run: |
          & "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer\installer.iss /DAppVersion=${{ steps.version.outputs.version }}

      - name: Upload installer artifact
        uses: actions/upload-artifact@v4
        with:
          name: system-monitor-installer
          path: installer\output\SystemMonitor-*-setup.exe
```

- [ ] **Step 3: Insert installer release asset upload**

After the existing `Upload release asset (only on tag push)` step, insert:

```yaml
      - name: Upload installer release asset (only on tag push)
        if: startsWith(github.ref, 'refs/tags/')
        uses: actions/upload-release-asset@v1
        with:
          upload_url: ${{ steps.create_release.outputs.upload_url }}
          asset_path: installer\output\SystemMonitor-${{ steps.version.outputs.version }}-setup.exe
          asset_name: SystemMonitor-${{ steps.version.outputs.version }}-setup.exe
          asset_content_type: application/octet-stream
```

- [ ] **Step 4: Verify the workflow parses**

Run: `python -c "import yaml,sys; yaml.safe_load(open('.github/workflows/windows-release.yml')); print('OK')"` — if Python with PyYAML is unavailable, do a careful visual review of indentation (steps must be list items at the same indent level as the existing steps).

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/windows-release.yml
git commit -m "ci: build and publish Inno Setup installer"
```

---

### Task 5: Updater — prefer the installer asset

**Files:**
- Modify: `src/updater.rs` (`check_for_updates`, the asset-selection loop)

**Interfaces:**
- Consumes: `GitHubRelease.assets` (existing struct).
- Produces: `self.update_info.download_url` now points to `SystemMonitor-<ver>-setup.exe` when present; falls back to the first portable exe/zip. Task 6 consumes this URL.

- [ ] **Step 1: Replace the asset-selection loop**

Current code (around line 72 of `src/updater.rs`):

```rust
                // Find the installer asset
                for asset in release.assets {
                    if asset.name.ends_with(".zip") || asset.name.ends_with(".exe") {
                        self.update_info.download_url = asset.browser_download_url;
                        break;
                    }
                }
```

Replace with:

```rust
                // Prefer the installer asset (SystemMonitor-<ver>-setup.exe);
                // fall back to the first portable exe/zip.
                let mut fallback_url = String::new();
                for asset in release.assets {
                    let name = asset.name.to_lowercase();
                    if name.contains("setup") && name.ends_with(".exe") {
                        self.update_info.download_url = asset.browser_download_url;
                        break;
                    }
                    if fallback_url.is_empty()
                        && (name.ends_with(".zip") || name.ends_with(".exe"))
                    {
                        fallback_url = asset.browser_download_url;
                    }
                }
                if self.update_info.download_url.is_empty() {
                    self.update_info.download_url = fallback_url;
                }
```

- [ ] **Step 2: Compile check**

Run: `cargo check`
Expected: OK (Task 7 re-runs it; failing early here is cheaper).

- [ ] **Step 3: Commit**

```bash
git add src/updater.rs
git commit -m "feat(updater): prefer installer asset for updates"
```

---

### Task 6: Updater — installed detection and silent install

**Files:**
- Modify: `src/updater.rs` (`impl Updater`, `download_and_install_update`)

**Interfaces:**
- Consumes: `download_url` (Task 5's asset selection).
- Produces: `Updater::is_installed() -> bool` and a silent-install branch that runs `system-monitor-new.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART` then exits. Portable path unchanged.

- [ ] **Step 1: Add `is_installed`**

Inside `impl Updater`, directly before `download_and_install_update`, add:

```rust
    fn is_installed(&self) -> bool {
        if let Ok(exe) = std::env::current_exe() {
            let path = exe.to_string_lossy().to_lowercase();
            if path.contains("\\program files\\") || path.contains("\\program files (x86)\\") {
                return true;
            }
        }
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            if hklm
                .open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\System Monitor")
                .is_ok()
            {
                return true;
            }
        }
        false
    }
```

- [ ] **Step 2: Detect installer URLs**

In `download_and_install_update`, the current line is:

```rust
        let is_exe = download_url.to_lowercase().ends_with(".exe") 
            || download_url.to_lowercase().contains(".exe?");
```

Add directly after it:

```rust
        let is_installer = download_url.to_lowercase().contains("setup");
```

- [ ] **Step 3: Add the silent-install branch**

The current code structure after the download is:

```rust
        if is_exe {
            #[cfg(target_os = "windows")]
            {
                ... powershell self-replace ...
                std::process::exit(0);
            }
```

Insert BEFORE that `if is_exe {` block:

```rust
        // Installed app + installer asset -> silent install (replaces exe,
        // shortcuts, and uninstall entry in one pass).
        if is_exe && is_installer && self.is_installed() {
            #[cfg(target_os = "windows")]
            {
                use std::process::Command;
                use std::os::windows::process::CommandExt;
                Command::new(&installer_path)
                    .creation_flags(0x08000000)
                    .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
                    .spawn()
                    .map_err(|e| format!("Failed to spawn installer: {}", e))?;
                std::process::exit(0);
            }
            #[cfg(not(target_os = "windows"))]
            {
                return Err("Installer updates are only supported on Windows".to_string());
            }
        }
```

- [ ] **Step 4: Compile check**

Run: `cargo check`
Expected: OK. (The `use winreg` inside the cfg block matches the existing pattern in `main.rs`; `winreg` is a `cfg(windows)` dependency.)

- [ ] **Step 5: Commit**

```bash
git add src/updater.rs
git commit -m "feat(updater): silent-install updates for installed app"
```

---

### Task 7: Full verification

**Files:**
- Modify: none

- [ ] **Step 1: Release build**

Run: `cargo build --release`
Expected: exit 0, `target\release\system-monitor.exe` refreshed with Task 5/6 changes.

- [ ] **Step 2: Recompile the installer with the fresh exe**

Run: `& "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer\installer.iss`
Expected: exit 0.

- [ ] **Step 3: Grep for leftovers**

Run: `git diff HEAD --stat` and search for any remaining bare `ends_with(".zip")`/`.exe` first-match logic in `src/updater.rs` that bypasses installer preference:
`grep -n "Find the installer asset" src/updater.rs`
Expected: no matches.

---

### Task 8: Update README installation docs

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: nothing new; documents the artifacts Task 3/4 produce.

- [ ] **Step 1: Replace the Quick Download block**

Current:

```markdown
### ⚡ Direct .exe Download

- **No installation required** - Just download and run!
- **Instant setup** - Click download → Run .exe → Done!
- **Auto-updates** - App automatically checks for updates every 24 hours
- **Size:** ~5.4 MB - Lightweight and fast
```

Replace with:

```markdown
### ⚡ Installer Download (Recommended)

- **Proper Windows install** - Installs to Program Files with Start Menu entry
- **Searchable** - Find it from Windows search / Start Menu
- **Clean uninstall** - Listed in Settings → Apps
- **Auto-updates** - App automatically checks for updates every 24 hours
- **Size:** ~5.4 MB - Lightweight and fast

Prefer portable? Use `system-monitor-<version>-windows-x64.exe` - no installation needed.
```

- [ ] **Step 2: Replace the Installation section**

Current section (from `## Installation` through the end of `### Option 2: Quick Build`'s code block) — replace the whole `## Installation` section with:

```markdown
## Installation

### Option 1: Installer (Recommended)

Download `SystemMonitor-<version>-setup.exe` from the [Releases page](https://github.com/Xenonesis/sysmon/releases).

The installer:

- Installs to `C:\Program Files\System Monitor`
- Adds a Start Menu entry (searchable from Windows search)
- Optionally creates a Desktop shortcut
- Registers the app in Settings → Apps for clean uninstall
- Requires administrator privileges (the app needs elevated rights for RAM cleaning, process management, and startup optimization)

### Option 2: Portable

Prefer running without installing? Download `system-monitor-<version>-windows-x64.exe` and run it directly. Settings are still stored per-user in `%APPDATA%\Xenonesis\SystemMonitor`.

### Building from source

1. Install [Rust](https://rustup.rs) 1.70+
2. `cargo build --release`
3. The executable is at `target\release\system-monitor.exe`
```

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: document installer as primary install path"
```

---

## Manual acceptance checklist (post-implementation, requires a Windows machine)

1. Run `SystemMonitor-<ver>-setup.exe` → app lands in `C:\Program Files\System Monitor`.
2. Windows Start search finds "System Monitor" (Start Menu shortcut exists).
3. Settings → Apps lists "System Monitor" with a working Uninstall.
4. Uninstall removes the app, shortcuts, and the entry (leave `%APPDATA%` settings — intended).
5. Update flow: with the app installed, trigger an update against a newer tag → installer runs silently, no second uninstall entry appears, version bumps.
6. Portable flow unchanged: raw exe still self-replaces via the existing PowerShell path.
