# System Monitor - Installation Script
# This script installs the System Monitor GUI application to your system

param(
    [string]$InstallPath = "$env:LOCALAPPDATA\Programs\SystemMonitor"
)

Write-Host "===================================" -ForegroundColor Cyan
Write-Host "  System Monitor - Installation" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan
Write-Host ""

# Check if executable exists
if (-not (Test-Path "target\release\system-monitor.exe")) {
    Write-Host "Error: Executable not found!" -ForegroundColor Red
    Write-Host "Please run build.ps1 first to build the application." -ForegroundColor Yellow
    exit 1
}

# Create installation directory
Write-Host "Creating installation directory..." -ForegroundColor Green
if (-not (Test-Path $InstallPath)) {
    New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
}

# Copy executable
Write-Host "Installing System Monitor to: $InstallPath" -ForegroundColor Green
Copy-Item "target\release\system-monitor.exe" "$InstallPath\system-monitor.exe" -Force

# Create desktop shortcut
Write-Host "Creating desktop shortcut..." -ForegroundColor Green
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$env:USERPROFILE\Desktop\System Monitor.lnk")
$Shortcut.TargetPath = "$InstallPath\system-monitor.exe"
$Shortcut.WorkingDirectory = $InstallPath
$Shortcut.Description = "System Monitor - Real-time system monitoring"
$Shortcut.Save()

# Create Start Menu shortcut
Write-Host "Creating Start Menu shortcut..." -ForegroundColor Green
$StartMenuPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$Shortcut = $WshShell.CreateShortcut("$StartMenuPath\System Monitor.lnk")
$Shortcut.TargetPath = "$InstallPath\system-monitor.exe"
$Shortcut.WorkingDirectory = $InstallPath
$Shortcut.Description = "System Monitor - Real-time system monitoring"
$Shortcut.Save()

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host ""
Write-Host "System Monitor has been installed to:" -ForegroundColor Cyan
Write-Host "  $InstallPath" -ForegroundColor White
Write-Host ""
Write-Host "Shortcuts created:" -ForegroundColor Cyan
Write-Host "  - Desktop: System Monitor.lnk" -ForegroundColor White
Write-Host "  - Start Menu: System Monitor.lnk" -ForegroundColor White
Write-Host ""
Write-Host "You can now:" -ForegroundColor Cyan
Write-Host "  1. Double-click the desktop shortcut" -ForegroundColor White
Write-Host "  2. Search for 'System Monitor' in Start Menu" -ForegroundColor White
Write-Host "  3. Pin to taskbar for quick access" -ForegroundColor White
Write-Host ""

$run = Read-Host "Would you like to launch System Monitor now? (Y/N)"
if ($run -eq "Y" -or $run -eq "y") {
    Write-Host "Launching System Monitor..." -ForegroundColor Green
    Start-Process "$InstallPath\system-monitor.exe"
}

Write-Host ""
Write-Host "Thank you for using System Monitor!" -ForegroundColor Cyan
