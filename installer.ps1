# System Monitor Professional Installer
# Provides a clean, user-friendly installation experience

param(
    [switch]$Silent,
    [switch]$Uninstall
)

# Configuration
$AppName = "System Monitor"
$AppVersion = "1.0.0"
$InstallDir = "$env:LOCALAPPDATA\Programs\SystemMonitor"
$ExeName = "system-monitor.exe"
$SourceExe = "$PSScriptRoot\target\release\$ExeName"

# Colors for output
$Green = "Green"
$Cyan = "Cyan"
$Yellow = "Yellow"
$Red = "Red"
$White = "White"

function Write-Header {
    Clear-Host
    Write-Host "=============================================" -ForegroundColor $Cyan
    Write-Host "   $AppName Setup" -ForegroundColor $Cyan
    Write-Host "   Version $AppVersion" -ForegroundColor $Cyan
    Write-Host "=============================================" -ForegroundColor $Cyan
    Write-Host ""
}

function Write-Step {
    param([string]$Message)
    Write-Host "→ $Message" -ForegroundColor $White
}

function Write-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor $Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠ $Message" -ForegroundColor $Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor $Red
}

function Test-Prerequisites {
    Write-Step "Checking prerequisites..."

    # Check if executable exists
    if (-not (Test-Path $SourceExe)) {
        Write-Error "Application executable not found: $SourceExe"
        Write-Host ""
        Write-Warning "The application needs to be built first."
        Write-Host "Please run: .\build.ps1" -ForegroundColor $White
        Write-Host ""
        return $false
    }

    Write-Success "Prerequisites check passed"
    return $true
}

function Install-Application {
    Write-Step "Installing $AppName..."

    try {
        # Create installation directory
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }

        # Copy executable
        Copy-Item $SourceExe "$InstallDir\$ExeName" -Force
        Write-Success "Application installed to: $InstallDir"

        return $true
    }
    catch {
        Write-Error "Failed to install application: $($_.Exception.Message)"
        return $false
    }
}

function Create-Shortcuts {
    Write-Step "Creating shortcuts..."

    try {
        $WshShell = New-Object -ComObject WScript.Shell

        # Desktop shortcut
        $DesktopShortcut = $WshShell.CreateShortcut("$env:USERPROFILE\Desktop\$AppName.lnk")
        $DesktopShortcut.TargetPath = "$InstallDir\$ExeName"
        $DesktopShortcut.WorkingDirectory = $InstallDir
        $DesktopShortcut.Description = "$AppName - Real-time system monitoring application"
        $DesktopShortcut.IconLocation = "$InstallDir\$ExeName,0"
        $DesktopShortcut.Save()
        Write-Success "Desktop shortcut created"

        # Start Menu shortcut
        $StartMenuPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
        if (-not (Test-Path $StartMenuPath)) {
            New-Item -ItemType Directory -Path $StartMenuPath -Force | Out-Null
        }

        $StartShortcut = $WshShell.CreateShortcut("$StartMenuPath\$AppName.lnk")
        $StartShortcut.TargetPath = "$InstallDir\$ExeName"
        $StartShortcut.WorkingDirectory = $InstallDir
        $StartShortcut.Description = "$AppName - Real-time system monitoring application"
        $StartShortcut.IconLocation = "$InstallDir\$ExeName,0"
        $StartShortcut.Save()
        Write-Success "Start Menu shortcut created"

        return $true
    }
    catch {
        Write-Warning "Failed to create shortcuts: $($_.Exception.Message)"
        return $false
    }
}

function Register-Uninstaller {
    Write-Step "Registering uninstaller..."

    try {
        # Create uninstall registry entry
        $RegPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$AppName"

        if (-not (Test-Path $RegPath)) {
            New-Item -Path $RegPath -Force | Out-Null
        }

        # Set uninstaller information
        Set-ItemProperty -Path $RegPath -Name "DisplayName" -Value $AppName
        Set-ItemProperty -Path $RegPath -Name "DisplayVersion" -Value $AppVersion
        Set-ItemProperty -Path $RegPath -Name "Publisher" -Value "Xenonesis"
        Set-ItemProperty -Path $RegPath -Name "InstallLocation" -Value $InstallDir
        Set-ItemProperty -Path $RegPath -Name "UninstallString" -Value 'powershell.exe -ExecutionPolicy Bypass -Command "& { Remove-Item ''$InstallDir'' -Recurse -Force; Remove-Item ''$env:USERPROFILE\Desktop\$AppName.lnk'' -ErrorAction SilentlyContinue; Remove-Item ''$env:APPDATA\Microsoft\Windows\Start Menu\Programs\$AppName.lnk'' -ErrorAction SilentlyContinue; Remove-Item ''HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$AppName'' -Recurse -ErrorAction SilentlyContinue; Write-Host ''System Monitor uninstalled successfully.'' }"'
        Set-ItemProperty -Path $RegPath -Name "NoModify" -Value 1
        Set-ItemProperty -Path $RegPath -Name "NoRepair" -Value 1

        Write-Success "Uninstaller registered"
        return $true
    }
    catch {
        Write-Warning "Failed to register uninstaller: $($_.Exception.Message)"
        return $false
    }
}

function Show-Completion {
    Write-Host ""
    Write-Host "=============================================" -ForegroundColor $Cyan
    Write-Host "   Installation Complete!" -ForegroundColor $Green
    Write-Host "=============================================" -ForegroundColor $Cyan
    Write-Host ""
    Write-Host "$AppName has been successfully installed!" -ForegroundColor $Green
    Write-Host ""
    Write-Host "Installation location:" -ForegroundColor $Cyan
    Write-Host "  $InstallDir" -ForegroundColor $White
    Write-Host ""
    Write-Host "You can now:" -ForegroundColor $Cyan
    Write-Host "  • Launch from Start Menu → $AppName" -ForegroundColor $White
    Write-Host "  • Use the desktop shortcut" -ForegroundColor $White
    Write-Host "  • Search for '$AppName' in Windows search" -ForegroundColor $White
    Write-Host "  • Uninstall via Windows Settings → Apps" -ForegroundColor $White
    Write-Host ""

    if (-not $Silent) {
        $launch = Read-Host "Launch $AppName now? (Y/N)"
        if ($launch -eq "Y" -or $launch -eq "y") {
            Write-Step "Starting $AppName..."
            Start-Process "$InstallDir\$ExeName"
        }
    }
}

function Uninstall-Application {
    Write-Step "Uninstalling $AppName..."

    try {
        # Remove installation directory
        if (Test-Path $InstallDir) {
            Remove-Item $InstallDir -Recurse -Force
            Write-Success "Application files removed"
        }

        # Remove shortcuts
        Remove-Item "$env:USERPROFILE\Desktop\$AppName.lnk" -ErrorAction SilentlyContinue
        Remove-Item "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\$AppName.lnk" -ErrorAction SilentlyContinue
        Write-Success "Shortcuts removed"

        # Remove registry entry
        Remove-Item "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$AppName" -Recurse -ErrorAction SilentlyContinue
        Write-Success "Uninstaller entry removed"

        Write-Host ""
        Write-Host "$AppName has been successfully uninstalled." -ForegroundColor $Green
        return $true
    }
    catch {
        Write-Error "Uninstallation failed: $($_.Exception.Message)"
        return $false
    }
}

# Main installation logic
if ($Uninstall) {
    Write-Header
    Write-Host "Uninstalling $AppName..." -ForegroundColor $Yellow
    Write-Host ""
    $result = Uninstall-Application
    exit [int](-not $result)
}

Write-Header

if (-not $Silent) {
    Write-Host "Welcome to the $AppName Setup Wizard" -ForegroundColor $White
    Write-Host ""
    Write-Host "This will install $AppName on your computer." -ForegroundColor $White
    Write-Host ""
    Write-Host "Installation Details:" -ForegroundColor $Cyan
    Write-Host "  • Location: $InstallDir" -ForegroundColor $White
    Write-Host "  • Shortcuts: Desktop and Start Menu" -ForegroundColor $White
    Write-Host "  • Uninstall: Available in Windows Settings" -ForegroundColor $White
    Write-Host ""

    $proceed = Read-Host "Continue with installation? (Y/N)"
    if ($proceed -ne "Y" -and $proceed -ne "y") {
        Write-Host "Installation cancelled." -ForegroundColor $Yellow
        exit 0
    }
    Write-Host ""
}

# Check prerequisites
if (-not (Test-Prerequisites)) {
    exit 1
}

# Install application
if (-not (Install-Application)) {
    exit 1
}

# Create shortcuts
Create-Shortcuts | Out-Null

# Register uninstaller
Register-Uninstaller | Out-Null

# Show completion
if (-not $Silent) {
    Show-Completion
}

Write-Host ""
Write-Host "Thank you for installing $AppName!" -ForegroundColor $Cyan