# Windows release QA matrix

Run the automated gate first:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Invoke-SysMonWindowsQa.ps1 -TelemetrySoakSeconds 300 -ProcessSoakMinutes 30
```

For a release candidate, run a 24-hour process soak on at least one physical machine:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Invoke-SysMonSoak.ps1 -DurationMinutes 1440
```

The automated gate verifies locked dependencies, formatting, Clippy, deterministic tests, real provider delivery, telemetry sampling quality, process survival, memory, CPU, thread count, and handle count. The checks below require representative Windows hardware or a disposable VM and must be attached to the release record.

## Required environments

| Environment | Standard user | Administrator | Required evidence |
| --- | --- | --- | --- |
| Windows 10 x64 | Yes | Yes | Launch, provider status, screenshot, soak report |
| Windows 11 x64 | Yes | Yes | Launch, provider status, screenshot, soak report |
| Intel integrated GPU | Yes | Optional | Utilization range and adapter identity |
| AMD GPU | Yes | Optional | Utilization range and adapter identity |
| NVIDIA GPU | Yes | Optional | NVML plus Windows-counter comparison |
| No dedicated GPU / VM | Yes | Optional | Graceful unavailable state, no warning storm |
| Laptop with battery | Yes | Optional | Charge, AC transition, sleep/resume |
| Desktop without battery | Yes | Optional | Explicit unavailable state, no crash |

## Metric accuracy

For CPU, memory, disk, network, and each detected GPU, record a 60-second controlled workload and compare SysMon with Task Manager or Performance Monitor. The median difference should be within 5 percentage points for utilization metrics. Sampling gaps must remain below 1.5 seconds while the window is visible. Explain larger device-specific differences in the release record.

## UI and lifecycle

- Walk through every navigation view at 900x600, 1100x800, and 150% DPI.
- Verify light, dark, and system themes; keyboard focus; scrolling; tooltips; tray restore; global hotkey; and Desktop HUD.
- Exercise minimize, tray mode, lock/unlock, sleep/resume, display disconnect/reconnect, and clean shutdown.
- Confirm paused and stale states never look like current telemetry.
- Refresh the real rendered Overview screenshot with `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\Update-SysMonScreenshots.ps1`; verify both updated assets visually before committing.

## Guarded actions in a disposable VM

- Verify every action shows risk, elevation requirement, reversibility, and confirmation before execution.
- Exercise process suspend/resume and verify Undo.
- Start and stop a disposable test service; verify dependencies and Undo. Never target a Windows-critical service.
- Disable and restore a disposable current-user startup entry using its exact locator.
- Quarantine and restore a disposable startup-folder shortcut; verify the local audit and backup record.
- Exercise denied actions as a standard user and confirm the UI reports access denial without losing state.
- Verify failed actions are audited but never offered as reversible.

Do not automate destructive actions against a developer workstation or shared runner. Use a disposable VM snapshot and restore it after the matrix.
