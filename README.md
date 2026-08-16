# SysMon

[![Rust CI](https://github.com/Xenonesis/sysmon/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/Xenonesis/sysmon/actions/workflows/rust-ci.yml)
[![Rust 1.85+](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/Windows-10%2F11-blue.svg)](https://www.microsoft.com/windows)
[![Version](https://img.shields.io/badge/version-3.7.2-green.svg)](CHANGELOG.md)

SysMon is a native Windows observability and diagnostics application written in Rust. It combines live CPU, memory, disk, network, process, service, startup, battery and GPU telemetry with evidence-based diagnostics and guarded system actions. The project targets a single goal: give a technically curious user the same depth of insight a professional operations team would have, without background services, cloud accounts, or opaque tweak scripts.

SysMon runs as a normal desktop application. Standard monitoring never requires administrator rights; elevation is requested only for the specific actions that genuinely need it, and every such action is previewed for risk, confirmed explicitly, and written to a local audit record.

## What makes it different

- **Unified TelemetryHub:** replaceable latest-snapshot delivery keeps the UI current without a growing event backlog. The interface always renders the newest sample instead of replaying a queue of stale ones.
- **Vendor-neutral GPU coverage:** NVIDIA NVML plus Windows/WMI adapters and GPU performance counters for Intel, AMD and other Windows GPUs. A machine without an NVIDIA card still gets meaningful GPU utilization and memory data.
- **Multi-resolution history:** bounded 60-second, 5-minute, 30-minute and 1-hour windows with current, minimum, maximum and average statistics. Memory use stays constant regardless of how long the application runs.
- **Explainable diagnostics:** findings include evidence, a recommendation and confidence instead of applying broad tweak scripts. SysMon tells you what it observed and why it matters; it never silently changes your system.
- **Local session recording:** opt-in JSONL capture for reproducing transient slowdowns; no automatic upload. Nothing leaves your machine unless you decide to share a file.
- **Guarded actions:** process, service, RAM and power actions show risk and administrator requirements before execution, then write a local audit record. Reversible actions offer Undo when the prior state is known.
- **Secure updates:** HTTPS/repository asset validation, bounded downloads and SHA-256 checksum verification against the checksum published with each release. An installer that fails verification is deleted, never executed.

## Main views

SysMon organizes its fourteen modules around the questions users actually ask:

- **Overview** — a single-screen summary of CPU, memory, GPU, disk and network health with quick status indicators.
- **Performance** — live graphs plus bounded summaries for the 60-second, 5-minute, 30-minute and 1-hour windows, each reporting average and maximum values.
- **Diagnostics** — evidence-based findings with severity, recommendation and confidence, plus opt-in session recording for transient problems.
- **CPU Cores** — per-core utilization so a single saturated thread is visible even when total CPU looks calm.
- **Processes** — search, sort and inspect running programs; kill, kill-tree, suspend, resume and priority actions behind explicit confirmation.
- **Services** — start, stop or restart Windows services with dependency visibility before you confirm.
- **Startup Manager** — inspect executable existence, publisher information, signature state, boot evidence and estimated impact; prefer reversible disable over permanent removal.
- **Storage** — capacity and usage for all detected storage devices.
- **Network** — real-time interface statistics with download and upload rates.
- **RAM Cleaner** — bounded working-set cleanup with exclusions, idle-only option and per-pass limits, logged locally.
- **Alerts** — threshold-based notifications for CPU, memory, GPU temperature and disk, deduplicated to avoid repeated noise.
- **System Information** — complete hardware and OS specification reference.
- **Settings** — refresh interval, alert thresholds, theme, tray behavior and the safety/audit history.
- **About** — version, update status and project links.

## Version comparison

| Capability | 1.x (2024) | 2.6.x (2026-01) | 3.7.2 (current) |
| --- | --- | --- | --- |
| GUI framework | egui / eframe | egui / eframe | egui / eframe |
| Telemetry engine | Single polling thread | Legacy polling thread | **TelemetryHub** (multi-tier, provider abstraction, background workers) |
| UI render frequency | Full poll per refresh | Full poll per refresh | **60 FPS decoupled** from 1–5 Hz hardware sampling |
| History resolution | ~2 min graphs | ~2 min graphs | **60s / 5m / 30m / 1hr** ring buffers with min/max/avg/peak |
| GPU support | NVIDIA only (NVML) | NVIDIA only (NVML) | **Vendor-neutral** — NVML + Windows/WMI adapters, Intel/AMD via counters |
| Diagnostics | — | — | **Evidence-based findings + confidence**, opt-in JSONL session recording |
| Action safety | — | — | **Risk preview, elevation disclosure, audit history, Undo** |
| Update verification | Plain download | HTTPS + basic checks | **SHA-256 checksum verification**, SBOM, build provenance |
| Supply chain / CI | — | Basic scripts | **Checksum + provenance release workflow**, Windows CI quality gates |
| Views / modules | 4 tabs | 7 tabs | **14 modules** |
| Themes | Single | Dark / Light | Dark / Light |

See the [changelog](CHANGELOG.md) for the complete per-version history.

## Architecture

The 3.x series is built around a stability-first rewrite that separates sampling, state and presentation into independent layers connected by immutable snapshots and typed commands.

### TelemetryHub

The TelemetryHub is the heart of the application. Dedicated background threads sample hardware at 1–5 Hz while the UI renders at up to 60 FPS. The hub publishes a *replaceable* latest snapshot: when the UI is ready to draw, it reads the newest sample and discards nothing, because there is no queue to fall behind. This design eliminates the classic failure mode where a slow frame causes telemetry events to accumulate and the interface drifts further and further behind reality.

### Provider abstraction

Each data source implements a common `TelemetryProvider` trait, so the hub is vendor-independent:

- `sysinfo` provides CPU, memory and portable core metrics.
- `nvml` provides detailed NVIDIA telemetry: temperature, power, clocks, fan and VRAM.
- `wmi` provides Windows hardware identity and thermal-zone information.
- `windows_gpu` provides vendor-neutral GPU engine and local-memory counters.

Providers fail in isolation. If one source errors — for example NVML on a machine without an NVIDIA GPU — the rest of the telemetry pipeline continues unaffected. A provider that fails five consecutive polls is disabled and logged once instead of retrying every second; a successful poll re-enables it. Diagnostics reports unavailable providers as informational findings so a missing optional driver is never confused with an application crash.

### Multi-resolution ring buffers

`MetricHistory` stores samples in bounded circular buffers for four windows: 60 seconds, 5 minutes, 30 minutes and 1 hour. Each window maintains running statistics — minimum, maximum, average and peak time — and evicted samples never leak into the summaries. Because the buffers are fixed-size, memory consumption is constant no matter how long SysMon stays open. When you need a persistent timeline beyond one hour, a diagnostic session recording is the intended tool.

### Polling scheduler

The `PollingScheduler` throttles sampling when the window is minimized or hidden. Background and tray mode reduces polling frequency by roughly five times to conserve CPU and battery, then restores full cadence when the window returns. Paused monitoring retains the last snapshot and marks it stale rather than silently showing outdated numbers.

### Command and event flow

UI pages receive read-only snapshot data and emit commands; they never call Windows APIs directly. An action worker handles blocking or destructive operations — process control, service control, RAM cleanup, power-plan changes — so the render thread is never stalled by an OS call. Every action carries a risk level, an elevation disclosure and a confirmation plan, and its outcome is appended to a local audit file.

### Source layout

```text
src/
├── main.rs          # startup and eframe bootstrap
├── app/             # UI state, commands, events, models
├── telemetry/       # TelemetryHub, ring buffers, scheduler
├── providers/       # sysinfo, NVML, WMI and Windows GPU adapters
├── monitoring/      # legacy engine, snapshots, rates, history
├── diagnostics/     # evidence-based finding rules
├── persistence/     # settings, sessions, action audit log
├── ui/              # pages, windows, components, theme
└── updater.rs       # release check and verified install
```

There is no async runtime, no database and no orchestration framework. Threads and mutexes are kept deliberately simple, and replacements are only considered when measurements justify them.

### Rate calculation and alert lifecycle

Disk and network rates are computed from saturating deltas: the first sample reports zero, counter resets never produce artificial spikes, and zero totals or invalid metric values are handled safely rather than propagating as division errors.

Alerts are modeled as explicit state transitions — `Normal → Triggered → Acknowledged → Resolved` — with at most one active alert per metric and device identity. Notification cooldown is separate from the bounded in-app history, sounds fire only on a new trigger, and every alert type can resolve where its source supports resolution, including disk and startup alerts. GPU resolution checks every adapter, not only the first, so a cooling card cannot hide behind a quiet one.

## Install

### System requirements

- Windows 10 or Windows 11, x64. ARM64 and 32-bit builds are not supported.
- No runtime dependencies beyond the OS; the MSVC runtime is statically linked into the release binary.
- NVIDIA drivers only for NVML-specific metrics (temperature, power, clocks, fan, VRAM). All other telemetry works without them.
- Approximately 40 MB of resident memory and under one percent of CPU during normal monitoring.

Use the installer from [GitHub Releases](https://github.com/Xenonesis/sysmon/releases/latest). SysMon verifies the downloaded installer's SHA-256 checksum against the checksum file published with the release and refuses to install when they do not match or when no checksum is published.

Installation steps:

1. Download the `.exe` installer and its `.sha256` checksum file from the latest release.
2. Run the installer. Standard monitoring works without elevation after installation.
3. Launch SysMon from the Start menu or desktop shortcut.
4. Open **Settings → About** to confirm the version and enable update checks.

> Version 3.7.2 must be published through the release workflow before installed clients can receive it. Do not distribute a locally built installer as a production update.

## Build from source

Requirements:

- Windows 10 or 11, x64
- Rust 1.85 or newer with the MSVC toolchain
- Visual Studio C++ Build Tools
- NVIDIA drivers only if NVML-specific metrics are required

```powershell
cargo build --locked --release --bin system-monitor
```

The binary is written to `target\release\system-monitor.exe`. Standard monitoring works without elevation. Windows requests administrator permission only for actions that require it.

The `--locked` flag is important: `Cargo.lock` is committed so every build resolves the exact dependency versions that CI tested. Two smoke-test binaries exist for hardware investigation — `test_wmi` exercises WMI deserialization and `test_cpu_temp` probes `ROOT\WMI` thermal queries (the latter explains its administrator requirement instead of panicking when denied).

## Quality checks

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release --bin system-monitor
```

CI runs the same commands on Windows. Hardware-dependent telemetry smoke tests remain ignored by default because results depend on the runner's devices and drivers; deterministic provider/hub tests run normally. The release workflow propagates native exit codes, so a failing format, lint or test step can never publish a release.

## Real-world use cases

**Why is my machine slow right now?** Open Diagnostics and press **Start recording** just before reproducing the problem. Reproduce the slowdown, press **Stop recording**, then review each finding's severity, evidence, recommendation and confidence. The finding names the component; Processes, Storage, Services or Startup Manager lets you inspect it directly.

**Is one core the bottleneck?** A video encode or a single-threaded game can pin one core at 100% while total CPU shows 25%. The CPU Cores view exposes per-core utilization so the saturation is obvious.

**What is eating my battery?** Background/tray mode throttles polling automatically, but you can also watch GPU engine counters and per-interface network rates to identify which subsystem stays busy while the machine should be idle.

**Did that cleanup actually help?** RAM cleaner passes report `trimmed`, `access_denied` or `errored` counts, and every manual or automatic cleanup is logged in the audit history, so before/after comparisons are grounded in records rather than impressions.

**Which startup items can I disable?** Startup Manager shows publisher information, signature state, Windows boot evidence and estimated impact. Disable first — the action is reversible — and only remove items you fully understand.

## Design and UI philosophy

SysMon's interface follows a deliberate "cockpit dense" design language documented in [DESIGN.md](DESIGN.md). The goals are information density, mechanical precision and zero decoration:

- **Flat structural surfaces** with 1px borders instead of floating, drop-shadow-heavy cards.
- **Monospace numerals** for all telemetry readouts so columns align and values can be scanned vertically.
- **A single accent color** reserved for active states and healthy thresholds, with red strictly reserved for high-load alerts, thermal warnings and destructive actions.
- **Asymmetric data layouts** — sidebar, primary graph and detail panel — rather than generic equal-width grids.
- **Dark and light themes** with instant switching, persisted in settings.

The result is a dashboard that behaves like an instrument panel: every metric has a defined slot, states such as loading, stale, unavailable, paused and action-pending are always visible, and nothing animates for decoration alone.

## Troubleshooting

- **No NVIDIA details:** install or update the NVIDIA driver, or simply ignore NVML status on a non-NVIDIA system. Vendor-neutral GPU counters still report engine and memory activity.
- **No vendor-neutral GPU counters:** update Windows and display drivers, then restart SysMon.
- **Action denied:** relaunch as administrator only if you trust and intend the exact action shown in the confirmation dialog.
- **No diagnostic finding:** record a session while the issue occurs; an idle snapshot cannot explain a transient spike.
- **Update rejected:** install only an official release whose published checksum matches. Do not bypass checksum or repository checks.
- **Empty services list or WMI errors:** these were addressed in 3.7.2 (`PascalCase` deserialization and shared COM security initialization); update if you are running an older build.
- **Settings appear reset:** invalid or corrupted settings files are logged and replaced with defaults by design; check the application log under the app-data directory.

### Reporting an issue effectively

When opening an issue, the fastest path to a fix is:

1. Note the SysMon version (Settings → About) and your Windows version.
2. Describe the expected versus observed behavior, and whether it reproduces consistently.
3. If the problem is transient, attach a diagnostic session recording captured while the issue occurs — after reviewing it for sensitive process names or paths.
4. For crashes or missing metrics, mention which providers Diagnostics reports as unavailable.

Please do not attach full audit logs or settings files unless asked; they can contain machine-specific metadata.

## Contributing

SysMon targets Windows 10/11 x64 and Rust 1.85 or newer. Contributions should keep changes focused, preserve standard-user monitoring, and put any system-changing behavior behind the action-plan confirmation and audit boundary.

A typical workflow:

1. Fork the repository and create a topic branch from `main`.
2. Make your change with tests where behavior is deterministic.
3. Run the four quality gates listed above until they all pass.
4. Open a pull request describing the problem, the fix and how you verified it on real hardware.

Provider changes must use normalized metric keys, return structured errors, avoid blocking the UI thread and include a deterministic test. Hardware-only tests should be ignored by default with a clear reason documented in the test. Security-sensitive changes to updates, process/service control, release workflows or persistence need explicit failure-path tests. Never weaken update checksum verification or add a bypass around it.

## Security best practices

- Monitoring works as a standard user; elevation is requested only for specific privileged actions and is always disclosed before confirmation.
- Diagnostic sessions and action audits are local files and may contain system or process metadata. Review them before sharing.
- Automatic updates require HTTPS, the official release repository, a bounded download, and a SHA-256 checksum match against the checksum file published with the release. Failed verification deletes the temporary installer and never runs it.
- Published builds must use the release workflow, which publishes the installer together with its SHA-256 checksum, an SPDX SBOM and a GitHub build provenance attestation. Locally built installers are development artifacts, not production updates.
- Report vulnerabilities through GitHub's private **Report a vulnerability** feature for `Xenonesis/sysmon`. Do not open public issues containing exploits, sensitive machine data or update-verification bypasses. See [SECURITY.md](SECURITY.md).
- Never commit passwords, access tokens or private diagnostic exports.

## Performance optimization tips

- **Leave the scheduler alone.** The default 1–5 Hz sampling with 60 FPS rendering is tuned so telemetry cost stays under one percent of CPU on typical hardware. Raising the refresh rate rarely improves insight but always raises cost.
- **Use tray mode.** Minimizing to the tray activates reduced polling automatically, which matters on battery.
- **Prefer recordings over faster polling.** For transient issues, a JSONL session captures far more evidence than a quicker refresh interval would.
- **Treat RAM cleanup as temporary.** Working-set trimming can reduce resident memory briefly, but it is not a substitute for fixing a leak or adding RAM. Keep automatic cleanup bounded by its interval, target, exclusions and per-pass limits.
- **Watch provider health.** A repeatedly failing provider is disabled after five consecutive failures; if a metric disappears, check Diagnostics for an informational provider finding before assuming a bug.

## Technology stack

- **Language:** Rust 2021 edition, 1.85+, MSVC toolchain, with `Cargo.lock` committed for reproducible builds.
- **GUI:** egui / eframe with hardware-accelerated rendering, dark and light themes.
- **Telemetry sources:** `sysinfo`, NVML bindings, the `wmi` crate and native Windows GPU performance counters.
- **Concurrency:** plain OS threads with `parking_lot` mutexes; no async runtime.
- **Persistence:** JSON settings, JSONL sessions and append-only JSONL audit logs under per-user app-data directories.
- **Packaging:** Inno Setup installer produced by the release workflow, with SHA-256 checksum, SPDX SBOM and GitHub build provenance.
- **CI:** GitHub Actions on Windows runners running format, Clippy, test and release-build gates.

## Privacy and data locations

SysMon has no telemetry-upload feature. Settings, diagnostic sessions, logs and system-action audit records stay under the current user's Windows application-data directories. A session is written only after the user presses **Start recording** on Diagnostics. The audit file is append-only JSONL recording action outcomes — never passwords or tokens.

## Release security

Tagged releases build the application and installer in CI, generate a SHA-256 checksum and SPDX SBOM, and attach GitHub build provenance. The updater verifies the installer checksum before installing. An installer is accepted only when the download is HTTPS, belongs to the expected release repository, is an `.exe` within the configured size limit, and hashes to the published checksum. See [release-rule.md](docs/release-rule.md) and [SECURITY.md](SECURITY.md).

## Roadmap

Planned directions, in rough priority order:

- **Diagnostics export** with sensitive command lines and user paths excluded by default, so findings can be shared with maintainers without leaking private data.
- **Alert center refinements** — active/resolved grouping, dismissal, cooldowns and per-source identity, building on the existing state-transition alert model.
- **Deeper process manager** details and tree visualization, extending the current kill-tree and priority actions.
- **Custom alert rules** and expanded threshold types beyond the current CPU, memory, GPU-temperature and disk set.
- Longer-horizon ideas such as cross-platform support, a web dashboard and plugin architecture remain explicitly out of scope until the Windows core is finished; they are tracked in the [changelog](CHANGELOG.md) future-releases section.

The roadmap deliberately favors depth on Windows over breadth across platforms. Every candidate feature is weighed against the project's invariants: standard-user monitoring, guarded actions, local-only data and verified updates.

## Frequently asked questions

**Does SysMon run a background service?** No. It is a normal desktop process. When you close the window or exit the tray icon, all sampling stops.

**Does it collect or upload any data?** No. There is no telemetry-upload feature at all. The only network activity is the optional update check against the official GitHub repository over HTTPS.

**Why do some metrics show as unavailable?** Providers are optional by design. NVML requires an NVIDIA driver, WMI thermal zones may require elevation, and GPU counters require current display drivers. Diagnostics surfaces each unavailable provider as an informational finding rather than an error.

**Can SysMon damage my system?** Guarded actions always preview their target, risk and reversibility before execution, and Undo is offered whenever a safe inverse operation is known. That said, process and service control are inherently powerful: confirm only actions you understand.

**How do I interpret the RAM cleaner results?** A successful pass reports how many working sets were trimmed. Freed memory is temporary — Windows repopulates working sets as needed — so treat it as a diagnostic aid, not a permanent optimization.

**Where are my recordings and audit logs?** Under the current user's Windows application-data directories, as newline-delimited JSON files. They may contain process names and hardware details, so review them before sharing.

**Can I run SysMon on a machine without a GPU or battery?** Yes. Missing hardware simply means the corresponding provider reports nothing; nothing crashes and no configuration is required.

## Documentation

- [User guide](USER_GUIDE.md)
- [Changelog](CHANGELOG.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Design system](DESIGN.md)

## License

MIT. See [LICENSE](LICENSE).
