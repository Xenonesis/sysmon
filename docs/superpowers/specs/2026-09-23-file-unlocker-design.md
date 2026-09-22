# File Unlocker & Extended Handle Inspector Design Specification

**Date:** 2026-09-23  
**Status:** Approved / In Implementation  
**Target Version:** 3.9.0  

---

## 1. Executive Summary & Problem Statement

In Windows, attempting to delete, move, or rename a file or eject a removable drive frequently fails with vague OS messages:
* *"The action can't be completed because the file is open in another program"*
* *"This device is currently in use. Close any programs or windows that might be using the device, and then try again."*

Standard Windows tools (such as Task Manager or Resource Monitor) and native APIs exhibit critical limitations:

### 1.1 Why Windows Restart Manager Alone Is Insufficient
SysMon previously integrated the Windows Restart Manager API (`rstrtmgr.dll`) via `RmRegisterResources`, `RmStartSession`, and `RmGetList`. While Restart Manager can discover which processes hold active resources on a path:
1. **Coarse-Grained Reporting:** It only returns the host Process IDs (`dwProcessId`) and application names. It does not provide the specific handle values (`HANDLE`), handle access masks, or exact handle paths.
2. **All-or-Nothing Process Termination:** Restart Manager's built-in resolution mechanism (`RmShutdown`) initiates full service shutdowns or process terminations.
3. **Severe Data Loss:** If a large application (such as an IDE like VS Code, an office suite like Microsoft Word, a database server, or Windows Explorer itself) holds a lock on a single cached file, terminating the entire process risks losing unsaved documents, disrupting background tasks, and degrading system responsiveness.

### 1.2 The Solution: Targeted Handle Closure via `DuplicateHandle`
The SysMon File Unlocker introduces targeted remote handle closure:
* Windows provides `DuplicateHandle` in `kernel32.dll` with the flag `DUPLICATE_CLOSE_SOURCE` (`0x00000001`).
* By duplicating the remote process's file handle with `DUPLICATE_CLOSE_SOURCE` and requesting no target handle (or closing the duplicate immediately), the kernel invalidates and closes the remote handle **without terminating or crashing the host process**.
* This releases the file lock instantly, allowing immediate rename, delete, or USB ejection while keeping the host application running.

---

## 2. Technical Architecture & Windows Internals

### 2.1 Handle Discovery Engine
To identify which exact handles correspond to a locked file or directory:
1. **Extended Handle Query:** Query `NtQuerySystemInformation` with `SystemExtendedHandleInformation` (class 64 / `0x40`) to retrieve all system-wide open handles (`SYSTEM_HANDLE_TABLE_ENTRY_INFO_EX`).
2. **Target Filtering:** Match against the target process ID and filter out handle types that are not File objects.
3. **Handle Duplication for Identification:** Duplicate candidate handles with `PROCESS_DUP_HANDLE` into SysMon's address space.
4. **Name Resolution:** Call `NtQueryObject(ObjectNameInformation)` or `GetFinalPathNameByHandleW` to retrieve the normalized NT path (`\Device\HarddiskVolumeX\...`) or Win32 path (`C:\...`).
5. **Path Matching:** Match the resolved path against the requested file or folder path.

### 2.2 Remote Handle Closure Mechanism
The core unlocking operation executes as follows:
```text
[SysMon Worker]
      │
      ├─ 1. OpenProcess(PROCESS_DUP_HANDLE, false, target_pid)
      │      └─► Obtains process handle `h_process`
      │
      ├─ 2. Guard Check:
      │      └─► Verify target is not PID 0, PID 4, or critical system process
      │
      ├─ 3. DuplicateHandle(
      │        h_process,          // Source process
      │        remote_handle,      // Target handle in source process
      │        GetCurrentProcess(),// Target process (SysMon)
      │        &mut h_dup,         // Duplicate handle receiver
      │        0,                  // Desired access (ignored with SAME_ACCESS)
      │        FALSE,              // Inherit handle
      │        DUPLICATE_CLOSE_SOURCE | DUPLICATE_SAME_ACCESS
      │     )
      │      └─► In Windows Kernel: Source handle in target_pid is atomically CLOSED!
      │
      ├─ 4. CloseHandle(h_dup)
      │      └─► SysMon discards the temporary duplicate handle
      │
      └─ 5. Audit Logging:
             └─► Record action in `action-audit.jsonl`
```

---

## 3. Data Models & API Interfaces

### 3.1 Data Structures in `src/storage/file_locks.rs`

#### `LockedHandleInfo`
Represents an individual locked handle held by a process:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedHandleInfo {
    /// Process ID owning the handle.
    pub process_id: u32,
    /// Numeric handle value (cast from Win32 HANDLE / usize).
    pub handle_val: usize,
    /// Resolved canonical file path associated with this handle.
    pub file_path: String,
    /// Win32 access mask (e.g. FILE_READ_DATA, FILE_WRITE_DATA, GENERIC_READ).
    pub access_mask: u32,
}
```

#### Associated Methods on `LockedHandleInfo`:
* `new(process_id: u32, handle_val: usize, file_path: impl Into<String>, access_mask: u32) -> Self`
* `is_valid(&self) -> bool`: Verifies `process_id` is valid and `handle_val` is not null (0) or `INVALID_HANDLE_VALUE` (`usize::MAX` or `0xFFFF_FFFF`).
* `is_valid_pid(pid: u32) -> bool`: Checks that PID is non-zero.
* `is_valid_handle_value(handle: usize) -> bool`: Checks handle syntax.
* `is_critical_process(name: &str, pid: u32) -> bool`: Rejects mutation on critical Windows components.

#### Updates to `LockingProcess` and `FileLockResult`
`LockingProcess` is extended with:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockingProcess {
    pub identity: Option<ProcessIdentity>,
    pub pid: u32,
    pub name: String,
    pub app_type: String,
    pub is_service: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handles: Vec<LockedHandleInfo>,
}
```

`FileLockResult` is extended with:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileLockResult {
    pub path: String,
    pub kind: InspectionKind,
    pub processes: Vec<LockingProcess>,
    pub error: Option<String>,
    pub files_scanned: usize,
    pub entries_skipped: usize,
    pub partial: bool,
    pub cancelled: bool,
    pub coverage: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handles: Vec<LockedHandleInfo>,
}
```

### 3.2 Function Signatures

#### `close_remote_handle`
```rust
/// Closes a specific remote file handle inside the target process using DuplicateHandle
/// with DUPLICATE_CLOSE_SOURCE, releasing the file lock without killing the host process.
pub fn close_remote_handle(pid: u32, handle: usize) -> Result<(), String>;
```

#### `close_all_handles_for_path`
```rust
/// Closes all discovered handles holding a lock on the given path across all locking processes.
pub fn close_all_handles_for_path(path: &str, handles: &[LockedHandleInfo]) -> Vec<Result<(), String>>;
```

---

## 4. Safety Constraints, Invariants & Security Guardrails

### 4.1 Critical Process Protection
Under no circumstance will SysMon close handles or terminate processes belonging to:
1. **PID <= 4:** System Idle Process (PID 0) and NT Kernel & System (PID 4).
2. **SysMon Self:** SysMon's own process ID (`std::process::id()`).
3. **Core Operating System Services:**
   * `csrss.exe` (Client/Server Runtime Subsystem)
   * `smss.exe` (Session Manager Subsystem)
   * `lsass.exe` (Local Security Authority Subsystem Service)
   * `services.exe` (Service Control Manager)
   * `winlogon.exe` (Windows Logon Application)
4. **`IsProcessCritical`:** Any process flagged as critical to operating system survival.

Attempting to close a handle on these processes immediately yields an error: `Refusing mutation of self or critical process`.

### 4.2 Application Stability & Risk Disclosures
Closing a handle out from under an application is a forceful operation. While it does not terminate the process:
* If the target application later attempts to read, write, or close the invalidated handle, it will receive `ERROR_INVALID_HANDLE` (`0x6`).
* If the application holds unsaved dirty buffers tied to that handle, those writes may fail.
* SysMon's UI must clearly disclose this risk with an informational tooltip and confirmation when closing handles.

### 4.3 Action Audit Logging
Every attempt to unlock a file (whether individual handle closure, batch unlock, or process termination) is persistently recorded to `%LOCALAPPDATA%\sysmon\action-audit.jsonl` with:
* Timestamp (RFC 3339)
* Target PID and process name
* Target file path and handle value
* Action type (`CloseFileHandle`, `UnlockAllHandles`, `TerminateProcess`)
* Outcome (`Success`, `PermissionDenied`, `Failed`)
* Error description if applicable

---

## 5. UI & User Experience Workflow

### 5.1 Drag-and-Drop File Inspection
* Users can drag any file or directory from Windows Explorer directly onto the SysMon window.
* SysMon captures the drop event in `eframe`/`egui`, sets the inspection path, and immediately triggers an asynchronous lock scan.

### 5.2 Granular Action Buttons
For each locking process:
1. **`[ 🔓 Close Handle (Keep App Alive) ]`:** Closes the exact handle locking the file. The process remains running.
2. **`[ ⚡ Unlock All ]` (Batch):** Located in the header; iterates over all handles across all processes and closes them in one click.
3. **`[ 🛑 Terminate Process ]` (Fallback):** For stubborn background processes or services that immediately recreate the lock, offers guarded process termination with audit tracking.

---

## 6. Implementation Milestones

* **Task 1 (This Task):** Define data models (`LockedHandleInfo`), add `close_remote_handle` signature and validation logic, define design spec, and add unit test suite in `tests/storage_file_locks_test.rs`.
* **Task 2:** Implement the Windows `DuplicateHandle(DUPLICATE_CLOSE_SOURCE)` engine, integrate `ActionCommand::CloseFileHandle`, and wire through async worker with audit logging.
* **Task 3:** Update the Storage UI page (`src/ui/pages/storage/lock_inspector.rs`) with drag-and-drop support, selective handle buttons, and batch unlock actions.
