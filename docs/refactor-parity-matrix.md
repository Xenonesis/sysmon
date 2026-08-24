# Refactor parity matrix

This matrix is the behavior contract for refactor-only changes. A page refactor is complete only when its existing
states, controls, side effects, and performance characteristics remain covered. Feature redesign belongs in a later,
separate change.

## Global gates

- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets -- -D warnings`
- `cargo test --locked --all-targets`
- Headless rendering for empty, nominal, error/degraded, filtered, selected, and elevated states where applicable
- Manual visual comparison at compact, standard, and wide window sizes in dark and light themes
- No direct Windows API, process launch, notification, persistence, or shared-data writes from `src/ui/pages`
- System-changing actions still pass through risk preview, confirmation, audit, result, and Undo when supported

## Services pilot

| Contract | Before refactor | Automated protection |
| --- | --- | --- |
| Empty/loading state | Loading telemetry card | Headless empty-state render |
| Summary | Total, running percentage, stopped, pending/paused, elevation status | Deterministic service-count test |
| Filtering | Name/display-name search and Running/Stopped filters | Page-state filter/sort test |
| Sorting | Display name, identifier, and state; repeated click reverses direction | Page-state and header-label tests |
| Selection | Clicking a row toggles the inspector | Page-state selection test and selected-state render |
| Inspector | Identity, copy snippets, Start/Stop/Restart actions | Selected-state render in standard and elevated modes |
| Table | Virtualized rows, stripes, hover, state pills, copy identifier, inline controls | Populated and filtered headless renders |
| Elevation | Controls disabled when standard user; elevation request available | Standard/elevated render fixtures |
| Guarded control | Service action becomes a confirmation plan before worker execution | App-shell intent-to-action-plan test |
| External console | `services.msc` launch | Typed `OpenServicesConsole` intent handled by app shell |

## Remaining page order

1. Alerts: isolate settings persistence, alert mutations, notification/sound simulation, navigation, and RAM-clean intent.
2. Processes: split the large table into columns, flat rows, tree rows, and action menu.
3. Diagnostics and Timeline: separate view models, query state, export intents, and evidence panels.
4. Network, RAM Cleaner, Storage, Performance, and Overview: finish oversized subcomponents and remove remaining direct side effects.
5. Application shell and domain hotspots: split `main.rs`, `monitoring/engine.rs`, `startup.rs`, and `timeline.rs` only after page contracts are stable.
