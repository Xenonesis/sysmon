# System Monitor - Uninstallation Script
# This script removes the System Monitor application from your system

param(
    [string]$InstallPath = "$env:LOCALAPPDATA\Programs\SystemMonitor"
)

Write-Host "===================================" -ForegroundColor Yellow
Write-Host "  System Monitor - Uninstallation" -ForegroundColor Yellow
Write-Host "===================================" -ForegroundColor Yellow
Write-Host ""

$confirm = Read-Host "Are you sure you want to uninstall System Monitor? (Y/N)"
if ($confirm -ne "Y" -and $confirm -ne "y") {
    Write-Host "Uninstallation cancelled." -ForegroundColor Green
    exit 0
}

Write-Host ""
Write-Host "Removing System Monitor..." -ForegroundColor Yellow

# Remove installation directory
if (Test-Path $InstallPath) {
    Write-Host "Removing installation directory..." -ForegroundColor Yellow
    Remove-Item -Path $InstallPath -Recurse -Force
    Write-Host "  ✓ Installation directory removed" -ForegroundColor Green
} else {
    Write-Host "  ℹ Installation directory not found" -ForegroundColor Gray
}

# Remove desktop shortcut
$desktopShortcut = "$env:USERPROFILE\Desktop\System Monitor.lnk"
if (Test-Path $desktopShortcut) {
    Write-Host "Removing desktop shortcut..." -ForegroundColor Yellow
    Remove-Item -Path $desktopShortcut -Force
    Write-Host "  ✓ Desktop shortcut removed" -ForegroundColor Green
} else {
    Write-Host "  ℹ Desktop shortcut not found" -ForegroundColor Gray
}

# Remove Start Menu shortcut
$startMenuShortcut = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\System Monitor.lnk"
if (Test-Path $startMenuShortcut) {
    Write-Host "Removing Start Menu shortcut..." -ForegroundColor Yellow
    Remove-Item -Path $startMenuShortcut -Force
    Write-Host "  ✓ Start Menu shortcut removed" -ForegroundColor Green
} else {
    Write-Host "  ℹ Start Menu shortcut not found" -ForegroundColor Gray
}

Write-Host ""
Write-Host "System Monitor has been successfully uninstalled!" -ForegroundColor Green
Write-Host ""
Write-Host "Note: If you pinned the app to taskbar, please remove it manually." -ForegroundColor Cyan
Write-Host ""
