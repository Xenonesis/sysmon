[CmdletBinding()]
param(
    [Parameter()]
    [string]$ExePath = "target\release\system-monitor.exe",

    [Parameter()]
    [ValidateRange(1, 60)]
    [int]$WarmupSeconds = 8
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "Screenshot capture requires Windows."
}

$resolvedExe = (Resolve-Path -LiteralPath $ExePath).Path
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$primaryPath = Join-Path $repositoryRoot "assets\screenshot.png"
$docsPath = Join-Path $repositoryRoot "docs\assets\screenshot.png"
$temporaryPath = Join-Path ([System.IO.Path]::GetTempPath()) "sysmon-screenshot-$PID.png"

$existing = Get-Process -Name "system-monitor" -ErrorAction SilentlyContinue
if ($existing) {
    throw "A system-monitor process is already running. Close it before capturing a clean screenshot."
}

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class SysMonCaptureNative {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int command);

    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(IntPtr hWnd, IntPtr insertAfter, int x, int y, int width, int height, uint flags);

    [DllImport("user32.dll")]
    public static extern int GetSystemMetrics(int index);

    [DllImport("user32.dll")]
    public static extern bool SetProcessDPIAware();

    public static IntPtr FindLargestVisibleWindow(int processId) {
        IntPtr largest = IntPtr.Zero;
        long largestArea = 0;
        EnumWindows(delegate(IntPtr hWnd, IntPtr lParam) {
            uint ownerProcessId;
            GetWindowThreadProcessId(hWnd, out ownerProcessId);
            if (ownerProcessId != (uint)processId || !IsWindowVisible(hWnd)) {
                return true;
            }

            RECT rect;
            if (!GetWindowRect(hWnd, out rect)) {
                return true;
            }
            long width = Math.Max(0, rect.Right - rect.Left);
            long height = Math.Max(0, rect.Bottom - rect.Top);
            long area = width * height;
            if (area > largestArea) {
                largest = hWnd;
                largestArea = area;
            }
            return true;
        }, IntPtr.Zero);
        return largest;
    }
}
"@
[SysMonCaptureNative]::SetProcessDPIAware() | Out-Null

$process = $null
try {
    $process = Start-Process -FilePath $resolvedExe -PassThru
    $deadline = (Get-Date).AddSeconds(30)
    $windowHandle = [IntPtr]::Zero
    $initialRect = New-Object SysMonCaptureNative+RECT
    do {
        Start-Sleep -Milliseconds 500
        $process.Refresh()
        $windowHandle = [SysMonCaptureNative]::FindLargestVisibleWindow($process.Id)
        $hasRect = $windowHandle -ne [IntPtr]::Zero -and
            [SysMonCaptureNative]::GetWindowRect($windowHandle, [ref]$initialRect)
        $initialWidth = $initialRect.Right - $initialRect.Left
        $initialHeight = $initialRect.Bottom - $initialRect.Top
    } while (
        -not $process.HasExited -and
        (-not $hasRect -or $initialWidth -lt 900 -or $initialHeight -lt 600) -and
        (Get-Date) -lt $deadline
    )

    if ($process.HasExited) {
        throw "SysMon exited before capture with code $($process.ExitCode)."
    }
    if (-not $hasRect -or $initialWidth -lt 900 -or $initialHeight -lt 600) {
        throw "SysMon did not expose capture-ready window bounds within 30 seconds."
    }

    [SysMonCaptureNative]::ShowWindow($windowHandle, 9) | Out-Null
    $screenWidth = [SysMonCaptureNative]::GetSystemMetrics(0)
    $screenHeight = [SysMonCaptureNative]::GetSystemMetrics(1)
    $fitScale = [Math]::Min(1.0, [Math]::Min($screenWidth / $initialWidth, $screenHeight / $initialHeight))
    $targetWidth = [Math]::Floor($initialWidth * $fitScale)
    $targetHeight = [Math]::Floor($initialHeight * $fitScale)
    if (-not [SysMonCaptureNative]::SetWindowPos(
        $windowHandle,
        [IntPtr]::Zero,
        0,
        0,
        $targetWidth,
        $targetHeight,
        0x0040
    )) {
        throw "Could not position the SysMon window inside the visible desktop."
    }
    [SysMonCaptureNative]::SetForegroundWindow($windowHandle) | Out-Null
    Start-Sleep -Seconds $WarmupSeconds
    $process.Refresh()

    $rect = New-Object SysMonCaptureNative+RECT
    if (-not [SysMonCaptureNative]::GetWindowRect($windowHandle, [ref]$rect)) {
        throw "Could not read the SysMon window bounds."
    }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -lt 900 -or $height -lt 600) {
        throw "Unexpected SysMon window size ${width}x${height}."
    }

    $bitmap = New-Object System.Drawing.Bitmap($width, $height)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
        }
        finally {
            $graphics.Dispose()
        }
        $outputBitmap = New-Object System.Drawing.Bitmap(1115, 837)
        try {
            $outputGraphics = [System.Drawing.Graphics]::FromImage($outputBitmap)
            try {
                $outputGraphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                $outputGraphics.DrawImage($bitmap, 0, 0, 1115, 837)
            }
            finally {
                $outputGraphics.Dispose()
            }
            $outputBitmap.Save($temporaryPath, [System.Drawing.Imaging.ImageFormat]::Png)
        }
        finally {
            $outputBitmap.Dispose()
        }
    }
    finally {
        $bitmap.Dispose()
    }

    if ((Get-Item -LiteralPath $temporaryPath).Length -lt 10000) {
        throw "Captured screenshot is unexpectedly small; existing assets were preserved."
    }
    Copy-Item -LiteralPath $temporaryPath -Destination $primaryPath -Force
    Copy-Item -LiteralPath $temporaryPath -Destination $docsPath -Force
    Write-Host "Updated screenshots (${width}x${height} capture -> 1115x837 asset): $primaryPath and $docsPath"
}
finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
    }
    Remove-Item -LiteralPath $temporaryPath -Force -ErrorAction SilentlyContinue
}
