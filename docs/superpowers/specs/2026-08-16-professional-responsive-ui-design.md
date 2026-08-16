# Design Specification: Professional & Responsive UI/UX Overhaul for SysMon

**Date:** 2026-08-16  
**Status:** Approved  
**Target Version:** 3.7.2+  
**Aesthetic Style:** Cockpit Dense / Terminal Noir (Strict adherence to `DESIGN.md`)

---

## 1. Overview & Objectives

SysMon is a native Windows system telemetry and observability application written in Rust using `egui` and `eframe`. While functionally robust, the current UI exhibits fixed-width elements that can feel cramped on small resolutions or underutilized on wide displays, plain text buttons in the sidebar without visual hierarchy, and basic metric cards.

This specification details a complete visual and interaction overhaul to transform SysMon into a sleek, highly responsive, professional "Cockpit Flight Deck" telemetry console.

### Key Goals:
1. **Dynamic Responsive Layouts**: Fluid metric card grid that adapts across window widths (`< 900px`, `900px–1200px`, `> 1200px`) without overflow or text clipping.
2. **Categorized Navigation Sidebar**: Clear visual grouping (`TELEMETRY`, `SYSTEM CONTROL`, `DIAGNOSTICS & HEALTH`) with accent indicators, hover feedback, and live alert badges.
3. **Cockpit Telemetry Aesthetics**: High-precision circular gauges with smooth tracks, monospace numeral readouts, status pills, and geometric 1px borders.
4. **Enhanced Data Density & Usability**: Polish all 14 pages with clean tabular alignment, inline quick actions, search/filter bars, and responsive table columns.
5. **60 FPS Performance & Resilience**: Decoupled rendering, zero UI-thread blocking, and smooth micro-interactions.

---

## 2. Palette, Typography & Visual Rules

### 2.1 Color System & 3-Way Theme Engine
The application supports **Dark (Terminal Noir)**, **Light (Clean Slate)**, and **System (Auto-Detect)**:

#### `AppTheme` Configuration:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppTheme {
    Dark,
    Light,
    System,
}
```
- **System Mode Detection**: Dynamically inspects Windows registry key `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme`. If `0` -> Dark, if `1` -> Light.

#### Dynamic Theme Palette Contract:
- **Dark Theme (Terminal Noir)**:
  - **Background Canvas**: `#09090B` (Zinc-950)
  - **Surface & Cards**: `#18181B` (Zinc-900)
  - **Surface Hover / Track**: `#27272A` (Zinc-800)
  - **Borders & Dividers**: `rgba(255, 255, 255, 0.08)`
  - **Primary Ink**: `#F4F4F5`
  - **Secondary / Muted**: `#A1A1AA`
- **Light Theme (Clean Slate)**:
  - **Background Canvas**: `#F4F4F5` (Zinc-100)
  - **Surface & Cards**: `#FFFFFF` (Pure White)
  - **Surface Hover / Track**: `#E4E4E7` (Zinc-200)
  - **Borders & Dividers**: `rgba(0, 0, 0, 0.08)`
  - **Primary Ink**: `#09090B` (Zinc-950)
  - **Secondary / Muted**: `#71717A` (Zinc-500)
- **Shared Semantic Accents**:
  - **Diagnostic Emerald**: `#10B981` (Normal / Healthy / Active)
  - **Warning Amber**: `#F59E0B` (Elevated 70–90%)
  - **Critical Red**: `#EF4444` (Critical >90% / Kill actions)
### 2.2 Typography Rules
- **Numbers & Telemetry**: Mandatory monospace numerals for percentages, frequencies, byte rates, PIDs, and memory sizes to prevent column jitter and ensure perfect tabular alignment.
- **UI Labels**: Controlled scale sans-serif with tight tracking and clear hierarchy established via weight and color contrast.
- **Banned Elements**: No emojis, no color gradients, no drop shadows, no generic 3-equal-column layouts, no pure black `#000000`.

---

## 3. Component Architecture & Responsive Mechanics

### 3.1 Breakpoint-Aware Layout System
Layouts calculate available width via `ui.available_width()` and adjust dynamic spacing:
- **Desktop Wide (`width >= 1150px`)**:
  - Overview: 5-card horizontal telemetry deck.
  - Side-by-side arrangement for hardware specs, startup status, and top processes.
- **Standard (`850px <= width < 1150px`)**:
  - Overview: 3-column / 2-row wrapped metric deck with adaptive card sizing.
  - Stacked tables with full-width scroll areas.
- **Compact (`width < 850px`)**:
  - Overview: 2-column stacked metric cards.
  - Sidebar automatically uses compact margins.
  - Tables maintain strict minimum column widths with horizontal scroll fallback to prevent text truncation.

### 3.2 Precision Circular Gauges & Progress Bars
- `paint_circular_gauge`:
  - Background track circle: `#27272A`, stroke width `4.0px`.
  - Progress arc: dynamically colored by load tier (`<70% Emerald`, `70–90% Amber`, `>90% Red`).
  - Centered monospace percentage readout with sub-label.
- Linear progress bars: 4px height, subtle rounded caps, smooth track.

---

## 4. Navigation & Layout Structure

### 4.1 Categorized Sidebar
The sidebar (`width: 190px`) is partitioned into clear semantic sections:

1. **Brand Header**:
   - Distinctive geometric diamond glyph + "SysMon" logo typography.
   - Version tag pill `v3.7.2`.
2. **`TELEMETRY`**:
   - `Overview`
   - `Performance`
   - `CPU Cores`
   - `Storage`
   - `Network`
3. **`SYSTEM CONTROL`**:
   - `Processes`
   - `Services`
   - `Startup Apps`
   - `RAM Cleaner`
4. **`DIAGNOSTICS & HEALTH`**:
   - `Diagnostics`
   - `System Info`
   - `Alerts` (with live counter badge `[ 2 ]` when alerts are active)
5. **Bottom Utility Dock**:
   - Pinned `Settings`, `Shortcuts`, `About` buttons.
   - Live telemetry heartbeat indicator (`Updated: HH:MM:SS`).

### 4.2 Active & Hover Tab States
- **Active Tab**: Deep Surface `#18181B` fill, 3px solid `#10B981` vertical left border indicator, bold Primary Ink text.
- **Hovered Tab**: `#27272A` background highlight with smooth cursor feedback.
- **Category Labels**: Tiny uppercase headers (`9.5px`, Zinc-500, bold, tracked wide).

### 4.3 Global Status Header
A 42px top bar anchored across the window:
- Left: Live hardware status pills (`CPU: 18.1%`, `RAM: 65.1%`, `GPU: 6.4%`, `NET: 1.2 MB/s`).
- Right: Quick-action triggers:
  - `[ 🧹 Clean RAM ]`: instant working-set trim with toast feedback.
  - `[ ⏺ Record ]`: one-click diagnostic session recorder.
  - `[ ⚠ Alerts ]`: status pill opening the alert drawer.

---

## 5. Page-by-Page Refinements

### 5.1 Overview Page (`src/ui/pages/overview.rs`)
- Responsive 5-card metric deck (CPU, RAM, GPU, Disk, Network).
- Monospace hardware summary strip with detected CPU, clock speed, GPU adapter, and uptime.
- Health status card with quick diagnostic shortcut.
- Top processes table preview with inline memory bars and quick inspect actions.

### 5.2 Processes Page (`src/ui/pages/processes.rs`)
- Search bar with instant clear `[×]`, process counter, and sort selector.
- High-density table with monospace PID, memory in MB, CPU %, and thread counts.
- Row hover highlights with quick action buttons (`Kill`, `Suspend`, `Details`).
- Reversible action confirmation modal showing target name, risk tier, and elevation requirement.

### 5.3 Performance Page (`src/ui/pages/performance.rs`)
- Multi-resolution graph switcher (`60s`, `5m`, `30m`, `1hr`).
- Statistical summary bar (`Min`, `Max`, `Avg`, `Peak time`) in tabular monospace format.
- High-contrast graph curves using theme accent colors.

### 5.4 Storage & Network Pages (`storage.rs`, `network.rs`)
- Storage: partition usage bars, mount paths, free/total capacity, active read/write transfer rates.
- Network: interface list, MAC/IP info, live upload/download speed sparkline bars, packet counters.

### 5.5 RAM Cleaner (`ram_cleaner.rs`)
- Dynamic dual-range memory visualizer (Used vs Target vs Excluded).
- Real-time cleanup result telemetry (`Freed: X MB | Skipped: Y | Passes: Z`).
- Direct link to persistent JSONL action audit history.

### 5.6 Diagnostics Page (`diagnostics.rs`)
- Live recording banner with elapsed timer and sample count.
- Structured anomaly finding cards with severity indicator (`Critical`, `Warning`, `Info`), evidence details, confidence score meter, and action recommendations.

---

## 6. Performance, Reliability & Verification

### 6.1 Performance Constraints
- UI render loop maintains 60 FPS without frame drops.
- Zero synchronous blocking calls in the UI thread.
- Memory allocations during rendering are minimal (reusing static strings, fixed-size format buffers).

### 6.2 Quality Gates & Verification
1. **Source Quality**:
   - `cargo fmt --all -- --check` passes with zero diffs.
   - `cargo clippy --locked --all-targets -- -D warnings` passes with zero warnings on Rust 1.85+.
   - `cargo test --locked --all-targets` passes 100% (50/50 unit & integration tests).
2. **Interactive Testing**:
   - User will run the application locally (`cargo run --release`) to manually review and verify all visual and responsive behaviors across window sizes.
   - Test resizing window from compact (800×600) to full HD (1920×1080).
   - Test navigation across all 14 modules.
   - Test dark/light theme switching and custom dialogs.
