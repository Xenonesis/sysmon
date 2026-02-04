# System Monitor - Build Environment Setup Script
# This script helps set up the build environment and compile the application

param(
    [switch]$InstallMSVC,
    [switch]$InstallMSYS2,
    [switch]$CheckOnly
)

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host " System Monitor - Build Environment Setup" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host ""

# Function to check if running as administrator
function Test-Administrator {
    $currentUser = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    return $currentUser.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

# Function to check Rust installation
function Test-Rust {
    try {
        $rustVersion = & rustc --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✓ Rust is installed: $rustVersion" -ForegroundColor Green
            return $true
        }
    }
    catch {}
    Write-Host "✗ Rust is not installed" -ForegroundColor Red
    return $false
}

# Function to check MinGW64 installation
function Test-MinGW64 {
    $paths = @(
        "C:\msys64\mingw64\bin",
        "C:\mingw64\bin",
        "$env:ProgramFiles\mingw-w64\mingw64\bin"
    )
    
    foreach ($path in $paths) {
        if (Test-Path "$path\gcc.exe") {
            $gccVersion = & "$path\gcc.exe" --version 2>$null | Select-Object -First 1
            if ($gccVersion -match "x86_64") {
                Write-Host "✓ MinGW64 found: $path" -ForegroundColor Green
                Write-Host "  $gccVersion" -ForegroundColor Gray
                return $path
            }
        }
    }
    
    Write-Host "✗ MinGW64 (64-bit) not found" -ForegroundColor Yellow
    return $null
}

# Function to check MSVC Build Tools
function Test-MSVC {
    $vsPaths = @(
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC",
        "${env:ProgramFiles}\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC"
    )
    
    foreach ($path in $vsPaths) {
        if (Test-Path $path) {
            Write-Host "✓ Visual Studio Build Tools found" -ForegroundColor Green
            return $true
        }
    }
    
    Write-Host "✗ Visual Studio Build Tools not found" -ForegroundColor Yellow
    return $false
}

# Check current environment
Write-Host "Checking build environment..." -ForegroundColor Cyan
Write-Host ""

$hasRust = Test-Rust
$mingw64Path = Test-MinGW64
$hasMSVC = Test-MSVC

Write-Host ""

# Determine what to do
if ($CheckOnly) {
    Write-Host "Environment check complete." -ForegroundColor Cyan
    
    if ($hasRust -and ($mingw64Path -or $hasMSVC)) {
        Write-Host "✓ Your system is ready to build!" -ForegroundColor Green
        exit 0
    } else {
        Write-Host "✗ Additional setup required" -ForegroundColor Yellow
        exit 1
    }
}

# Install Rust if not present
if (-not $hasRust) {
    Write-Host "Rust is required. Please install it from:" -ForegroundColor Yellow
    Write-Host "https://rustup.rs/" -ForegroundColor White
    Write-Host ""
    $install = Read-Host "Open installation page in browser? (Y/N)"
    if ($install -eq "Y" -or $install -eq "y") {
        Start-Process "https://rustup.rs/"
    }
    Write-Host "Please run this script again after installing Rust." -ForegroundColor Yellow
    exit 1
}

# Decide on toolchain
if ($InstallMSVC) {
    Write-Host "Installing MSVC toolchain..." -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Visual Studio Build Tools are required." -ForegroundColor Yellow
    Write-Host "Download from: https://visualstudio.microsoft.com/downloads/" -ForegroundColor White
    Write-Host ""
    Write-Host "During installation, select:" -ForegroundColor Yellow
    Write-Host "  ► Desktop development with C++" -ForegroundColor White
    Write-Host ""
    $install = Read-Host "Open download page in browser? (Y/N)"
    if ($install -eq "Y" -or $install -eq "y") {
        Start-Process "https://visualstudio.microsoft.com/downloads/"
    }
    Write-Host ""
    Write-Host "After installation, run:" -ForegroundColor Cyan
    Write-Host "  rustup default stable-x86_64-pc-windows-msvc" -ForegroundColor White
    Write-Host "  cargo build --release" -ForegroundColor White
    exit 0
}
elseif ($InstallMSYS2) {
    Write-Host "Installing MSYS2/MinGW64 toolchain..." -ForegroundColor Cyan
    Write-Host ""
    Write-Host "MSYS2 is required for MinGW64." -ForegroundColor Yellow
    Write-Host "Download from: https://www.msys2.org/" -ForegroundColor White
    Write-Host ""
    Write-Host "After installation:" -ForegroundColor Yellow
    Write-Host "  1. Open MSYS2 MinGW 64-bit terminal" -ForegroundColor White
    Write-Host "  2. Run: pacman -Syu" -ForegroundColor White
    Write-Host "  3. Run: pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-toolchain" -ForegroundColor White
    Write-Host "  4. Add C:\msys64\mingw64\bin to your PATH" -ForegroundColor White
    Write-Host ""
    $install = Read-Host "Open download page in browser? (Y/N)"
    if ($install -eq "Y" -or $install -eq "y") {
        Start-Process "https://www.msys2.org/"
    }
    Write-Host ""
    Write-Host "After installation, run:" -ForegroundColor Cyan
    Write-Host "  rustup default stable-x86_64-pc-windows-gnu" -ForegroundColor White
    Write-Host "  cargo build --release" -ForegroundColor White
    exit 0
}

# Automatic detection and recommendation
Write-Host "Determining best build option..." -ForegroundColor Cyan
Write-Host ""

if ($hasMSVC) {
    Write-Host "✓ MSVC Build Tools detected!" -ForegroundColor Green
    Write-Host "Configuring Rust to use MSVC toolchain..." -ForegroundColor Cyan
    
    rustup default stable-x86_64-pc-windows-msvc
    
    Write-Host ""
    Write-Host "Building application..." -ForegroundColor Cyan
    cargo build --release
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✓ Build successful!" -ForegroundColor Green
        Write-Host "Executable: target\release\system-monitor.exe" -ForegroundColor White
        Write-Host ""
        Write-Host "You can now run:" -ForegroundColor Cyan
        Write-Host "  .\install.ps1  - to install the application" -ForegroundColor White
        Write-Host "  .\target\release\system-monitor.exe  - to run directly" -ForegroundColor White
    }
    else {
        Write-Host ""
        Write-Host "✗ Build failed!" -ForegroundColor Red
        Write-Host "Please check the errors above." -ForegroundColor Yellow
        exit 1
    }
}
elseif ($mingw64Path) {
    Write-Host "✓ MinGW64 detected!" -ForegroundColor Green
    Write-Host "Adding MinGW64 to PATH for this session..." -ForegroundColor Cyan
    
    $env:PATH = "$mingw64Path;$env:PATH"
    rustup default stable-x86_64-pc-windows-gnu
    
    Write-Host ""
    Write-Host "Building application..." -ForegroundColor Cyan
    cargo build --release
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✓ Build successful!" -ForegroundColor Green
        Write-Host "Executable: target\release\system-monitor.exe" -ForegroundColor White
        Write-Host ""
        Write-Host "You can now run:" -ForegroundColor Cyan
        Write-Host "  .\install.ps1  - to install the application" -ForegroundColor White
        Write-Host "  .\target\release\system-monitor.exe  - to run directly" -ForegroundColor White
    }
    else {
        Write-Host ""
        Write-Host "✗ Build failed!" -ForegroundColor Red
        Write-Host "Please check the errors above." -ForegroundColor Yellow
        exit 1
    }
}
else {
    Write-Host "No suitable build tools found." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Please choose an option:" -ForegroundColor Cyan
    Write-Host "  1. Install Visual Studio Build Tools (Recommended)" -ForegroundColor White
    Write-Host "  2. Install MSYS2/MinGW64" -ForegroundColor White
    Write-Host "  3. Cancel" -ForegroundColor White
    Write-Host ""
    $choice = Read-Host "Enter choice (1-3)"
    
    switch ($choice) {
        "1" {
            & $PSCommandPath -InstallMSVC
        }
        "2" {
            & $PSCommandPath -InstallMSYS2
        }
        default {
            Write-Host "Setup cancelled." -ForegroundColor Yellow
            exit 0
        }
    }
}
