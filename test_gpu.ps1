try {
    $r = Get-CimInstance -Namespace 'root\cimv2' -ClassName 'Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine' -ErrorAction Stop
    Write-Host 'GPUCounters ROWS:' $r.Count
    $r | ForEach-Object { Write-Host 'Name:' $_.Name ' Util:' $_.UtilizationPercentage }
} catch {
    Write-Host 'GPUCounters CLASS NOT AVAILABLE:' $_.Exception.Message
}