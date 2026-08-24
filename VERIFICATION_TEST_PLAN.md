# SysMon v3.7.0 — Functional Verification Test Plan

Repeatable procedures for re-verifying every documented feature. Run from the repo
root (`C:/Users/Acer/Desktop/sysmon`) on Windows 10/11 x64. Each case lists exact
commands, acceptance criteria, and where evidence is recorded.

**Environment used for the 2026-08-16 audit**

| Item | Value |
| --- | --- |
| OS | Windows 11 Home (build 10.0.26200) |
| CPU | Intel Core i9-12900H |
| GPU | Intel Iris Xe + NVIDIA GeForce RTX 3060 Laptop |
| Toolchain | rustc/cargo 1.96.0 stable-x86_64-pc-windows-msvc |
| Linker | lld-link (`.cargo/config.toml`) — present |
| Inno Setup | `C:\Program Files (x86)\Inno Setup 6\iscc.exe` — present |

> CI pins Rust **1.85.0**; local audit used 1.96.0. Note any rustfmt/clippy deltas
> this causes when comparing local vs CI results (see Report §6, finding F-02).

---

## 0. Feature inventory (source of truth)

Derived from `README.md`, `USER_GUIDE.md`, `CHANGELOG.md` (3.7.0), `DESIGN.md`,
`docs/release-rule.md`, and the module tree under `src/`.

### Core telemetry & engine
| ID | Feature | Source |
| --- | --- | --- |
| C-01 | TelemetryHub multi-tier delivery (60 FPS UI decoupled from 1–5 Hz sampling) | README, CHANGELOG |
| C-02 | Provider abstraction (`TelemetryProvider` trait) | CHANGELOG |
| C-03 | SysinfoProvider (CPU/memory/portable metrics) | USER_GUIDE |
| C-04 | NvmlProvider (NVIDIA temp/power/clocks/fan/VRAM) | USER_GUIDE |
| C-05 | WmiProvider (hardware identity, thermals) | USER_GUIDE |
| C-06 | WindowsGpuProvider (vendor-neutral GPU engine/memory counters) | README, USER_GUIDE |
| C-07 | Multi-resolution ring buffers (60s/5m/30m/1hr; min/max/avg/peak) | README, CHANGELOG |
| C-08 | PollingScheduler with background/tray 5× throttle | CHANGELOG |
| C-09 | Graceful provider error isolation (no crash on provider failure) | CHANGELOG |

### Diagnostics & actions
| ID | Feature | Source |
| --- | --- | --- |
| D-01 | Evidence-based diagnostics (severity, evidence, recommendation, confidence) | README, USER_GUIDE |
| D-02 | Opt-in JSONL session recording (Start/Stop recording) | USER_GUIDE |
| A-01 | Guarded action plans (risk preview, elevation disclosure, confirmation) | README, USER_GUIDE |
| A-02 | Process actions: kill, kill-tree, suspend, resume, priority | USER_GUIDE |
| A-03 | Service actions: start/stop/restart | USER_GUIDE |
| A-04 | RAM cleaner manual + automatic (bounded, 5-pass, exclusions, idle-only) | USER_GUIDE |
| A-05 | Power-plan selection/apply | USER_GUIDE |
| A-06 | Append-only action audit trail (JSONL) + Undo for reversible actions | USER_GUIDE |

### UI views (14 modules)
| ID | View |
| --- | --- |
| U-01 | Overview |
| U-02 | Performance |
| U-03 | Diagnostics |
| U-04 | CPU Cores |
| U-05 | Processes |
| U-06 | Services |
| U-07 | Startup Manager |
| U-08 | Storage |
| U-09 | Network |
| U-10 | RAM Cleaner |
| U-11 | Alerts |
| U-12 | System Information |
| U-13 | Settings |
| U-14 | About |

### App services & integration
| ID | Feature | Source |
| --- | --- | --- |
| S-01 | Settings persistence (JSON, validated/clamped) | USER_GUIDE, CHANGELOG |
| S-02 | Theme dark/light switching | CHANGELOG |
| S-03 | Alerts/notifications (CPU/mem/GPU-temp/disk thresholds, dedup) | USER_GUIDE |
| S-04 | CSV and JSON snapshot export | USER_GUIDE |
| S-05 | Tray mode / background polling reduction | USER_GUIDE |
| S-06 | Single-instance guard | main.rs |
| S-07 | Daily rotating logs + crash-report directory | main.rs |
| S-08 | Global hotkey (RAM clean) | CHANGELOG (7464c0c) |
| S-09 | Startup Manager boot evidence/signature/impact | USER_GUIDE |

### Updates, supply chain, CI
| ID | Feature | Source |
| --- | --- | --- |
| P-01 | Updater: HTTPS + repo asset validation, bounded download | README, release-rule |
| P-02 | Updater: SHA-256 checksum verification against published `.sha256` asset | README, release-rule |
| P-03 | Release workflow (checksum, SBOM, provenance; no signing secrets) | README, SECURITY, release-rule |
| P-04 | Windows CI quality gates (lockfile, fmt, clippy -D warnings, test, build) | README, rust-ci.yml |
| P-05 | Installer build via Inno Setup (`installer.iss`) | build.md rule |
| P-06 | Docs site dynamic download resolver (GitHub Releases API) | release-rule |

---

## 1. Static & build verification

### TC-1.1 Formatting gate
```powershell
cargo fmt --all -- --check
```
- **Pass:** exit 0, no diffs.
- **Fail:** exit 1 with `Diff in …` blocks. Record each file.

### TC-1.2 Strict lint gate
```powershell
cargo clippy --locked --all-targets -- -D warnings
```
- **Pass:** exit 0.
- **Fail:** exit 101; capture `error:` count and file distribution
  (`… | grep -E "^\s+-->" | sort | uniq -c`).

### TC-1.3 Lockfile integrity
```powershell
cargo metadata --locked --format-version 1 --no-deps
```
- **Pass:** exit 0 (committed `Cargo.lock` resolves).

### TC-1.4 Release build
```powershell
cargo build --locked --release --bin system-monitor
```
- **Pass:** `Finished release profile`; binary at `target\release\system-monitor.exe`.
- Record warning count.

---

## 2. Automated test suite

### TC-2.1 Unit + integration tests
```powershell
cargo test --locked --all-targets
```
- **Pass:** `test result: ok. N passed; 0 failed`. Record pass/fail/ignored counts.
- Hardware smoke test `telemetry::tests::hardware_telemetry_smoke_test` is
  **ignored by design** (device-dependent). Do not count it as a failure.

### TC-2.2 Provider smoke binaries (device-dependent)
```powershell
cargo build --locked --release --bins
.\target\release\test_wmi.exe        # Win32_Service query via wmi crate
.\target\release\test_cpu_temp.exe   # ROOT\WMI MSAcpi_ThermalZoneTemperature
```
- Record stdout and decoded HRESULT. Reference decodings:
  - `0x80010119` = `RPC_E_TOO_LATE` (CoInitializeSecurity already called in-process)
  - `0x80041002` = `WBEM_E_NOT_FOUND`
  - `0x80041003` = `WBEM_E_ACCESS_DENIED` (ROOT\WMI thermal needs admin)
- **Machine WMI health control** (must pass for provider failures to be app bugs):
```powershell
(Get-CimInstance Win32_Service | Measure-Object).Count   # expect > 0
Get-CimInstance Win32_VideoController | Select Name       # expect GPU list
```

---

## 3. Runtime verification

### TC-3.1 Launch & stability
```powershell
.\target\release\system-monitor.exe
```
- Let run ≥ 60 s. **Pass:** process stays alive, window
  `System Monitor v3.8.0` visible, no crash dialog, RSS stable.
- Capture window screenshot as evidence (see §3.4).

### TC-3.2 Single-instance guard (S-06)
- Launch a second copy while the first runs.
- **Pass:** second instance shows "System Monitor is already running" and does not
  open a second main window.

### TC-3.3 Log & persistence inspection
```powershell
Get-ChildItem "$env:APPDATA\Xenonesis\SystemMonitor\config"
Get-ChildItem "$env:LOCALAPPDATA\Xenonesis\SystemMonitor\data" -Recurse
Get-Content "$env:LOCALAPPDATA\Xenonesis\SystemMonitor\data\logs\system-monitor.log.*" -Tail 40
```
- **S-01 Pass:** `settings.json` present, valid JSON, values within clamped ranges.
- **S-07 Pass:** daily `system-monitor.log.YYYY-MM-DD` written; `crash-reports\` exists.
- **A-06 Pass:** `action-audit.jsonl` append-only JSONL with timestamp/action/risk/result.
- **C-09 evidence:** count `Provider poll failed` lines per provider:
```powershell
Select-String -Path "$env:LOCALAPPDATA\Xenonesis\SystemMonitor\data\logs\system-monitor.log.*" `
  -Pattern 'provider="([a-z_]+)"' | ForEach-Object { $_.Matches[0].Groups[1].Value } |
  Group-Object | Sort-Object Count -Descending
```
  App must remain alive regardless of provider failure count.

### TC-3.4 UI evidence capture
- Screenshot the main window (title `System Monitor v3.8.0`). Walk all 15 modules
  (U-01…U-15); for each record: renders, shows live data, no blank/error region.
- Verify DESIGN.md compliance while walking views (no Inter font, no pure black,
  no gradients/emoji, mono numerals).

### TC-3.5 Feature exercise (manual, per view)
| Case | Action | Acceptance |
| --- | --- | --- |
| TC-3.5a | Diagnostics → guided diagnosis (D-02) | 15-sample baseline, reproduce phase, JSONL under `data\sessions\`, ranked evidence and contributor link |
| TC-3.5b | Processes → kill/suspend/resume a test process (A-02) | Risk/elevation dialog shown; audit row written; Undo offered for suspend |
| TC-3.5c | Services → start/stop/restart (A-03) | Confirmation dialog; state changes; audit row |
| TC-3.5d | RAM Cleaner → manual clean (A-04) | Runs, logs `RAM clean complete freed_mb=…`; audit row |
| TC-3.5e | Power plan change (A-05) | Confirmation; plan applied |
| TC-3.5f | Settings → change refresh/theme, restart (S-01,S-02) | Persisted across restart; theme switches |
| TC-3.5g | Alerts thresholds (S-03) | Threshold saved; notification dedup on repeat trigger |
| TC-3.5h | Export CSV/JSON (S-04) | File written with current snapshot |
| TC-3.5i | Minimize to tray (S-05) | Polling interval increases (background mode) |

> Destructive actions (kill/stop) must target a disposable test process/service only.

---

## 4. Update, signing & supply-chain verification

### TC-4.1 Published release asset inventory
```powershell
curl -s https://api.github.com/repos/Xenonesis/sysmon/releases/latest |
  jq -r '.tag_name, (.assets[] | "\(.name)  \(.size)  \(.content_type)")'
```
- **Pass (per release-rule §Release steps):** installer `.exe`, `.sha256`, SPDX
  `.spdx.json` present, **and** build provenance attached.

### TC-4.2 Checksum integrity
```powershell
curl -sL -o sm.exe  https://github.com/Xenonesis/sysmon/releases/download/v3.7.0/SystemMonitor-3.7.0-setup.exe
curl -sL -o sm.sha  https://github.com/Xenonesis/sysmon/releases/download/v3.7.0/SystemMonitor-3.7.0-setup.exe.sha256
(Get-FileHash sm.exe -Algorithm SHA256).Hash.ToLowerInvariant()
Get-Content sm.sha
```
- **Pass:** computed SHA-256 equals the `.sha256` value.

### TC-4.3 Release integrity without Authenticode (P-02/P-03)
```powershell
# The release contract no longer requires an Authenticode signature.
# Integrity comes from the published SHA-256 checksum + GitHub build provenance.
gh attestation verify sm.exe --repo Xenonesis/sysmon   # or check the release's Attestations tab
```
- **Pass:** build provenance attestation present for the installer, issued by the
  `Xenonesis/sysmon` release workflow; computed SHA-256 equals the `.sha256` value
  (TC-4.2).
- **Fail:** no attestation, or checksum mismatch.

### TC-4.4 Updater acceptance logic
```powershell
cargo test --locked updater:: -- --nocapture
```
- **Pass:** `verifies_installer_checksum`, `parses_sha256_checksum_file`,
  `accepts_expected_release_asset`, `rejects_untrusted_asset_urls`,
  `accepts_expected_checksum_asset`, `rejects_untrusted_checksum_urls` all green.
- Cross-check: a tampered installer (any byte changed) must be rejected by
  `verify_sha256`; a release without a `.sha256` asset must not be offered as an
  update (`check_for_updates` requires both URLs).

### TC-4.5 Local installer build (P-05)
```powershell
& 'C:\Program Files (x86)\Inno Setup 6\iscc.exe' /DAppVersion=3.7.0 installer.iss
```
- **Pass:** `Successful compile`; `downloads\SystemMonitor-3.7.0-setup.exe` created.
- Per `.agents/rules/build.md`: deliverable is always the `*-setup.exe`, never the
  bare `system-monitor.exe`.

### TC-4.6 Docs download resolver (P-06)
- Open `docs/index.html` (or the Pages URL). Confirm `#latestVersion` and download
  buttons resolve via the GitHub Releases API; confirm no installer `.exe` is
  committed under `docs/downloads/` (`.gitignore` must block it).

---

## 5. CI verification

### TC-5.1 CI status on the release commit
```powershell
curl -s "https://api.github.com/repos/Xenonesis/sysmon/actions/runs?per_page=10" |
  jq -r '.workflow_runs[] | "\(.name) | \(.head_sha[0:7]) | \(.conclusion)"'
```
- For each failing run, fetch step-level conclusions:
```powershell
curl -s "https://api.github.com/repos/Xenonesis/sysmon/actions/runs/<RUN_ID>/jobs" |
  jq -r '.jobs[] | (.name+": "+.conclusion), (.steps[] | "   "+.name+": "+.conclusion)")'
```
- **Pass:** `Rust CI` quality job green on the shipped commit. **Fail:** any of
  lockfile/fmt/clippy/test/build red while a release was still published.

---

## 6. Result recording template

For every case record: `Case ID | Feature | Command/Step | Expected | Actual | PASS/FAIL | Evidence path`.
Aggregate into `VERIFICATION_REPORT.md`. Severity rubric:

- **Critical** — breaks a headline/differentiating capability or the secure-update
  contract; or ships a release that fails the project's own mandatory gates.
- **Major** — a documented core feature is non-functional at runtime, even if the
  app degrades gracefully.
- **Minor** — hygiene/limited-impact issues; does not block primary use.
