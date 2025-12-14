# System Monitor - Build Script
# This script builds the GUI System Monitor application

Write-Host "Building System Monitor GUI Application..." -ForegroundColor Cyan
Write-Host ""

# Check if Rust is installed
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Error: Rust/Cargo not found!" -ForegroundColor Red
    Write-Host "Please install Rust from: https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

Write-Host "Building release version (optimized)..." -ForegroundColor Green
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "Build successful!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Executable location: target\release\system-monitor.exe" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "You can now:" -ForegroundColor Cyan
    Write-Host "  1. Run it directly: .\target\release\system-monitor.exe" -ForegroundColor White
    Write-Host "  2. Copy it anywhere on your system" -ForegroundColor White
    Write-Host "  3. Create a desktop shortcut" -ForegroundColor White
    Write-Host ""
    
    $run = Read-Host "Would you like to run the application now? (Y/N)"
    if ($run -eq "Y" -or $run -eq "y") {
        Write-Host "Starting System Monitor..." -ForegroundColor Green
        Start-Process "target\release\system-monitor.exe"
    }
} else {
    Write-Host ""
    Write-Host "Build failed!" -ForegroundColor Red
    Write-Host "Please check the error messages above." -ForegroundColor Yellow
    exit 1
}
