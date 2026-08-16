# Big Pages Refactoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decompose monolithic UI page files (`overview.rs`, `startup_manager.rs`, `system_info.rs`, `processes.rs`, `settings.rs`, `performance.rs`, `network.rs`) into cohesive, modular sub-component packages under `src/ui/pages/` while maintaining 100% feature parity, layout fidelity, type safety, and test coverage.

**Architecture:** Each large page file is transformed into a module directory (e.g., `src/ui/pages/overview/`) containing a coordinator `mod.rs` exposing the canonical `pub(crate) fn show(...)` entry point alongside focused sub-component widgets (`metric_cards.rs`, `hardware_banner.rs`, etc.). This preserves the exact public interface and tab routing contract with `src/main.rs` while isolating rendering logic, state management, and user action handlers into independently testable files under 250 lines each.

**Tech Stack:** Rust 2021 (1.85+), `eframe` / `egui` 0.28, `parking_lot`, `chrono`, `sysinfo`, Windows API / WMI bindings.

## Global Constraints

- **Zero Functionality Loss:** Every UI element, card, table column, sort selector, search filter, action button, tooltip, confirmation dialog, status badge, and responsive layout breakpoint must render and behave identically to the pre-refactor state.
- **Contract Stability:** The public entry point for all pages remains `pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData)` (or `pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui)` for self-loading pages like Startup Manager).
- **Module Declarations:** `src/ui/pages/mod.rs` continues to declare `pub(crate) mod <page>;` with zero breaking changes to existing imports.
- **Quality Gates:** Every task must pass `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, and all unit and UI render tests (`cargo test --locked`).
- **File Length Target:** No individual sub-component file should exceed 250 lines.

---

### Task 1: Refactor Overview Page into Modular Subcomponents

**Files:**
- Create: `src/ui/pages/overview/mod.rs`
- Create: `src/ui/pages/overview/metric_cards.rs`
- Create: `src/ui/pages/overview/hardware_banner.rs`
- Create: `src/ui/pages/overview/core_bars.rs`
- Create: `src/ui/pages/overview/health_deck.rs`
- Create: `src/ui/pages/overview/top_processes.rs`
- Remove: `src/ui/pages/overview.rs`
- Test: `tests/overview_render_test.rs` (or embedded test module in `src/ui/pages/overview/mod.rs`)

**Interfaces:**
- Consumes: `crate::SystemMonitorApp`, `crate::SystemData`, `crate::ui::components::*`, `crate::ui::theme::ThemePalette`
- Produces:
  - `pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData)`
  - `pub(crate) fn calculate_metric_grid_rows(avail_w: f32) -> Vec<Vec<usize>>`
  - `pub(crate) fn format_uptime(uptime_secs: u64) -> String`

- [ ] **Step 1: Write the test verifying metric grid calculation and full page rendering**

```rust
// Add to src/ui/pages/overview/mod.rs in tests module
#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::engine::SystemMonitorApp;
    use crate::app::models::SystemData;

    #[test]
    fn test_calculate_metric_grid_rows_breakpoints() {
        assert_eq!(calculate_metric_grid_rows(1200.0), vec![vec![0, 1, 2, 3, 4]]);
        assert_eq!(calculate_metric_grid_rows(1050.0), vec![vec![0, 1, 2, 3, 4]]);
        assert_eq!(calculate_metric_grid_rows(900.0), vec![vec![0, 1, 2], vec![3, 4]]);
        assert_eq!(calculate_metric_grid_rows(700.0), vec![vec![0, 1, 2], vec![3, 4]]);
        assert_eq!(calculate_metric_grid_rows(650.0), vec![vec![0, 1], vec![2, 3], vec![4]]);
        assert_eq!(calculate_metric_grid_rows(400.0), vec![vec![0, 1], vec![2, 3], vec![4]]);
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(0), "0d 0h 0m");
        assert_eq!(format_uptime(3665), "0d 1h 1m");
        assert_eq!(format_uptime(86400 + 7200 + 180), "1d 2h 3m");
    }

    #[test]
    fn test_overview_render_all_states() {
        let mut app = SystemMonitorApp::test_app();
        let mut data = SystemData::default();
        data.memory_total = 16 * 1024 * 1024 * 1024;
        data.memory_used = 8 * 1024 * 1024 * 1024;
        data.memory_percentage = 50.0;
        data.cpu_usage = 25.0;

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui, &data);
            });
        });
    }
}
```

- [ ] **Step 2: Implement `src/ui/pages/overview/metric_cards.rs`**

```rust
use crate::ui::components::*;
use crate::ui::theme::ThemePalette;
use crate::app::models::SystemData;
use eframe::egui;

pub(crate) struct MetricCard {
    pub(crate) title: &'static str,
    pub(crate) accent: egui::Color32,
    pub(crate) value_text: String,
    pub(crate) subtitle: String,
    pub(crate) fraction: f32,
    pub(crate) color: egui::Color32,
    pub(crate) status_label: &'static str,
}

pub(crate) fn paint_metric_card(ui: &mut egui::Ui, cr: egui::Rect, card: &MetricCard, is_dark: bool) {
    let card_bg = ThemePalette::bg_card(is_dark);
    let card_border = egui::Stroke::new(1.0, ThemePalette::border(is_dark));
    let card_rnd = egui::Rounding::same(6.0);

    ui.painter().rect_filled(cr, card_rnd, card_bg);
    ui.painter().rect_stroke(cr, card_rnd, card_border);

    ui.painter().circle_filled(cr.min + egui::vec2(14.0, 14.0), 3.0, card.accent);
    ui.painter().text(
        cr.min + egui::vec2(22.0, 8.0),
        egui::Align2::LEFT_TOP,
        card.title,
        egui::FontId::monospace(10.5),
        ThemePalette::text_secondary(is_dark),
    );

    let status_bg = card.color.gamma_multiply(if is_dark { 0.15 } else { 0.12 });
    let status_border = egui::Stroke::new(1.0, card.color.gamma_multiply(0.4));
    let badge_rect = egui::Rect::from_min_size(egui::pos2(cr.max.x - 72.0, cr.min.y + 8.0), egui::vec2(60.0, 18.0));
    ui.painter().rect_filled(badge_rect, egui::Rounding::same(3.0), status_bg);
    ui.painter().rect_stroke(badge_rect, egui::Rounding::same(3.0), status_border);
    ui.painter().text(
        badge_rect.center(),
        egui::Align2::CENTER_CENTER,
        card.status_label,
        egui::FontId::monospace(9.0),
        card.color,
    );

    ui.painter().text(
        cr.min + egui::vec2(14.0, 32.0),
        egui::Align2::LEFT_TOP,
        &card.value_text,
        egui::FontId::monospace(22.0),
        ThemePalette::text_primary(is_dark),
    );

    let bar_margin_x = 14.0;
    let bar_w = cr.width() - (bar_margin_x * 2.0);
    let bar_h = 4.5;
    let bar_y = cr.min.y + 66.0;
    let bar_track_rect = egui::Rect::from_min_size(egui::pos2(cr.min.x + bar_margin_x, bar_y), egui::vec2(bar_w, bar_h));
    let bar_rnd = egui::Rounding::same(2.0);

    ui.painter().rect_filled(bar_track_rect, bar_rnd, ThemePalette::bg_deepest(is_dark));
    ui.painter().rect_stroke(bar_track_rect, bar_rnd, egui::Stroke::new(1.0, ThemePalette::bg_track(is_dark)));

    let filled_w = (bar_w * card.fraction.clamp(0.0, 1.0)).max(2.0);
    let bar_fill_rect = egui::Rect::from_min_size(bar_track_rect.min, egui::vec2(filled_w, bar_h));
    ui.painter().rect_filled(bar_fill_rect, bar_rnd, card.color);

    ui.painter().text(
        cr.min + egui::vec2(14.0, cr.height() - 11.0),
        egui::Align2::LEFT_BOTTOM,
        &card.subtitle,
        egui::FontId::monospace(10.0),
        ThemePalette::text_dimmed(is_dark),
    );
}

pub(crate) fn build_overview_cards(data: &SystemData, is_dark: bool) -> [MetricCard; 5] {
    let cpu_c = get_usage_color(data.cpu_usage);
    let mem_c = get_usage_color(data.memory_percentage);

    let net_total_rate = data.network_info.iter().map(|n| n.received_rate + n.transmitted_rate).sum::<f64>();
    let net_download_rate = data.network_info.iter().map(|n| n.received_rate).sum::<f64>();
    let net_upload_rate = data.network_info.iter().map(|n| n.transmitted_rate).sum::<f64>();
    let net_c = if net_total_rate > 25.0 {
        ThemePalette::STATUS_CRITICAL
    } else if net_total_rate > 5.0 {
        ThemePalette::STATUS_WARNING
    } else if net_total_rate > 0.05 {
        ThemePalette::STATUS_HEALTHY
    } else {
        ThemePalette::text_dimmed(is_dark)
    };

    let disk_total_rate = data.disk_read_rate + data.disk_write_rate;
    let disk_c = if disk_total_rate > 100.0 {
        ThemePalette::STATUS_CRITICAL
    } else if disk_total_rate > 20.0 {
        ThemePalette::STATUS_WARNING
    } else if disk_total_rate > 0.05 {
        ThemePalette::STATUS_HEALTHY
    } else {
        ThemePalette::text_dimmed(is_dark)
    };

    let (gpu_sub, gpu_frac, gpu_c) = if let Some(gpu) = data.gpu_info.first() {
        let c = get_usage_color(gpu.utilization);
        let sub = if let (Some(u), Some(t)) = (gpu.memory_used, gpu.memory_total) {
            format!("{:.0}/{:.0} MB", bytes_to_mb(u), bytes_to_mb(t))
        } else if let Some(mhz) = gpu.clock_mhz {
            format!("{} MHz", mhz)
        } else {
            if gpu.name.chars().count() > 20 {
                let truncated: String = gpu.name.chars().take(18).collect();
                format!("{}…", truncated)
            } else {
                gpu.name.clone()
            }
        };
        (sub, (gpu.utilization / 100.0).clamp(0.0, 1.0), c)
    } else {
        ("Not detected".to_string(), 0.0, ThemePalette::text_dimmed(is_dark))
    };

    let cpu_sub = if let Some(temp) = data.cpu_temperature {
        format!("{} Cores · {:.0}°C", data.cpu_cores.len(), temp)
    } else {
        format!("{} Cores", data.cpu_cores.len())
    };

    [
        MetricCard {
            title: "CPU LOAD",
            accent: ThemePalette::ACCENT_PRIMARY,
            value_text: format!("{:.1}%", data.cpu_usage),
            subtitle: cpu_sub,
            fraction: (data.cpu_usage / 100.0).clamp(0.0, 1.0),
            color: cpu_c,
            status_label: if data.cpu_usage > 90.0 {
                "CRITICAL"
            } else if data.cpu_usage > 70.0 {
                "ELEVATED"
            } else {
                "NOMINAL"
            },
        },
        MetricCard {
            title: "MEMORY",
            accent: ThemePalette::ACCENT_ACTIVE,
            value_text: format!("{:.1}%", data.memory_percentage),
            subtitle: format!("{:.1} / {:.1} GB", bytes_to_gb(data.memory_used), bytes_to_gb(data.memory_total)),
            fraction: (data.memory_percentage / 100.0).clamp(0.0, 1.0),
            color: mem_c,
            status_label: if data.memory_percentage > 90.0 {
                "CRITICAL"
            } else if data.memory_percentage > 75.0 {
                "ELEVATED"
            } else {
                "NOMINAL"
            },
        },
        MetricCard {
            title: "GPU ENGINE",
            accent: ThemePalette::text_secondary(is_dark),
            value_text: if data.gpu_info.is_empty() {
                "N/A".to_string()
            } else {
                format!("{:.1}%", data.gpu_info[0].utilization)
            },
            subtitle: gpu_sub,
            fraction: gpu_frac,
            color: gpu_c,
            status_label: if data.gpu_info.is_empty() {
                "STANDBY"
            } else if data.gpu_info[0].utilization > 90.0 {
                "CRITICAL"
            } else {
                "ONLINE"
            },
        },
        MetricCard {
            title: "STORAGE I/O",
            accent: ThemePalette::text_secondary(is_dark),
            value_text: format_rate(disk_total_rate),
            subtitle: format!("R: {} · W: {}", format_rate(data.disk_read_rate), format_rate(data.disk_write_rate)),
            fraction: ((disk_total_rate / 200.0).clamp(0.0, 1.0) as f32),
            color: disk_c,
            status_label: if disk_total_rate > 100.0 {
                "CRITICAL"
            } else if disk_total_rate > 20.0 {
                "ACTIVE"
            } else {
                "IDLE"
            },
        },
        MetricCard {
            title: "NETWORK FLOW",
            accent: ThemePalette::text_secondary(is_dark),
            value_text: format_rate(net_total_rate),
            subtitle: format!("↓ {} · ↑ {}", format_rate(net_download_rate), format_rate(net_upload_rate)),
            fraction: ((net_total_rate / 10.0).clamp(0.0, 1.0) as f32),
            color: net_c,
            status_label: if net_total_rate > 25.0 {
                "HEAVY"
            } else if net_total_rate > 1.0 {
                "STREAM"
            } else {
                "QUIET"
            },
        },
    ]
}

pub(crate) fn paint_overview_grid(ui: &mut egui::Ui, data: &SystemData, is_dark: bool) {
    let avail_w = ui.available_width();
    let cards = build_overview_cards(data, is_dark);
    let card_spacing = 8.0;
    let card_height = 104.0;
    let rows = super::calculate_metric_grid_rows(avail_w);

    for row_indices in rows {
        let count = row_indices.len() as f32;
        let card_w = if count == 1.0 && avail_w < 700.0 {
            (avail_w - card_spacing) / 2.0
        } else {
            (avail_w - card_spacing * (count - 1.0).max(0.0)) / count
        };

        let (row_rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, card_height), egui::Sense::hover());

        for (col_i, &card_i) in row_indices.iter().enumerate() {
            let x = row_rect.min.x + (card_w + card_spacing) * col_i as f32;
            let card_rect = egui::Rect::from_min_size(egui::pos2(x, row_rect.min.y), egui::vec2(card_w, card_height));
            paint_metric_card(ui, card_rect, &cards[card_i], is_dark);
        }
        ui.add_space(card_spacing);
    }
}
```

- [ ] **Step 3: Implement `hardware_banner.rs`, `core_bars.rs`, `health_deck.rs`, `top_processes.rs`, and coordinator `mod.rs`**

- [ ] **Step 4: Verify test suite and quality gates**

Run: `cargo test --locked --lib ui::pages::overview`
Run: `cargo fmt --all -- --check && cargo clippy --locked --all-targets -- -D warnings`
Expected: PASS with 0 warnings.

- [ ] **Step 5: Commit overview refactoring**

```bash
git add src/ui/pages/overview/
git rm src/ui/pages/overview.rs
git commit -m "refactor(ui): modularize overview page into sub-component package"
```

---

### Task 2: Refactor Startup Manager Page into Modular Subcomponents

**Files:**
- Create: `src/ui/pages/startup_manager/mod.rs`
- Create: `src/ui/pages/startup_manager/summary_card.rs`
- Create: `src/ui/pages/startup_manager/filter_bar.rs`
- Create: `src/ui/pages/startup_manager/item_card.rs`
- Create: `src/ui/pages/startup_manager/action_handler.rs`
- Remove: `src/ui/pages/startup_manager.rs`

**Interfaces:**
- Consumes: `crate::SystemMonitorApp`, `crate::startup::*`, `crate::privilege::*`
- Produces:
  - `pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui)`
  - `pub(crate) fn impact_tier_badge_color(tier: &ImpactTier, is_dark: bool) -> (&'static str, egui::Color32)`

- [ ] **Step 1: Write headless UI tests for startup manager states in `src/ui/pages/startup_manager/mod.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::engine::SystemMonitorApp;
    use crate::startup::{ImpactTier, Recommendation, StartupItem};

    #[test]
    fn test_impact_tier_badge_colors() {
        let (lh, ch) = impact_tier_badge_color(&ImpactTier::High, true);
        assert_eq!(lh, "HIGH");
        assert_eq!(ch, ThemePalette::STATUS_CRITICAL);

        let (lm, cm) = impact_tier_badge_color(&ImpactTier::Medium, true);
        assert_eq!(lm, "MED");
        assert_eq!(cm, ThemePalette::STATUS_WARNING);

        let (ll, cl) = impact_tier_badge_color(&ImpactTier::Low, true);
        assert_eq!(ll, "LOW");
        assert_eq!(cl, ThemePalette::STATUS_HEALTHY);
    }

    #[test]
    fn test_startup_manager_render_all_states() {
        let mut app = SystemMonitorApp::test_app();
        app.startup_items_loaded = true;
        app.startup_items_loading = false;
        app.startup_items = vec![
            StartupItem {
                name: "Test App Normal".into(),
                command: r#""C:\Program Files\Test\app.exe" --silent"#.into(),
                enabled: true,
                source: "Registry (HKCU)".into(),
                exe_path: Some(r#"C:\Program Files\Test\app.exe"#.into()),
                exe_exists: true,
                publisher: Some("Test Corp".into()),
                is_signed: Some(true),
                impact_tier: ImpactTier::High,
                recommendation: Recommendation::Keep,
                reason: "Test reason".into(),
            },
        ];

        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(&mut app, ui);
            });
        });
    }
}
```

- [ ] **Step 2: Implement `summary_card.rs` and `filter_bar.rs`**
- [ ] **Step 3: Implement `item_card.rs` and `action_handler.rs`**
- [ ] **Step 4: Implement coordinator `src/ui/pages/startup_manager/mod.rs`**
- [ ] **Step 5: Verify test suite and quality gates**

Run: `cargo test --locked`
Run: `cargo fmt --all -- --check && cargo clippy --locked --all-targets -- -D warnings`
Expected: PASS with 0 warnings.

- [ ] **Step 6: Commit startup manager refactoring**

```bash
git add src/ui/pages/startup_manager/
git rm src/ui/pages/startup_manager.rs
git commit -m "refactor(ui): modularize startup manager into sub-component package"
```

---

### Task 3: Refactor System Information Page into Modular Subcomponents

**Files:**
- Create: `src/ui/pages/system_info/mod.rs`
- Create: `src/ui/pages/system_info/os_platform.rs`
- Create: `src/ui/pages/system_info/cpu_arch.rs`
- Create: `src/ui/pages/system_info/memory_specs.rs`
- Create: `src/ui/pages/system_info/battery_diag.rs`
- Create: `src/ui/pages/system_info/gpu_display.rs`
- Remove: `src/ui/pages/system_info.rs`

**Interfaces:**
- Consumes: `crate::SystemMonitorApp`, `crate::SystemData`, `crate::ui::components::*`
- Produces:
  - `pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData)`

- [ ] **Step 1: Write headless UI tests for system info in `src/ui/pages/system_info/mod.rs`**
- [ ] **Step 2: Implement `os_platform.rs` and `cpu_arch.rs`**
- [ ] **Step 3: Implement `memory_specs.rs`, `battery_diag.rs`, and `gpu_display.rs`**
- [ ] **Step 4: Implement coordinator `src/ui/pages/system_info/mod.rs`**
- [ ] **Step 5: Verify test suite and quality gates**

Run: `cargo test --locked`
Run: `cargo fmt --all -- --check && cargo clippy --locked --all-targets -- -D warnings`
Expected: PASS with 0 warnings.

- [ ] **Step 6: Commit system info refactoring**

```bash
git add src/ui/pages/system_info/
git rm src/ui/pages/system_info.rs
git commit -m "refactor(ui): modularize system info into sub-component package"
```

---

### Task 4: Refactor Process Monitor & Settings Pages

**Files:**
- Create: `src/ui/pages/processes/mod.rs`
- Create: `src/ui/pages/processes/toolbar.rs`
- Create: `src/ui/pages/processes/table.rs`
- Remove: `src/ui/pages/processes.rs`
- Create: `src/ui/pages/settings/mod.rs`
- Create: `src/ui/pages/settings/general.rs`
- Create: `src/ui/pages/settings/ram_cleaner_config.rs`
- Create: `src/ui/pages/settings/alerts_config.rs`
- Create: `src/ui/pages/settings/telemetry_config.rs`
- Remove: `src/ui/pages/settings.rs`

**Interfaces:**
- Consumes: `crate::SystemMonitorApp`, `crate::processes::*`, `crate::persistence::settings::*`
- Produces:
  - `pub(crate) fn show(app: &mut crate::SystemMonitorApp, ui: &mut egui::Ui, data: &SystemData)`

- [ ] **Step 1: Write headless UI test for processes and settings in their test modules**
- [ ] **Step 2: Implement `src/ui/pages/processes/` components**
- [ ] **Step 3: Implement `src/ui/pages/settings/` components**
- [ ] **Step 4: Verify test suite and quality gates**

Run: `cargo test --locked`
Run: `cargo fmt --all -- --check && cargo clippy --locked --all-targets -- -D warnings`
Expected: PASS with 0 warnings.

- [ ] **Step 5: Commit processes and settings refactoring**

```bash
git add src/ui/pages/processes/ src/ui/pages/settings/
git rm src/ui/pages/processes.rs src/ui/pages/settings.rs
git commit -m "refactor(ui): modularize processes and settings pages into sub-component packages"
```

---

### Task 5: Refactor Performance & Network Pages

**Files:**
- Create: `src/ui/pages/performance/mod.rs`
- Create: `src/ui/pages/performance/plots.rs`
- Create: `src/ui/pages/performance/rate_summary.rs`
- Remove: `src/ui/pages/performance.rs`
- Create: `src/ui/pages/network/mod.rs`
- Create: `src/ui/pages/network/interfaces.rs`
- Create: `src/ui/pages/network/sockets.rs`
- Remove: `src/ui/pages/network.rs`

- [ ] **Step 1: Write UI tests for performance and network rendering**
- [ ] **Step 2: Implement `performance/` components**
- [ ] **Step 3: Implement `network/` components**
- [ ] **Step 4: Verify test suite and quality gates**

Run: `cargo test --locked`
Run: `cargo fmt --all -- --check && cargo clippy --locked --all-targets -- -D warnings`
Expected: PASS with 0 warnings.

- [ ] **Step 5: Commit performance and network refactoring**

```bash
git add src/ui/pages/performance/ src/ui/pages/network/
git rm src/ui/pages/performance.rs src/ui/pages/network.rs
git commit -m "refactor(ui): modularize performance and network pages into sub-component packages"
```

---

### Task 6: Final Integration, Installer Build, and End-to-End Verification

**Files:**
- Modify: `Cargo.lock` (if needed)
- Output: `downloads/SystemMonitor-3.7.5-setup.exe`

- [ ] **Step 1: Run comprehensive workspace verification**

Run: `cargo fmt --all -- --check`
Run: `cargo clippy --locked --all-targets -- -D warnings`
Run: `cargo test --locked`
Expected: ALL checks pass with 0 errors and 0 warnings.

- [ ] **Step 2: Build release binary and verify Inno Setup installer**

```powershell
cargo build --locked --release --bin system-monitor
& "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer.iss /DAppVersion=3.7.5
```
Expected: Successfully generates `downloads/SystemMonitor-3.7.5-setup.exe`.

- [ ] **Step 3: Final commit and tag update**

```bash
git add .
git commit -m "chore: complete modularization of big pages with zero functionality loss"
```
