# Refactor Remaining Big Pages — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the remaining oversized UI page files into focused sub-component modules with zero behavior change, then delete unused imports/files.

**Architecture:** Continue the established pattern (see `services/`, `overview/`, `system_info/`): each big page becomes a module directory whose `mod.rs` keeps the canonical `pub(crate) fn show(...)` entry point; sibling files hold cohesive `pub(super)` paint functions moved verbatim. No new abstractions, no signatures changed — file moves only. Domain hotspots (`main.rs`, `monitoring/engine.rs`) are OUT OF SCOPE per `docs/refactor-parity-matrix.md` phase ordering (pages first, shell last).

**Tech Stack:** Rust 2024 (1.95), eframe/egui 0.36, existing headless-render test pattern (`ctx.run_ui(...).textures_delta.clear()`).

**Spec:** `docs/refactor-parity-matrix.md` (behavior contract) · predecessor: `docs/superpowers/plans/2026-08-16-refactor-big-pages.md` (pages 1–7 done).

## Global Constraints

- **Zero functionality loss:** every control, layout branch, tooltip, intent, and side effect identical. Moves are verbatim (`git mv`-style cut-paste), no rewrites.
- **Entry points unchanged:** `src/ui/pages/mod.rs` keeps the same `pub(crate) mod <page>;` declarations; callers in `src/main.rs` (lines ~1747–1760) and `src/ui/dialogs.rs` keep calling `<page>::show(...)`.
- **Gates after every task:** `cargo fmt --all` then `cargo fmt --all -- --check` · `cargo clippy --locked --all-targets -- -D warnings` · `cargo test --locked`. All green before commit.
- **File length target:** sub-components under ~400 lines (alerts `paint_active_incidents_feed` is one 170-line fn — don't split the fn itself).
- **Side-effect rule (parity matrix):** UI pages keep deferring state writes to post-render blocks exactly as today. Do not "improve" them.

## Function inventories (source of truth)

`src/ui/pages/alerts.rs` (1097 lines):
- `show` (line 6) — stays in `mod.rs`
- `paint_status_headline` (242)
- `paint_control_hub_actions` (269)
- `toggle_button` (354), `secondary_button` (379), `accent_button` (393)
- `paint_proximity_matrix` (411)
- `paint_proximity_row` (587), `paint_proximity_bar` (628), `struct ProximityRow` (~575)
- `paint_nominal_health_board` (674)
- `paint_active_incidents_feed` (816)
- tests module (983–1097) — stays in `mod.rs`

`src/ui/pages/storage.rs` (774):
- `show` (6) — stays; body sections 1 (I/O banner), 2 (volume cards), 3 (S.M.A.R.T.) stay inline (they're one visual card group, ~230 lines total) OR move as `volume_cards.rs` — Task decides: move all three to `volumes.rs`
- `paint_disk_perf_card` (298)
- `paint_lock_inspector_card` (378)
- `paint_reclaimer_card` (545)
- tests (706–774) — stays

`src/ui/pages/diagnostics.rs` (540):
- `show` (9) — stays
- inline: session recorder banner (17–108), session history/exporter (109–219), findings (220–322)
- `paint_bsod_history_card` (328)
- tests (461–540)

`src/ui/pages/processes/table.rs` (516):
- `paint_process_table` (7) — header row + `header_button` closure, scroll body with row closure containing sort/affinity/priority/suspend/kill actions

`src/ui/pages/startup_manager/mod.rs` (368):
- `impact_tier_badge_color` (15), `filter_and_sort_indices` (24), `show` (88), tests (224–368)

---

### Task 1: Split `alerts.rs` → `alerts/` package

**Files:**
- Create: `src/ui/pages/alerts/mod.rs` (from old `alerts.rs`: `show` + imports + tests)
- Create: `src/ui/pages/alerts/control_hub.rs` (`paint_status_headline`, `paint_control_hub_actions`)
- Create: `src/ui/pages/alerts/buttons.rs` (`toggle_button`, `secondary_button`, `accent_button`)
- Create: `src/ui/pages/alerts/proximity.rs` (`ProximityRow` struct, `paint_proximity_matrix`, `paint_proximity_row`, `paint_proximity_bar`)
- Create: `src/ui/pages/alerts/health_board.rs` (`paint_nominal_health_board`)
- Create: `src/ui/pages/alerts/incidents_feed.rs` (`paint_active_incidents_feed`)
- Delete: `src/ui/pages/alerts.rs`

**Interfaces:**
- Consumes: `crate::SystemMonitorApp`, `crate::SystemData`, `Tab`, `AlertType`, `AlertInfo`, `crate::ui::components::*`, `ThemePalette`
- Produces: same `pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData)`; internal fns become `pub(super) fn ...` (signatures byte-identical)

**Steps:**
- [ ] `mkdir src/ui/pages/alerts`; cut each function block verbatim into its file above its `use crate::ui::components::*; use crate::ui::theme::ThemePalette; use crate::*; use eframe::egui;` header (only imports each file actually needs — clippy verifies)
- [ ] In each new file: `fn` → `pub(super) fn`; `struct ProximityRow` → `pub(super) struct ProximityRow` (+ its fields `pub(super)`)
- [ ] `mod.rs` gets: `mod buttons; mod control_hub; mod health_board; mod incidents_feed; mod proximity;` + `use buttons::*;` etc. (or explicit `use` of each moved fn — pick one style, explicit preferred)
- [ ] `git rm src/ui/pages/alerts.rs` (content already moved) — `src/ui/pages/mod.rs` line 2 unchanged (`pub(crate) mod alerts;` resolves to the directory)
- [ ] Gates: fmt, clippy `-D warnings`, `cargo test --locked alert` (4 headless tests must pass: nominal, active incidents, compact, wide)
- [ ] Commit: `refactor(ui): split alerts page into subcomponents`

### Task 2: Split `storage.rs` → `storage/` package

**Files:**
- Create: `src/ui/pages/storage/mod.rs` (old `show` + tests)
- Create: `src/ui/pages/storage/volumes.rs` (I/O banner + volume cards + S.M.A.R.T. section — extract from `show` body into `pub(super) fn paint_volumes(app: &mut ..., ui, data, is_dark)`)
- Create: `src/ui/pages/storage/perf.rs` (`paint_disk_perf_card`)
- Create: `src/ui/pages/storage/lock_inspector.rs` (`paint_lock_inspector_card`)
- Create: `src/ui/pages/storage/reclaimer.rs` (`paint_reclaimer_card`)
- Delete: `src/ui/pages/storage.rs`

**Interfaces:**
- Produces: same `show(app, ui, data)`; moved fns → `pub(super)`, signatures unchanged

**Steps:**
- [ ] Extract `show`'s sections 1–3 (lines ~16–296) into `paint_volumes` in `volumes.rs`; `mod.rs` body becomes: header → scroll → `paint_volumes(...)` → `perf::paint_disk_perf_card(...)` → `lock_inspector::paint_lock_inspector_card(app, ...)` → `reclaimer::paint_reclaimer_card(app, ...)`
- [ ] `git rm src/ui/pages/storage.rs`
- [ ] Gates + `cargo test --locked storage` (3 tests incl. lock/categories render)
- [ ] Commit: `refactor(ui): split storage page into subcomponents`

### Task 3: Split `diagnostics.rs` body → focused cards

**Files:**
- Modify: `src/ui/pages/diagnostics.rs` → becomes `src/ui/pages/diagnostics/mod.rs` (dir already exists with `guided.rs`)
- Create: `src/ui/pages/diagnostics/recorder.rs` (session recorder banner + history/exporter, lines ~17–219 → `pub(super) fn paint_recorder(app, ui, is_dark)` + `pub(super) fn paint_history(app, ui, is_dark)`)
- Create: `src/ui/pages/diagnostics/findings.rs` (lines ~220–322 → `pub(super) fn paint_findings(app, ui, data, is_dark)`)
- Create: `src/ui/pages/diagnostics/bsod.rs` (`paint_bsod_history_card` → `pub(super)`)

**Steps:**
- [ ] Move file to `diagnostics/mod.rs`, add `mod bsod; mod findings; mod recorder;` (keep `mod guided;`)
- [ ] `show` body becomes: `guided::show` → `recorder::paint_recorder` → `recorder::paint_history` → `findings::paint_findings` → `bsod::paint_bsod_history_card` (order identical to current line order)
- [ ] Gates + `cargo test --locked diagnostic` (guided-flow + crash tests)
- [ ] Commit: `refactor(ui): split diagnostics page into focused cards`

### Task 4: Split `processes/table.rs` row actions

**Files:**
- Modify: `src/ui/pages/processes/table.rs` — keep `paint_process_table` shell (card frame, widths, sticky header, `header_button` closure, `ScrollArea::show_rows` closure skeleton)
- Create: `src/ui/pages/processes/row_actions.rs` — the per-row action cluster (Kill/Suspend/Resume/Tree/⚙ priority+affinity menu, lines ~180–516 row-closure body) as `pub(super) fn paint_row_actions(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, process: &crate::processes::ProcessInfo, is_dark: bool, intents: &mut Vec<crate::app::commands::UiIntent>)` — check the existing row body: if it mutates `app.affinity_change`/`app.queued_priority` directly, pass `app` (it does: `app.affinity_change = Some(...)`) so signature is `fn paint_row_actions(app, ui, process, is_dark)`; intents flow stays exactly as-is inside
- Keep `pub(super)` on `paint_process_table`

**Steps:**
- [ ] Cut row-closure body verbatim; adjust only: `ui` and captured locals become parameters (name collisions resolved by same names)
- [ ] Gates + `cargo test --locked process` (render + existing process tests)
- [ ] Commit: `refactor(ui): extract process table row actions`

### Task 5: Extract `startup_manager/mod.rs` page body

**Files:**
- Modify: `src/ui/pages/startup_manager/mod.rs` — keep `impact_tier_badge_color`, `filter_and_sort_indices`, tests
- Create: `src/ui/pages/startup_manager/page.rs` — `pub(super) fn show(app, ui)` moved verbatim (lazy loader thread + filter bar + summary + item list)

**Steps:**
- [ ] Move `show` to `page.rs`; `mod.rs` gets `mod page;` and `pub(crate) use page::show;` (keeps `startup_manager::show` path for `main.rs` unchanged)
- [ ] Gates + `cargo test --locked startup` (badge/filter/all-states render tests)
- [ ] Commit: `refactor(ui): extract startup manager page body`

### Task 6: Trim remaining mid-size pages (optional per ponytail — SKIP unless a file grows)

- timeline.rs (331), ram_cleaner.rs (288), cpu_cores.rs (192), network/* (all <330): already cohesive single-responsibility paint flows. **No action.** Splitting = churn without benefit.

### Task 7: Dead code & import sweep

**Files:** all touched files.

**Steps:**
- [ ] `cargo clippy --locked --all-targets -- -D warnings` (unused imports already fail the gate — this is the sweep)
- [ ] `cargo +nightly udeps` — SKIP (no nightly guarantee). Instead: `rg` for orphans:
      - `rg -n "pub(crate) fn|pub(super) fn" src/ui/pages/ | sort` vs call sites — any moved fn with zero callers gets deleted
      - Check `src/ui/pages/mod.rs` declares exactly the 16 modules that exist; remove none unless file deleted
- [ ] `find src -name "*.rs" | xargs wc -l | sort -rn | head -5` — confirm no UI page file >600 lines remains (alerts/mod.rs target ~350)
- [ ] Full gates: fmt, clippy, `cargo test --locked` (183+ tests), `cargo build --locked --release`
- [ ] Launch release binary headless check: start app, verify window title, kill (manual smoke — optional, gates above are the contract)
- [ ] Commit: `chore(ui): remove dead imports after page refactor`

## Self-review

- Spec coverage: parity matrix order items 1 (Alerts) → Task 1; 2 (Processes) → Task 4; 3 (Diagnostics/Timeline) → Task 3 (+ timeline deliberately untouched, cohesive); 4 (Network/RAM/Storage/Performance/Overview oversized subs) → Task 2 (storage only oversized); 5 (shell/domain) → explicitly out of scope, matches "pages" request.
- Placeholder scan: none — every task lists exact functions and line ranges.
- Type consistency: all moved signatures byte-identical; only `pub(super)` visibility added.
- Risk: moving `use crate::*;` glob between files could change name resolution — clippy `-D warnings` + per-page render tests are the tripwire; fix by adding the specific missing `use` per file.
