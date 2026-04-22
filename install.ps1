$ErrorActionPreference = "Stop"
$appName = "System Monitor"
$exeName = "system-monitor.exe"
$installDir = "$env:LOCALAPPDATA\Programs\SystemMonitor"
$exePath = "$installDir\$exeName"

Write-Host "Installing $appName..."

# Create installation directory
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
}

# Copy executable
$sourceExe = ".\target\release\$exeName"
if (-not (Test-Path $sourceExe)) {
    Write-Error "Release build not found! Please run 'cargo build --release' first."
    exit 1
}
Copy-Item -Path $sourceExe -Destination $exePath -Force
Write-Host "Copied executable to $exePath"

# Create Start Menu Shortcut
$WshShell = New-Object -ComObject WScript.Shell
$startMenuPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\$appName.lnk"
$shortcut = $WshShell.CreateShortcut($startMenuPath)
$shortcut.TargetPath = $exePath
$shortcut.WorkingDirectory = $installDir
$shortcut.IconLocation = "$exePath, 0"
$shortcut.Save()
Write-Host "Created Start Menu shortcut at $startMenuPath"

# Create Desktop Shortcut
$desktopPath = [System.Environment]::GetFolderPath('Desktop')
$desktopShortcutPath = "$desktopPath\$appName.lnk"
$desktopShortcut = $WshShell.CreateShortcut($desktopShortcutPath)
$desktopShortcut.TargetPath = $exePath
$desktopShortcut.WorkingDirectory = $installDir
$desktopShortcut.IconLocation = "$exePath, 0"
$desktopShortcut.Save()
Write-Host "Created Desktop shortcut at $desktopShortcutPath"

Write-Host "`nInstallation complete! You can now launch System Monitor from your Desktop or Start Menu."
