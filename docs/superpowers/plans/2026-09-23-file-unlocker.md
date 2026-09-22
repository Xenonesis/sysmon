# File Unlocker & Extended Handle Inspector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform SysMon's storage lock inspector into a full-featured, professional File Unlocker & Handle Inspector with drag-and-drop / context integration, specific file-handle closure without killing the whole process, and 1-click batch unlock.

**Architecture:** Extend `src/storage/file_locks.rs` with `NtQuerySystemInformation(SystemExtendedHandleInformation)` and `DuplicateHandle(DUPLICATE_CLOSE_SOURCE)` to inspect exact open file handles and safely close specific handles without terminating the host process. Surface this in `src/ui/pages/storage/lock_inspector.rs` with batch unlock actions, audit logging, and direct Explorer integration.

**Tech Stack:** Rust 1.85+, Windows API (`windows-sys` / `ntapi` / `winapi` for `NtQuerySystemInformation`, `DuplicateHandle`), `eframe`/`egui`.

**Spec:** `docs/superpowers/specs/2026-09-23-file-unlocker-design.md`

## Global Constraints

- Never terminate a critical Windows system process (csrss, lsass, smss, winlogon, services).
- Every handle close or process termination must write an audit entry to `action-audit.jsonl`.
- Support Windows 10 (19041+) and Windows 11 natively without third-party drivers or kernel drivers.
- All handle enumeration must be bounded and non-blocking (timeout protected with thread cancellation).

---

### Task 1: Spec & Handle Inspection Data Structures

**Files:**
- Create: `docs/superpowers/specs/2026-09-23-file-unlocker-design.md`
- Modify: `src/storage/file_locks.rs:1-60`
- Test: `tests/storage_file_locks_test.rs`

**Interfaces:**
- Produces: `pub struct LockedHandleInfo { pub process_id: u32, pub handle_val: usize, pub file_path: String, pub access_mask: u32 }`
- Produces: `pub fn close_remote_handle(pid: u32, handle: usize) -> Result<(), String>`

- [ ] **Step 1: Write design spec**
Create `docs/superpowers/specs/2026-09-23-file-unlocker-design.md` detailing the handle closing API, risk disclosures, and UI workflow.

- [ ] **Step 2: Write failing unit test for `LockedHandleInfo` and handle validation**
Add test in `tests/storage_file_locks_test.rs` ensuring locked handle representation parses and validates valid and invalid handle values safely.

- [ ] **Step 3: Run test to verify it fails**
Run: `cargo test --test storage_file_locks_test`
Expected: FAIL due to missing `LockedHandleInfo` struct and methods.

- [ ] **Step 4: Implement `LockedHandleInfo` and handle close signatures in `src/storage/file_locks.rs`**
Add data models with serde support and handle attributes.

- [ ] **Step 5: Run tests and verify PASS**
Run: `cargo test --test storage_file_locks_test`
Expected: PASS.

- [ ] **Step 6: Commit**
```bash
git add docs/superpowers/specs/2026-09-23-file-unlocker-design.md src/storage/file_locks.rs tests/storage_file_locks_test.rs
git commit -m "feat(storage): define locked handle models and close signatures"
```

---

### Task 2: Implement Windows `DuplicateHandle(DUPLICATE_CLOSE_SOURCE)` Unlock Engine

**Files:**
- Modify: `src/storage/file_locks.rs:120-280`
- Modify: `src/app/commands.rs`
- Modify: `src/app/worker.rs`
- Test: `tests/storage_file_locks_test.rs`

**Interfaces:**
- Consumes: `LockedHandleInfo`
- Produces: `ActionCommand::CloseFileHandle { pid: u32, handle: usize, path: String }`
- Produces: `ActionCommand::UnlockAllProcessesForPath { path: String }`

- [ ] **Step 1: Write failing integration test for handle closing on dummy file**
Create a test that opens a temporary file in Rust, discovers its lock, calls `close_remote_handle`, and verifies the file can immediately be renamed/deleted.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test storage_file_locks_test -- test_close_remote_handle`
Expected: FAIL with unimplemented handle closing logic.

- [ ] **Step 3: Implement remote handle duplication & closure**
Using `OpenProcess(PROCESS_DUP_HANDLE)` and `DuplicateHandle(..., DUPLICATE_CLOSE_SOURCE)`, close the remote process handle safely. Write result to audit log.

- [ ] **Step 4: Wire `ActionCommand::CloseFileHandle` into `src/app/worker.rs`**
Ensure execution passes through privilege checks and local action auditing.

- [ ] **Step 5: Run test to verify PASS**
Run: `cargo test --test storage_file_locks_test -- test_close_remote_handle`
Expected: PASS.

- [ ] **Step 6: Commit**
```bash
git add src/storage/file_locks.rs src/app/commands.rs src/app/worker.rs tests/storage_file_locks_test.rs
git commit -m "feat(storage): implement remote handle closure and action commands"
```

---

### Task 3: UI Enhancement: Drag-and-Drop, Selective Handle Closing & 1-Click Unlock All

**Files:**
- Modify: `src/ui/pages/storage/lock_inspector.rs`
- Modify: `src/main.rs` (file drop support in egui)
- Test: `tests/storage_file_locks_test.rs`

**Interfaces:**
- Consumes: `ActionCommand::CloseFileHandle`, `ActionCommand::UnlockAllProcessesForPath`
- Produces: Enhanced UI card with `[ 🔓 Close Handle (Keep App Alive) ]`, `[ ⚡ Unlock All ]`, and `[ 🛑 Terminate Process ]`.

- [ ] **Step 1: Write failing UI model test**
Verify lock inspector view model handles both individual handle close and batch unlock dispatch.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test storage_file_locks_test -- test_lock_inspector_model`
Expected: FAIL.

- [ ] **Step 3: Update `src/ui/pages/storage/lock_inspector.rs`**
Add:
1. `[ 🔓 Unlock (Close Handle) ]` button next to each locking process (does not crash or kill the app).
2. Header action `[ ⚡ Unlock File (Close All Locks) ]`.
3. Support for drag-and-dropping files directly onto the inspection area from Windows Explorer.
4. Clear danger warnings and audit trail link.

- [ ] **Step 4: Verify full test suite and clippy**
Run: `cargo test --locked --all-targets` and `cargo clippy --locked --all-targets -D warnings`
Expected: All tests pass, 0 warnings.

- [ ] **Step 5: Commit**
```bash
git add src/ui/pages/storage/lock_inspector.rs src/main.rs tests/storage_file_locks_test.rs
git commit -m "feat(ui): add 1-click handle unlock and drag-and-drop inspection"
```
