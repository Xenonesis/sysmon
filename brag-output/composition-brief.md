# Hyperframes Composition Brief: SysMon

## Objective
Create a high-energy, technically authoritative, and visually stunning launch brag video for SysMon.

## Output
- Composition directory: `brag-output/composition/`
- Rendered video: `brag-output/brag.mp4`
- Format: landscape — 1920x1080
- Duration: 19.5 seconds

## Source Material
- Project root: `.`
- Primary files read: `README.md`, `src/ui/theme.rs`, `src/ui/hud.rs`, `Cargo.toml`
- Product name: SysMon
- Tagline: "Native Windows Observability. Pure Rust. Zero Bloat."
- Key UI moments to recreate:
  - Metric Telemetry HUD Cards (CPU, RAM, NVMe Disk, GPU)
  - 1-Click Working-Set RAM Cleaner with live memory drop counter
  - Forensic Process Tree hierarchy with PID identity safety
- Copy that must appear verbatim:
  - "Still waiting for Task Manager to open?"
  - "Native Windows Observability. Pure Rust. Zero Bloat."
  - "No background services. No cloud accounts. Zero opaque scripts."
  - "1-Click Surgical RAM Cleaner"
  - "🌲 Hierarchical Process Forensics"

## Creative Direction
- Tone preset: `polished` with high-octane engineering energy
- Creative direction: Sleek terminal cockpit meets high-end developer engineering
- Interpretation: Obsidian carbon aesthetic, vivid diagnostic emerald accents, crisp monospace numerals, razor-sharp transitions.
- Hook: A sluggish Windows default state dramatically shattered by native 60 FPS Rust telemetry.
- Outro / punchline: "SysMon v3.8.1 — Pure Rust. Zero Bloat. 100% Local."
- Avoid:
  - Generic SaaS buzzwords
  - Slow cheesy fades or corporate stock footage
  - Blurry or low-contrast text

## Visual Identity
- Background: `#09090B` (Terminal Deep Base)
- Card Surface: `#18181B` (Zinc Dark Surface)
- Card Border: `rgba(255, 255, 255, 0.08)`
- Primary Accent: `#10B981` (Diagnostic Emerald)
- Secondary Accent: `#F59E0B` (Telemetry Amber)
- Status Critical: `#EF4444` (Critical Red)
- Primary Text: `#F4F4F5` (Zinc 100)
- Muted Text: `#A1A1AA` (Zinc 400)
- Fonts: `ui-monospace, "SF Mono", Menlo, Monaco, "Cascadia Code", "JetBrains Mono", Consolas, monospace` for data; system sans-serif for headlines.

## Storyboard
Total duration: 19.5s

1. Scene 1 — Hook: The Sluggish Default (0.0s - 3.5s)
   - Visual: Red warning card `Task Manager: NOT RESPONDING` shattered by green laser cut into `ENTER SYSMON`.
2. Scene 2 — Telemetry Cockpit (3.5s - 8.0s)
   - Visual: 4 high-density metric cards (CPU 4.8 GHz, RAM 64 GB, NVMe 850 MB/s, RTX 4090 42°C).
3. Scene 3 — 1-Click Surgical RAM Cleaner (8.0s - 12.5s)
   - Visual: Memory gauge dropping from 89% down to 34% with flash badge `+2.41 GB Recovered`.
4. Scene 4 — Process Forensics (12.5s - 16.5s)
   - Visual: Nested Process Tree with CPU affinity, disk throughput, and PID verification badge.
5. Scene 5 — Authority Outro (16.5s - 19.5s)
   - Visual: SysMon emblem, version 3.8.1, GitHub open source badges, clean finish.

## Audio
- Audio role: Driving electronic rhythm with precise UI clicks and impact accents.
- Music: `assets/music/music.mp3`
- SFX:
  - `assets/sfx/impact.ogg` at 0.1s and 16.6s
  - `assets/sfx/switch.ogg` at 3.5s and 12.5s
  - `assets/sfx/click.ogg` at 9.2s (RAM clean click)
  - `assets/sfx/clean.ogg` at 9.6s (RAM memory drop whoosh)
- Music treatment: Starts at 0.0s, loop/play through 19.5s with 1.5s fade out at end.
- Beat sync: Locked to major cues (1.60s, 5.80s, 10.53s, 16.6s).
