[CmdletBinding()]
param(
    [Parameter()]
    [string]$ExePath = "target\release\system-monitor.exe",

    [Parameter()]
    [ValidateRange(1, 1440)]
    [int]$DurationMinutes = 10,

    [Parameter()]
    [ValidateRange(1, 60)]
    [int]$SampleSeconds = 5,

    [Parameter()]
    [ValidateRange(64, 8192)]
    [int]$MaxWorkingSetMB = 768,

    [Parameter()]
    [ValidateRange(1, 100)]
    [double]$MaxAverageCpuPercent = 15,

    [Parameter()]
    [string]$OutputPath = "artifacts\sysmon-soak-report.json"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$resolvedExe = (Resolve-Path -LiteralPath $ExePath).Path
$existing = Get-Process -Name "system-monitor" -ErrorAction SilentlyContinue
if ($existing) {
    throw "A system-monitor process is already running. Close it before the isolated soak test."
}

$outputFullPath = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $OutputPath))
$outputDirectory = Split-Path -Parent $outputFullPath
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null

$process = $null
$samples = [System.Collections.Generic.List[object]]::new()
$startedAt = [DateTimeOffset]::UtcNow

try {
    $process = Start-Process -FilePath $resolvedExe -PassThru -WindowStyle Hidden
    $deadline = [DateTimeOffset]::UtcNow.AddMinutes($DurationMinutes)
    $previousCpu = 0.0
    $previousAt = [DateTimeOffset]::UtcNow

    while ([DateTimeOffset]::UtcNow -lt $deadline) {
        Start-Sleep -Seconds $SampleSeconds
        $process.Refresh()
        if ($process.HasExited) {
            throw "SysMon exited during soak with code $($process.ExitCode)."
        }

        $now = [DateTimeOffset]::UtcNow
        $cpuSeconds = $process.TotalProcessorTime.TotalSeconds
        $elapsedSeconds = [Math]::Max(($now - $previousAt).TotalSeconds, 0.001)
        $cpuPercent = (($cpuSeconds - $previousCpu) / $elapsedSeconds / [Environment]::ProcessorCount) * 100.0
        $samples.Add([pscustomobject]@{
            timestamp_utc = $now.ToString("O")
            working_set_mb = [Math]::Round($process.WorkingSet64 / 1MB, 2)
            private_memory_mb = [Math]::Round($process.PrivateMemorySize64 / 1MB, 2)
            cpu_percent = [Math]::Round([Math]::Max($cpuPercent, 0.0), 2)
            threads = $process.Threads.Count
            handles = $process.HandleCount
        })
        $previousCpu = $cpuSeconds
        $previousAt = $now
    }
}
finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
        $process.WaitForExit(10000) | Out-Null
    }
}

if ($samples.Count -eq 0) {
    throw "The soak test produced no process samples."
}

$maxWorkingSet = ($samples | Measure-Object -Property working_set_mb -Maximum).Maximum
$averageCpu = ($samples | Measure-Object -Property cpu_percent -Average).Average
$report = [pscustomobject]@{
    executable = $resolvedExe
    started_at_utc = $startedAt.ToString("O")
    duration_minutes = $DurationMinutes
    sample_seconds = $SampleSeconds
    sample_count = $samples.Count
    maximum_working_set_mb = [Math]::Round($maxWorkingSet, 2)
    average_cpu_percent = [Math]::Round($averageCpu, 2)
    limits = [pscustomobject]@{
        maximum_working_set_mb = $MaxWorkingSetMB
        maximum_average_cpu_percent = $MaxAverageCpuPercent
    }
    samples = $samples
}
$report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $outputFullPath -Encoding utf8

if ($maxWorkingSet -gt $MaxWorkingSetMB) {
    throw "Working set exceeded ${MaxWorkingSetMB} MB. Report: $outputFullPath"
}
if ($averageCpu -gt $MaxAverageCpuPercent) {
    throw "Average CPU exceeded ${MaxAverageCpuPercent}%. Report: $outputFullPath"
}

Write-Host "SysMon soak passed. Report: $outputFullPath"
Write-Output $report
