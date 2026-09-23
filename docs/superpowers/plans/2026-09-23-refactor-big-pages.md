# Refactor Big Pages into Focused Components - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decompose the remaining oversized UI page files (`src/ui/pages/timeline.rs`, `src/ui/pages/processes/table.rs`, `src/ui/pages/ram_cleaner.rs`, `src/ui/pages/alerts/mod.rs`, and `src/ui/pages/network/interfaces.rs`) into clean, focused sub-component modules under 200 lines each, with 100% feature parity, no unused imports or orphaned files, zero clippy warnings, and clean tests.

**Architecture:** Each monolithic or oversized page/component is transformed into or refactored into cohesive modular sub-components with a lightweight coordinator `mod.rs` (or coordinator parent), separating rendering into distinct concerns (e.g. controls, plots, cards, tables, details panels). All public and crate signatures remain 100% stable.

**Tech Stack:** Rust 2021 (1.85+), `eframe` / `egui` 0.31, `egui_plot` 0.31, `parking_lot`, `chrono`.

## Global Constraints
- **Zero Functionality Loss:** Every UI element, plot, envelope algorithm, virtualized row, sort handler, drag-and-drop cue, action trigger, and test must function identically to the pre-refactor state.
- **Contract & Export Stability:** Entry points (`pub(crate) fn show(...)`, `paint_process_table(...)`, etc.) preserve their exact signatures and behavior.
- **Cleanliness:** No unused imports (`unused_imports`), dead code warnings, or leftover orphaned files.
- **Quality Gates:** Every task must pass `cargo test --locked --all-targets`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo fmt --check`.
- **Target File Size:** All created/refactored files aim for under 200 lines (and strictly <= 250 lines).

---

### Task 1: Decompose `src/ui/pages/timeline.rs` into Modular Package `src/ui/pages/timeline/`

**Files:**
- Create: `src/ui/pages/timeline/mod.rs` (~110 lines) - Header, enable banner, range picker, coordination.
- Create: `src/ui/pages/timeline/plots.rs` (~130 lines) - Envelope aggregation algorithm and utilization + disk/network throughput plots.
- Create: `src/ui/pages/timeline/events.rs` (~120 lines) - Event rail list, paging, time formatting, event kind badges.
- Create: `src/ui/pages/timeline/analysis.rs` (~70 lines) - Evidence-based analysis card and contributor details.
- Remove: `src/ui/pages/timeline.rs`
- Modify: `src/ui/pages/mod.rs` (update module reference if needed, though `pub(crate) mod timeline;` works identically for folder modules).

**Interfaces:**
- Consumes: `crate::SystemMonitorApp`, `crate::timeline::*`, `crate::ui::components::*`, `crate::ui::theme::ThemePalette`.
- Produces: `pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui)`.

- [ ] **Step 1: Create `src/ui/pages/timeline/plots.rs` with `envelope` and plot rendering**
- [ ] **Step 2: Create `src/ui/pages/timeline/events.rs` with event rail and pagination**
- [ ] **Step 3: Create `src/ui/pages/timeline/analysis.rs` with evidence analysis display**
- [ ] **Step 4: Create `src/ui/pages/timeline/mod.rs` binding all subcomponents, remove old `timeline.rs`**
- [ ] **Step 5: Verify tests and quality gates (`cargo test --test timeline*` and `cargo clippy`)**
- [ ] **Step 6: Commit changes**

```bash
git add src/ui/pages/timeline/ src/ui/pages/mod.rs
git rm src/ui/pages/timeline.rs
git commit -m "refactor(ui): modularize timeline page into sub-component package"
```

---

### Task 2: Modularize Process Table in `src/ui/pages/processes/`

**Files:**
- Create: `src/ui/pages/processes/details_panel.rs` (~70 lines) - Process inspection details card (executable, command line, cwd, parent PID, etc.).
- Create: `src/ui/pages/processes/table_header.rs` (~90 lines) - Sticky header rendering, column widths computation, and sort button handlers.
- Modify: `src/ui/pages/processes/table.rs` (reduce from 433 lines to ~170 lines) - Virtualized row rendering coordination.
- Modify: `src/ui/pages/processes/mod.rs` - Register new modules `details_panel` and `table_header`.

**Interfaces:**
- Consumes: `crate::processes::ProcessInfo`, `ProcessSortColumn`, `paint_row_actions`.
- Produces: `pub(super) fn paint_process_table(...)`, `pub(super) fn paint_process_details(...)`.

- [ ] **Step 1: Extract `details_panel.rs` from `table.rs`**
- [ ] **Step 2: Extract `table_header.rs` (column widths and sortable headers) from `table.rs`**
- [ ] **Step 3: Refactor `table.rs` to consume header and details subcomponents**
- [ ] **Step 4: Verify with `cargo test --locked --all-targets` and `cargo clippy`**
- [ ] **Step 5: Commit changes**

```bash
git add src/ui/pages/processes/
git commit -m "refactor(ui): split process table into table_header, details_panel, and row renderer"
```

---

### Task 3: Decompose `src/ui/pages/ram_cleaner.rs` into Modular Package `src/ui/pages/ram_cleaner/`

**Files:**
- Create: `src/ui/pages/ram_cleaner/mod.rs` (~60 lines) - Coordinator `show` function.
- Create: `src/ui/pages/ram_cleaner/status_card.rs` (~70 lines) - Memory status card, progress bar, elevation pill.
- Create: `src/ui/pages/ram_cleaner/manual_clean.rs` (~60 lines) - Manual working set clean card and trigger.
- Create: `src/ui/pages/ram_cleaner/policy_config.rs` (~120 lines) - Auto-cleaning sliders, exclusions, toggles.
- Create: `src/ui/pages/ram_cleaner/stats_card.rs` (~50 lines) - Session statistics, freed MB, clean passes.
- Remove: `src/ui/pages/ram_cleaner.rs`
- Modify: `src/ui/pages/mod.rs` (remains `pub(crate) mod ram_cleaner;`).

**Interfaces:**
- Consumes: `crate::SystemMonitorApp`, `SystemData`, `ThemePalette`.
- Produces: `pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData)`.

- [ ] **Step 1: Create `src/ui/pages/ram_cleaner/status_card.rs` and `stats_card.rs`**
- [ ] **Step 2: Create `src/ui/pages/ram_cleaner/manual_clean.rs` and `policy_config.rs`**
- [ ] **Step 3: Create `src/ui/pages/ram_cleaner/mod.rs` and remove `src/ui/pages/ram_cleaner.rs`**
- [ ] **Step 4: Verify with `cargo test --locked --all-targets` and `cargo clippy`**
- [ ] **Step 5: Commit changes**

```bash
git add src/ui/pages/ram_cleaner/ src/ui/pages/mod.rs
git rm src/ui/pages/ram_cleaner.rs
git commit -m "refactor(ui): modularize ram cleaner page into sub-component package"
```

---

### Task 4: Modularize Alerts Page Coordinator (`src/ui/pages/alerts/mod.rs`)

**Files:**
- Create: `src/ui/pages/alerts/dispatcher.rs` (~70 lines) - Alert state mutations, test alert simulation, audio chime trigger, tab navigation, and clear actions.
- Modify: `src/ui/pages/alerts/mod.rs` (reduce from 368 lines to ~130 lines) - Coordinator layout and unit tests.

**Interfaces:**
- Consumes: `AlertInfo`, `AlertType`, `SystemData`.
- Produces: `pub(super) fn handle_alert_actions(...)`.

- [ ] **Step 1: Extract `src/ui/pages/alerts/dispatcher.rs` for alert state modification and simulation**
- [ ] **Step 2: Refactor `src/ui/pages/alerts/mod.rs` to use `dispatcher.rs` and streamline UI layout**
- [ ] **Step 3: Verify existing headless and alert incident tests in `alerts/mod.rs`**
- [ ] **Step 4: Commit changes**

```bash
git add src/ui/pages/alerts/
git commit -m "refactor(ui): streamline alerts coordinator and extract alert action dispatcher"
```

---

### Task 5: Modularize Network Interfaces Page (`src/ui/pages/network/interfaces.rs`)

**Files:**
- Create: `src/ui/pages/network/adapter_card.rs` (~120 lines) - Individual network adapter card rendering, IP badges, MAC address, link speed, and socket counters.
- Modify: `src/ui/pages/network/interfaces.rs` (reduce from 328 lines to ~140 lines) - Interface list aggregation, bandwidth summary, and filtering.

**Interfaces:**
- Consumes: `crate::network::NetworkInterfaceInfo`, `ThemePalette`.
- Produces: `pub(super) fn paint_adapter_card(...)`.

- [ ] **Step 1: Extract `adapter_card.rs` from `interfaces.rs`**
- [ ] **Step 2: Refactor `interfaces.rs` to iterate and render adapter cards cleanly**
- [ ] **Step 3: Verify network tests and clippy**
- [ ] **Step 4: Commit changes**

```bash
git add src/ui/pages/network/
git commit -m "refactor(ui): extract network adapter card component from interfaces page"
```

---

### Task 6: Final Verification, Unused Imports Cleanup & Quality Gates

**Files:**
- Audit all touched files for any unused imports, unnecessary re-exports, or stale comments.

- [ ] **Step 1: Run comprehensive tests across all packages**
Run: `cargo test --locked --all-targets`
Expected: 387+ tests PASS, 0 failures.

- [ ] **Step 2: Run strict clippy with all warnings denied**
Run: `cargo clippy --locked --all-targets -- -D warnings`
Expected: 0 errors, 0 warnings.

- [ ] **Step 3: Check code formatting**
Run: `cargo fmt --check`
Expected: Clean with no discrepancies.

- [ ] **Step 4: Verify file size report**
Verify that all touched files are well under 250 lines.

- [ ] **Step 5: Commit final cleanup if any adjustments made**
