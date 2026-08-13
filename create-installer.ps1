# System Monitor - Distribution Builder
# Produces ONLY the installable Inno Setup.exe. Never ships a portable bare exe.

param(
    [switch]$Clean
)

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "   System Monitor - Distribution Builder" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host ""

$ErrorActionPreference = "Stop"

$AppName = "SystemMonitor"
$cargoToml = Get-Content "Cargo.toml" -Raw
if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
    $Version = $matches[1]
} else {
    throw "Cannot read version from Cargo.toml"
}
$DistDir = "dist"
$DownloadsDir = "downloads"
$Installer = "SystemMonitor-$Version-setup.exe"

# Clean previous distribution leftovers
if ($Clean) {
    Write-Host "-> Cleaning previous distributions..." -ForegroundColor White
    Remove-Item "$DistDir\*" -Force -Recurse -ErrorAction SilentlyContinue
    Remove-Item "$DownloadsDir\*" -Force -Recurse -ErrorAction SilentlyContinue
}

# Build release + compile the Inno Setup.exe (build.ps1 does both)
Write-Host "-> Building application and installer..." -ForegroundColor White
& ".\build.ps1" -NoLaunch -AllowDevelopmentCertificate
if ($LASTEXITCODE -ne 0) {
    Write-Host "[FAIL] Build failed with exit code $LASTEXITCODE" -ForegroundColor Red
    exit 1
}

# The one deliverable: the Inno installer exe produced by build.ps1
$installerPath = "$DownloadsDir\$Installer"
if (-not (Test-Path $installerPath)) {
    Write-Host "[FAIL] Installer not found: $installerPath (is Inno Setup 6 installed?)" -ForegroundColor Red
    exit 1
}

# Place a copy in dist/ (single installer file, no portable bundle)
New-Item -ItemType Directory -Path "$DistDir" -Force | Out-Null
Remove-Item "$DistDir\$AppName-v$Version.zip" -Force -ErrorAction SilentlyContinue
Remove-Item "$DistDir\$AppName-v$Version" -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item $installerPath "$DistDir\" -Force

# Purge any stray portable artifacts (bare exes, zips, bundles)
Get-ChildItem $DownloadsDir -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match 'SystemMonitor-v\d+\.\d+\.\d+\.exe' -and $_.Name -notmatch '-setup' -and $_.Name -ne "SystemMonitor-Setup-v$Version.exe" } |
    Remove-Item -Force
Get-ChildItem $DownloadsDir -Filter *.zip -ErrorAction SilentlyContinue | Remove-Item -Force

Write-Host ""
Write-Host "=============================================" -ForegroundColor Green
Write-Host "   Installer Ready (installable only)" -ForegroundColor Green
Write-Host "=============================================" -ForegroundColor Green
Write-Host ""
Write-Host "  $DownloadsDir\$Installer" -ForegroundColor White
Write-Host "  $DistDir\$Installer" -ForegroundColor White
Write-Host ""
Write-Host "No portable build is produced. Users install via the Setup.exe." -ForegroundColor Cyan
Write-Host ""