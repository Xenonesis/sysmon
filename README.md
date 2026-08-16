# SysMon

[![Rust CI](https://github.com/Xenonesis/sysmon/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/Xenonesis/sysmon/actions/workflows/rust-ci.yml)
[![Rust 1.85+](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/Windows-10%2F11-blue.svg)](https://www.microsoft.com/windows)
[![Version](https://img.shields.io/badge/version-3.7.1-green.svg)](CHANGELOG.md)

SysMon is a native Windows observability and diagnostics application written in Rust. It combines live CPU, memory, disk, network, process, service, startup, battery and GPU telemetry with evidence-based diagnostics and guarded system actions.

## What makes it different

- **Unified TelemetryHub:** replaceable latest-snapshot delivery keeps the UI current without a growing event backlog.
- **Vendor-neutral GPU coverage:** NVIDIA NVML plus Windows/WMI adapters and GPU performance counters for Intel, AMD and other Windows GPUs.
- **Multi-resolution history:** bounded 60-second, 5-minute, 30-minute and 1-hour windows with current, minimum, maximum and average statistics.
- **Explainable diagnostics:** findings include evidence, a recommendation and confidence instead of applying broad tweak scripts.
- **Local session recording:** opt-in JSONL capture for reproducing transient slowdowns; no automatic upload.
- **Guarded actions:** process, service, RAM and power actions show risk and administrator requirements before execution, then write a local audit record. Reversible actions offer Undo when the prior state is known.
- **Secure updates:** HTTPS/repository asset validation, bounded downloads, Authenticode validation and a build-time pinned publisher certificate.

## Main views

Overview, Performance, Diagnostics, CPU Cores, Processes, Services, Startup Manager, Storage, Network, RAM Cleaner, Alerts, System Information, Settings and About.

## Version comparison

| Capability | 1.x (2024) | 2.6.x (2026-01) | 3.7.1 (current) |
| --- | --- | --- | --- |
| GUI framework | egui / eframe | egui / eframe | egui / eframe |
| Telemetry engine | Single polling thread | Legacy polling thread | **TelemetryHub** (multi-tier, provider abstraction, background workers) |
| UI render frequency | Full poll per refresh | Full poll per refresh | **60 FPS decoupled** from 1–5 Hz hardware sampling |
| History resolution | ~2 min graphs | ~2 min graphs | **60s / 5m / 30m / 1hr** ring buffers with min/max/avg/peak |
| GPU support | NVIDIA only (NVML) | NVIDIA only (NVML) | **Vendor-neutral** — NVML + Windows/WMI adapters, Intel/AMD via counters |
| Diagnostics | — | — | **Evidence-based findings + confidence**, opt-in JSONL session recording |
| Action safety | — | — | **Risk preview, elevation disclosure, audit history, Undo** |
| Update verification | Plain download | HTTPS + basic checks | **Authenticode + pinned publisher thumbprint**, checksum + SBOM |
| Supply chain / CI | — | Basic scripts | **Signed release workflow**, provenance, Windows CI quality gates |
| Views / modules | 4 tabs | 7 tabs | **14 modules** |
| Themes | Single | Dark / Light | Dark / Light |

See the [changelog](CHANGELOG.md) for the complete per-version history.

## Install

Use a signed installer from [GitHub Releases](https://github.com/Xenonesis/sysmon/releases/latest). SysMon intentionally refuses automatic installation when a release is unsigned or signed by a certificate other than the publisher pinned into that build.

> Version 3.7.1 must be published through the signed release workflow before installed clients can receive it. Do not distribute a locally self-signed build as a production update.

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

## Quality checks

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release --bin system-monitor
```

CI runs the same commands on Windows. Hardware-dependent telemetry smoke tests remain ignored by default because results depend on the runner's devices and drivers; deterministic provider/hub tests run normally.

## Privacy and data locations

SysMon has no telemetry-upload feature. Settings, diagnostic sessions, logs and system-action audit records stay under the current user's Windows application-data directories. A session is written only after the user presses **Start recording** on Diagnostics.

## Release security

Tagged releases require production certificate secrets, sign both the application and installer, verify that both match the updater's pinned thumbprint, generate a SHA-256 checksum and SPDX SBOM, and attach GitHub build provenance. See [release-rule.md](docs/release-rule.md) and [SECURITY.md](SECURITY.md).

## Documentation

- [User guide](USER_GUIDE.md)
- [Changelog](CHANGELOG.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)

## License

MIT. See [LICENSE](LICENSE).
