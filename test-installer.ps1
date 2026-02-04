# Test the installer automatically
Push-Location "dist\test-extract"

# Start the installer process
$process = Start-Process ".\installer.ps1" -PassThru -NoNewWindow

# Wait a moment for the installer to start
Start-Sleep -Seconds 2

# Send "Y" followed by Enter
[System.Windows.Forms.SendKeys]::SendWait("Y{ENTER}")

# Wait for installer to complete
$process.WaitForExit()

Write-Host "Installer exited with code: $($process.ExitCode)"

Pop-Location