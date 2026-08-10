# Design System: SysMon

## 1. Visual Theme & Atmosphere
A raw, "Cockpit Dense" interface with a confident "Terminal Noir" aesthetic. The atmosphere is highly technical, dense, and mechanically precise—like an advanced diagnostic console. It relies on strict geometric alignment, zero gradient fluffs, and raw structural dividers instead of floating cards. 
**Attributes:** Density 8, Variance 5, Motion 6.

## 2. Color Palette & Roles
- **Terminal Base** (`#09090B`) — Primary background surface (Zinc-950). NEVER pure black.
- **Deep Surface** (`#18181B`) — Secondary background for embedded panels (Zinc-900).
- **Raw Outline** (`rgba(255, 255, 255, 0.1)`) — Structural 1px borders and tabular dividers.
- **Primary Ink** (`#F4F4F5`) — Primary telemetry text, headlines, and core data points.
- **Muted Steel** (`#A1A1AA`) — Secondary text, metadata, labels, and inactive states.
- **Diagnostic Emerald** (`#10B981`) — Single accent color used sparingly for active states, healthy thresholds, and primary actions. Desaturated, completely flat.
- **Critical Alert** (`#EF4444`) — Semantic color strictly reserved for high-load alerts, thermal warnings, and destructive process actions.

## 3. Typography Rules
- **Telemetry & Numbers (Mono):** `JetBrains Mono` or `Geist Mono`. Mandatory for all numerical data, timestamps, graphs, CPU cores, and metric readouts to maintain strict column alignment.
- **Labels & UI Text (Sans):** `Geist` or `Satoshi`. Track-tight, controlled scale.
- **Hierarchy:** Established through font weight and color contrast (e.g., Muted Steel vs Primary Ink), not massive text sizes. 
- **Banned Fonts:** `Inter`, any generic serifs (`Times New Roman`, `Georgia`).

## 4. Component Stylings
- **Cards/Panels:** Strict avoidance of floating, drop-shadow heavy cards. For this high-density layout, use flat surfaces with 1px `Raw Outline` borders or simply border-top dividers to separate data rows. Corner radiuses must be minimal (`4px` / `8px` max).
- **Buttons:** Flat, brutalist. Tactile push feedback (`-1px translate Y`) on active state. Accent fill for primary; 1px border outline for secondary. No outer glows or hover shadows.
- **Data Tables/Grids:** Extremely dense. Zebra-striping is BANNED. Use tight spatial spacing and subtle typography color contrast for row delineation.
- **Loaders:** Skeletal, rectilinear shimmers matching the exact layout dimensions of the missing data. No generic circular CSS spinners.
- **Inputs:** Label above, helper text below. 1px solid border, changes to Diagnostic Emerald on focus. No floating labels.

## 5. Layout Principles
- **Grid-First:** Asymmetric data layouts. Never use the generic "3 equal width columns" structure. Instead, employ offset proportions (e.g., 20% Sidebar, 50% Primary Graph, 30% Process Details).
- **Strict Spatial Zones:** No overlapping elements. Every metric has a defined, mechanical slot in the UI.
- **Density Over Spacing:** As a telemetry dashboard, internal padding is tight (e.g., `0.5rem` to `1rem`). Information density is high but remains highly readable due to strict alignment and monospace font usage.
- **Alignment:** Centered layouts are BANNED. Content must be strictly left-aligned or tabular-aligned for readability.

## 6. Motion & Interaction
- **Physics:** Spring physics for all interactive state changes (e.g., `stiffness: 100, damping: 20`).
- **Perpetual Micro-Interactions:** Active telemetry feeds should have a subtle, hardware-accelerated heartbeat or pulse on the data points themselves (not the containers).
- **Performance:** Hardware-accelerated transforms (`translate`, `scale`) and `opacity` only. Never animate layout properties (`width`, `height`, `top`).

## 7. Anti-Patterns (BANNED)
- NO emojis anywhere in the interface.
- NO pure black (`#000000`).
- NO neon / outer glow shadows (no AI cyberpunk clichés).
- NO gradients on text, buttons, or backgrounds. Solid colors only.
- NO `Inter` font.
- NO rounded-pill (fully rounded) buttons or extreme corner radiuses.
- NO overlapping floating elements or uncontained dropdowns that obscure vital telemetry data.
- NO generic AI copywriting ("Elevate your system", "Next-Gen monitoring").
- NO circular loading spinners.
- NO 3-column equal grid layouts.
