# Changelog

All notable changes to System Monitor are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [3.7.5] - 2026-08-16

### Added
- **Active Network Sockets Table with Process Resolution:** Real-time TCP and UDP socket connection monitor displaying Local Address, Remote Address, State (`ESTABLISHED`, `LISTEN`, `TIME_WAIT`, `CLOSE_WAIT`), Process ID, and resolved Process Name. Includes live text filtering and socket state selector.
- **Physical Storage Hardware & S.M.A.R.T. Health Detection:** Automatic physical drive scanner querying `Win32_DiskDrive` and storage interfaces (`NVMe SSD`, `SATA SSD`, `USB`, `Virtual`). Displays drive health status, S.M.A.R.T. status, media type, and capacity in the Storage page.
- **Process Table Per-Process Disk I/O & Network Rates:** Process view and Process Manager now display real-time live per-process read and write bandwidth rates (R: X KB/s, W: Y KB/s) with 1-click `Disk I/O` sorting.
- **Floating Desktop HUD Mini-Widget:** Precision always-on-top floating telemetry HUD (`Ctrl + M` or `[ ◰ HUD ]` header toggle) showing live CPU%, RAM%, GPU%, disk active throughput, and network rates with instant quick-clean RAM button.
- **Process Tree Hierarchy View:** Hierarchical parent-child process tree mode (`🌲 Process Tree`) in Process Manager with collapsible branch indentation (`├──`, `└──`, `│  `).
- **CPU Core Affinity Controls:** Interactive process processor affinity controls via `SetProcessAffinityMask` on Windows with 1-click All Cores, Core 0, Core 1, and custom mask presets.
- **Battery Health Diagnostics & Power Plan Switcher:** Real-time battery charge state, AC power detection, battery saver status, and 1-click switcher between Windows power schemes (*Balanced*, *High Performance*, *Power Saver*) in System Information.
- **Telemetry Session CSV Exporter & Summary Analytics:** 1-click export of recorded JSONL telemetry sessions into standard multi-column CSVs and automatic aggregate statistics (average/peak CPU, RAM, GPU, and total network throughput).
- **Crash Resilience:** Safe monotonic time math across telemetry scheduling and rolling ring buffers, eliminating underflow panics on system startup.

---

## [3.7.3] - 2026-08-16

### Added
- **Alert Audio Chime Toggle:** Added explicit controls to toggle alert notification sound on or off independently from general system event sounds. Users can toggle alert sound directly from the Alerts page (quick `🔔 Sound: ON` / `🔕 Sound: OFF` button) and in Application Settings under both General Preferences and Alert Thresholds.
- **Sidebar Icons & Logo:** Upgraded collapsed and expanded sidebar navigation with high-contrast system icons and rendered the official application logo in both collapsed and expanded sidebar headers.

---

## [3.7.2] - 2026-08-16

### Fixed
- **WMI providers restored:** `wmi` and `windows_gpu` telemetry providers failed at runtime with `RPC_E_TOO_LATE` (0x80010119) because `CoInitializeSecurity` may succeed only once per process. All WMI call sites now share a single COM security initialization (`providers::init_com`) and fall back to `COMLibrary::without_security()` when the context already exists. Hardware identity, thermal-zone and vendor-neutral GPU counter telemetry work again.
- **Provider failure backoff:** a provider that fails five consecutive polls is disabled and logged once, instead of retrying (and log-spamming) every second. A successful poll re-enables it.
- **RAM cleaner actually trims now:** `EmptyWorkingSet` requires `PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA`; the cleaner opened processes with query rights only, so every trim failed with access denied and nothing was ever freed. Cleanup logs now report `trimmed` / `access_denied` / `errored` instead of a misleading `success`/`failed` pair.
- **Services view deserialization:** `Win32_Service` rows were deserialized without `rename_all = "PascalCase"`, which surfaced as `WBEM_E_NOT_FOUND` and left the WMI service list empty.
- **Quality gates restored:** removed 24 unused imports so `cargo clippy --locked --all-targets -- -D warnings` passes, and re-applied `cargo fmt` so the formatting gate passes.
- **Release pipeline hardened:** the release workflow propagates native command exit codes so a failing fmt/clippy/test step can no longer publish a release, publishes the installer with its SHA-256 checksum and SPDX SBOM, and attaches GitHub build provenance. The unsigned release workflow was removed.
- **Smoke binaries:** `test_wmi` deserializes with `PascalCase` field names and reports errors gracefully; `test_cpu_temp` no longer panics and explains the administrator requirement for `ROOT\WMI` thermal queries.

### Changed
- **Update verification without a paid certificate:** Authenticode signature checks are replaced by SHA-256 checksum verification. The release workflow publishes the installer together with its `.sha256` file, and the updater downloads both, verifies the installer hash, and refuses to install on mismatch or when no checksum is published. Releases no longer require signing secrets; GitHub build provenance remains attached to every release.

> Version 3.7.1 was prepared during audit remediation; its changes folded into 3.7.2.

---

## [3.7.0] - 2026-08-13

### Added
- **TelemetryHub Architecture:** Multi-tier telemetry hub running on dedicated background threads, separating UI render frequency (60 FPS) from hardware polling (1-5 Hz).
- **Provider Abstraction:** Modular `TelemetryProvider` trait supporting vendor-independent data sources (`SysinfoProvider`, `NvmlProvider`, `WmiProvider`).
- **Multi-Resolution Ring Buffers:** `MetricHistory` bounded circular buffers with running statistics (min, max, avg, peak time) across 60s, 5m, 30m, and 1hr time windows.
- **Polling Scheduler:** `PollingScheduler` with background/tray mode throttling (5x reduced polling when minimized to conserve CPU/power).
- **Graceful Error Isolation:** Independent provider failure handling — individual provider errors (e.g. missing NVML/WMI) no longer affect main telemetry or application stability.
- **Integration Test Suite:** Automated deep telemetry flow integration test suite (`test_deep_telemetry_flow`).
- **Vendor-neutral GPU telemetry:** Windows GPU adapters and performance counters complement detailed NVIDIA NVML metrics.
- **Diagnostics workspace:** Evidence-based findings with confidence and opt-in local JSONL session recording.
- **Safe action plans:** Risk previews, elevation disclosure, confirmation, local audit history and Undo for known reversible operations.
- **Windows CI:** Locked dependency resolution, formatting, strict Clippy, tests and release compilation on Rust 1.85.

### Changed
- Production application consumes TelemetryHub's replaceable latest snapshot without FIFO backlog lag.
- Updater downloads enforce timeouts, bounded reads, content-length validation, escaped literal paths and cleanup after verification failure.
- Automatic RAM cleanup is bounded and recorded in persistent action audit trail.
- `Cargo.lock` committed for reproducible application builds.

### Fixed
- History minimum and maximum values no longer retain samples evicted from the active window.
- Snapshot delivery cannot fall behind through a FIFO backlog.

---

## [2.6.1] - 2026-08-09

### Fixed
- **Updater Thumbprint Matching:** Updated updater verification to accept self-signed development builds by pinned thumbprint so local and staging update flows work without errors.
- **Windows Action Validation:** Hardened Windows process and service action parameter validation to reject invalid control commands before OS dispatch.

---

## [2.6.0] - 2026-08-09

### Added
- **Storage Monitoring Tab:** Monitor all detected physical and logical drives with used capacity, free space, and volume paths.
- **Network Monitoring Tab:** Real-time per-interface download and upload rates with active throughput history.
- **System Information Tab:** Detailed hardware, motherboard, BIOS, GPU driver, and OS build specifications.
- **Settings Panel:** Configurable application preferences, themes, polling intervals, and alert threshold configuration.
- **Theme Engine:** Instant Dark mode and Light mode switching with system preference detection.
- **RAM Auto-Clean Customization:** Configurable trigger thresholds, per-pass caps (up to 16 GB), idle-only gating, and process exclusions.
- **Global Hotkey:** Global `Ctrl + Alt + C` hotkey for instant memory optimization from anywhere in Windows.
- **Alert Deduplication:** State-transition alert engine preventing repeated noise notifications.

### Changed
- Eliminated portable binary distribution in favor of canonical Windows setup installer (`SystemMonitor-<version>-setup.exe`).
- Connected UI through typed command channels and background event loops.

---

## [2.5.0] - 2026-08-08

### Added
- **Power Plan Tray Toggle:** Switch installed Windows power schemes (*Balanced*, *High Performance*, *Power Saver*) directly from the system tray menu.
- **Desktop Mini-Widget (v1):** Compact floating desktop HUD overlay displaying live CPU, RAM, GPU, and network rates.
- **Unified 8px Design System:** Standardized border radiuses, spacing scales, and high-contrast color tokens.

---

## [2.4.0] - 2026-08-07

### Added
- **Per-Process Disk I/O:** Live tracking of per-process read and write activity.
- **Windows Services Tab:** Full enumeration of Windows services with run states, display names, and service control options.
- **Battery Telemetry:** Laptop battery charge level, status, and AC online sensing via WMI.
- **Deep GPU Metrics via NVML:** Core clocks, power consumption in Watts, and fan speeds for NVIDIA GPUs.
- **Process Pro Details & Kill-Tree:** Detailed process inspector showing command lines, working directories, and recursive process sub-tree termination.

---

## [2.3.0] - 2026-08-07

### Added
- **Inno Setup Windows Installer:** Professional setup wizard with desktop shortcut, Start Menu integration, and clean uninstaller (`AppId` registry integration).
- **Silent Background Updater:** Seamless updater downloading and launching setup installers for installed clients.
- **Installer-First Distribution:** Automated GitHub Actions build producing official setup executables.

---

## [2.2.0] - 2026-08-05

### Added
- **RAM Cleaner:** Working-set memory optimization utilizing native Windows APIs.
- **Startup Manager:** Registry and startup folder scanner with boot impact estimation and reversible enable/disable toggles.
- **Process Suspension:** Suspend and resume background process execution.
- **Privilege Elevation Assistant:** Transparent UAC elevation disclosure and handling for administrative system tasks.

---

## [1.4.0] - 2026-02-06

### Added
- **CSV Data Export:** Export real-time and historical telemetry data to CSV files for external analysis.
- **Windows Auto-Start:** Configurable toggle to start System Monitor automatically on Windows user login.

---

## [1.3.0] - 2026-02-06

### Added
- **Process Search & Filtering:** Instant text search across process names and PIDs.
- **Quick Copy:** 1-click clipboard copying for system metrics and hardware specifications.
- **Always-on-Top Window Mode:** Pin System Monitor above other desktop application windows.
- **Global Keyboard Shortcuts:** Fast keyboard navigation across monitoring views.

---

## [1.2.0] - 2026-02-05

### Added
- **Per-Core CPU Utilization:** Multi-core bar charts displaying individual utilization percentages for every logical CPU core.
- **Process Manager Controls:** Process priority adjustment (`High`, `Above Normal`, `Normal`, `Below Normal`, `Idle`).

---

## [1.1.0] - 2026-02-05

### Added
- **Network Throughput Graphs:** Live rolling bandwidth history charts for download and upload traffic.
- **Notification Threshold System:** Toast notifications on high CPU, RAM, or temperature thresholds.
- **Export Utilities:** Snapshot export to JSON format.

---

## [1.0.0] - 2026-02-05

### Added
- **Initial Windows GUI Release:** First official stable release built with Rust and egui/eframe.
- **Core Hardware Telemetry:** Real-time CPU, RAM, disk, and GPU utilization monitoring.
- **Process Table:** Top process monitoring sorted by memory and CPU usage.
- **Multi-Tab Interface:** Intuitive navigation across Overview, Performance, Processes, and About.
- **Windows GUI Subsystem:** Clean executable launch without background console windows (`windows_subsystem = "windows"`).

---

## [0.1.0] - 2025-12-15

### Added
- Initial project prototype and proof-of-concept system monitor application.

---

[3.7.5]: https://github.com/Xenonesis/sysmon/compare/v3.7.2...v3.7.5
[3.7.3]: https://github.com/Xenonesis/sysmon/compare/v3.7.2...v3.7.3
[3.7.2]: https://github.com/Xenonesis/sysmon/compare/v3.7.0...v3.7.2
[3.7.0]: https://github.com/Xenonesis/sysmon/compare/v2.6.1...v3.7.0
[2.6.1]: https://github.com/Xenonesis/sysmon/compare/v2.6.0...v2.6.1
[2.6.0]: https://github.com/Xenonesis/sysmon/compare/v2.5.0...v2.6.0
[2.5.0]: https://github.com/Xenonesis/sysmon/compare/v2.4.0...v2.5.0
[2.4.0]: https://github.com/Xenonesis/sysmon/compare/v2.3.0...v2.4.0
[2.3.0]: https://github.com/Xenonesis/sysmon/compare/v2.2.0...v2.3.0
[2.2.0]: https://github.com/Xenonesis/sysmon/compare/v1.4.0...v2.2.0
[1.4.0]: https://github.com/Xenonesis/sysmon/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/Xenonesis/sysmon/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/Xenonesis/sysmon/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/Xenonesis/sysmon/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/Xenonesis/sysmon/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/Xenonesis/sysmon/releases/tag/v0.1.0
