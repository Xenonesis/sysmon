# System Monitor - Complete Build Environment Setup
# This script installs all required tools for building the System Monitor application

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "   System Monitor - Build Setup" -ForegroundColor Cyan
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

# Function to check MSVC Build Tools
function Test-MSVC {
    $vsPaths = @(
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC"
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

# Function to install Rust
function Install-Rust {
    Write-Host "Installing Rust..." -ForegroundColor Cyan
    Write-Host ""

    try {
        # Download and run rustup installer
        $installerUrl = "https://win.rustup.rs/x86_64"
        $installerPath = "$env:TEMP\rustup-init.exe"

        Write-Host "Downloading Rust installer..." -ForegroundColor Gray
        Invoke-WebRequest -Uri $installerUrl -OutFile $installerPath -UseBasicParsing

        Write-Host "Running Rust installer..." -ForegroundColor Gray
        Write-Host "(This will open a new window - follow the prompts)" -ForegroundColor Yellow
        Write-Host ""

        Start-Process -FilePath $installerPath -Wait

        # Refresh environment
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")

        # Verify installation
        if (Test-Rust) {
            Write-Host "✓ Rust installed successfully!" -ForegroundColor Green
            return $true
        } else {
            Write-Host "✗ Rust installation failed" -ForegroundColor Red
            return $false
        }
    }
    catch {
        Write-Host "✗ Failed to install Rust: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Function to install Visual Studio Build Tools
function Install-MSVC {
    Write-Host "Installing Visual Studio Build Tools..." -ForegroundColor Cyan
    Write-Host ""

    try {
        $installerUrl = "https://aka.ms/vs/17/release/vs_buildtools.exe"
        $installerPath = "$env:TEMP\vs_buildtools.exe"

        Write-Host "Downloading Visual Studio Build Tools..." -ForegroundColor Gray
        Write-Host "(This may take a few minutes)" -ForegroundColor Gray
        Invoke-WebRequest -Uri $installerUrl -OutFile $installerPath -UseBasicParsing

        Write-Host "Installing Visual Studio Build Tools..." -ForegroundColor Gray
        Write-Host "(This will open the installer - select 'Desktop development with C++')" -ForegroundColor Yellow
        Write-Host ""

        # Run installer with specific workload
        $args = "--quiet", "--wait", "--norestart", "--nocache", "--installPath", "`"${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools`"", "--add", "Microsoft.VisualStudio.Workload.VCTools", "--includeRecommended"

        $process = Start-Process -FilePath $installerPath -ArgumentList $args -Wait -PassThru

        if ($process.ExitCode -eq 0 -or $process.ExitCode -eq 3010) {
            Write-Host "✓ Visual Studio Build Tools installed successfully!" -ForegroundColor Green
            return $true
        } else {
            Write-Host "✗ Visual Studio Build Tools installation failed (Exit code: $($process.ExitCode))" -ForegroundColor Red
            return $false
        }
    }
    catch {
        Write-Host "✗ Failed to install Visual Studio Build Tools: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Main setup logic
Write-Host "Checking current environment..." -ForegroundColor Cyan
Write-Host ""

$hasRust = Test-Rust
$hasMSVC = Test-MSVC

Write-Host ""

if ($hasRust -and $hasMSVC) {
    Write-Host "✓ All required tools are already installed!" -ForegroundColor Green
    Write-Host ""
    Write-Host "You can now build the application:" -ForegroundColor Cyan
    Write-Host "  .\build.ps1" -ForegroundColor White
    Write-Host ""
    Write-Host "Or use the one-click installer:" -ForegroundColor Cyan
    Write-Host "  .\one-click-install.ps1" -ForegroundColor White
    exit 0
}

# Install missing components
$installRust = -not $hasRust
$installMSVC = -not $hasMSVC

if ($installRust) {
    Write-Host "Rust needs to be installed." -ForegroundColor Yellow
}
if ($installMSVC) {
    Write-Host "Visual Studio Build Tools need to be installed." -ForegroundColor Yellow
}

Write-Host ""
$proceed = Read-Host "Continue with installation? (Y/N)"
if ($proceed -ne "Y" -and $proceed -ne "y") {
    Write-Host "Installation cancelled." -ForegroundColor Yellow
    exit 0
}

Write-Host ""

# Install Rust first
if ($installRust) {
    if (-not (Install-Rust)) {
        Write-Host ""
        Write-Host "Failed to install Rust. Please install it manually from:" -ForegroundColor Red
        Write-Host "https://rustup.rs/" -ForegroundColor White
        exit 1
    }
    Write-Host ""
}

# Install MSVC Build Tools
if ($installMSVC) {
    if (-not (Install-MSVC)) {
        Write-Host ""
        Write-Host "Failed to install Visual Studio Build Tools. Please install manually:" -ForegroundColor Red
        Write-Host "1. Download from: https://visualstudio.microsoft.com/downloads/" -ForegroundColor White
        Write-Host "2. Select 'Build Tools for Visual Studio'" -ForegroundColor White
        Write-Host "3. Install 'Desktop development with C++' workload" -ForegroundColor White
        exit 1
    }
    Write-Host ""
}

# Configure Rust toolchain
Write-Host "Configuring Rust toolchain..." -ForegroundColor Cyan
rustup default stable-x86_64-pc-windows-msvc
Write-Host "✓ Rust configured to use MSVC toolchain" -ForegroundColor Green
Write-Host ""

# Final verification
Write-Host "Verifying installation..." -ForegroundColor Cyan
Write-Host ""

$finalRustCheck = Test-Rust
$finalMSVCCheck = Test-MSVC

if ($finalRustCheck -and $finalMSVCCheck) {
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host "   Setup Complete!" -ForegroundColor Green
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "All required tools are now installed!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Cyan
    Write-Host "  1. Build the application: .\build.ps1" -ForegroundColor White
    Write-Host "  2. Install the app: .\install.ps1" -ForegroundColor White
    Write-Host "  3. Or use one-click: .\one-click-install.ps1" -ForegroundColor White
    Write-Host ""
    Write-Host "You can now run the System Monitor application!" -ForegroundColor Green
} else {
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host "   Setup Incomplete" -ForegroundColor Red
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Some components may not have installed correctly." -ForegroundColor Yellow
    Write-Host "Please check the error messages above and try again." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "For manual installation help, see:" -ForegroundColor Cyan
    Write-Host "  SETUP_GUIDE.md" -ForegroundColor White
    exit 1
}