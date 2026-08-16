# SysMon v3.7.0 — Functional Verification Report

**Audit date:** 2026-08-16 · **Commit:** `1bdd786` (= tag `v3.7.0`, = GitHub release) ·
**Environment:** Windows 11 Home 26200, i9-12900H, Intel Iris Xe + RTX 3060 Laptop,
rustc 1.96.0 stable-msvc (CI pins 1.85.0) · **Procedures:** `VERIFICATION_TEST_PLAN.md`

## 1. Executive summary

The application **builds, launches, and runs stably**; its deterministic logic
(46 unit/integration tests) is fully green; persistence, audit trail, logging,
single-instance guard, installer build, checksum integrity and the docs site all
work as documented.

However, the audit found **2 critical, 4 major, and 3 minor defects**:

- The **published v3.7.0 release is unsigned** with no build provenance, while
  README/SECURITY/CHANGELOG/release-rule all describe a signed, pinned-thumbprint,
  provenance-backed release contract. The release workflows contain **no signing
  step at all**.
- The **release workflow's quality gate is ineffective**: PowerShell does not
  propagate native command exit codes, so `cargo fmt`/`clippy` failures inside the
  `Verify source quality` step did not fail the job. The release was published
  while **Rust CI was red on the same commit** (formatting failure).
- Two of four telemetry providers (**`wmi`, `windows_gpu`**) fail at runtime with
  `RPC_E_TOO_LATE` — a double `CoInitializeSecurity` bug — on a machine whose WMI
  is provably healthy. Hardware identity, thermal-zone and vendor-neutral GPU
  counter telemetry are therefore unavailable, and `windows_gpu` retries its
  failing init **every second** (3,131 WARN lines in one day).
- The documented formatting/linting gates (`cargo fmt --check`,
  `cargo clippy -D warnings`) **fail on the shipped code** (24 unused-import
  errors across 3 files).

**Score:** 22 verified PASS · 6 FAIL · 12 NOT EXERCISED (manual-UI paths; listed
for follow-up, not counted as failures).

---

## 2. Verification method

1. Feature inventory extracted from `README.md`, `USER_GUIDE.md`, `CHANGELOG.md`,
   `DESIGN.md`, `docs/release-rule.md`, `.agents/rules/build.md`, and `src/` module
   tree (full table in `VERIFICATION_TEST_PLAN.md` §0).
2. Documented quality gates executed verbatim (README "Quality checks").
3. Release binary built and launched; process observed 11+ minutes; window
   enumerated; second-instance guard exercised; persistence/log/audit files
   inspected; provider failures tallied from live logs.
4. Machine WMI health verified independently (PowerShell CIM/DCOM queries) to
   separate environment faults from application faults.
5. Published GitHub release pulled: asset inventory, SHA-256 checksum,
   Authenticode signature, provenance attestation checked.
6. CI run/job/step conclusions fetched from the GitHub Actions API for the
   release commit.

---

## 3. Feature status matrix

Legend: ✅ PASS (verified) · ❌ FAIL (defect, see §4) · ◐ PARTIAL (logic tested,
runtime path not exercised) · ⬜ NOT EXERCISED (requires manual UI interaction)

### Core telemetry & engine
| ID | Feature | Status | Evidence |
| --- | --- | --- | --- |
| C-01 | TelemetryHub delivery | ✅ | Hub thread ran 11+ min; snapshot consumed by UI; `hub_publishes_latest_data_and_history` green |
| C-02 | Provider abstraction | ✅ | 4 providers registered; trait tests green |
| C-03 | SysinfoProvider | ✅ | No `provider="sysinfo"` failures in any log; feeds live CPU/mem |
| C-04 | NvmlProvider | ✅ | No `provider="nvml"` failures; RTX 3060 present (inferred from log absence) |
| C-05 | WmiProvider | ❌ | **F-03** — `Init failed: COM init: HRESULT 0x80010119` |
| C-06 | WindowsGpuProvider | ❌ | **F-03/F-04** — same HRESULT, retried every second |
| C-07 | Multi-resolution ring buffers | ✅ | 8 tests incl. `evicted_peak_no_longer_affects_stats` (the 3.7.0 min/max fix) |
| C-08 | PollingScheduler + background throttle | ✅ | 4 tests incl. `background_mode_increases_interval` |
| C-09 | Provider error isolation | ✅ | App survived 366 provider failures in one session, 3,131 in another; no crash |

### Diagnostics & guarded actions
| ID | Feature | Status | Evidence |
| --- | --- | --- | --- |
| D-01 | Evidence-based findings | ◐ | `high_cpu_produces_bottleneck_finding`, `quiet_snapshot_is_healthy` green; live findings view not exercised |
| D-02 | JSONL session recording | ⬜ | Requires Diagnostics UI; `data\sessions\` created lazily on first recording |
| A-01 | Risk/elevation confirmation dialogs | ⬜ | Requires UI interaction |
| A-02 | Process kill/suspend/resume/priority | ◐ | `kill_order_*`, `build_tree_maps_parents` green; live action not exercised |
| A-03 | Service start/stop/restart | ⬜ | Requires UI interaction |
| A-04 | RAM cleaner manual + automatic | ✅ | Audit JSONL shows automatic passes running every 30 s with freed-byte accounting; stop-condition/exclusion/clamp tests green (counter caveat: **F-07**) |
| A-05 | Power plan apply | ◐ | `parses_canonical_guid`, `rejects_malformed_guid` green; live apply not exercised |
| A-06 | Action audit trail + Undo | ✅ | `action-audit.jsonl`: 97 valid JSONL rows, timestamp/action/risk/result/initiator fields |

### UI views (U-01…U-14)
| Status | Detail |
| --- | --- |
| ⬜ all 14 | App launches, main window `System Monitor v3.7.0` (1115×837) visible and stable; per-view rendering/data checks require manual walkthrough (test plan TC-3.5). No egui automation hook available in this audit. |

### App services & integration
| ID | Feature | Status | Evidence |
| --- | --- | --- | --- |
| S-01 | Settings persistence + clamping | ✅ | `%APPDATA%\Xenonesis\SystemMonitor\config\settings.json` valid; all values within `validated()` ranges; round-trip test green |
| S-02 | Theme switching | ⬜ | Manual |
| S-03 | Alerts + dedup | ⬜ | Manual; no alert errors in logs |
| S-04 | CSV/JSON export | ⬜ | Manual; export code compiles |
| S-05 | Tray/background mode | ⬜ | Manual; scheduler test covers throttle logic |
| S-06 | Single-instance guard | ✅ | Second launch showed "System Monitor is already running", no second window |
| S-07 | Rotating logs + crash dir | ✅ | Daily `system-monitor.log.YYYY-MM-DD` written (UTC names); `crash-reports\` exists |
| S-08 | Global hotkey | ⬜ | Code present (`GlobalHotKeyManager`); not exercised |
| S-09 | Startup Manager evidence | ⬜ | Manual |

### Updates, supply chain, CI
| ID | Feature | Status | Evidence |
| --- | --- | --- | --- |
| P-01 | Updater URL/asset validation | ✅ | `accepts_expected_release_asset`, `rejects_untrusted_asset_urls` green |
| P-02 | Updater signature/pin logic | ✅* | Policy tests green; *runtime contract violated by the unsigned published release — **F-01** |
| P-03 | Signed release workflow | ❌ | **F-01** — published installer `NotSigned`, no provenance, no signing step in either workflow |
| P-04 | CI quality gates | ❌ | **F-02** — Rust CI red on release commit; release gate ineffective |
| P-05 | Installer build (Inno Setup) | ✅ | `iscc /DAppVersion=3.7.0` → `Successful compile`, `downloads\SystemMonitor-3.7.0-setup.exe` |
| P-06 | Docs download resolver | ✅ | All `href`/`src` resolve; `#latestVersion`=v3.7.0; no installer committed under `docs/downloads/` (gitignore enforced); Pages deploy succeeded |

### Build & test infrastructure
| Check | Status | Evidence |
| --- | --- | --- |
| Lockfile (`cargo metadata --locked`) | ✅ | Resolves clean |
| `cargo fmt --all -- --check` | ❌ | **F-05** — diffs in 3 files |
| `cargo clippy --locked --all-targets -- -D warnings` | ❌ | **F-06** — 24 errors |
| `cargo test --locked --all-targets` | ✅ | **46 passed, 0 failed, 1 ignored** (hardware smoke, ignored by design) |
| `cargo build --locked --release` | ✅ | Finished in 23.5 s; 24 warnings (same unused imports as F-06) |
| Version consistency | ✅ | Cargo.toml = tag = release = docs = README badge = installer fallback, all 3.7.0 |
| Release checksum | ✅ | Published SHA-256 `531121e2…ed6568` matches recomputed hash exactly |
| Release asset inventory | ◐ | Installer + `.sha256` + SPDX SBOM present; **provenance attestation missing** (part of F-01) |

---

## 4. Failure findings (detailed)

### 🔴 F-01 — CRITICAL — Published v3.7.0 release is unsigned; documented signing contract not implemented

**Expected (README §Release security, SECURITY.md, release-rule.md, CHANGELOG 3.7.0):**
tagged releases are signed with a production certificate; both `system-monitor.exe`
and the installer carry Authenticode signatures matching the thumbprint pinned into
the updater; releases include SHA-256 checksum, SPDX SBOM, and GitHub build
provenance; the workflow "fails closed when certificate secrets are missing".

**Observed:**
```
Get-AuthenticodeSignature SystemMonitor-3.7.0-setup.exe  (downloaded from GitHub release v3.7.0)
Status  : NotSigned
```
- Release assets: only `SystemMonitor-3.7.0-setup.exe`, `.sha256`, `.spdx.json`.
  **No provenance attestation.**
- Neither `.github/workflows/windows-release.yml` nor
  `windows-unsigned-release.yml` contains any signing step, certificate import,
  thumbprint pinning (`SYSMON_SIGNER_THUMBPRINT`), or provenance generation.
- Git history explains it: commit `85cc20c` "Fix release automation by using
  unsigned build".
- Local `downloads\SystemMonitor-3.6.0-setup.exe` is signed with a self-signed
  `CN=System Monitor Development Only` cert (untrusted root) — consistent with
  the dev-only escape hatch in `sign-binary.ps1`, and correctly never published.

**Impact:** The secure-update contract is broken end-to-end. The updater's
acceptance logic (`sig_acceptable`) requires at minimum a Windows-trusted `Valid`
signature (chain-trust fallback when no thumbprint is pinned); a `NotSigned`
installer is rejected. So installed clients **cannot auto-update** to the
published release, and the project's headline "Secure updates" claim is false for
the shipped artifact. README itself warns: "Do not distribute a locally self-signed
build as a production update" — yet the production release is unsigned.

**Troubleshooting:**
1. Implement the documented signing steps in `windows-release.yml`: import
   `WINDOWS_SIGNING_PFX_BASE64`/`_PASSWORD` secrets, sign both binaries with
   `signtool`, derive the thumbprint and pass it as `SYSMON_SIGNER_THUMBPRINT` at
   compile time, verify installer signer == pinned thumbprint before publishing,
   and attach `attestations/build-provenance`.
2. Make the workflow fail closed when secrets are absent (currently the unsigned
   workflow is the one that runs on tags).
3. Delete or clearly quarantine `windows-unsigned-release.yml` so a tag push can
   never publish an unsigned release again; re-tag/re-publish v3.7.0 signed.

---

### 🔴 F-02 — CRITICAL — Release quality gate ineffective; release published while CI red

**Expected:** README/CI require `cargo fmt --check` + `clippy -D warnings` +
`cargo test` to pass before a release.

**Observed (GitHub Actions API, commit `1bdd786`):**
- `Rust CI` run 31732108777 → job `quality`: **failure** — step
  `Check formatting: failure`; clippy/test/build steps skipped.
- `Build Windows Release` run 31732108771 → job `build-windows`: **success**,
  including step `Verify source quality: success` — on the *same commit* where
  Rust CI's formatting check failed.

**Root cause:** in `windows-release.yml` the gate is one `pwsh` script:
```powershell
cargo metadata --locked … | Out-Null
cargo fmt --all -- --check          # exits 1 → PowerShell ignores native exit code
cargo clippy … -- -D warnings       # exits 101 → ignored
cargo test …                        # exits 0 → step reports success
```
PowerShell does not fail a step on a native command's non-zero exit code unless
`$ErrorActionPreference`/`PSNativeCommandUseErrorActionPreference` or explicit
`$LASTEXITCODE` checks are used. The step's result reflects only the last command.
The release was therefore published from code that fails the project's own
formatting and linting gates (confirmed locally: F-05, F-06).

**Troubleshooting:**
1. Add after each command: `if ($LASTEXITCODE -ne 0) { throw "… failed" }`, or set
   `$ErrorActionPreference = 'Stop'; $PSNativeCommandUseErrorActionPreference = $true`
   at the top of the script (same fix needed in `windows-unsigned-release.yml`).
2. Re-run gates on `1bdd786`; they will fail until F-05/F-06 are fixed.

---

### 🟠 F-03 — MAJOR — `wmi` and `windows_gpu` providers fail at runtime (double COM security init)

**Expected (USER_GUIDE §Providers):** `wmi` supplies hardware identity/thermal
information; `windows_gpu` supplies vendor-neutral GPU engine/local-memory
counters. Machine WMI is healthy — control checks pass:
```
(Get-CimInstance Win32_Service).Count  → 295
Get-CimInstance Win32_VideoController  → Intel Iris Xe + RTX 3060
Get-WmiObject (DCOM path)              → works
Get-Service Winmgmt                    → Running
```

**Observed (live app log, 2026-08-15/16 session):**
```
WARN Provider poll failed provider="wmi"
     error=Init failed: COM init: HRESULT Call failed with: 0x80010119
WARN Provider poll failed provider="windows_gpu"
     error=Init failed: HRESULT Call failed with: 0x80010119   (×366 in 6 min)
```
`0x80010119` = **`RPC_E_TOO_LATE`**: `CoInitializeSecurity` may succeed only once
per process. The legacy engine (`monitoring/engine.rs:39`,
`SystemMonitor::new()` → `COMLibrary::new()`) initializes COM security first on
the monitoring thread; every later `COMLibrary::new()` in
`providers/wmi_provider.rs:30` and `providers/windows_gpu_provider.rs:59` then
fails at `init_security()`. The `wmi` crate even ships the remedy:
`COMLibrary::without_security()` for exactly this scenario.

**Impact:** hardware identity (motherboard/BIOS/OS details), thermal zones, and
vendor-neutral GPU counters — a headline 3.7.0 feature ("Vendor-neutral GPU
coverage") — are silently unavailable on this healthy machine. Diagnostics will
report these providers as unavailable (informational), masking an app bug as an
environment condition.

**Troubleshooting:**
1. Initialize COM security **once** process-wide (first `COMLibrary::new()`), and
   have all subsequent providers/connections use `COMLibrary::without_security()`
   (or share one `Rc<COMLibrary>` across the hub and legacy engine).
2. Add a regression test that constructs the legacy engine and a WMI-backed
   provider in one process and asserts both connect (currently only single-provider
   smoke tests exist, which is why CI never caught this).

---

### 🟠 F-04 — MAJOR — Failed provider retries every second (log spam, no backoff)

**Observed:** `windows_gpu` poll interval is 1 s; after init failure the hub
re-polls and re-logs the identical `InitFailed` every second:
- 2026-08-13 log: **3,131** `windows_gpu` WARN lines + 9 `wmi`.
- 2026-08-15 log: **693** `windows_gpu` WARN lines in ~12 min.

Error isolation (C-09) correctly prevents a crash, but there is no
backoff/disable for persistently failing providers.

**Impact:** unbounded log growth (~0.5–1 MB/day of identical lines), wasted CPU
for doomed COM init attempts, and real warnings drowned in noise.

**Troubleshooting:** after N consecutive identical init failures, mark the
provider unavailable, stop scheduling it (or back off exponentially), and log a
single terminal WARN. `PollingScheduler` already supports per-provider intervals —
extend it with a disabled state.

---

### 🟠 F-05 — MAJOR — `cargo fmt --all -- --check` fails (documented gate)

**Observed:** exit 1 with diffs in:
- `src/app/models.rs` (import ordering, stray blank lines, trailing EOF newline)
- `src/main.rs` (import ordering/grouping, `tray_icon` use formatting)
- `src/monitoring/engine.rs` (import ordering, line-length wraps, CSV write formatting)

This is the exact failure that turned Rust CI red on the release commit (F-02).

**Troubleshooting:** run `cargo fmt --all`, commit, and keep the fmt gate
blocking (it already is in `rust-ci.yml` once F-02's pwsh gate is also fixed).

---

### 🟠 F-06 — MAJOR — `cargo clippy --locked --all-targets -- -D warnings` fails: 24 errors

**Observed:** all 24 are `unused_imports`, distributed:
```
10  src/main.rs
 9  src/app/models.rs
 5  src/monitoring/engine.rs
```
Examples: `std::fs`, `Duration`, `sysinfo::{Disks, Networks, Pid, System}`,
`nvml_wrapper::Nvml`, most `tray_icon` items, `wmi::COMLibrary`, `Mutex`/`RwLock`,
`crate::telemetry::TelemetrySnapshot`. These are leftovers of the staged
legacy→TelemetryHub migration (CHANGELOG: "legacy polling remains only for richer
… views during staged migration") — imports outlived their call sites.

**Impact:** the documented strict-lint gate fails; CI quality job cannot go green;
dead imports obscure which legacy paths are still live.

**Troubleshooting:** `cargo fix --bin system-monitor --allow-no-vcs` resolves all
24 mechanically; then re-run clippy to confirm zero warnings.

---

### 🟡 F-07 — MINOR — RAM-cleaner success/failure counters misleading

**Observed:** `RAM clean complete freed_mb=1 success=0 failed=341` — 341
"failures" are `OpenProcess`/`EmptyWorkingSet` access denials on protected/system
processes (expected without elevation), counted identically to real errors.
`freed_mb` is a system-wide memory delta, not the sum of trimmed working sets, so
it can read 0 even when trims succeeded (audit shows many `Freed 0 bytes` rows).

**Troubleshooting:** split counters into `trimmed / access_denied / errored`;
optionally compute freed bytes per-process (`GetProcessMemoryInfo` before/after)
instead of the global delta.

---

### 🟡 F-08 — MINOR — `test_cpu_temp` smoke binary panics; requires elevation

**Observed:**
```
thread 'main' panicked at src\bin\test_cpu_temp.rs:9:10:
query failed: HResultError { hres: -2147217405 }   = 0x80041003 WBEM_E_ACCESS_DENIED
```
`ROOT\WMI\MSAcpi_ThermalZoneTemperature` requires administrator rights; the
binary `.expect()`s instead of reporting gracefully. Dev-only tool, but it
currently always panics when run unelevated.

**Troubleshooting:** replace `.expect("query failed")` with a graceful message
naming the access-denied cause and the elevation requirement.

---

### 🟡 F-09 — MINOR — `test_wmi` smoke binary returns `WBEM_E_NOT_FOUND` despite healthy WMI

**Observed:** standalone `test_wmi.exe` (fresh process, `Win32_Service` from
`ROOT\CIMV2`) prints `Error: HResultError { hres: -2147217406 }` =
`0x80041002 WBEM_E_NOT_FOUND`, while PowerShell CIM and DCOM queries return 295
services on the same machine. In-app WMI fails with a *different* code
(`RPC_E_TOO_LATE`, F-03), so this is a separate, unresolved issue in the
`wmi 0.9.3` query path on this system.

**Troubleshooting:** instrument `WMIConnection` creation vs query separately
(connect to `ROOT\CIMV2`, then run `SELECT … FROM Win32_Service`); check whether
the crate's `ExecQuery` flags/locale trip on this WMI repository; consider
upgrading the `wmi` crate (0.9 → current) as part of the F-03 fix.

---

### Observation (not scored)

- **Memory footprint:** RSS ≈ 210 MB steady-state vs CHANGELOG 2.6.0's historical
  "~35–40 MB" claim. Expected growth from TelemetryHub + 14 modules + histories,
  but worth a budget if footprint matters.
- **UI visual verification:** a window screenshot was captured during the audit
  but the audit environment has no vision model, so pixel-level DESIGN.md
  compliance (fonts, colors, layout) was **not** assessed. Recapture per test
  plan TC-3.4.

---

## 5. Fully functional features (verified this audit)

**Build/test:** lockfile integrity · release build · 46/46 unit+integration tests
(ring buffers incl. evicted min/max fix, scheduler incl. background throttle,
provider registration/publication, process filter/sort/tree/kill-order, power GUID
parsing, diagnostics finding generation, updater URL + signature policy, settings
round-trip, RAM-cleaner clamps/exclusions/stop-conditions).

**Runtime:** app launch · 11+ min stability, no crash · single-instance guard ·
settings persistence with validation clamps · append-only JSONL action audit
(97 rows incl. automatic RAM-clean entries) · daily rotating logs + crash-report
dir · TelemetryHub error isolation (app alive through 366 provider failures) ·
sysinfo + NVML providers healthy · automatic RAM cleaner executing on schedule.

**Distribution:** Inno Setup installer build (`/DAppVersion=3.7.0`) · GitHub
release v3.7.0 exists, tag == HEAD · published SHA-256 checksum matches byte-for-byte ·
SBOM present · docs site assets resolve, dynamic version resolver wired,
installers correctly gitignored · Pages deploy green · version strings consistent
everywhere (3.7.0).

---

## 6. Severity summary

| Severity | Count | Findings |
| --- | --- | --- |
| 🔴 Critical | 2 | F-01 unsigned published release / signing contract unimplemented · F-02 release quality gate ineffective, released while CI red |
| 🟠 Major | 4 | F-03 WMI+GPU providers dead (RPC_E_TOO_LATE) · F-04 1 Hz retry/log spam · F-05 fmt gate fails · F-06 clippy gate fails (24 errors) |
| 🟡 Minor | 3 | F-07 RAM-clean counters misleading · F-08 test_cpu_temp panic/elevation · F-09 test_wmi WBEM_E_NOT_FOUND |

## 7. Recommended fix order

1. **F-05 + F-06** (`cargo fmt`, `cargo fix`) — unblocks every gate; ~10 min.
2. **F-02** — harden both release workflows' pwsh gates so 1 can never recur.
3. **F-01** — implement real signing + thumbprint pinning + provenance; re-publish
   v3.7.0 signed (or yank and re-tag). Until then, auto-update is dead by design.
4. **F-03 + F-04** — single COM-security init shared by legacy engine and hub
   (`without_security()` thereafter); backoff/disable for persistently failing
   providers; add a cross-provider COM regression test.
5. **F-07…F-09** — hygiene fixes.
6. Manual pass through test plan TC-3.4/TC-3.5 (14 views + destructive actions on
   disposable targets) to clear the ⬜ items.

**Re-verification:** rerun `VERIFICATION_TEST_PLAN.md` top to bottom; all TC-1.x
and TC-5.1 must be green on the fix commit before re-tagging.

---

## 8. Resolution addendum (2026-08-16, commit `4e3d3ab`, version 3.7.1)

All nine findings were fixed and re-verified on this machine. Status after the fix
commit:

| Finding | Fix | Re-verification evidence |
| --- | --- | --- |
| F-01 unsigned release | `windows-release.yml` now fails closed without secrets, signs app + installer, verifies signer vs pinned thumbprint, attaches provenance; unsigned workflow deleted | Workflow rewritten; requires production secrets + a `v3.7.1` tag to publish (not done here — no secrets in this environment) |
| F-02 ineffective gate | `$ErrorActionPreference='Stop'` + `$PSNativeCommandUseErrorActionPreference=$true` in every pwsh quality step | Gates now propagate native exit codes |
| F-03 WMI/GPU providers dead | Shared `providers::init_com()` with `RPC_E_TOO_LATE` → `without_security()` fallback at all 5 call sites | Live run: **0** `Provider poll failed` lines (was 366/session); `test_wmi` → `Success: 295` |
| F-04 1 Hz retry spam | Hub disables provider after 5 consecutive failures; re-enables on success; 2 new regression tests | 0 disabling events needed (providers now succeed); 48/48 tests green |
| F-05 fmt gate | `cargo fmt --all` | `cargo fmt --all -- --check` → exit 0 |
| F-06 clippy gate | Removed 24 unused imports | `cargo clippy --locked --all-targets -- -D warnings` → exit 0 |
| F-07 RAM-clean counters | Split into `trimmed`/`access_denied`/`errored` | Live log: `trimmed=124 access_denied=157 errored=1` |
| F-08 test_cpu_temp panic | Graceful error + elevation hint | Prints access-denied message, exit 1, no panic |
| F-09 test_wmi NOT_FOUND | `rename_all = "PascalCase"` on `Win32_Service` (also fixed the empty Services view) | `Success: 295` |

**Additional defect found and fixed during remediation:** `EmptyWorkingSet`
requires `PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA`; the cleaner opened
processes with query rights only, so it had **never actually freed memory**
(historical `success=0` / `Freed 0 bytes`). After the fix the first pass freed
17 MB with `trimmed=128`.

**Final gate state (v3.7.1):** lockfile OK · fmt OK · clippy OK · 48 passed /
0 failed / 1 ignored · release build OK · Inno Setup installer OK · app launches,
window `System Monitor v3.7.1`, stable, zero provider failures.

**Remaining manual items:** the 14-view UI walkthrough (test plan TC-3.4/TC-3.5).
The release itself ships as **v3.7.2** (the never-published 3.7.1 version was
folded into it; see §9) by pushing the tag — no signing secrets required.

---

## 9. Policy change addendum (2026-08-16): paid Authenticode signing removed

After the §8 remediation, the project owner directed that the paid code-signing
certificate (`WINDOWS_SIGNING_PFX_BASE64` / `WINDOWS_SIGNING_PFX_PASSWORD`) be
removed. The release and update-integrity contract was cut over to a free model:

| Layer | Before (paid) | After (free) |
| --- | --- | --- |
| Release signing | Authenticode via signtool + pinned thumbprint | **None** — signing removed entirely |
| Installer integrity | Signature validity + thumbprint pin | **SHA-256 checksum** published as a `.sha256` release asset; updater verifies the downloaded installer against it before writing/executing |
| Build authenticity | Publisher certificate | **GitHub build provenance attestation** (sigstore, attached by the release workflow) |
| Transport | HTTPS + repo-pinned asset URLs | unchanged |
| SBOM | SPDX JSON | unchanged |

Changes made:

- `src/updater.rs`: `verify_authenticode` / `sig_acceptable` / `SIGNER_THUMBPRINT`
  removed. `download_and_install_update` now takes the checksum URL, downloads the
  `.sha256` asset (HTTPS, repo-pinned, size-bounded), and verifies the installer
  hash before writing it to disk. `check_for_updates` offers an update only when
  both the installer and its checksum asset are published. New tests:
  `verifies_installer_checksum`, `parses_sha256_checksum_file`,
  `accepts_expected_checksum_asset`, `rejects_untrusted_checksum_urls`.
- `.github/workflows/windows-release.yml`: PFX import, signtool signing,
  thumbprint pinning and signature-verification steps removed; quality gates,
  checksum, SBOM and provenance attestation retained. No secrets required.
- `sign-binary.ps1` deleted; signing removed from `build.ps1` and `release.ps1`.
- SECURITY.md, README.md, USER_GUIDE.md, docs/release-rule.md, docs/index.html,
  CONTRIBUTING.md and CHANGELOG.md updated to the checksum + provenance contract.

**Security tradeoff (accepted by owner):** a checksum published next to the file
does not protect against a compromised release account rewriting both artifacts;
the pinned Authenticode certificate did. Residual mitigations: GitHub build
provenance (independently signed by GitHub, verifiable with `gh attestation
verify`), HTTPS-only repo-pinned downloads, and size-bounded fetches.
