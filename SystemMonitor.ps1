# System Monitor - PowerShell Edition
# Real-time CPU, RAM, and GPU monitoring
# Usage: .\SystemMonitor.ps1 [-RefreshInterval 2] [-TopProcessCount 15]

param(
    [int]$RefreshInterval = 2,
    [int]$TopProcessCount = 15
)

function Get-ColorFromPercentage {
    param([float]$Percentage)
    if ($Percentage -lt 50) { return "Green" }
    elseif ($Percentage -lt 75) { return "Yellow" }
    else { return "Red" }
}

function Draw-ProgressBar {
    param(
        [float]$Percentage,
        [int]$Width = 60
    )
    $filled = [math]::Floor(($Percentage / 100) * $Width)
    $empty = $Width - $filled
    $color = Get-ColorFromPercentage -Percentage $Percentage
    
    $bar = ("█" * $filled) + ("░" * $empty)
    Write-Host $bar -ForegroundColor $color
}

function Get-GPUInfo {
    try {
        # Try to get NVIDIA GPU info using nvidia-smi
        $nvidiaSmi = "C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe"
        if (Test-Path $nvidiaSmi) {
            $output = & $nvidiaSmi --query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu --format=csv,noheader,nounits 2>$null
            if ($output) {
                $parts = $output -split ","
                return @{
                    Name = $parts[0].Trim()
                    Utilization = [float]$parts[1].Trim()
                    MemoryUsed = [float]$parts[2].Trim()
                    MemoryTotal = [float]$parts[3].Trim()
                    Temperature = [int]$parts[4].Trim()
                    Available = $true
                }
            }
        }
    } catch {
        # GPU monitoring not available
    }
    return @{ Available = $false }
}

function Show-SystemMonitor {
    Clear-Host
    
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    
    # Header
    Write-Host ("=" * 75) -ForegroundColor Cyan
    Write-Host "SYSTEM MONITOR " -ForegroundColor Cyan -NoNewline
    Write-Host "[$timestamp]" -ForegroundColor Gray
    Write-Host ("=" * 75) -ForegroundColor Cyan
    Write-Host ""
    
    # Memory Information
    $computerInfo = Get-CimInstance -ClassName Win32_OperatingSystem
    $totalRAM = [math]::Round($computerInfo.TotalVisibleMemorySize / 1MB, 2)
    $freeRAM = [math]::Round($computerInfo.FreePhysicalMemory / 1MB, 2)
    $usedRAM = [math]::Round($totalRAM - $freeRAM, 2)
    $memPercentage = [math]::Round(($usedRAM / $totalRAM) * 100, 1)
    
    Write-Host "MEMORY USAGE" -ForegroundColor Yellow
    Write-Host "   Total: $totalRAM GB"
    Write-Host "   Used:  $usedRAM GB ($memPercentage%)"
    Write-Host "   Free:  $freeRAM GB"
    Write-Host "   " -NoNewline
    Draw-ProgressBar -Percentage $memPercentage -Width 60
    Write-Host ""
    
    # CPU Information
    $cpuUsage = (Get-Counter '\Processor(_Total)\% Processor Time' -ErrorAction SilentlyContinue).CounterSamples.CookedValue
    $cpuUsage = [math]::Round($cpuUsage, 1)
    
    Write-Host "CPU USAGE" -ForegroundColor Yellow
    Write-Host "   Usage: $cpuUsage%"
    Write-Host "   " -NoNewline
    Draw-ProgressBar -Percentage $cpuUsage -Width 60
    Write-Host ""
    
    # GPU Information
    $gpuInfo = Get-GPUInfo
    if ($gpuInfo.Available) {
        Write-Host "GPU USAGE" -ForegroundColor Yellow
        Write-Host "   Name: $($gpuInfo.Name)"
        Write-Host "   Utilization: $($gpuInfo.Utilization)%"
        Write-Host "   " -NoNewline
        Draw-ProgressBar -Percentage $gpuInfo.Utilization -Width 60
        
        $gpuMemPercentage = [math]::Round(($gpuInfo.MemoryUsed / $gpuInfo.MemoryTotal) * 100, 1)
        Write-Host "   Memory: $($gpuInfo.MemoryUsed) MB / $($gpuInfo.MemoryTotal) MB ($gpuMemPercentage%)"
        
        $tempColor = if ($gpuInfo.Temperature -lt 70) { "Green" } 
                     elseif ($gpuInfo.Temperature -lt 85) { "Yellow" } 
                     else { "Red" }
        Write-Host "   Temperature: " -NoNewline
        Write-Host "$($gpuInfo.Temperature)°C" -ForegroundColor $tempColor
        Write-Host ""
    }
    
    # Top Processes
    Write-Host "TOP $TopProcessCount PROCESSES BY MEMORY" -ForegroundColor Yellow
    Write-Host ("-" * 75) -ForegroundColor DarkGray
    Write-Host ("{0,-8} {1,-30} {2,-12} {3,-12}" -f "PID", "NAME", "MEMORY", "CPU %") -ForegroundColor White
    Write-Host ("-" * 75) -ForegroundColor DarkGray
    
    $processes = Get-Process | Sort-Object WorkingSet -Descending | Select-Object -First $TopProcessCount
    
    foreach ($proc in $processes) {
        $memoryMB = [math]::Round($proc.WorkingSet / 1MB, 2)
        $cpuPercent = if ($proc.CPU) { [math]::Round($proc.CPU, 1) } else { 0 }
        
        $memColor = if ($memoryMB -gt 500) { "Red" }
                    elseif ($memoryMB -gt 200) { "Yellow" }
                    else { "Green" }
        
        $procName = if ($proc.Name.Length -gt 28) { 
            $proc.Name.Substring(0, 27) + "..." 
        } else { 
            $proc.Name 
        }
        
        Write-Host ("{0,-8} {1,-30} " -f $proc.Id, $procName) -NoNewline
        Write-Host ("{0,-12} " -f "$memoryMB MB") -ForegroundColor $memColor -NoNewline
        Write-Host ("{0,-12}" -f "$cpuPercent%") -ForegroundColor Gray
    }
    
    Write-Host ""
    Write-Host ("=" * 75) -ForegroundColor Cyan
    Write-Host "Press Ctrl+C to exit | Refreshing every $RefreshInterval seconds..." -ForegroundColor Gray
}

# Main loop
Write-Host "`n================================================================" -ForegroundColor Green
Write-Host "          SYSTEM MONITOR - PowerShell Edition v1.0              " -ForegroundColor Green
Write-Host "================================================================" -ForegroundColor Green
Write-Host "`nInitializing..." -ForegroundColor Cyan
Start-Sleep -Milliseconds 800

try {
    while ($true) {
        Show-SystemMonitor
        Start-Sleep -Seconds $RefreshInterval
    }
} catch {
    Write-Host "`n`nMonitor stopped." -ForegroundColor Yellow
}
