try {
    $r = Get-CimInstance -Namespace 'root\wmi' -ClassName 'MSAcpi_ThermalZoneTemperature' -ErrorAction Stop
    Write-Host 'ROWS:' $r.Count
    $r | ForEach-Object { Write-Host 'CurrentTemperature:' $_.CurrentTemperature }
} catch {
    Write-Host 'WMI CLASS NOT AVAILABLE:' $_.Exception.Message
}