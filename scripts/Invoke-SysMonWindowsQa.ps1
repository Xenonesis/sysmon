[CmdletBinding()]
param(
    [Parameter()]
    [ValidateRange(10, 86400)]
    [int]$TelemetrySoakSeconds = 60,

    [Parameter()]
    [ValidateRange(1, 1440)]
    [int]$ProcessSoakMinutes = 10,

    [Parameter()]
    [switch]$SkipReleaseBuild,

    [Parameter()]
    [switch]$SkipHardware
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

function Invoke-NativeChecked {
    param([Parameter(Mandatory)][scriptblock]$Command, [Parameter(Mandatory)][string]$Name)
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

Invoke-NativeChecked { cargo metadata --locked --format-version 1 --no-deps | Out-Null } "locked metadata"
Invoke-NativeChecked { cargo fmt --all -- --check } "formatting"
Invoke-NativeChecked { cargo clippy --locked --all-targets -- -D warnings } "Clippy"
Invoke-NativeChecked { cargo test --locked --all-targets } "test suite"

if (-not $SkipHardware) {
    Invoke-NativeChecked {
        cargo test --locked hardware_telemetry_smoke_test -- --ignored
    } "hardware telemetry smoke"

    try {
        $env:SYSMON_SOAK_SECONDS = $TelemetrySoakSeconds.ToString()
        Invoke-NativeChecked {
            cargo test --locked hardware_telemetry_soak_test -- --ignored --nocapture
        } "hardware telemetry soak"
    }
    finally {
        Remove-Item Env:SYSMON_SOAK_SECONDS -ErrorAction SilentlyContinue
    }
}

if (-not $SkipReleaseBuild) {
    Invoke-NativeChecked { cargo build --locked --release --bin system-monitor } "release build"
    & "$PSScriptRoot\Invoke-SysMonSoak.ps1" -DurationMinutes $ProcessSoakMinutes
}

Write-Host "Windows QA automation passed. Complete the hardware and privileged-action matrix in docs/WINDOWS_QA_MATRIX.md."
