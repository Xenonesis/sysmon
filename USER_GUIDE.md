# System Monitor User Guide

## Getting started

Launch SysMon normally for monitoring. The Overview and sidebar show current CPU, memory, GPU, disk and network state. Monitoring does not require administrator rights. When an operation needs elevation, SysMon identifies that requirement in its confirmation dialog.

## Find a slowdown

1. Open **Diagnostics**.
2. Press **Start recording** just before reproducing the problem.
3. Reproduce the slowdown, then press **Stop recording**.
4. Review each finding's severity, evidence, recommendation and confidence.
5. Use **Processes**, **Storage**, **Services** or **Startup Manager** to inspect the component named by the evidence.

Recordings are newline-delimited JSON files in the local SysMon application-data directory. They are never uploaded automatically and may contain process names and hardware details, so review them before sharing.

## Performance history

The Performance page shows live graphs plus bounded summaries for four time windows: 60 seconds, 5 minutes, 30 minutes and 1 hour. Each summary reports average and maximum values. The in-memory windows are designed to remain bounded.

## Trusted Timeline

Timeline history is off by default. To use it, open **Settings → Local diagnostic timeline**, enable history, and choose 1, 7, or 30 days of retention. SysMon then records five-second system metrics, the deduplicated top CPU/memory/disk-I/O contributors, and alert, provider, action, pause/resume, and power transitions to `history/timeline.sqlite3` in local application data.

Open **Timeline** and select 15 minutes, 1 hour, 6 hours, 24 hours, or 7 days. Selecting an event shows the closest recorded metrics, contributing processes, changes against the preceding five-minute baseline, and evidence completeness. SysMon displays **Insufficient data** when it cannot support a conclusion.

Timeline never stores command lines, executable paths, working directories, usernames, or remote IP addresses. Use **Export incident** to create a sanitized directory containing `summary.json`, `metrics.csv`, `processes.csv`, and `events.json`. Settings shows current storage usage and requires a second confirmation before deleting history. The database is capped at 512 MiB; a storage error does not stop live monitoring.

## Providers and GPU behavior

SysMon combines multiple providers:

- `sysinfo` for CPU, memory and portable core metrics.
- `nvml` for detailed NVIDIA telemetry such as temperature, power, clocks, fan and VRAM.
- `wmi` for Windows hardware identity and thermal information.
- `windows_gpu` for vendor-neutral Windows GPU engine and local-memory counters.

An unavailable NVML provider is expected on a machine without an NVIDIA GPU. Diagnostics reports unavailable providers as informational findings so a missing optional driver is not confused with an application crash.

## Processes, services and startup items

Use **Processes** to search, sort and inspect running programs. Kill, kill-tree, suspend, resume and priority actions first display their target, risk, elevation requirement and reversibility.

Use **Services** to start, stop or restart Windows services. Check the service name and dependencies before confirming.

Use **Startup Manager** to inspect executable existence, publisher information, signature state, Windows boot evidence and estimated impact. Prefer reversible disable/enable actions over permanent removal.

## RAM cleaner and power plans

Working-set cleanup can reduce a process's resident memory temporarily, but it is not a substitute for fixing a leak or adding RAM. Automatic cleanup is bounded by its interval, target, exclusions, idle-only option, maximum-per-pass budget and five-pass limit. Manual and automatic cleanups are logged locally.

Power-plan changes affect performance, thermals and battery life. Confirm the selected plan before applying it.

## Safety and action history

Every action routed through SysMon's action worker has a risk level and confirmation plan. Open **Settings → Safety and Audit → View System Action History** to see timestamp, initiator, result and message. Undo appears only when SysMon knows a safe inverse operation, such as resume after suspend or start after stop.

The audit file is append-only JSONL in the local application-data directory. It records action outcomes, not passwords or tokens.

## Alerts, export and tray mode

Configure CPU, memory, GPU-temperature and disk thresholds under Settings/Alerts. Notifications are deduplicated to avoid repeated alerts for the same condition.

CSV and JSON export capture the current system snapshot. The tray menu can show the app, pause monitoring, open Process Manager, request RAM cleanup and choose a power plan. Background/tray mode reduces polling frequency.

## Updates

SysMon checks the official GitHub repository for releases. An installer is accepted only when all of the following hold:

- the download is HTTPS and belongs to the expected release repository;
- it is an `.exe` installer and remains within the configured size limit;
- the release publishes a SHA-256 checksum file next to the installer;
- the checksum entry names that exact installer asset;
- the downloaded installer's SHA-256 hash matches that published checksum.

The release workflow also verifies GitHub attestations for the installer, checksum, and SBOM before publication. If client-side checksum verification fails, SysMon deletes the temporary installer and does not run it.

## Troubleshooting

- **No NVIDIA details:** install/update the NVIDIA driver, or ignore NVML status on a non-NVIDIA system.
- **No vendor-neutral GPU counters:** update Windows/display drivers and restart SysMon.
- **Action denied:** relaunch as administrator only if you trust and intend the exact action shown.
- **No diagnostic finding:** record a session while the issue occurs; an idle snapshot cannot explain a transient spike.
- **Update rejected:** install only an official release whose published checksum matches. Do not bypass checksum or repository checks.
