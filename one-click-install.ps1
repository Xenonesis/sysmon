# System Monitor - One-Click Installer  
# This script automates the complete installation process

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "   System Monitor - Automated Installer" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host ""

$ErrorActionPreference = "Continue"

# Check if executable exists
if (Test-Path "target\release\system-monitor.exe") {
    Write-Host "✓ Executable found!" -ForegroundColor Green
    Write-Host ""
    
    $InstallPath = "$env:LOCALAPPDATA\Programs\SystemMonitor"
    
    # Create installation directory
    Write-Host "Installing System Monitor..." -ForegroundColor Cyan
    if (-not (Test-Path $InstallPath)) {
        New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
    }
    
    # Copy executable
    Copy-Item "target\release\system-monitor.exe" "$InstallPath\system-monitor.exe" -Force
    Write-Host "✓ Application installed to: $InstallPath" -ForegroundColor Green
    
    # Create desktop shortcut
    Write-Host "✓ Creating desktop shortcut..." -ForegroundColor Green
    $WshShell = New-Object -ComObject WScript.Shell
    $Shortcut = $WshShell.CreateShortcut("$env:USERPROFILE\Desktop\System Monitor.lnk")
    $Shortcut.TargetPath = "$InstallPath\system-monitor.exe"
    $Shortcut.WorkingDirectory = $InstallPath
    $Shortcut.Description = "System Monitor - Real-time system monitoring"
    $ Shortcut.Save()
    
    # Create Start Menu shortcut
    Write-Host "✓ Creating Start Menu shortcut..." -ForegroundColor Green
    $StartMenuPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
    $Shortcut = $WshShell.CreateShortcut("$StartMenuPath\System Monitor.lnk")
    $Shortcut.TargetPath = "$InstallPath\system-monitor.exe"
    $Shortcut.WorkingDirectory = $InstallPath
    $Shortcut.Description = "System Monitor - Real-time system monitoring"
    $Shortcut.Save()
    
    Write-Host ""
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host "   Installation Complete!" -ForegroundColor Green
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Shortcuts created:" -ForegroundColor Cyan
    Write-Host "  ✓ Desktop: System Monitor.lnk" -ForegroundColor White
    Write-Host "  ✓ Start Menu: System Monitor.lnk" -ForegroundColor White
    Write-Host ""
    Write-Host "You can now:" -ForegroundColor Cyan
    Write-Host "  • Double-click the desktop shortcut" -ForegroundColor White
    Write-Host "  • Search for 'System Monitor' in Start Menu" -ForegroundColor White
    Write-Host "  • Pin to taskbar for quick access" -ForegroundColor White
    Write-Host ""
    
    $run = Read-Host "Launch System Monitor now? (Y/N)"
    if ($run -eq "Y" -or $run -eq "y") {
        Write-Host "Starting System Monitor..." -ForegroundColor Green
        Start-Process "$InstallPath\system-monitor.exe"
    }
    
    Write-Host ""
    Write-Host "Thank you for using System Monitor!" -ForegroundColor Cyan
    Write-Host ""
}
else {
    Write-Host "✗ Executable not found!" -ForegroundColor Red
    Write-Host ""
    Write-Host "The application needs to be built first." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Please choose an option:" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "1. Auto-build (requires build tools)" -ForegroundColor White
    Write-Host "2. Setup build environment" -ForegroundColor White
    Write-Host "3. View setup guide" -ForegroundColor White
    Write-Host "4. Exit" -ForegroundColor White
    Write-Host ""
    
    $choice = Read-Host "Enter choice (1-4)"
    
    switch ($choice) {
        "1" {
            Write-Host ""
            Write-Host "Attempting to build..." -ForegroundColor Cyan
            Write-Host ""
            
            # Try to build
            if (Get-Command cargo -ErrorAction SilentlyContinue) {
                cargo build --release
                
                if ($LASTEXITCODE -eq 0) {
                    Write-Host ""
                    Write-Host "✓ Build successful!" -ForegroundColor Green
                    Write-Host ""
                    Write-Host "Run this script again to install." -ForegroundColor Cyan
                }
                else {
                    Write-Host ""
                    Write-Host "✗ Build failed!" -ForegroundColor Red
                    Write-Host ""
                    Write-Host "You may need to setup the build environment." -ForegroundColor Yellow
                    Write-Host "Run: .\setup-build-environment.ps1" -ForegroundColor White
                }
            }
            else {
                Write-Host "✗ Rust/Cargo not found!" -ForegroundColor Red
                Write-Host ""
                Write-Host "Please install Rust from: https://rustup.rs/" -ForegroundColor White
                Start-Process "https://rustup.rs/"
            }
        }
        "2" {
            Write-Host ""
            Write-Host "Running build environment setup..." -ForegroundColor Cyan
            .\setup-build-environment.ps1
        }
        "3" {
            Write-Host ""
            Write-Host "Opening setup guide..." -ForegroundColor Cyan
            if (Test-Path "SETUP_GUIDE.md") {
                Start-Process "SETUP_GUIDE.md"
            }
            else {
                Write-Host "Setup guide not found." -ForegroundColor Red
            }
        }
        "4" {
            Write-Host "Installation cancelled." -ForegroundColor Yellow
            exit 0
        }
        default {
            Write-Host "Invalid choice." -ForegroundColor Red
            exit 1
        }
    }
}
