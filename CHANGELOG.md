# Changelog

All notable changes to System Monitor will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.5.0] - 2026-08-13

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
- **Signed supply-chain workflow:** Production certificate requirement, signer pinning, signed binaries, checksums, SPDX SBOM and GitHub provenance.
- **Windows CI:** Locked dependency resolution, formatting, strict Clippy, tests and release compilation on Rust 1.85.

### Changed
- The production application now consumes TelemetryHub's replaceable latest snapshot; legacy polling remains only for richer process, disk, network and system-detail views during staged migration.
- Updater downloads now have timeouts, bounded reads, content-length validation, escaped literal paths and cleanup after verification failure.
- Automatic RAM cleanup is bounded and included in the persistent action audit trail.
- `Cargo.lock` is committed for reproducible application builds.

### Fixed
- History minimum and maximum values no longer retain samples evicted from the active window.
- Snapshot delivery cannot fall behind through a FIFO backlog.
- Release instructions now match the installer asset and certificate contract.

## [2.6.0] - 2026-01-20

### Added
- **Storage Monitoring Tab** - Monitor all storage devices with capacity and usage
- **Network Monitoring Tab** - Real-time network interface statistics with download/upload rates
- **System Information Tab** - Complete system specifications and details
- **Settings Panel** - Configurable application settings with persistence
- **Theme Support** - Dark mode and Light mode with instant switching
- **Tools Menu** - New menu for export and utility functions
- **Persistent Settings** - Settings saved to JSON config file
- **Configurable Refresh Interval** - Adjust update frequency (1-10 seconds)
- **Notification System** (Experimental) - Alert thresholds for CPU, memory, and temperature
- Historical performance graphs for CPU, Memory, and GPU (last 2 minutes)
- Multi-tab navigation with 7 tabs total
- Quick Stats sidebar panel
- Professional menu bar with View, Tools, and Help menus
- Color-coded progress bars and indicators
- Comprehensive documentation (9 files, 93+ KB)

### Changed
- Upgraded from terminal-based to full GUI application
- Increased window size to 1100x800 (from 900x800)
- Enhanced UI with better spacing and organization
- Improved navigation with sidebar tabs
- Better color coding system throughout
- More efficient data collection

### Technical
- Updated to Rust 2021 edition
- Added dependencies: serde, serde_json, directories, notify-rust
- Implemented settings persistence system
- Added disk and network monitoring via sysinfo
- Multi-threaded architecture (GUI + monitoring threads)
- Hardware-accelerated rendering with egui
- Build optimizations (LTO, strip)

### Documentation
- README.md - Updated with all new features
- NEW_FEATURES.md - Detailed feature breakdown
- WHATS_NEW.md - Complete changelog
- USER_GUIDE.md - Comprehensive user manual (16.6 KB)
- FEATURE_SHOWCASE.md - Visual tour (24.6 KB)
- INSTALLATION_GUIDE.md - Detailed installation instructions
- QUICK_START.md - Quick start guide
- COMPLETE_SUMMARY.md - Transformation overview
- GUI_FEATURES.md - Technical feature details
- CHANGELOG.md - This file

### Performance
- Memory usage: ~35-40 MB (up from ~30 MB)
- CPU impact: Still < 1%
- Update frequency: Configurable (default 2 seconds)
- Startup time: < 1 second

## [0.1.0] - 2024-12-14

### Added
- Initial GUI application
- CPU usage monitoring
- Memory (RAM) monitoring
- GPU monitoring (NVIDIA only)
- Process monitoring (top 15 by memory)
- Basic multi-tab layout (4 tabs)
- Overview, Performance, Processes, About tabs
- Color-coded indicators
- Real-time updates every 2 seconds
- Build and install scripts
- Basic documentation

---

## Future Releases

### [1.1.0] - Planned
- Export data to JSON/CSV
- Process management (kill/suspend)
- Complete notification system
- Historical data export
- Custom alert rules
- Network usage graphs
- Disk I/O monitoring

### [1.2.0] - Planned
- System tray icon
- Minimize to tray
- Auto-start with Windows
- Multiple configuration profiles
- Custom dashboard layouts
- Widget system
- Plugin architecture

### [2.0.0] - Future
- Cross-platform support (Linux, macOS)
- Web dashboard
- Remote monitoring
- Database logging
- Advanced analytics
- Machine learning predictions
- API for integrations

---

[1.0.0]: https://github.com/Xenonesis/sysmon/releases/tag/v1.0.0
[0.1.0]: https://github.com/Xenonesis/sysmon/releases/tag/v0.1.0
