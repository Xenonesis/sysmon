# System Monitor - Distribution Builder
# Creates a complete installer package for distribution

param(
    [switch]$Clean,
    [switch]$NoZip
)

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "   System Monitor - Distribution Builder" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host ""

$ErrorActionPreference = "Stop"

# Configuration
$AppName = "SystemMonitor"
$Version = "1.0.0"
$DistDir = "dist"
$BuildDir = "$DistDir\$AppName-v$Version"

function Write-Step {
    param([string]$Message)
    Write-Host "→ $Message" -ForegroundColor White
}

function Write-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-Error {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

# Clean previous builds
if ($Clean) {
    Write-Step "Cleaning previous builds..."
    if (Test-Path $DistDir) {
        Remove-Item $DistDir -Recurse -Force
    }
    Write-Success "Cleaned previous builds"
}

# Create distribution directory
Write-Step "Creating distribution directory..."
New-Item -ItemType Directory -Path $BuildDir -Force | Out-Null
Write-Success "Created distribution directory: $BuildDir"

# Build the application
Write-Step "Building application..."
try {
    & ".\build.ps1" -NoLaunch
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed with exit code $LASTEXITCODE"
    }
}
catch {
    Write-Error "Build failed: $($_.Exception.Message)"
    exit 1
}
Write-Success "Application built successfully"

# Copy application files
Write-Step "Copying application files..."
Copy-Item "target\release\system-monitor.exe" "$BuildDir\" -Force
Write-Success "Copied executable"

# Copy installer files
Write-Step "Copying installer files..."
$installerFiles = @(
    "installer.ps1",
    "setup.bat",
    "LICENSE",
    "README.md",
    "USER_GUIDE.md"
)

foreach ($file in $installerFiles) {
    if (Test-Path $file) {
        Copy-Item $file $BuildDir -Force
    }
}
Write-Success "Copied installer files"

# Create additional documentation
Write-Step "Creating installation instructions..."
$installInstructions = @"
System Monitor Installation Instructions
=======================================

Quick Installation:
1. Double-click "setup.bat"
2. Follow the installation wizard
3. Launch System Monitor from Start Menu or Desktop

Manual Installation:
1. Run: PowerShell -ExecutionPolicy Bypass -File installer.ps1
2. Or run: .\installer.ps1 from PowerShell

Uninstallation:
- Use Windows Settings → Apps → System Monitor → Uninstall
- Or run: .\installer.ps1 -Uninstall

For more information, see README.md and USER_GUIDE.md

System Requirements:
- Windows 10 or later
- No additional dependencies required

Contact: https://github.com/Xenonesis/sysmon
"@

$installInstructions | Out-File "$BuildDir\INSTALL.txt" -Encoding UTF8
Write-Success "Created installation instructions"

# Create version info
Write-Step "Creating version information..."
$versionInfo = @"
System Monitor v$Version
Built on: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
Repository: https://github.com/Xenonesis/sysmon
"@

$versionInfo | Out-File "$BuildDir\VERSION.txt" -Encoding UTF8
Write-Success "Created version information"

# Create ZIP archive (optional)
if (-not $NoZip) {
    Write-Step "Creating ZIP archive..."
    $zipPath = "$DistDir\$AppName-v$Version.zip"
    if (Test-Path $zipPath) {
        Remove-Item $zipPath -Force
    }

    # Create ZIP using PowerShell
    Compress-Archive -Path "$BuildDir\*" -DestinationPath $zipPath -Force
    Write-Success "Created ZIP archive: $zipPath"
}

# Show summary
Write-Host ""
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "   Distribution Package Created!" -ForegroundColor Green
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Distribution contents:" -ForegroundColor Cyan
Get-ChildItem $BuildDir | ForEach-Object {
    Write-Host "  • $($_.Name)" -ForegroundColor White
}
Write-Host ""
Write-Host "Installation methods:" -ForegroundColor Cyan
Write-Host "  1. Double-click: $BuildDir\setup.bat" -ForegroundColor White
Write-Host "  2. PowerShell: $BuildDir\installer.ps1" -ForegroundColor White
Write-Host "  3. Silent install: $BuildDir\installer.ps1 -Silent" -ForegroundColor White
Write-Host ""

if (-not $NoZip) {
    $zipSize = (Get-Item "$DistDir\$AppName-v$Version.zip").Length / 1MB
    Write-Host "ZIP Archive:" -ForegroundColor Cyan
    Write-Host "  • Location: $DistDir\$AppName-v$Version.zip" -ForegroundColor White
    Write-Host "  • Size: $([math]::Round($zipSize, 2)) MB" -ForegroundColor White
    Write-Host ""
}

Write-Host "Ready for distribution! Users can now install by running setup.bat" -ForegroundColor Green
Write-Host ""

# Test the installer
$test = Read-Host "Test the installer now? (Y/N)"
if ($test -eq "Y" -or $test -eq "y") {
    Write-Host ""
    Write-Step "Testing installer..."
    Push-Location $BuildDir
    try {
        & ".\installer.ps1" -Silent
        Write-Success "Installer test completed"
    }
    finally {
        Pop-Location
    }
}