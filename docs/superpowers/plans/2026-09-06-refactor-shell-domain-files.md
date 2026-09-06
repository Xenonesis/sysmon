# Refactor Shell & Domain Big Files — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the remaining oversized files (`monitoring/engine.rs` 2367 ln, `main.rs` 2114, `startup.rs` 1819, `timeline.rs` 1500) into focused modules with zero behavior change, then prune dead imports/files.

**Architecture:** Same verbatim-move pattern as the two prior UI plans. `SystemMonitor` (hardware pollers) moves from `engine.rs` into sibling files under `src/monitoring/`; `SystemMonitorApp` stays in `engine.rs` (re-exported via `monitoring/mod.rs` keeps every existing import path working). `main.rs` keeps `fn main` + module wiring; the giant `eframe::App::ui` body splits into `impl` files beside it. `startup.rs` and `timeline.rs` become module directories. No signature changes anywhere — moves plus visibility only.

**Tech Stack:** Rust 2024 (1.95), eframe/egui 0.36, parking_lot, rusqlite, winreg.

**Spec:** `docs/refactor-parity-matrix.md` ("Application shell and domain hotspots: split `main.rs`, `monitoring/engine.rs`, `startup.rs`, and `timeline.rs` only after page contracts are stable" — they are, both prior plans shipped).

## Global Constraints

- **Zero functionality loss.** Verbatim cut-paste moves; no rewrites, no signature changes, no `cfg` reordering. The only permitted edits: `fn` → `pub(super) fn` (or `pub(crate)` where cross-module), plus adding `use` lines the new file needs.
- **Import-path stability.** `src/lib.rs:24 pub use crate::monitoring::engine::*;` and `src/main.rs:30 pub(crate) use crate::monitoring::engine::*;` must keep resolving after every task. Same for `crate::startup::*` and `crate::timeline::*` consumers (grep lists below).
- **Gates after every task** (`set -o pipefail`; pipe into tail swallows exit codes otherwise): `cargo fmt --all` then `cargo fmt --all -- --check` · `cargo clippy --locked --all-targets -- -D warnings` · `cargo test --locked`. Baseline: 358 passed / 0 failed / 4 ignored.
- **Windows-cfg blocks move as whole units.** `#[cfg(target_os = "windows")]` fn pairs (impl + non-windows fallback) never split across files.
- **Tests move with their code** — each extracted test module goes to the file owning the tested fn; counts must stay 358+/4.

## Consumer maps (verified by grep this session — the contract to preserve)

- `SystemMonitorApp`: `main.rs` (impl blocks at 37/61, tests), `ui/pages/alerts/mod.rs:246`, `ui/pages/startup_manager/mod.rs:90` (tests), plus ~20 `crate::SystemMonitorApp` uses via glob re-export.
- `SystemMonitor`: `app/worker.rs:33,51`, `main.rs:667`, `models.rs` (def), `engine.rs` (impl).
- `snapshot_from_data` / `load_icon` / `load_tray_icon`: `main.rs:2001`, `ui/pages/diagnostics/findings.rs:9`, `engine.rs:1999,2138`.
- `startup::` fns used outside: `high_impact_count` (main.rs:154, engine.rs:2101, health_deck.rs:120, startup_manager/page.rs:54), `disable_startup`/`reenable_startup`/`quarantine_startup`/`restore_startup` (worker.rs:77+), `expand_env_vars`/`parse_exe_from_command` (startup.rs internal + UI card).
- `timeline::TimelineHandle` used in `main.rs` (record_snapshot/take_query_result/record_event/shutdown etc.) and `ui/pages/timeline.rs`.

---

### Task 1: Split `SystemMonitor` impl out of `engine.rs`

**Files:**
- Create: `src/monitoring/hardware.rs` — `impl SystemMonitor` blocks: `get_memory_info`, `get_cpu_usage`, `get_top_processes`, `get_timeline_processes`, `get_cpu_cores_info`, `get_swap_info` (engine.rs lines ~95–290), plus `get_disk_info`, `get_disk_io`, `get_network_info`, `get_system_info` (~lines 592–760)
- Create: `src/monitoring/gpu_wmi.rs` — `get_gpu_info`, `get_battery_wmi`, `get_gpu_info_wmi`, `get_cpu_temperature_wmi`, `get_wmi_system_details` + their non-windows fallback twins (~lines 355–590)
- Create: `src/monitoring/process_actions.rs` — `kill_process`, `suspend_process`, `resume_process`, `clean_ram`, `set_process_priority` (win + fallback pairs, ~lines 190–355)
- Create: `src/monitoring/alerts_logic.rs` — `check_alerts` (static fn, ~line 640)
- Modify: `src/monitoring/mod.rs` — add `mod` declarations + `pub(crate) use hardware::*;` etc. is NOT needed (methods stay inherent on `SystemMonitor` defined in `app/models.rs`; Rust allows multiple `impl` blocks in other files of the same crate)
- Modify: `src/monitoring/engine.rs` — remove moved blocks

**Interfaces:** all moved fns keep exact signatures; private ones become `pub(crate)` only if called cross-module (they aren't — all called via `self.` within the same impl split; keep them private by putting the new files' impl blocks with `fn` unchanged — inherent methods need no visibility change when staying `fn` only if the impl is in the same module... **correction**: inherent private methods are visible within the defining module only, so moved methods MUST become `pub(crate)` (or the call sites in `hardware.rs`'s own impl block see each other fine, but callers like `run_monitoring_loop` in engine.rs call `monitor.get_top_processes(...)` cross-module → mark every moved method `pub(crate)`).

**Steps:**
- [ ] Move blocks verbatim; bump `fn` → `pub(crate) fn` on moved methods; add needed `use` per file (clippy will name every gap)
- [ ] Windows/non-windows twin methods move together with their cfg attributes
- [ ] Gates + `cargo test --locked` (358)
- [ ] Commit: `refactor(monitoring): split SystemMonitor impl into focused modules`

### Task 2: Split `SystemMonitorApp` construction + exports out of `engine.rs`

**Files:**
- Create: `src/monitoring/app_shell.rs` — `impl SystemMonitorApp { new, test_app }` (lines ~912–1990, incl. tray/menu/hotkey setup inside `new`)
- Create: `src/monitoring/exports.rs` — `export_diagnostics`, `export_to_csv`, `export_data_to_json`, `queue_action`, `start_ram_clean` (lines ~1991–2126) + `mod csv_export_tests` (lines 2335–end) moves with `export_to_csv`
- Create: `src/monitoring/snapshot_convert.rs` — `snapshot_from_data` (~line 2138), `load_icon`, `load_tray_icon` (~2264–2284)
- Modify: `engine.rs` — keeps `struct SystemMonitorApp` (790), `impl Drop` (2127), `mod alert_tests` (2285; `check_alerts` test stays with Drop or moves to alerts_logic — move it there), plus `pub(crate) use` re-shims so `engine::*` glob still exposes every moved item: `pub(crate) use crate::monitoring::app_shell::*;` won't work for inherent impls (impls don't glob) — **impls need nothing**: glob consumers import the TYPE from models via engine's existing `pub use crate::app::models::*` in lib.rs; `SystemMonitorApp` struct itself stays in engine.rs, so all `impl` blocks anywhere in crate are legal.
- Verify: `grep -rn "SystemMonitorApp::test_app\|export_to_csv\|snapshot_from_data\|load_icon" src/` call sites still compile (they go through `crate::` glob = engine::* re-export of snapshot_convert items — add `pub(crate) use snapshot_convert::*;` in engine.rs)

**Steps:**
- [ ] Move blocks; visibility `pub(crate)` already on exports; add re-shim `use` lines in engine.rs for the free fns
- [ ] Move `csv_export_tests` and `alert_tests` modules to owning files
- [ ] Gates + tests (358 incl. both moved test modules)
- [ ] Commit: `refactor(monitoring): extract app shell, exports and snapshot conversion`

### Task 3: Split `main.rs` `eframe::App::ui` body into impl files

**Files:**
- Create: `src/app_shell/mod.rs` with `mod`s below + `impl SystemMonitorApp` files:
  - `src/app_shell/event_pump.rs` — event receiver loop + keyboard shortcuts + hotkey/tray MenuEvent handling (main.rs 66–~505)
  - `src/app_shell/top_bars.rs` — update banner (447), sidebar (886), global status bar (1419–1755)
  - `src/app_shell/overlays.rs` — Export CSV/JSON/Alerts windows, Settings window, HUD widget window, dialogs calls (726–885, 1385–1418)
  - `src/app_shell/central.rs` — CentralPanel tab dispatch (1747–1776)
  - `src/app_shell/input_pending.rs` — post-render action flush (kill/suspend/resume/priority/affinity/auto-ram-clean, ~604–725)
- Modify: `src/main.rs` — `fn ui` becomes: pump events (inline call chain), call top_bars, overlays, central, flush; keep `fn main` + `mod` wiring + `mod tests` (1784), `mod persistence_tests`, `mod ram_cleaner_tests`
- Keep `impl eframe::App for SystemMonitorApp` in main.rs; its body calls `self::app_shell::ui_shell(self, ui, &ctx)` — extract whole body into one `pub(crate) fn ui_shell(app: &mut SystemMonitorApp, ui: &mut egui::Ui)` in `app_shell/mod.rs` (simplest safe cut: one fn, not five, if borrow slicing is risky — decide by reading the borrow structure; prefer the five-file split but fall back to single `ui_shell` in mod.rs, still ~700 lines total but off main.rs)

**Steps:**
- [ ] Extract `ui_shell` single fn first (safe), run gates
- [ ] Then split ui_shell's sections into the files above; each takes `app: &mut SystemMonitorApp, ui, ...` params it needs
- [ ] Gates + tests (358; main.rs `mod tests` stays — it uses `SystemMonitorApp::test_app()`, unchanged path)
- [ ] Commit: `refactor(shell): extract app-shell UI from main.rs`

### Task 4: `startup.rs` → `startup/` package

**Files:**
- Create: `src/startup/mod.rs` — types (ImpactTier, Recommendation, StartupLocator, StartupItem, BootDiagnostics, StartupOptimizationEntry, StartupSortColumn; lines 1–137) + `get_startup_items` + `high_impact_count` + re-export shim `pub use` from siblings + `mod tests`
- Create: `src/startup/parsing.rs` — `expand_env_vars`, `parse_exe_from_command`, `new_item` (140–255)
- Create: `src/startup/collect.rs` — `ps_run`, registry/folder/task-scheduler collectors, `get_startup_data` win + fallback (256–590)
- Create: `src/startup/enrich.rs` — `enrich_startup_items`, `score_startup_items` (596–717)
- Create: `src/startup/boot_diagnostics.rs` — `get_boot_diagnostics` win + fallback (~718–772)
- Create: `src/startup/actions.rs` — quarantine/restore/disable/reenable/remove/open_file_location/search_online + `quarantine_exists` (~773–1693) + quarantine helpers
- Create: `src/startup/sorting.rs` — sort/filter helpers (1694–end)
- Delete: `src/startup.rs`
- Verify consumers: `crate::startup::{...}` paths unchanged (dir module resolves identically); `worker.rs`, `main.rs`, UI pages compile untouched.

**Steps:**
- [ ] Move verbatim per section map; `pub(crate) fn` for cross-file helpers (collect→enrich etc.), `pub` stays `pub`
- [ ] Move `mod tests` (find its location; it tests parsing/sorting mostly) to the owning file; split if trivial, else keep whole in `mod.rs`
- [ ] Gates + tests (358)
- [ ] Commit: `refactor(startup): split into focused modules`

### Task 5: `timeline.rs` → `timeline/` package

**Files:**
- Create: `src/timeline/mod.rs` — `TimelineRange`, `TimelineQuery`, `TimelineEventKind`, `TimelineEvent`, `TimelineUiState` types + `TimelineHandle` impl (public API surface, lines 1–476) + `mod tests`
- Create: `src/timeline/worker.rs` — `run_worker`, `timeline_db_path`, `ensure_connection`, `migrate`, `prune` (477–677, 1091–1170)
- Create: `src/timeline/records.rs` — `write_snapshot`, `metric_from_snapshot`, `select_process_union`, `record_derived_events`, `insert_event` (678–904)
- Create: `src/timeline/query.rs` — `query_window`, `contributors_near` (905–1121)
- Create: `src/timeline/export.rs` — `export_window`, `timestamp_rfc3339`, `sanitize_text`, `validate_retention`, `now_ms`, `system_time_ms` (1122–1500)
- Delete: `src/timeline.rs`
- Consumers: `main.rs` uses `self.timeline.*` methods (all on `TimelineHandle`, stays in mod.rs) and `crate::timeline::TimelineEvent::new` — unchanged.

**Steps:**
- [ ] Move verbatim; worker fns become `pub(super)`; db path helpers `pub(super)`
- [ ] Gates + tests (358)
- [ ] Commit: `refactor(timeline): split into worker, records, query and export modules`

### Task 6: Dead-import sweep + final verification

**Steps:**
- [ ] `cargo clippy --locked --all-targets -- -D warnings` (gate does the unused-import sweep)
- [ ] Zero-unused-fn scan (pattern from prior plans: rg declared `pub(crate)/pub(super) fn` vs call sites) on all new dirs
- [ ] `wc -l`: no file >~900 lines except engine.rs remainder (~300) and app_shell split totals; `main.rs` ≤ ~700 (main + tests + wiring)
- [ ] Full gates: fmt, clippy, `cargo test --locked` (358/0/4), `cargo build --locked --release`
- [ ] Live smoke on release binary: launch, walk 13 nav pages via orca (title check only), Free RAM → audit row, kill
- [ ] Commit if cleanup produced diffs: `chore: prune unused imports after shell refactor`

## Self-review

- Spec coverage: parity-matrix phase 5 items all mapped (engine→Tasks 1–2, main→3, startup→4, timeline→5). Pages phase done in prior plans.
- Placeholder risk: Task 3 has an explicit fallback (single `ui_shell` if borrow slicing fails) — decision documented in-task, not deferred.
- Type consistency: all signatures byte-identical; only `pub(crate)`/`pub(super)` added. Re-export shims specified exactly (`pub(crate) use snapshot_convert::*;` in engine.rs).
- Test parity tracked numerically (358/0/4 at every gate).
- Known hazard from prior plan: `set -o pipefail` before every gate chain — committed to per-task step text.
