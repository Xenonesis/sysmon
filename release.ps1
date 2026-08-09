# System Monitor - Release Rule
# One-command release: bump version, build, refresh dist, update README + website,
# delete old builds, and publish so installed users get the update notification.

param(
    [string]$Version = "",          # e.g. 2.7.0 ; omit to reuse Cargo.toml version
    [string]$Changelog = "",        # short release note for README changelog
    [switch]$Sign,                  # Authenticode-sign the installer exe (needs trusted cert)
    [switch]$Publish                # commit, tag, push, create GitHub release + deploy site
)

$ErrorActionPreference = "Stop"
$RepoRoot = $PSScriptRoot
Set-Location $RepoRoot

function Write-Step { param([string]$M) Write-Host "-> $M" -ForegroundColor White }
function Write-Ok   { param([string]$M) Write-Host "[OK] $M" -ForegroundColor Green }

# 1. Resolve / bump version -----------------------------------------------
$cargo = Get-Content "Cargo.toml" -Raw
if (-not ($cargo -match 'version\s*=\s*"([^"]+)"')) { throw "Cannot read version from Cargo.toml" }
$current = $Matches[1]
if ($Version -ne "" -and $Version -ne $current) {
    if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Version must be X.Y.Z, got '$Version'" }
    Write-Step "Bumping version $current -> $Version"
    (Get-Content "Cargo.toml" -Raw) -replace "version\s*=\s*`"$current`"", "version = `"$Version`"" | Set-Content "Cargo.toml" -NoNewline
    $current = $Version
} else {
    Write-Step "Using existing version $current"
}
$tag = "v$current"

# 2. Update README.md ------------------------------------------------------
Write-Step "Updating README.md"
$readme = Get-Content "README.md" -Raw
$readme = $readme -replace 'Version-[\d.]+-gray', "Version-$current-gray"
$readme = $readme -replace 'Download SysMon Installer \(v[\d.]+\)', "Download SysMon Installer (v$current)"
if ($Changelog -ne "") {
    $entry = "### [$current] - $Changelog"
    $readme = $readme -replace "(## Changelog\r?\n\r?\n)", "`$1$entry`r`n"
}
Set-Content "README.md" $readme -NoNewline
Write-Ok "README.md updated"

# 3. Update website (docs/ GitHub Pages source) ------------------------------
if (Test-Path "docs\index.html") {
    Write-Step "Updating docs/index.html"
    $html = Get-Content "docs\index.html" -Raw
    $html = $html -replace 'v\d+\.\d+\.\d+', "v$current"
    Set-Content "docs\index.html" $html -NoNewline
    Write-Ok "docs/index.html updated"
} else {
    Write-Host "[WARN] docs/index.html missing; website version tags not updated" -ForegroundColor Yellow
}

# 4. Build release ------------------------------------------------------------
Write-Step "Building release (cargo build --release)"
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }
Write-Ok "Release build OK"

# 5. Create installer + dist ---------------------------------------------------
Write-Step "Creating installer package"
& ".\create-installer.ps1"
if ($LASTEXITCODE -ne 0) { throw "create-installer.ps1 failed" }
Write-Ok "Installer package created"

# 6. Delete old builds, keep only the new one ----------------------------------
Write-Step "Removing old builds from dist/ and downloads/"
Get-ChildItem "dist" -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -notlike "*$current*" -and $_.Name -match 'SystemMonitor-v\d' } |
    ForEach-Object {
        Write-Host "   deleting $($_.Name)" -ForegroundColor DarkGray
        Remove-Item $_.FullName -Recurse -Force
    }
Get-ChildItem "downloads" -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match 'SystemMonitor-v\d+\.\d+\.\d+' -and $_.Name -notlike "*v$current*" } |
    ForEach-Object {
        Write-Host "   deleting $($_.Name)" -ForegroundColor DarkGray
        Remove-Item $_.FullName -Force
    }
Write-Ok "dist/ and downloads/ contain only v$current"

# 7. Optional Authenticode signing ----------------------------------------------
if ($Sign) {
    Write-Step "Signing installer"
    & ".\sign-binary.ps1" -FilePath "downloads\SystemMonitor-$current-setup.exe"
    Write-Host "[WARN] Self-signed signatures are NOT 'Valid' to Get-AuthenticodeSignature on" -ForegroundColor Yellow
    Write-Host "       user machines. The in-app updater refuses installs unless the signature" -ForegroundColor Yellow
    Write-Host "       verifies Valid. Use a trusted code-signing cert for real auto-updates." -ForegroundColor Yellow
}

# 8. Publish: commit, tag, push, GitHub release, website deploy ------------------
if ($Publish) {
    Write-Step "Publishing $tag"
    git add -A
    git commit -m "release: v$current" | Out-Null
    git tag $tag
    git push origin HEAD --tags
    Write-Ok "Committed and pushed $tag"

    # ── Distribute ONLY the installer ──
    $setupPath = "downloads\SystemMonitor-$current-setup.exe"
    if (-not (Test-Path $setupPath)) {
        Write-Host "[WARN] Installer not found at $setupPath; release skipped" -ForegroundColor Yellow
    } elseif (Get-Command gh -ErrorAction SilentlyContinue) {
        gh release create $tag $setupPath --title "SystemMonitor v$current" --notes "$Changelog"
        Write-Ok "GitHub release created; installed apps will show the update notification"
    } else {
        Write-Host "[WARN] gh CLI not found. Create the release manually to notify installed users:" -ForegroundColor Yellow
        Write-Host "    gh release create $tag `"$setupPath`"" -ForegroundColor Yellow
        Write-Host "    (or: https://github.com/Xenonesis/sysmon/releases/new -> tag $tag, attach $setupPath)" -ForegroundColor Yellow
    }

    # Deploy website (docs/ to GitHub Pages)
    if (Test-Path "docs\index.html") {
        & ".\deploy-website.ps1" -Deploy
    } else {
        Write-Host "[WARN] website files absent; skipping deploy" -ForegroundColor Yellow
    }
} else {
    Write-Host "`nRelease artifacts ready locally. Re-run with -Publish to push, tag," -ForegroundColor Cyan
    Write-Host "create the GitHub release (installed-user notifications) and deploy the site." -ForegroundColor Cyan
}
Write-Host "`nDone: v$current" -ForegroundColor Green
