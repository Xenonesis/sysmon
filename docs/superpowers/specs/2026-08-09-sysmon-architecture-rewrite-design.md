# SysMon Architecture Rewrite and Stability Release Design

**Date:** 2026-08-09
**Status:** Approved design

## Goal

Stability-first rewrite of the Windows Rust system monitor, followed by three low-risk features: a full process manager, a deduplicated alert center, and diagnostics export. Preserve current tabs, settings compatibility, tray behavior, installer behavior, and existing export formats where practical.

## Scope

### Stability

- Correct network and disk delta rates.
- Remove hardcoded developer-path debug logging and per-frame logging.
- Use one long-lived monitoring worker; UI actions must not recreate monitoring backends.
- Cache/throttle WMI and other expensive providers.
- Add typed action results and visible UI states.
- Harden startup, service, power-plan, RAM-cleaner, settings, and updater behavior.
- Add deterministic tests and Windows manual verification.

### Features

1. Full process manager with all-process search, sorting, details, confirmations, and action feedback.
2. Alert center with active/resolved grouping, dismissal, clearing, cooldowns, source identity, and resolution.
3. Diagnostics export excluding sensitive command lines and user paths by default.

Explicitly out of scope: cloud sync, remote monitoring, plugins, database storage, widget redesign, chart overhaul, and multi-window architecture.

## Architecture

```text
src/
├── main.rs              # startup and eframe bootstrap
├── app/                 # UI state, commands, events
├── monitoring/          # service, snapshots, rates, alerts, history
├── windows/             # GPU, temperature, processes, services, startup, power, privilege
├── persistence/         # settings and diagnostics
├── updater/             # release check and explicit install
└── ui/                  # pages, windows, components
```

`MonitoringService` owns `System`, `Disks`, `Networks`, NVML, and WMI resources. It runs one worker and publishes immutable `SystemSnapshot` values. `SystemMonitorApp` owns presentation state, sends commands, and consumes events. An action worker handles blocking/destructive operations.

No async runtime, database, or new orchestration framework. Existing threads and `parking_lot::Mutex` remain unless measurements justify replacement.

## Data flow and sampling

- UI settings flow to the monitoring worker through commands.
- Worker publishes latest snapshot and requests repaint.
- UI never holds a lock during OS calls.
- CPU/RAM and normal counters use configured refresh interval.
- CPU temperature samples every five seconds.
- Services sample on page entry, action, or every 30 seconds.
- System details sample once.
- Battery samples every ten seconds.
- Hidden windows use reduced CPU/RAM polling and pause other providers.
- Paused monitoring retains the last snapshot and marks it stale.

Rates use saturating deltas. First sample is zero. Counter resets do not create spikes. Zero totals and invalid metric values are handled safely.

## Reliability and safety

### Alerts

Alerts are state transitions: `Normal → Triggered → Acknowledged → Resolved`. One active alert exists per metric/device identity. Notification cooldown is separate from bounded in-app history. Sounds fire only on a new trigger. All alert types can resolve where the source supports resolution, including disk and startup alerts. GPU resolution checks every GPU, not only the first.

### Settings

Existing `settings.json` remains readable. Missing fields get defaults. Values are range-validated. Saves use temporary file plus flush and rename. Invalid settings are logged and replaced by defaults. No migration framework.

### Windows operations

Startup identity is exact, registry moves roll back on source-delete failure, startup-folder matching is exact, service restart polls stopped/running state, power GUID parsing returns errors instead of silently using a nil GUID, process actions expose OS errors, and RAM cleaning cannot run concurrently.

### Updater

Checks remain background and silent. Installation requires explicit user confirmation. Only HTTPS assets from the expected GitHub repository are accepted. Downloads use unique temporary paths and a bounded size. Authenticode verification is required before launch. Failed validation never exits the app. Successful installer spawn performs controlled shutdown.

### Diagnostics

Use rotating `tracing` logs under app data. Remove hardcoded `C:\Users\Acer\Desktop\sysmon\dbg.txt` logging. Avoid per-frame logs and throttle repeated provider failures. Diagnostics export omits command lines and user paths by default.

## UI

Pages receive read-only snapshot data and emit commands; they do not call Windows APIs directly. State is divided into UI state, snapshot store, command bus, event inbox, and settings store.

Visible states include loading, stale, unavailable, error, paused, and action pending. Current tabs and tray behavior remain.

## Testing

Pure tests cover rates, resets, invalid metrics, alert transitions/cooldowns, settings validation/atomic-save behavior, version comparison, GUID parsing, startup parsing/identity, process-tree order/cycles, and service restart decisions.

Trait-backed providers allow fake integration tests for snapshot publication, refresh changes, pause/resume, typed action results, worker shutdown, and updater rejection cases.

Windows manual matrix covers Windows 10/11, standard/admin users, NVIDIA/non-NVIDIA/no GPU, no battery, multiple adapters, missing WMI classes, high DPI, installed/dev builds, startup settings, tray behavior, updates, and failure paths.

Release commands:

```powershell
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
.\create-installer.ps1
```

## Migration order

1. Add pure monitoring, rate, alert, and persistence modules with tests.
2. Introduce commands, events, snapshots, and worker boundaries.
3. Move polling into `MonitoringService`.
4. Move Windows actions into `ActionService`.
5. Replace direct UI calls with commands/events.
6. Harden updater and settings.
7. Migrate pages to read-only snapshot state.
8. Add the three selected features.
9. Delete obsolete duplicate paths after caller migration.
10. Run release checks and Windows manual matrix.

## Acceptance criteria

- No automatic update installation.
- No hardcoded developer paths.
- No repeated monitor initialization for actions.
- Accurate network/disk rates after counter resets.
- No duplicate active alerts per source.
- Process, service, startup, power, and RAM actions report real outcomes.
- Optional provider failure does not make the app unusable.
- No new compiler warnings.
- Existing user files, including current uncommitted files, remain untouched.
