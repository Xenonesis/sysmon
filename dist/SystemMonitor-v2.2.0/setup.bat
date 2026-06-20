@echo off
REM System Monitor Installer
REM This batch file provides a clean installer experience

title System Monitor Setup
echo.
echo ============================================
echo    System Monitor Setup
echo ============================================
echo.

REM Check if running as administrator (optional)
net session >nul 2>&1
if %errorLevel% == 0 (
    echo Administrator privileges detected.
) else (
    echo Note: Administrator privileges not detected.
    echo Installation will be for current user only.
)
echo.

REM Run the PowerShell installer
powershell.exe -ExecutionPolicy Bypass -File "%~dp0installer.ps1"

REM Check if installation was successful
if %errorLevel% == 0 (
    echo.
    echo ============================================
    echo    Installation Complete!
    echo ============================================
    echo.
    echo System Monitor has been installed successfully.
    echo.
    echo You can now:
    echo   - Launch from Start Menu
    echo   - Use the desktop shortcut (if created)
    echo   - Search for "System Monitor" in Windows search
    echo.
    pause
) else (
    echo.
    echo ============================================
    echo    Installation Failed
    echo ============================================
    echo.
    echo Please check the error messages above.
    echo.
    pause
)