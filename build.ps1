# System Monitor - Build Script
# This script builds the GUI System Monitor application

param(
    [switch]$Help,
    [switch]$NoLaunch
)

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "   System Monitor - Build Script" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host ""

if ($Help) {
    Write-Host "Usage: .\build.ps1 [options]" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Options:" -ForegroundColor Cyan
    Write-Host "  -Help         Show this help message" -ForegroundColor White
    Write-Host ""
    Write-Host "This script builds the System Monitor application." -ForegroundColor White
    Write-Host ""
    Write-Host "Requirements:" -ForegroundColor Cyan
    Write-Host "  • Rust toolchain (rustup)" -ForegroundColor White
    Write-Host "  • Visual Studio Build Tools (MSVC toolchain)" -ForegroundColor White
    Write-Host "  • MinGW conflicts automatically resolved" -ForegroundColor White
    Write-Host ""
    Write-Host "Important Notes:" -ForegroundColor Yellow
    Write-Host "  • Build requires MSVC toolchain (not MinGW)" -ForegroundColor White
    Write-Host "  • This script automatically handles MinGW conflicts" -ForegroundColor White
    Write-Host "  • Application runs as a native Windows GUI" -ForegroundColor White
    Write-Host ""
    Write-Host "For setup help, run:" -ForegroundColor Cyan
    Write-Host "  .\setup-build-environment.ps1" -ForegroundColor White
    Write-Host ""
    Write-Host "Or see SETUP_GUIDE.md for detailed instructions." -ForegroundColor White
    exit 0
}

# Check if Rust is installed
Write-Host "Checking Rust installation..." -ForegroundColor Cyan
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "✗ Rust/Cargo not found!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Rust is required to build this application." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Install from: https://rustup.rs/" -ForegroundColor White
    Write-Host ""
    $open = Read-Host "Open installation page? (Y/N)"
    if ($open -eq "Y" -or $open -eq "y") {
        Start-Process "https://rustup.rs/"
    }
    exit 1
}

$rustVersion = & rustc --version
Write-Host "✓ Rust found: $rustVersion" -ForegroundColor Green
Write-Host ""

# Check active toolchain
$toolchain = & rustup show active-toolchain
Write-Host "Active toolchain: $toolchain" -ForegroundColor Cyan

# Check for MinGW in PATH (can cause conflicts with MSVC)
$mingwInPath = $env:PATH -split ';' | Where-Object { $_ -like '*mingw*' -or $_ -like '*MinGW*' }
if ($mingwInPath) {
    Write-Host "⚠️  MinGW detected in PATH - this can cause build conflicts!" -ForegroundColor Yellow
    Write-Host "   Temporarily removing MinGW from PATH for this build..." -ForegroundColor Gray
    $originalPath = $env:PATH
    $env:PATH = ($env:PATH -split ';' | Where-Object { $_ -notlike '*mingw*' -and $_ -notlike '*MinGW*' }) -join ';'
    Write-Host "✓ PATH cleaned for MSVC build" -ForegroundColor Green
    Write-Host ""
}

# Verify MSVC toolchain is active
if ($toolchain -notlike "*msvc*") {
    Write-Host "⚠️  Non-MSVC toolchain detected!" -ForegroundColor Yellow
    Write-Host "   Switching to MSVC toolchain for Windows builds..." -ForegroundColor Gray
    rustup default stable-x86_64-pc-windows-msvc
    Write-Host "✓ Switched to MSVC toolchain" -ForegroundColor Green
    Write-Host ""
}

# Stop any running instances
Write-Host "Stopping any running instances..." -ForegroundColor Cyan
Stop-Process -Name "system-monitor" -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# Build
Write-Host "Building release version (optimized)..." -ForegroundColor Green
Write-Host "This may take a few minutes on first build..." -ForegroundColor Gray
Write-Host ""

$buildStart = Get-Date
cargo build --release
$buildEnd = Get-Date
$buildTime = ($buildEnd - $buildStart).TotalSeconds

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host "   Build Successful!" -ForegroundColor Green
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Build time: $([math]::Round($buildTime, 1)) seconds" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Executable: target\release\system-monitor.exe" -ForegroundColor Yellow
    
    # Get file size
    $exeSize = (Get-Item "target\release\system-monitor.exe").Length / 1MB
    Write-Host "Size: $([math]::Round($exeSize, 2)) MB" -ForegroundColor Gray
    Write-Host ""
    
    # ── Enterprise Metadata Verification ──
    Write-Host "═══════════════════════════════════════════════" -ForegroundColor Magenta
    Write-Host "   Enterprise EXE Metadata" -ForegroundColor Magenta
    Write-Host "═══════════════════════════════════════════════" -ForegroundColor Magenta
    $versionInfo = (Get-Item "target\release\system-monitor.exe").VersionInfo
    Write-Host "  Company:     $($versionInfo.CompanyName)" -ForegroundColor White
    Write-Host "  Product:     $($versionInfo.ProductName)" -ForegroundColor White
    Write-Host "  Description: $($versionInfo.FileDescription)" -ForegroundColor White
    Write-Host "  Version:     $($versionInfo.FileVersion)" -ForegroundColor White
    Write-Host "  Copyright:   $($versionInfo.LegalCopyright)" -ForegroundColor White
    Write-Host ""
    
    # ── SHA256 Hash (Audit Trail) ──
    $hash = (Get-FileHash "target\release\system-monitor.exe" -Algorithm SHA256).Hash
    Write-Host "  SHA256: $hash" -ForegroundColor DarkGray
    Write-Host "═══════════════════════════════════════════════" -ForegroundColor Magenta
    Write-Host ""
    
    # Create downloads folder and copy build
    Write-Host "Saving build to downloads folder..." -ForegroundColor Cyan
    $downloadsFolder = "downloads"
    $docsDownloadsFolder = "docs\downloads"
    if (-not (Test-Path $downloadsFolder)) {
        New-Item -ItemType Directory -Path $downloadsFolder -Force | Out-Null
    }
    if (-not (Test-Path $docsDownloadsFolder)) {
        New-Item -ItemType Directory -Path $docsDownloadsFolder -Force | Out-Null
    }
    
    # Get version from Cargo.toml
    $cargoToml = Get-Content "Cargo.toml" -Raw
    if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
        $version = $matches[1]
    } else {
        $version = "1.0.0"
    }

    # ── Code Signing (Main EXE) ──
    if (Test-Path "sign-binary.ps1") {
        Write-Host "Signing executable..." -ForegroundColor Cyan
        & .\sign-binary.ps1 -FilePath "target\release\system-monitor.exe"
    }

    # ── Compile Installer (Inno Setup) ──
    $isccPath = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
    $installerExists = $false
    if (Test-Path $isccPath) {
        Write-Host "Inno Setup found. Compiling installer..." -ForegroundColor Cyan
        & $isccPath "installer.iss" | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Installer SystemMonitor-Setup.exe compiled to downloads folder." -ForegroundColor Green
            $installerExists = $true
            # Sign the installer
            if (Test-Path "sign-binary.ps1") {
                Write-Host "Signing installer..." -ForegroundColor Cyan
                & .\sign-binary.ps1 -FilePath "downloads\SystemMonitor-Setup.exe"
            }
            # Copy versioned installer
            Copy-Item "downloads\SystemMonitor-Setup.exe" "downloads\SystemMonitor-Setup-v$version.exe" -Force
            Copy-Item "downloads\SystemMonitor-Setup.exe" "docs\downloads\SystemMonitor-Setup-v$version.exe" -Force
            Copy-Item "downloads\SystemMonitor-Setup.exe" "docs\downloads\SystemMonitor-Setup.exe" -Force
        } else {
            Write-Host "❌ Failed to compile installer." -ForegroundColor Red
        }
    } else {
        Write-Host "ℹ️ Inno Setup (ISCC.exe) not found. Skipping installer compilation." -ForegroundColor Yellow
        Write-Host "   Install Inno Setup 6 if you want to build the setup wizard." -ForegroundColor DarkGray
    }
    
    # Copy executable to downloads folders with version naming
    $versionedName = "SystemMonitor-v$version.exe"
    $latestName = "SystemMonitor-latest.exe"
    
    # Remove old versions, keep only .gitkeep
    Get-ChildItem "$downloadsFolder\*.exe" -ErrorAction SilentlyContinue | Remove-Item -Force
    Get-ChildItem "$docsDownloadsFolder\*.exe" -ErrorAction SilentlyContinue | Remove-Item -Force
    
    # Root downloads folder
    Copy-Item "target\release\system-monitor.exe" "$downloadsFolder\$versionedName" -Force
    Copy-Item "target\release\system-monitor.exe" "$downloadsFolder\$latestName" -Force
    
    # Docs downloads folder (for GitHub Pages)
    Copy-Item "target\release\system-monitor.exe" "$docsDownloadsFolder\$versionedName" -Force
    Copy-Item "target\release\system-monitor.exe" "$docsDownloadsFolder\$latestName" -Force
    
    Write-Host "✓ Build saved to downloads folders:" -ForegroundColor Green
    Write-Host "  • $downloadsFolder\$versionedName" -ForegroundColor White
    Write-Host "  • $downloadsFolder\$latestName (latest)" -ForegroundColor White
    Write-Host "  • $docsDownloadsFolder\$versionedName (GitHub Pages)" -ForegroundColor White
    Write-Host "  • $docsDownloadsFolder\$latestName (GitHub Pages)" -ForegroundColor White
    Write-Host ""
    
    Write-Host "Next steps:" -ForegroundColor Cyan
    Write-Host "  1. Run: .\target\release\system-monitor.exe" -ForegroundColor White
    Write-Host "  2. Install: .\install.ps1" -ForegroundColor White
    Write-Host "  3. Or use: .\one-click-install.ps1" -ForegroundColor White
    Write-Host ""
    
    if (-not $NoLaunch) {
        $run = Read-Host "Launch System Monitor now? (Y/N)"
        if ($run -eq "Y" -or $run -eq "y") {
            Write-Host ""
            Write-Host "Starting System Monitor..." -ForegroundColor Green
            Start-Process "target\release\system-monitor.exe"
        }
    }
    
    Write-Host ""
    Write-Host "Build completed successfully!" -ForegroundColor Green
    
    # Restore original PATH if it was modified
    if ($mingwInPath) {
        $env:PATH = $originalPath
        Write-Host "✓ PATH restored" -ForegroundColor Gray
    }
} else {
    # Restore original PATH if it was modified
    if ($mingwInPath) {
        $env:PATH = $originalPath
        Write-Host "✓ PATH restored" -ForegroundColor Gray
    }
    
    Write-Host ""
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host "   Build Failed!" -ForegroundColor Red
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Common issues:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "1. Missing build tools:" -ForegroundColor Cyan
    Write-Host "   Run: .\setup-build-environment.ps1" -ForegroundColor White
    Write-Host ""
    Write-Host "2. Wrong toolchain:" -ForegroundColor Cyan
    Write-Host "   For MSVC: rustup default stable-x86_64-pc-windows-msvc" -ForegroundColor White
    Write-Host "   For GNU:  rustup default stable-x86_64-pc-windows-gnu" -ForegroundColor White
    Write-Host ""
    Write-Host "3. MinGW conflict:" -ForegroundColor Cyan
    Write-Host "   MinGW in PATH can cause MSVC build failures" -ForegroundColor White
    Write-Host "   This script automatically handles this" -ForegroundColor White
    Write-Host ""
    Write-Host "4. See full setup guide:" -ForegroundColor Cyan
    Write-Host "   Open SETUP_GUIDE.md" -ForegroundColor White
    Write-Host ""
    
    if (-not $NoLaunch) {
        $help = Read-Host "Run build environment setup? (Y/N)"
        if ($help -eq "Y" -or $help -eq "y") {
            Write-Host ""
            .\setup-build-environment.ps1
        }
    }
    
    exit 1
}
