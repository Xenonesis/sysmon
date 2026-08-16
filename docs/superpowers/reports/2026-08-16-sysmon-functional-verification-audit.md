# SysMon v3.7.0 — Functional Verification Audit

Date: 2026-08-16
Commit under test: `1bdd786` (Refactor main.rs into engine and models … bump version to 3.7.0)
Environment: Windows x64, Rust 1.96.0 / cargo 1.96.0 (MSVC), standard-user context
Auditor: automated functional verification (build gates, unit tests, smoke tests, runtime launch, static review)

---

## 1. Feature inventory (from README.md / USER_GUIDE.md / Cargo.toml)

### Core functionalities
| ID | Feature | Source of requirement |
|----|---------|----------------------|
| F01 | Release build produces `system-monitor.exe` | README "Build from source" |
| F02 | Format gate `cargo fmt --all -- --check` | README "Quality checks", CI |
| F03 | Lint gate `cargo clippy --locked --all-targets -- -D warnings` | README "Quality checks", CI |
| F04 | Unit test suite `cargo test --locked --all-targets` | README "Quality checks", CI |
| F05 | TelemetryHub: provider registration, latest-snapshot delivery, history stats | README "Unified TelemetryHub" |
| F06 | Multi-resolution history (60s / 5m / 30m / 1h, min/max/avg, bounded) | README, USER_GUIDE "Performance history" |
| F07 | Provider abstraction: sysinfo / NVML / WMI / windows_gpu, graceful degradation | USER_GUIDE "Providers and GPU behavior" |
| F08 | Counter-rate computation safe across resets and zero-elapsed | stability spec (rates) |
| F09 | Diagnostics: evidence-based findings with severity + confidence | README "Explainable diagnostics" |
| F10 | Process list: filter (name/PID), sort (mem/CPU/name), tree build, kill-tree ordering | USER_GUIDE "Processes" |
| F11 | Guarded actions: risk level, reversibility/Undo classification, audit records | README "Guarded actions" |
| F12 | Power-plan GUID parsing/validation | USER_GUIDE "RAM cleaner and power plans" |
| F13 | Settings persistence: save/load round-trip, defaults, clamping | USER_GUIDE "Settings" |
| F14 | RAM cleaner: stop conditions (target/budget/empty), exclusions, 5-pass bound | USER_GUIDE "RAM cleaner" |
| F15 | Updater: HTTPS + repo URL validation, `.exe`-only, size caps | USER_GUIDE "Updates" |
| F16 | Updater: Authenticode + pinned publisher thumbprint enforcement | USER_GUIDE "Updates", README "Secure updates" |
| F17 | Updater: installed-only check (Program Files / uninstall registry key) | updater.rs contract |
| F18 | Telemetry scheduler: per-provider intervals, background-mode slowdown | README (tray/background mode) |
| F19 | Hardware telemetry smoke test (real providers on this machine) | README (CI note on ignored smoke tests) |
| F20 | GUI application launches and stays running | USER_GUIDE "Getting started" |
| F21 | Single-instance enforcement (named mutex) | main.rs contract |
| F22 | Crash reporting (panic hook → crash file + MessageBox) and rolling file logs | main.rs contract |
| F23 | Installer (Inno Setup) AppId consistent with updater pinned AppId | installer.iss / updater.rs |

### Secondary functionalities (static/deferred verification)
| ID | Feature | Verification mode |
|----|---------|-------------------|
| S01 | 14 UI views (Overview … About) | Static: all page modules present and compiled |
| S02 | Tray menu (show/pause/procman/RAM clean/power plans) + Ctrl+Alt+C hotkey | Static only — needs interactive session |
| S03 | Alerts: thresholds, dedup by key, 5-min notification cooldown, sounds | Logic reviewed; needs runtime trigger |
| S04 | CSV/JSON export of snapshot | Static only — needs interactive session |
| S05 | Session recording (opt-in JSONL) | Static only — needs interactive session |
| S06 | Service start/stop/restart actions | Static only — needs elevation + interactive |
| S07 | Startup manager enable/disable | Static only — needs interactive |
| S08 | Actual update download + install flow | Not testable (requires signed release published) |
| S09 | Installer build (`create-installer.ps1` / Inno Setup) | Not run (Inno Setup not invoked in this audit) |
| S10 | Signed release workflow / provenance / SBOM | CI-side; not locally testable |

---

## 2. Test procedures (repeatable)

All commands run from the repository root on Windows (PowerShell):

```powershell
# Gate 1 — toolchain
cargo --version; rustc --version

# Gate 2 — formatting
cargo fmt --all -- --check

# Gate 3 — lint
cargo clippy --locked --all-targets -- -D warnings

# Gate 4 — release build
cargo build --locked --release --bin system-monitor

# Gate 5 — unit tests (deterministic)
cargo test --locked --all-targets

# Gate 6 — hardware smoke test (machine-dependent)
cargo test --locked --bin system-monitor -- --ignored hardware_telemetry_smoke_test

# Gate 7 — provider diagnostic utilities
cargo run --locked --bin test_wmi
cargo run --locked --bin test_cpu_temp

# Gate 8 — runtime launch stability (15 s soak)
$p = Start-Process -FilePath "target\release\system-monitor.exe" -PassThru
Start-Sleep -Seconds 15
if (-not $p.HasExited) { "ALIVE"; Stop-Process -Id $p.Id -Force } else { "EXITED code=$($p.ExitCode)" }
```

Acceptance criteria: every gate must exit 0; Gate 8 must report ALIVE after 15 s.

---

## 3. Results

| Gate | Feature | Result | Detail |
|------|---------|--------|--------|
| 1 | Toolchain | PASS | cargo/rustc 1.96.0 ≥ required 1.85 |
| 2 | F02 formatting | **FAIL** | Diffs in `src/app/models.rs`, `src/main.rs`, `src/monitoring/engine.rs` (import ordering, trailing blank lines, line-width wraps) |
| 3 | F03 clippy | **FAIL** | 24 errors, all `unused_imports` (promoted to errors by `-D warnings`), concentrated in `main.rs` and `engine.rs` after the 3.7.0 refactor |
| 4 | F01 release build | PASS | Finished in 2m31s; 24 warnings (same unused imports) |
| 5 | F04 unit tests | PASS | **46 passed, 0 failed, 1 ignored** (0.11 s) |
| 6 | F19 hardware smoke | PASS | `hardware_telemetry_smoke_test` ok (3.33 s) — real sysinfo/NVML/WMI providers deliver snapshots on this machine |
| 7a | WMI service query util | **FAIL** | `test_wmi`: `HResultError { hres: -2147217406 }` (0x80041002 WBEM_E_NOT_FOUND) |
| 7b | WMI thermal util | **FAIL** | `test_cpu_temp`: panics — `query failed: HResultError { hres: -2147217405 }` (0x80041003 WBEM_E_ACCESS_DENIED; `MSAcpi_ThermalZoneTemperature` requires elevation) |
| 8 | F20 GUI launch | **FAIL (environment-confounded)** | Reproducible abort `0xC0000409` (Rust panic with `panic = "abort"`) 6–20 s after launch; panic hook fired (crash-report write attempted). See §5 |

### Unit-test coverage map (Gate 5 — all PASS)
- F05/F06/F18: `telemetry::tests::hub_registers_provider`, `hub_publishes_latest_data_and_history`, `ring_buffer::*` (7 tests), `scheduler::*` (4 tests)
- F08: `monitoring::rates::*` (4 tests incl. counter-reset and zero-elapsed)
- F09: `diagnostics::tests::high_cpu_produces_bottleneck_finding`, `quiet_snapshot_is_healthy`
- F10: `processes::tests::*` (9 tests: filter, sort, tree, kill-order incl. cycle safety)
- F11: `app::actions::tests::kill_tree_is_critical_and_irreversible`, `suspend_has_resume_undo`
- F12: `power::tests::rejects_malformed_guid`, `parses_canonical_guid`
- F13/F14: `tests::save_and_load_round_trip`, `validation_clamps_user_ranges`, `settings_defaults_and_clamps`, `exclusion_matches_case_insensitively`, `stop_conditions_cover_target_budget_and_empty`, `test_battery_info_default`
- F15/F16: `updater::tests::*` (4 tests: asset URL allow/deny, pinned-signer accept, tamper/wrong-signer/unsigned reject)
- Channel plumbing: `app::tests::monitoring_channel_delivers_shutdown`

### Static verification results
- F23 PASS: `installer.iss` AppId `{3F2A9C41-…-5D8E4F1A7B62}` matches `INSTALLER_APP_ID` in [updater.rs](file:///c:/Users/Acer/Desktop/sysmon/src/updater.rs#L31).
- F15/F16 PASS (code review): HTTPS-only + repo-prefix + `.exe` URL validation, 100 MB installer / 1 MB metadata caps, Authenticode status + thumbprint pinning with explicit unsigned rejection ([updater.rs](file:///c:/Users/Acer/Desktop/sysmon/src/updater.rs#L261-L360)).
- F21 PASS (code review): `Global\SystemMonitorSingleInstance` named mutex with user-visible MessageBox and clean exit(0) ([main.rs](file:///c:/Users/Acer/Desktop/sysmon/src/main.rs#L1390-L1427)).
- F22 PASS (code review): panic hook writes crash file + MessageBox; rolling daily file logs via tracing-appender ([main.rs](file:///c:/Users/Acer/Desktop/sysmon/src/main.rs#L1429-L1517)).
- S01 PASS (compile-level): all 14 page modules + process-manager window compile into the release binary.
- F17 PASS (code review): update check short-circuits for non-installed (portable/dev) builds.

---

## 4. Failure log (detailed)

### FAIL-1 — Formatting gate (Gate 2)
- Command: `cargo fmt --all -- --check` → exit 1
- Files: `src/app/models.rs` (import order, double blank lines, trailing newline), `src/main.rs` (import order/grouping), `src/monitoring/engine.rs` (import order, long-line wraps, blank line)
- Error sample: `Diff in src\app\models.rs:7: +use crate::telemetry::TelemetrySnapshot;` (cfg-import ordering)

### FAIL-2 — Clippy gate (Gate 3)
- Command: `cargo clippy --locked --all-targets -- -D warnings` → exit 101
- 24 × `error: unused import(s)`, e.g.:
  - `src\main.rs:35` — `Disks`, `Networks`, `Pid`, `System` from sysinfo
  - `src\main.rs:40-42` — `CheckMenuItem`, `Menu`, `MenuItem`, `Submenu`, `TrayIconBuilder`, `TrayIcon`
  - `src\monitoring\engine.rs` — `Mutex`, `RwLock`, `Arc`, `thread`, `Duration`, `Pid`, `wmi::COMLibrary`, `wmi::Variant`, `privilege`, `startup`, `error`, `ProcessInfo`, `BootDiagnostics`, `StartupItem`, …
- Root cause: the 3.7.0 refactor (`1bdd786`) moved code out of `main.rs` into `engine.rs`/`models.rs` but left the original `use` statements behind. CI (`rust-ci.yml`) runs this exact gate, so **CI would be red on this commit**.

### FAIL-3 — WMI diagnostic utilities (Gate 7)
- `test_wmi`: `Error: HResultError { hres: -2147217406 }` (0x80041002, WBEM_E_NOT_FOUND) for `Win32_Service` query in this session.
- `test_cpu_temp`: `panicked at src\bin\test_cpu_temp.rs:9:10: query failed: HResultError { hres: -2147217405 }` (0x80041003, WBEM_E_ACCESS_DENIED — `ROOT\WMI\MSAcpi_ThermalZoneTemperature` requires administrator rights).
- Note: the main application treats these as optional and degrades gracefully (historical log 2026-08-13 shows `Provider poll failed provider="wmi"` warnings with the app continuing normally). The *utilities* however use `.unwrap()/.expect()` and abort instead of reporting.

### FAIL-4 — GUI runtime launch (Gate 8)
- Observed: release binary aborts with `0xC0000409` (STATUS_STACK_BUFFER_OVERRUN — Rust's panic-abort code under `panic = "abort"`) between 6 and 20 s after launch, across repeated runs. The custom panic hook fired each time (attempted `crash_*.log` writes observed).
- Confound: the audit terminal runs inside a sandbox that **denies writes** to `C:\Users\Acer\AppData\Local\Xenonesis\SystemMonitor\data\` (verified: probe write returned "Access denied"). The app's rolling log appender and crash-report file both target that directory; `tracing-appender`'s default non-blocking worker panics when its file write fails. With `LOCALAPPDATA` redirected to a writable directory the process survived the 18 s soak.
- Verdict: **cannot be classified as a product defect from inside this sandbox.** The panic message itself is unrecoverable here because the crash-report write is also blocked. Prior runs on this machine (logs 2026-06-20 … 2026-08-13, versions 2.7.0–3.5.0) show normal operation and graceful shutdowns.
- Re-verification required outside the sandbox (see §6, R1).

---

## 5. Severity categorization of non-functional features

| Severity | ID | Issue | Impact |
|----------|----|-------|--------|
| **Critical** | FAIL-4 | GUI process aborts 6–20 s after launch in the audit environment (panic-abort; root cause unconfirmed — sandbox-blocked log writes are the leading hypothesis) | If reproducible outside the sandbox, the application is unusable; blocks release of 3.7.0 |
| **Major** | FAIL-2 | Clippy gate fails with 24 unused-import errors | CI quality gate red; README/CI contract ("zero warnings") violated; would block the signed release workflow |
| **Major** | FAIL-1 | rustfmt gate fails in 3 files | CI gate red; same blocking effect |
| **Minor** | FAIL-3a | `test_wmi` utility errors (WBEM_E_NOT_FOUND) in standard-user session | Debug utility only; main app degrades gracefully |
| **Minor** | FAIL-3b | `test_cpu_temp` utility panics instead of printing an error (thermal WMI namespace needs admin) | Debug utility only; `.expect()` should be graceful error reporting |

---

## 6. Summary

### Fully functional (verified this session)
1. Release build (F01) — clean compile, 2m31s.
2. Deterministic unit test suite (F04) — 46/46 pass.
3. TelemetryHub registration/publication/history (F05, F06), scheduler incl. background mode (F18).
4. Counter-rate safety (F08), diagnostics engine (F09).
5. Process filter/sort/tree/kill-order logic (F10); action risk + Undo classification (F11).
6. Power GUID validation (F12); settings persistence & clamping (F13); RAM-cleaner stop conditions (F14).
7. Updater URL/size/Authenticode/pinned-thumbprint security logic (F15, F16, F17) — unit-tested + code-reviewed.
8. Hardware telemetry on this machine (F19) — real providers deliver data.
9. Installer↔updater AppId consistency (F23); single-instance, crash-hook and logging architecture (F21, F22) — code-reviewed.

### Non-functional / failing
1. **Critical (pending confirmation):** GUI launch stability (FAIL-4) — reproducible panic-abort in sandboxed runs; must be re-verified with write access to the app-data directory.
2. **Major:** CI quality gates — `cargo fmt --check` and `cargo clippy -D warnings` both fail (FAIL-1, FAIL-2) due to leftover imports from the 3.7.0 refactor.
3. **Minor:** WMI debug utilities fail/panic on this machine without elevation (FAIL-3).

### Not verifiable in this session (deferred to human QA)
Interactive GUI behavior of the 14 views, tray menu, global hotkey, desktop notifications, alert triggering, CSV/JSON export dialogs, session recording, service/startup mutations, real update download, installer compilation, and the signed release workflow.

---

## 7. Troubleshooting recommendations

- **R1 (Critical — FAIL-4):** Launch `target\release\system-monitor.exe` from a normal (non-sandboxed) shell so `%LOCALAPPDATA%\Xenonesis\SystemMonitor\data\` is writable; wait 30 s. If it survives, the crash was a sandbox artifact (log-write panic) — still consider hardening: `tracing_appender::non_blocking`'s worker panics on write failure; route it through a `WorkerBuilder` with a non-panicking error handler. If it crashes, read the newly written `crash-reports\crash_*.log` for the panic location and fix.
- **R2 (Major — FAIL-2):** Remove the 24 unused imports (`cargo fix --bin system-monitor -p system-monitor` will apply all suggestions automatically), then re-run Gate 3.
- **R3 (Major — FAIL-1):** Run `cargo fmt --all` and commit; re-run Gate 2.
- **R4 (Minor — FAIL-3):** In `src/bin/test_cpu_temp.rs` and `src/bin/test_wmi.rs`, replace `.unwrap()/.expect()` with `match` + printed error + non-zero exit, and document that the thermal namespace requires an elevated prompt.
- **R5 (Process):** Re-run Gates 2–8 after R1–R4; the gate commands in §2 are directly reusable for re-verification of each fix.

---

## Appendix — raw evidence
- fmt diff files: `src/app/models.rs`, `src/main.rs`, `src/monitoring/engine.rs`
- clippy: 24 × `unused_imports`, exit 101
- build: `Finished release profile [optimized] target(s) in 2m 31s`, 24 warnings
- tests: `test result: ok. 46 passed; 0 failed; 1 ignored`
- smoke: `test telemetry::tests::hardware_telemetry_smoke_test ... ok` (3.33 s)
- test_wmi: `Error: HResultError { hres: -2147217406 }`
- test_cpu_temp: `panicked at src\bin\test_cpu_temp.rs:9:10: query failed: HResultError { hres: -2147217405 }`
- launch: `PROCESS EXITED EARLY code=-1073740791` (0xC0000409); crash-hook writes attempted at 01:54:31, 01:55:09, 01:59:25, 02:02:53
- sandbox denial: `Access to the path 'C:\Users\Acer\AppData\Local\Xenonesis\SystemMonitor\data\logs\…' is denied`
