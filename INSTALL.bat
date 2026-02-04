@echo off
echo ============================================
echo    System Monitor - Quick Installer
echo ============================================
echo.
echo This will install System Monitor on your computer.
echo.
pause

PowerShell.exe -ExecutionPolicy Bypass -File "%~dp0one-click-install.ps1"

pause
