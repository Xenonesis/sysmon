# SysMon Stability Rewrite — Verification Report

Date: 2026-08-09
Branch: `chore/sysmon-architecture-rewrite`
Baseline commit: `ac7bbd8` (spec) → head: `ea71b6f`

## Acceptance criteria

| Criterion | Evidence |
|---|---|
| No auto-update install without user click | Update check no longer calls `download_and_install_update`; only Ctrl+U / Install button path downloads, and only after `validate_asset_url` + size cap + `verify_authenticode`. |
| No hardcoded developer paths | `dbg.txt` logger removed (Task 3). `rg -i "dbg.txt|C:\\\\Users\\\\Acer" src/` → no matches. |
| No repeated `SystemMonitor::new()` in action paths | Action worker owns one `SystemMonitor`; kill/suspend/resume/priority/RAM/service/power route through `ActionCommand` → worker. RAM cleaner still creates per-op monitors in two legacy auto/manual threads. |
| Accurate rates after counter reset | `counter_rate` uses `saturating_sub` deltas; first sample `initialized: false` → zero; reset yields zero, not spike. Tests `first_sample_is_zero`, `reset_yields_zero`. |
| No duplicate active alerts | Alert dedup by `AlertInfo::key()` (metric + device identity); multi-GPU resolution checks all GPUs. |
| Real action outcomes | Service restart = stop → poll Stopped → start → poll Running (bounded). Power GUID parsing returns `Result` (invalid GUID rejected + tested). |
| App usable when optional provider fails | Worker tolerates NVML/WMI failures; provider_status populated from actual availability; battery/GPU optional. |
| No new warnings | `cargo check --all-targets` zero warnings; strict `RUSTFLAGS=-Dwarnings cargo check --all-targets` passes. |
| Uncommitted user files untouched | Main checkout `C:\Users\Acer\Desktop\sysmon` untouched; all work in sibling worktree. |

## Gates run

| Gate | Result |
|---|---|
| `cargo check --all-targets` | PASS |
| `RUSTFLAGS=-Dwarnings cargo check --all-targets` | PASS |
| `cargo test --all-targets` | PASS — 24 passed, 0 failed |
| `cargo build --release` | PASS — 1m34s |
| `create-installer.ps1 -NoZip` | PASS — dist/SystemMonitor-v2.5.0 built |
| `cargo fmt --check` | NOT RUN — rustfmt component not installed (`cargo-fmt.exe` missing); manual style pass + `git diff --check` clean instead |
| clippy | NOT RUN — `cargo-clippy.exe` not installed; zero warnings under `-Dwarnings` is the substitute gate |
| Manual smoke (launch release exe 10s, standard user context) | PASS — process ran, no crash, no stderr output, manifest `asInvoker` |

## Manual matrix (deferred items)

Full Windows 10/11 matrix (admin vs standard, NVIDIA vs non-NVIDIA, no battery, multiple
adapters, missing WMI classes, high DPI, installed vs dev build) requires hardware/env
not available in this session. Deferred to human QA; the architecture now isolates each
optional provider so failures degrade gracefully.

## Remaining known limitations (ponytail notes)

- `MonitoringCommand::RefreshNow` / `Shutdown` wired in worker, not yet triggered by UI.
- Startup item mutations still call `startup::*` directly from UI; transactional
  registry moves deferred (ActionCommand variants removed as dead until then).
- RAM cleaner legacy threads create per-op monitors; acceptable for a rare user action.
- `ActionError::NotFound`/`Unavailable`/`ProviderFailed` are typed-contract variants not
  yet constructed by any code path.
