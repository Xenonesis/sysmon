@echo off
echo ============================================
echo    System Monitor - Quick Builder
echo ============================================
echo.
echo This will build System Monitor from source.
echo.
echo Requirements:
echo - Rust (rustup)
echo - Visual Studio Build Tools OR MSYS2
echo.
pause

PowerShell.exe -ExecutionPolicy Bypass -File "%~dp0build.ps1"

pause
