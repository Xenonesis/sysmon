SysMon 2.0 — Product, Architecture & Feature Roadmap

1. Project Vision

SysMon is a professional, high-performance Windows system monitoring, diagnostics, optimization, and gaming-performance application built in Rust.

The goal is to evolve the current project into a complete Windows Performance Center while preserving its strongest characteristics:

Native Rust performance

egui / eframe desktop UI

Hardware-accelerated rendering

Deep Windows integration

Real-time telemetry

Message-driven architecture

Low resource usage

Distinctive Terminal Noir / Cockpit Dense design language

Secure installer-only distribution

The application should feel like a combination of:

Windows Task Manager

Process Explorer

HWiNFO

NVIDIA App

Windows Performance tools

A lightweight gaming telemetry overlay

A safe and transparent PC optimization utility

The objective is not to create a bloated “one-click booster.” Every feature should be measurable, transparent, safe, and useful.

2. Final Technology Direction

Core Stack

Language: Rust 2021

Target: x86_64-pc-windows-msvc

GUI: egui + eframe

Charts: egui_plot

UI Extras: egui_extras

Images: image

Concurrency: std::thread, channels, background workers

System Telemetry: sysinfo

NVIDIA: nvml-wrapper

Windows APIs: windows, windows-sys, ntapi, winreg

WMI: wmi

Notifications: notify-rust

Tray: tray-icon, muda

Persistence: serde, serde_json

Export: csv

Logging: tracing, tracing-subscriber, tracing-appender

Installer: Inno Setup 6

Automation: PowerShell build/sign/package scripts

Framework Decision

Do not migrate to:

Tauri

Electron

Slint

Iced

Dioxus

The current egui / eframe architecture is already the correct base for this product.

The focus should be:

Improve architecture, telemetry depth, diagnostics, UX, and visual polish instead of rewriting the application in another framework.

3. Current Architecture — What Should Stay

The existing architecture is already strong and should remain conceptually intact.

Presentation Layer

The eframe::App layer should:

Render UI

Read current application state

Dispatch user actions

Never perform blocking OS work

Never directly call NVML, WMI, Win32 APIs, or heavy sysinfo refreshes

Background Telemetry

Telemetry must continue to run outside the UI thread.

The UI should consume prepared snapshots instead of querying the operating system directly.

Message Passing

The current ActionCommand / AppChannels pattern is a good design.

Keep command-based communication between:

UI

Telemetry workers

Windows operations

Export subsystem

Diagnostics

Settings

Avoid shared mutable state on the render path wherever possible.

4. Architecture Improvements

4.1 Replace One Giant SystemMonitor with TelemetryHub

Current conceptual design:

SystemMonitor
 ├── sysinfo
 ├── NVML
 └── WMI

Recommended design:

                     TelemetryHub
                          │
            ┌─────────────┼─────────────┐
            │             │             │
       Fast Pollers   Slow Pollers   Diagnostics
            │             │             │
            ▼             ▼             ▼
       CPU / RAM /     WMI / BIOS      ETW
       GPU / Disk     Hardware Info   PresentMon
       Network
            │             │             │
            └─────────────┴─────────────┘
                          │
                 Normalized Metrics
                          │
                   History Buffers
                          │
                   Latest Snapshot
                          │
                          ▼
                         UI

4.2 Provider-Based Architecture

Introduce provider traits so hardware vendors and data sources can evolve independently.

Example:

GpuProvider
├── NvidiaProvider
├── AmdProvider
└── GenericGpuProvider

Possible future providers:

TelemetryProvider
├── SysinfoProvider
├── NvmlProvider
├── AdlxProvider
├── WmiProvider
├── EtwProvider
├── PresentMonProvider
└── WindowsNativeProvider

This makes the system modular and avoids vendor-specific logic leaking into the UI.

5. Telemetry Scheduling

A 60 FPS chart does not require 60 hardware queries per second.

Render frequency and telemetry frequency must remain separate.

Recommended default polling rates:

Data

Suggested Polling

UI rendering

Up to 60 FPS while active

CPU

4–5 Hz

RAM

4–5 Hz

NVIDIA / GPU telemetry

4–5 Hz

Disk I/O

2–4 Hz

Network

2–4 Hz

Process list

1–2 Hz

Storage capacity

0.2–0.5 Hz

Services

On demand / slow refresh

WMI hardware identity

Startup + manual refresh

BIOS / Motherboard

Startup only

Background / Tray Mode

When minimized or running only in the tray:

UI rendering       Nearly idle
Telemetry          0.5–1 Hz
Alerts             Active
Logging            Active
Tray updates       Active

This keeps idle CPU usage extremely low.

6. Snapshot Delivery

Avoid allowing an unlimited backlog of telemetry snapshots.

Telemetry data becomes stale quickly.

Conceptual model:

Commands:
UI ─────────────────────────► Worker

Telemetry:
Worker ──► Latest Snapshot ──► UI

The UI should usually consume the newest snapshot rather than replay every old snapshot.

Recommended strategies:

Bounded channel

Capacity 1–2 telemetry queue

Latest-value store

Ring buffers for historical chart data

Documentation should describe the architecture as:

Message-passing architecture without shared-state locking on the render path.

Avoid claiming universal lock-free behavior unless explicitly guaranteed by the implementation.

7. History & Ring Buffers

Historical metrics should not grow forever.

Each metric should use bounded history.

Example:

MetricHistory
├── 60 seconds
├── 5 minutes
├── 30 minutes
├── 1 hour
└── Session

Possible implementation:

VecDeque<MetricPoint>

or a custom circular buffer.

Track:

Current

Minimum

Maximum

Average

Peak time

Warning events

8. Product Navigation

Recommended top-level structure:

SYSTEM
├── Dashboard
├── Performance
│   ├── CPU
│   ├── GPU
│   ├── Memory
│   ├── Disk
│   └── Network
│
├── Processes
├── Hardware
├── Gaming
├── Optimize
├── Windows
├── Alerts
├── Reports
└── Settings

9. Dashboard

The Dashboard should provide an immediate system overview.

Main Cards

CPU

GPU

RAM

Storage

Network

System health

Active alerts

Example:

CPU / PACKAGE
────────────────────────────
LOAD          42%     ▲ 4.2%
CLOCK         4.62 GHz
TEMP          67°C
POWER         71.2 W

Dashboard Sections

Current performance

Top resource consumers

Current bottleneck

System health status

Recent alerts

Pinned metrics

Quick actions

Active performance profile

Gaming status

10. CPU Monitoring

Include:

Total utilization

Per-core utilization

Logical processor count

Physical core count

Current clock

Maximum clock

Temperature where available

Package power where available

Process CPU usage

Historical usage graph

CPU saturation alerts

Context switching diagnostics where available

Throttling indicators where available

Advanced view may later include:

Effective clock

Processor groups

NUMA information

Core parking

Scheduler information

11. GPU Monitoring

NVIDIA

Keep nvml-wrapper.

Expose:

GPU utilization

VRAM usage

Temperature

Fan speed

Core clock

Memory clock

Power draw

Power limit

GPU processes

Driver information

PCIe information

Historical graphs

AMD

Add a provider using AMD ADLX when practical.

Architecture:

GpuProvider
├── NVIDIA → NVML
├── AMD    → ADLX
└── Generic

Intel

Add a generic or dedicated provider later where reliable APIs are available.

The product should not structurally assume that every machine contains an NVIDIA GPU.

12. Memory Monitoring

Display:

Used memory

Available memory

Cached memory

Commit usage

Commit limit

Pagefile usage

Memory pressure

Per-process memory

Historical usage

Working-Set Trim

Rename the current RAM Cleaner functionality.

Avoid misleading branding such as:

RAM Boost

Instant RAM Cleaner

Free 5 GB RAM

Speed Up PC

Recommended location:

Optimize
└── Advanced Memory Tools
    └── Trim Working Sets

Explain clearly:

Removes pages from process working sets where possible. Applications may reload those pages when needed.

The tool should show:

Current available memory

Current memory pressure

Expected action

Processes affected

Administrative requirements

Result after operation

13. Storage Monitoring

Include:

Disk list

Partitions

Used / free capacity

Read throughput

Write throughput

Disk activity

Disk latency where possible

Process disk usage

Historical charts

Future advanced support:

SMART health

SSD temperature

NVMe information

Drive health score

Large-file analysis

Temporary-file scanner

14. Network Monitoring

Include:

Download speed

Upload speed

Total transferred

Interface name

Link speed

IP information

Network state

Historical throughput

Advanced:

Per-process network traffic

Active TCP connections

Active UDP endpoints

Remote endpoint inspection

Latency

DNS information

Interface errors

Packet loss where practical

15. Process Manager

The process manager should become one of SysMon's flagship features.

Core

PID

Process name

CPU

RAM

GPU

Disk

Network

Status

User

Path

Command line

Actions

Kill

Kill process tree

Suspend

Resume

Set priority

Set CPU affinity

Open executable location

Copy PID

Copy path

Advanced

Process tree

Parent PID

Threads

Handles

Loaded DLLs/modules

Executable signature

Publisher

Integrity level

Architecture

Start time

Critical Windows processes must be clearly identified and protected by warnings.

16. Service Manager

Current functionality should evolve into:

Running / stopped status

Startup type

Service description

Executable path

Start

Stop

Restart

Change startup mode

Search

Filter

Critical services should display strong warnings.

Never make destructive service changes silently.

17. Startup Manager

Support common Windows startup sources.

Display:

Application

Publisher

Startup location

Source

Executable path

Enabled state

Estimated impact

Signature status

Actions:

Enable

Disable

Open location

View registry source

Inspect executable

18. ETW Diagnostics

Add Event Tracing for Windows as an advanced diagnostic engine.

This can transform SysMon from a monitor into a genuine diagnostic application.

Possible ETW modules:

ETW
├── CPU Scheduling
├── Processes
├── Threads
├── Disk I/O
├── File Activity
├── Memory
├── Page Faults
├── Network
└── System Events

Diagnostic Capture

Example:

DIAGNOSTIC CAPTURE

● RECORDING       00:37

[x] CPU Scheduling
[x] Process Activity
[x] Disk I/O
[x] Memory
[ ] Network

[ STOP & ANALYZE ]

Analysis Results

Example:

Top CPU Consumer
chrome.exe              28.4%

Highest Disk Latency
game.exe                32 ms

Context Switches
18,241 / sec

Hard Faults
126 / sec

Probable Bottleneck
CPU-bound workload

19. Gaming Performance

Add a dedicated Gaming module.

Use PresentMon or another appropriate Windows telemetry path for frame analysis.

Metrics

FPS

Average FPS

Minimum FPS

1% Low

Frame time

Frame pacing

CPU frame time

GPU frame time

GPU busy

Render latency

Presented frames

Dropped frames where available

Example:

GAME PERFORMANCE
────────────────────────────
FPS                 143
1% LOW              112
FRAME TIME          6.9 ms
GPU BUSY            5.8 ms
BOTTLENECK          GPU

20. Gaming Overlay

Optional always-on-top overlay.

User-selectable metrics:

FPS

Frame time

CPU utilization

GPU utilization

CPU temperature

GPU temperature

VRAM

RAM

Power

Network

Overlay settings:

Position

Opacity

Scale

Metric selection

Update rate

Hotkey

Hide automatically outside games

21. Performance Profiles

Add profiles:

Silent

Balanced

Performance

Gaming

Custom

Profiles may control:

Windows power plan

SysMon polling rate

Notifications

Overlay behavior

Supported GPU settings

Background monitoring level

All changes must be visible to the user.

22. Optimization Module

Optimization should be safe, measurable, and reversible.

Recommended sections:

Optimize
├── Power
├── Startup
├── Temporary Files
├── Background Apps
├── Services
└── Advanced Memory Tools

Each action should follow:

Analyze
  ↓
Show Proposed Changes
  ↓
User Approval
  ↓
Apply
  ↓
Verify
  ↓
Undo / Restore

Avoid magical claims such as:

Boost FPS instantly

Double performance

Fix all lag

Clean RAM for speed

23. Alerts

Support customizable alerts for:

CPU temperature

GPU temperature

GPU hotspot

Sustained CPU load

Sustained GPU load

Memory pressure

Low disk capacity

Disk health

GPU power

Fan failure where available

Network disconnect

Process resource spikes

Delivery:

Windows toast

In-app alert

Tray icon state

Logging

24. Reports & Export

Support:

Current snapshot

Session report

Hardware report

Performance report

Diagnostic report

Formats:

CSV

JSON

Optional later:

HTML report

PDF report

Reports should include:

Timestamp

Machine summary

Metric min / avg / max

Alerts

Selected processes

Diagnostic findings

25. Command Palette

Add a fast global command palette.

Shortcut:

Ctrl + K

Example:

> gpu

GPU Performance
GPU Processes
GPU Temperature Alert
Open Gaming Overlay
Export GPU Metrics

This dramatically improves power-user UX.

26. Global Search

Global search should locate:

Processes

Services

Startup entries

Settings

Hardware

Features

Example:

Search: chrome

Process       chrome.exe
Startup       Google Chrome
Service       Google Updater

27. Pinned Metrics

Allow users to pin metrics to their Dashboard.

Examples:

CPU temperature

GPU power

GPU temperature

VRAM

RAM usage

Network download

Disk activity

FPS

This makes the Dashboard user-specific without making configuration complicated.

28. Compare Mode

Allow users to compare system state.

Example:

                IDLE      CURRENT        Δ
CPU              4%          63%       +59
RAM            5.1GB        7.8GB      +2.7
GPU              0%          97%       +97
GPU POWER        18W          142W      +124

Useful for:

Before / after optimization

Idle vs gaming

Balanced vs Performance mode

Before / after application launch

29. Timeline Event Markers

Charts should support event markers.

Example:

─────────│─────────────│──────────────
         ↑             ↑
      Game Start    Thermal Alert

Possible events:

Game started

Power profile changed

Service stopped

Process launched

Thermal warning

GPU throttling

Optimization executed

30. Terminal Noir 2.0 Design System

Do not replace SysMon's visual identity with generic rounded SaaS UI.

Keep:

Terminal Noir

Cockpit Dense

Mechanical precision

Monochrome Zinc structure

Diagnostic Emerald

Critical Alert Red

Monospace telemetry alignment

Dense data views

Maintain Existing Anti-Patterns Ban

Avoid:

Rounded pill buttons

Large generic cards everywhere

Gradients

Decorative drop shadows

Generic loading spinners

Excessive whitespace

Cute consumer-app styling

Improve Instead

Add:

Strong typography hierarchy

Precise spacing system

High-quality table layouts

Keyboard navigation

Focus states

Hover states

Compact tooltips

Inline loaders

Skeleton placeholders where appropriate

Empty states

Clear warning states

Better graph annotations

Responsive panel sizing

31. Recommended UI Structure

Example:

┌──────────────────────────────────────────────────────────────┐
│ SYS//MON                     SEARCH      ALERTS       14:32 │
├──────────────┬───────────────────────────────────────────────┤
│ DASHBOARD    │ CPU / PACKAGE                                 │
│ PERFORMANCE  │ ───────────────────────────────────────────── │
│ PROCESSES    │ LOAD      42%         TEMP        67°C        │
│ HARDWARE     │ CLOCK     4.62 GHz    POWER       71.2 W      │
│ GAMING       │                                               │
│ OPTIMIZE     │ ▁▂▂▃▄▅▇▆▅▄▃▄▅▆▇▅▄▃▂▃▄▅                 │
│ WINDOWS      │                                               │
│ ALERTS       │ TOP PROCESSES                                 │
│ REPORTS      │ PID    PROCESS         CPU      RAM            │
│ SETTINGS     │ 8420   game.exe       38.2%    4.7 GB         │
│              │ 1976   chrome.exe      8.4%    1.8 GB         │
└──────────────┴───────────────────────────────────────────────┘

32. Charts

Charts must prioritize accuracy over decoration.

Include:

Hover tooltip

Current value

Min / avg / max

Warning threshold

Time range

Pause

Zoom where useful

Timeline markers

Grid controls

Possible ranges:

60 seconds

5 minutes

30 minutes

1 hour

Session

Do not visually smooth data in a way that implies measurements that did not occur.

33. Safety Architecture

Any destructive, elevated, or system-changing action should follow:

ActionCommand
      ↓
Permission Check
      ↓
Precondition Check
      ↓
Explain Exact Operation
      ↓
Execute
      ↓
Verify Result
      ↓
Audit Log

Examples:

Kill process

Stop service

Disable startup app

Change power plan

Trim working sets

Registry modification

GPU tuning

34. Privilege Handling

SysMon should not require administrator access for normal monitoring.

Use elevation only when required.

Example:

Monitoring               Standard User
Process Inspection       Standard User where possible
Kill protected process   Elevation
Service modification     Elevation
Registry system changes  Elevation
Power plan changes       Elevation where required

If elevation is denied:

Continue monitoring

Disable only the affected action

Explain why

Never crash the entire application

35. Reliability Requirements

SysMon must gracefully handle:

NVIDIA GPU absent

NVML initialization failure

WMI failure

Permission denied

Corrupt settings

Missing registry key

Service API failure

Process disappearing during inspection

Installer update failure

Hardware metric unavailable

Unsupported Windows build

No single telemetry provider should be able to crash the application.

36. Logging

Use structured logs.

Recommended categories:

telemetry
gpu
windows
process
service
startup
diagnostics
optimizer
ui
installer
update

Support:

Rolling logs

Maximum file size

Retention limit

Debug mode

User-exportable diagnostic bundle

37. Crash Recovery

Add:

Panic logging

Last session crash marker

Settings recovery

Safe mode

Corrupt config backup

Crash report export

Example startup behavior:

Previous SysMon session did not exit normally.

[ Start Normally ]
[ Safe Mode ]
[ Open Logs ]

38. Installer & Distribution

Keep the installer-only approach.

Pipeline:

Cargo Build
   ↓
Release Optimization
   ↓
Binary Verification
   ↓
Code Signing
   ↓
Inno Setup
   ↓
Installer Signing
   ↓
Distribution

Recommended release artifact:

SysMon-vX.Y.Z-setup.exe

Do not leave unsigned standalone executables in the public release directory.

39. Cargo Release Profile

Current high-optimization release configuration is reasonable:

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true

Measure changes rather than assuming every flag improves every workload.

40. Auto Update

Future production builds should support:

Signed update metadata

Secure download

Hash validation

Signature validation

Version comparison

Installer handoff

Rollback strategy

Updates should never blindly execute an unsigned binary.

41. Privacy

Recommended default policy:

SysMon telemetry remains on the local device unless the user explicitly enables a network-dependent feature.

Avoid collecting:

Process names

Hardware identifiers

Telemetry history

Diagnostic traces

unless the user deliberately exports or submits them.

42. Accessibility

Terminal Noir can remain dense while still supporting:

Keyboard navigation

Logical tab order

Visible focus indicators

High contrast

Scalable fonts

Color-independent warning indicators

Screen-reader metadata where practical

Do not rely only on red/green color differences.

43. Performance Targets

Suggested goals:

Idle / Tray

Near-zero UI rendering overhead

Minimal telemetry polling

Stable memory footprint

No busy loops

Active UI

Smooth 60 FPS interaction

No blocking calls on render thread

No WMI calls inside frame rendering

No NVML calls inside frame rendering

No unbounded metric history

No unbounded message backlog

Process Table

Large process lists should remain responsive through:

Cached sorting

Efficient filtering

Minimal allocations

Virtualized / efficient rows where possible

Reduced formatting work per frame

44. Testing Strategy

Unit Tests

Test:

Metric calculations

History buffers

Alert thresholds

Command validation

Settings parsing

Settings migrations

Provider normalization

Compare mode calculations

Integration Tests

Test:

Worker communication

Provider fallback

Process actions

Service actions

Registry operations

Installer scripts

Export pipeline

UI Tests

Test:

Navigation

Search

Command palette

Process tables

Warnings

Settings

Empty states

Provider failure states

45. Benchmarking

Benchmark:

CPU usage while idle

CPU usage while dashboard is open

GPU rendering overhead

Memory usage

Snapshot processing time

Process table sorting

History-buffer updates

ETW processing

PresentMon ingestion

The monitoring app itself must not become a significant workload.

46. Final Architecture

┌────────────────────────────────────────────────────────┐
│                TERMINAL NOIR UI                        │
│                egui / eframe                           │
├────────────────────────────────────────────────────────┤
│ Navigation │ Search │ Charts │ Tables │ Commands       │
├────────────────────────────────────────────────────────┤
│                    App Core                            │
│ Actions │ Alerts │ Profiles │ History │ Reports       │
├────────────────────────────────────────────────────────┤
│                 Telemetry Hub                          │
│ Scheduler │ Aggregator │ Ring Buffers │ Snapshots     │
├────────────────────────────────────────────────────────┤
│                   Providers                            │
│ sysinfo │ NVML │ ADLX │ WMI │ Win32 │ ETW            │
│                         PresentMon                     │
├────────────────────────────────────────────────────────┤
│                Windows Operations                      │
│ Processes │ Services │ Registry │ Power │ Startup     │
├────────────────────────────────────────────────────────┤
│              Persistence / Logging                     │
│ serde │ JSON │ CSV │ tracing │ migrations             │
├────────────────────────────────────────────────────────┤
│             Build / Signing / Installer                │
│ Cargo │ PowerShell │ Code Signing │ Inno Setup        │
└────────────────────────────────────────────────────────┘

47. Development Priority

P0 — Architecture & Performance

Implement first:

TelemetryHub

Provider abstraction

Polling scheduler

Latest snapshot delivery

Ring buffers

Background-mode throttling

Provider error isolation

Better logging

P1 — Terminal Noir 2.0

Implement:

Unified component system

Navigation redesign

Command palette

Global search

Excellent process tables

Better charts

Pinned metrics

Keyboard UX

Empty / error / loading states

P2 — Telemetry Expansion

Implement:

Richer CPU metrics

Richer GPU metrics

Better memory statistics

Disk metrics

Network metrics

Hardware provider normalization

AMD support

P3 — Advanced Diagnostics

Implement:

ETW engine

Process tree

Thread inspection

Modules / DLLs

Handles where practical

Bottleneck analysis

P4 — Gaming

Implement:

PresentMon

FPS

Frame time

1% lows

GPU busy

Gaming dashboard

Overlay

P5 — Safe Optimization

Implement:

Power profiles

Startup optimization

Temporary-file cleanup

Background process analysis

Safe service suggestions

Advanced working-set trim

Undo / restore

P6 — Product Hardening

Implement:

Crash recovery

Config migrations

Accessibility

Installer hardening

Auto-update

Code-signing validation

Automated testing

Benchmarks

48. Features That Should Not Become Marketing Gimmicks

Avoid turning SysMon into a fake optimizer.

Do not make claims like:

Instantly double FPS

Clean RAM = faster PC

One-click fix all lag

Maximum boost guaranteed

Registry cleaning improves gaming automatically

Instead show:

What was detected

Why it matters

What will change

Required privilege

Potential risk

Actual measured result

How to undo it

49. Product Identity

SysMon should position itself as:

A precise, local-first Windows performance and diagnostics cockpit for users who want to understand and control their system.

It should feel:

Technical

Trustworthy

Fast

Dense

Professional

Native

Transparent

Not:

Flashy

Gimmicky

Overanimated

Generic SaaS

Fake “gaming booster”

50. Final Recommendation

Do not rewrite the application.

Keep:

Rust
+
egui / eframe
+
Terminal Noir
+
Background Workers
+
Message Passing
+
Native Windows APIs

Evolve:

SystemMonitor
      ↓
TelemetryHub

Basic GPU Support
      ↓
Vendor Provider Architecture

Basic Monitoring
      ↓
Monitoring + Diagnostics + Gaming

RAM Cleaner
      ↓
Advanced Working-Set Tools

Static Dashboard
      ↓
Pinned Metrics + Search + Command Palette

Simple Charts
      ↓
Historical Analysis + Event Markers + Compare Mode

System Utility
      ↓
Professional Windows Performance Center

The strongest long-term product direction is:

SysMon = native Windows telemetry + advanced diagnostics + gaming performance + safe system controls, delivered through a unique Terminal Noir interface.

This direction preserves the excellent existing foundation while giving the project enough depth and product quality to compete with serious Windows monitoring and diagnostic tools.