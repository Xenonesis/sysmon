# Brag Plan: SysMon

## What is this app?
SysMon is a high-performance native Windows system monitor and diagnostics cockpit written in pure Rust (eframe/egui), delivering 1–5 Hz decoupled telemetry without cloud telemetry, background services, or opaque scripts.

## The angle
The Windows Task Manager is notoriously sluggish and hogs memory, while heavy tools like HWiNFO or Armory Crate are bloated. SysMon brings surgical speed, zero-bloat Rust engineering, instant RAM cleaner, and live forensic process hierarchy right to your desktop.

## Hook (first 2-3 seconds)
"Task Manager feels slow? 100% disk usage mystery?" A stark, high-contrast terminal prompt flashing with brutal typography, immediately shattered by pure Rust 60FPS diagnostic telemetry.

## Key moments (the middle)
- **Real-time Diagnostic Cockpit**: Ultra-dense HUD with live CPU (4.8 GHz), RAM (64 GB DDR5), NVMe SSD, and RTX 4090 GPU metrics with glowing emerald sparks.
- **Process Forensics & 🌲 Process Tree**: Real-time tree hierarchy with CPU affinity and native Creation-Time process identity safety.
- **1-Click Surgical RAM Cleaner**: Instantly trimming 2.4 GB of working-set bloat with an auditable local ledger and zero-delay feedback.

## Outro / punchline
"No background services. No cloud accounts. Zero opaque scripts. Just pure Rust on Windows." Lands on the authoritative SysMon logo and GitHub badge.

## User flow worth showing
1. **Instant HUD Launch**: Pressing `Ctrl + M` summons the sleek diagnostic HUD.
2. **RAM Surge Detection & 1-Click Trim**: RAM meter hits 88% → user clicks `⚡ Quick Clean` → working sets drop to 32% with green success flash.
3. **Forensic Process Tree**: Unmasking rogue CPU cycles in the nested tree hierarchy.

## Tone
- Preset: `polished` (with a high-tech developer edge)
- Creative direction: Sleek terminal cockpit meets high-end developer engineering
- Interpretation: Confident, fast-paced, razor-sharp typography, Emerald (#10B981) diagnostics on obsidian carbon (#09090B).

## Format: landscape — 1920x1080
## Duration: 19.5s

## Visual identity (from the project)
- Background: `#09090B` (Terminal Deep Base)
- Surface/Card: `#18181B` (Zinc Dark Surface)
- Accent: `#10B981` (Diagnostic Emerald)
- Secondary Accent: `#F59E0B` (Telemetry Amber)
- Text Primary: `#F4F4F5` (Zinc 100)
- Text Muted: `#A1A1AA` (Zinc 400)
- Display font: `JetBrains Mono, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif`
- Strongest visual element: High-density metric telemetry cards, circular progress rings, and emerald diagnostic badges.

## Share copy (draft)
Windows Task Manager taking 5 seconds to open is over. SysMon gives you pro-grade Windows diagnostics written in pure Rust with 1-click RAM cleaner, process tree forensics, and zero background services.

## Audio direction
- Role: Upbeat modern electronic rhythm with punchy beat accents.
- Music: `happy-beats-business-moves-vol-11-by-ende-dot-app.mp3`
- Music treatment: Starts at 0.0s, steady drive, gentle fade-out during final logo (17.5s - 19.5s).
- Music cue guidance:
  - Strong cues: 1.60s (Hook payoff), 5.80s (Cockpit reveal), 10.53s (RAM clean trigger), 15.26s (Tree inspection), 17.37s (Final logo landing).
  - Beat grid: 114.84 BPM (~0.522s per beat).
- SFX posture: Crisp motion-matched UI clicks, impact drops on major reveals, clean swooshes on stat changes.
- Restraint rule: Audio should emphasize precision and speed, not chaotic noise.

## Storyboard

### Scene 1 — The Hook: Breaking the Sluggish Default — 3.5s (0.0s - 3.5s)
- **Visual**: Dark screen with glowing terminal cursor. Staggered text: "Still waiting for Task Manager to open?" followed by a red status tag `CPU: 100% DISK: 100% UNRESPONSIVE`. Then cut sharply with an emerald laser slice into the SysMon prompt.
- **Audio intent**: Dramatic build with a clean impact hit on beat 1.60s.
- **Transition**: Hard geometric wipe to Scene 2.

### Scene 2 — Telemetry Cockpit Reveal — 4.5s (3.5s - 8.0s)
- **Visual**: Full-screen 4-column HUD card grid popping in on the beat (5.80s).
  - Card 1: CPU `4.82 GHz` (16 Cores | 8P + 8E) — 38%
  - Card 2: Memory `64 GB DDR5` — 74%
  - Card 3: Storage `2 TB NVMe SSD` — 850 MB/s Read
  - Card 4: GPU `RTX 4090` — 42°C | 28% Load
- **Audio intent**: Pulsing rhythm, card snap ticks.
- **Transition**: Slide left with emerald trail.

### Scene 3 — 1-Click Surgical RAM Cleaner — 4.5s (8.0s - 12.5s)
- **Visual**: Focus on the RAM Cleaner module. Memory bar at 89% (57.1 GB / 64 GB). Cursor simulates a click on `[ ⚡ TRIM WORKING SETS ]`.
  - Flash animation: Working sets flush!
  - Counter rapidly drops from `89%` down to `34%` (`21.8 GB`).
  - Badge appears: `+2.41 GB Recovered | 0 Crashes | Audited`.
- **Audio intent**: Click SFX at 10.53s, followed by whoosh/reclaimer sound.
- **Transition**: Smooth vertical push.

### Scene 4 — Forensic Process Tree & Timeline — 4.0s (12.5s - 16.5s)
- **Visual**: The 🌲 `Process Tree` unfolds with hierarchical indentation, live disk I/O, and CPU core affinity masks.
  - Tag: `ProcessIdentity Safe (PID + Native Creation Timestamp)`.
  - Zero false-PID mutations. Instant kill/suspend preview.
- **Audio intent**: Fast mechanical tick accents.
- **Transition**: Zoom into emerald core logo.

### Scene 5 — Outro & Authority Punchline — 3.0s (16.5s - 19.5s)
- **Visual**: Glowing `SYSMON` logo in bold JetBrains Mono.
  - Tagline: "Native Windows Observability. Pure Rust. Zero Bloat."
  - Badges: `v3.8.1` | `100% Local` | `MIT Licensed` | `github.com/Xenonesis/sysmon`
- **Audio intent**: Music swell into clean fade-out on 19.0s.

**Music mood for this video:** Confident, precision electronic beats.
**Audio summary:** Fast, energetic technical showcase driving from the frustration of sluggish default tools to the surgical elegance of pure Rust engineering.
