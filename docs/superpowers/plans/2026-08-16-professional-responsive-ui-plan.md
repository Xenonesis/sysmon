# Professional & Responsive UI/UX Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform SysMon's user interface into a sleek, responsive, cockpit-dense telemetry flight deck with a 3-way theme engine (Dark / Light / System auto-detect), categorized navigation sidebar, dynamic multi-column responsive grid, and polished data views across all 14 modules.

**Architecture:** Extend `AppSettings` with an `AppTheme` enum and dynamic `ThemePalette` queryable via active theme or Windows registry. Upgrade `src/ui/components.rs` with precision circular gauges and responsive card frames. Modernize `src/main.rs` with a grouped 3-tier sidebar, 42px global command header, and responsive content layout. Upgrade individual page views (`overview.rs`, `processes.rs`, `services.rs`, `startup_manager.rs`, `diagnostics.rs`, `ram_cleaner.rs`, `settings.rs`) for strict monospace data alignment and fluid column scaling.

**Tech Stack:** Rust 2021, `egui` / `eframe` 0.28, `winreg` 0.56, `sysinfo` 0.30, `chrono` 0.4.

## Global Constraints

- Strict adherence to `DESIGN.md`: Terminal Noir dark palette (`#09090B` / `#18181B`), Clean Slate light palette (`#F4F4F5` / `#FFFFFF`), Diagnostic Emerald (`#10B981`), Warning Amber (`#F59E0B`), Critical Red (`#EF4444`).
- Monospace numerals mandatory for all numerical telemetry, timestamps, byte rates, PIDs, and table data.
- Zero UI thread blocking. All layout computations must remain lockless or minimal-lock.
- Responsive breakpoints: `< 850px` (Compact / Stacked), `850px – 1150px` (Standard Grid), `>= 1150px` (Wide Desktop).
- Per user directive: Do NOT run `git commit` or `git push` during execution so the user can test the UI locally first.

---

### Task 1: 3-Way Theme Engine (`AppTheme` & Dynamic `ThemePalette`)

**Files:**
- Modify: `src/app/models.rs`
- Modify: `src/ui/theme.rs`
- Modify: `src/monitoring/engine.rs:957-1035`
- Test: `src/ui/theme.rs` (inline test module)

**Interfaces:**
- Produces:
  - `pub enum AppTheme { Dark, Light, System }` in `src/app/models.rs`
  - `ThemePalette::is_dark(theme: AppTheme) -> bool` in `src/ui/theme.rs`
  - `ThemePalette::bg_deepest(is_dark: bool) -> egui::Color32`
  - `ThemePalette::bg_surface(is_dark: bool) -> egui::Color32`
  - `ThemePalette::bg_card(is_dark: bool) -> egui::Color32`
  - `ThemePalette::bg_track(is_dark: bool) -> egui::Color32`
  - `ThemePalette::border(is_dark: bool) -> egui::Color32`
  - `ThemePalette::text_primary(is_dark: bool) -> egui::Color32`
  - `ThemePalette::text_secondary(is_dark: bool) -> egui::Color32`
  - `ThemePalette::text_dimmed(is_dark: bool) -> egui::Color32`

- [ ] **Step 1: Define `AppTheme` enum and backward-compatible deserialization in `src/app/models.rs`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
    System,
}
```
Update `AppSettings` struct: replace `pub(crate) theme_dark: bool` with `pub(crate) theme: AppTheme`, and provide backward compatibility helper for serde if `theme_dark` is encountered in legacy `settings.json`.

- [ ] **Step 2: Implement dynamic palette and Windows theme detection in `src/ui/theme.rs`**

Add Windows registry lookup for `System` mode:
```rust
#[cfg(target_os = "windows")]
pub fn is_windows_dark_mode() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize") {
        if let Ok(val) = key.get_value::<u32, _>("AppsUseLightTheme") {
            return val == 0;
        }
    }
    true // default to dark if unreadable
}

#[cfg(not(target_os = "windows"))]
pub fn is_windows_dark_mode() -> bool {
    true
}
```

Implement dynamic resolution methods on `ThemePalette` taking `is_dark: bool` while preserving the semantic constants (`ACCENT_PRIMARY`, `STATUS_HEALTHY`, `STATUS_WARNING`, `STATUS_CRITICAL`).

- [ ] **Step 3: Write unit tests for `AppTheme` resolution and palette contrast**

In `src/ui/theme.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::models::AppTheme;

    #[test]
    fn theme_resolution_works() {
        assert!(ThemePalette::is_dark_mode(AppTheme::Dark));
        assert!(!ThemePalette::is_dark_mode(AppTheme::Light));
    }

    #[test]
    fn dynamic_palette_contrast() {
        let dark_bg = ThemePalette::bg_deepest(true);
        let dark_text = ThemePalette::text_primary(true);
        assert_ne!(dark_bg, dark_text);

        let light_bg = ThemePalette::bg_deepest(false);
        let light_text = ThemePalette::text_primary(false);
        assert_ne!(light_bg, light_text);
    }
}
```

- [ ] **Step 4: Update `monitoring/engine.rs` visuals configuration**

Update `monitoring/engine.rs` to configure `egui::Visuals` based on `ThemePalette::is_dark_mode(settings.theme)`.

- [ ] **Step 5: Run tests to verify**

Run: `cargo test --locked theme::tests`
Expected: PASS

---

### Task 2: Precision UI Components & Responsive Layout Helpers

**Files:**
- Modify: `src/ui/components.rs`
- Test: `src/ui/components.rs` (inline test module)

**Interfaces:**
- Consumes: `ThemePalette`, `egui`
- Produces:
  - `pub(crate) fn paint_section_header(ui: &mut egui::Ui, text: &str, is_dark: bool)`
  - `pub(crate) fn paint_circular_gauge(ui: &mut egui::Ui, center: egui::Pos2, radius: f32, fraction: f32, color: egui::Color32, is_dark: bool)`
  - `pub(crate) fn card_frame(is_dark: bool) -> egui::Frame`
  - `pub(crate) fn status_pill(ui: &mut egui::Ui, label: &str, color: egui::Color32, is_dark: bool)`

- [ ] **Step 1: Update circular gauge with anti-aliased backdrop track and proportional typography**

Refactor `paint_circular_gauge` in `src/ui/components.rs` to render:
1. Outer subtle border / shadow stroke.
2. Background track circle (`ThemePalette::bg_track(is_dark)` with 4.5px stroke).
3. Foreground progress arc using `color` with smooth end caps.
4. Clean monospace center readout with proportional sub-label.

- [ ] **Step 2: Add responsive card frame and status pill helpers**

Implement `card_frame(is_dark: bool) -> egui::Frame`:
```rust
pub(crate) fn card_frame(is_dark: bool) -> egui::Frame {
    egui::Frame::none()
        .fill(ThemePalette::bg_surface(is_dark))
        .stroke(egui::Stroke::new(1.0, ThemePalette::border(is_dark)))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(14.0, 12.0))
}

pub(crate) fn status_pill(ui: &mut egui::Ui, label: &str, color: egui::Color32, is_dark: bool) {
    let frame = egui::Frame::none()
        .fill(color.gamma_multiply(if is_dark { 0.15 } else { 0.12 }))
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.4)))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(8.0, 3.0));
    frame.show(ui, |ui| {
        ui.label(egui::RichText::new(label).size(11.0).strong().color(color));
    });
}
```

- [ ] **Step 3: Update linear progress bar and details row for dynamic themes**

Ensure `paint_progress_bar` and `details_row` use `is_dark` theme-aware background tracks and text colors.

- [ ] **Step 4: Write component unit tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_and_byte_formatting_are_monospace_safe() {
        assert_eq!(format_rate(0.0), "0 B/s");
        assert_eq!(format_rate(1.5), "1.5 MB/s");
        assert_eq!(format_rate(1024.0), "1.00 GB/s");
    }
}
```

- [ ] **Step 5: Run tests to verify**

Run: `cargo test --locked ui::components::tests`
Expected: PASS

---

### Task 3: Categorized Navigation Sidebar & Global Command Header

**Files:**
- Modify: `src/main.rs:650-888`
- Modify: `src/app/models.rs` (Tab categories / helper methods)

**Interfaces:**
- Consumes: `ThemePalette`, `components::*`, `AppTheme`
- Produces:
  - 3-Category Sidebar (`TELEMETRY`, `SYSTEM CONTROL`, `DIAGNOSTICS & HEALTH`) with active accent indicator and alert pill.
  - 42px Global Status & Quick Action Header with Clean RAM and Record toggles.

- [ ] **Step 1: Refactor Sidebar in `src/main.rs`**

Replace lines 650–770 with categorized grouping:
1. **Brand Bar**: Painted emerald diamond glyph, "SysMon" logo typography, version pill `v3.7.2`.
2. **Section Groups**:
   - `TELEMETRY`: Overview, Performance, CPU Cores, Storage, Network.
   - `SYSTEM CONTROL`: Processes, Services, Startup Apps, RAM Cleaner.
   - `DIAGNOSTICS & HEALTH`: Diagnostics, System Info, Alerts (with dynamic count badge `[ N ]` when alerts > 0).
3. **Tab Button Styling**:
   - Active: Deep Surface fill, 3px solid `#10B981` vertical left edge line, bold Primary Ink.
   - Inactive: Transparent fill, Muted Steel text, `#27272A` on hover.
4. **Utility Dock**: Pinned `Settings`, `Shortcuts`, `About` with live heartbeat label (`Updated: HH:MM:SS`).

- [ ] **Step 2: Modernize Global Status Header in `src/main.rs`**

Refactor `egui::TopBottomPanel::top("global_status_bar")`:
- Left side: Live telemetry metrics (`CPU: XX.X%`, `RAM: XX.X%`, `GPU: XX.X%`, `NET: X.X MB/s`) with threshold color pills.
- Right side:
  - `[ 🧹 Clean RAM ]` button triggering one-click manual cleanup.
  - `[ ⏺ Record ]` button toggling diagnostic session recording.
  - `[ ⚠ N Alerts ]` badge opening Alerts view directly.

- [ ] **Step 3: Verify compilation and run test suite**

Run: `cargo clippy --locked --all-targets -- -D warnings && cargo test --locked`
Expected: PASS

---

### Task 4: Responsive Overview Dashboard Redesign

**Files:**
- Modify: `src/ui/pages/overview.rs`

**Interfaces:**
- Consumes: `SystemData`, `ThemePalette`, `components::*`
- Produces:
  - Responsive 5-card metric flight deck (CPU, RAM, GPU, Disk, Network) with dynamic column wrapping.
  - Monospace hardware summary strip with detected CPU, clock speed, GPU adapter, and uptime.
  - Redesigned Top Processes table preview with monospace alignment and memory ratio bars.

- [ ] **Step 1: Implement Dynamic Breakpoint-Aware Metric Grid in `overview.rs`**

Calculate available width and wrap cards accordingly:
- If `avail_width >= 1100.0`: 5 cards across in 1 row.
- If `750.0 <= avail_width < 1100.0`: 3 cards on row 1, 2 cards on row 2.
- If `avail_width < 750.0`: 2 cards per row.

Each card renders:
- Monospace title with uppercase tracking.
- Centered circular progress gauge with dynamic threshold color.
- Bold primary metric value (e.g., `18.1%`, `10.4 GB`, `1.2 MB/s`).
- Secondary subtitle (e.g., `12 Cores · 3.8 GHz`, `4.8 GB Free`, `▲ 120 KB/s`).

- [ ] **Step 2: Modernize Hardware Spec & Uptime Banner**

Replace the generic group with a sleek 1px-bordered spec strip:
- Detected CPU model with frequency and thermal badge.
- Dedicated GPU model name (`NVIDIA GeForce RTX 3060 Laptop GPU` or Intel/AMD adapter).
- Monospace Uptime pill (`Uptime: 1d 7h 15m`).
- Health status indicator dot (`● System Healthy`).

- [ ] **Step 3: Redesign Top Processes Preview Table**

Format the top processes table:
- Monospace columns for PID, Process Name, Memory (MB), CPU %.
- Inline visual memory bar.
- Quick link to open full Process Manager window.

- [ ] **Step 4: Test Overview rendering and responsiveness**

Run: `cargo test --locked --all-targets`
Expected: PASS

---

### Task 5: System Control Pages Polish (`processes.rs`, `services.rs`, `startup_manager.rs`, `ram_cleaner.rs`)

**Files:**
- Modify: `src/ui/pages/processes.rs`
- Modify: `src/ui/pages/services.rs`
- Modify: `src/ui/pages/startup_manager.rs`
- Modify: `src/ui/pages/ram_cleaner.rs`

**Interfaces:**
- Consumes: `ThemePalette`, `components::*`, `ActionCommand`

- [ ] **Step 1: Polish `processes.rs`**
  - Integrated search input with clear button `[×]` and process count pill `Showing 142 / 295`.
  - Column sort headers with active sort direction indicator (`▲` / `▼`).
  - Monospace tabular alignment for PID, CPU %, Memory MB.
  - Clean row hover highlight with inline action triggers (`Kill`, `Suspend`, `Details`).

- [ ] **Step 2: Polish `services.rs`**
  - Service state pills (`RUNNING` in Emerald, `STOPPED` in Dimmed Steel).
  - Search filter by display name or service name.
  - Guarded action buttons with clear elevation requirement indicators.

- [ ] **Step 3: Polish `startup_manager.rs`**
  - High-contrast impact tier badges (`HIGH` Red, `MED` Amber, `LOW` Emerald).
  - Boot time benchmark readout in monospace font.
  - Signed status indicator (`Verified Publisher` vs `Unsigned`).

- [ ] **Step 4: Polish `ram_cleaner.rs`**
  - Dual memory visualizer bar (Used vs Target vs Excluded).
  - Clean slider controls with numeric percentage inputs.
  - Real-time result cards showing `Trimmed: X MB | Skipped: Y | Passes: Z`.

- [ ] **Step 5: Run tests**

Run: `cargo test --locked --all-targets`
Expected: PASS

---

### Task 6: Diagnostics, Settings & Remaining Views Polish

**Files:**
- Modify: `src/ui/pages/diagnostics.rs`
- Modify: `src/ui/pages/settings.rs`
- Modify: `src/ui/pages/performance.rs`
- Modify: `src/ui/pages/storage.rs`
- Modify: `src/ui/pages/network.rs`
- Modify: `src/ui/pages/system_info.rs`
- Modify: `src/ui/pages/alerts.rs`
- Modify: `src/ui/pages/about.rs`
- Modify: `src/ui/pages/cpu_cores.rs`

**Interfaces:**
- Consumes: `ThemePalette`, `components::*`, `AppTheme`

- [ ] **Step 1: Upgrade `settings.rs` for 3-Way Theme Selector**
  - Replace checkbox with 3-way radio/segmented selector: `● Dark (Terminal Noir)`, `○ Light (Clean Slate)`, `○ System (Auto-Detect)`.
  - Live preview update when theme changes.

- [ ] **Step 2: Polish `diagnostics.rs`**
  - Anomaly finding cards with severity badges, evidence metrics, confidence meters, and recommendation steps.
  - Active session recorder banner with pulsing recording indicator and elapsed time.

- [ ] **Step 3: Polish `performance.rs`, `storage.rs`, `network.rs`, `system_info.rs`, `alerts.rs`, `about.rs`, `cpu_cores.rs`**
  - Apply monospace typography to all metric tables and history summaries.
  - Standardize 1px `Raw Outline` borders and responsive scroll areas.

- [ ] **Step 4: Verify full test suite and quality gates**

Run:
```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```
Expected: All gates pass with 0 errors/warnings.

---

### Task 7: Full System Verification & Interactive Review Build

**Files:**
- Verify: All compiled binaries and assets.

- [ ] **Step 1: Build release executable**

Run: `cargo build --locked --release --bin system-monitor`
Expected: `target\release\system-monitor.exe` compiled successfully.

- [ ] **Step 2: Build Inno Setup installer**

Run: `"C:\Program Files (x86)\Inno Setup 6\iscc.exe" /DAppVersion=3.7.2 installer.iss`
Expected: `downloads\SystemMonitor-3.7.2-setup.exe` compiled successfully.

- [ ] **Step 3: Launch application for user visual testing**

Run: `powershell -NoProfile -Command "Start-Process .\target\release\system-monitor.exe"`
Instruct user to interactively test the UI across window resizing, dark/light/system theme switching, and navigation across all 14 pages.
