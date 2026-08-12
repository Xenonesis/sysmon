# Contributing

SysMon targets Windows 10/11 x64 and Rust 1.85 or newer. Keep changes focused, preserve standard-user monitoring, and put system-changing behavior behind the action-plan confirmation and audit boundary.

Before opening a pull request, run:

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release --bin system-monitor
```

Provider changes should use normalized metric keys, return structured errors, avoid blocking the UI thread and include a deterministic test. Hardware-only tests should be ignored by default with a clear reason.

Security-sensitive changes to updates, process/service control, release workflows or persistence need failure-path tests. Never weaken signer pinning or add an unsigned-update bypass.
