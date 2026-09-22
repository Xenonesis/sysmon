use system_monitor::storage::file_locks::{
    FileLockResult, InspectionKind, LockedHandleInfo, LockingProcess, close_remote_handle,
};

#[test]
fn test_locked_handle_info_construction_and_fields() {
    let handle = LockedHandleInfo::new(1234, 0x24, "C:\\test\\locked_document.docx", 0x120089);
    assert_eq!(handle.process_id, 1234);
    assert_eq!(handle.handle_val, 0x24);
    assert_eq!(handle.file_path, "C:\\test\\locked_document.docx");
    assert_eq!(handle.access_mask, 0x120089);
}

#[test]
fn test_locked_handle_info_validity_invariants() {
    // Valid handle and PID
    let valid = LockedHandleInfo::new(1234, 0x24, "C:\\test\\file.txt", 0x100000);
    assert!(valid.is_valid());
    assert!(LockedHandleInfo::is_valid_pid(1234));
    assert!(LockedHandleInfo::is_valid_handle_value(0x24));

    // PID 0 (System Idle Process) is invalid
    let zero_pid = LockedHandleInfo::new(0, 0x24, "C:\\test\\file.txt", 0x100000);
    assert!(!zero_pid.is_valid());
    assert!(!LockedHandleInfo::is_valid_pid(0));

    // Handle 0 (NULL) is invalid
    let zero_handle = LockedHandleInfo::new(1234, 0, "C:\\test\\file.txt", 0x100000);
    assert!(!zero_handle.is_valid());
    assert!(!LockedHandleInfo::is_valid_handle_value(0));

    // Handle usize::MAX (INVALID_HANDLE_VALUE on 64-bit) is invalid
    let max_handle = LockedHandleInfo::new(1234, usize::MAX, "C:\\test\\file.txt", 0x100000);
    assert!(!max_handle.is_valid());
    assert!(!LockedHandleInfo::is_valid_handle_value(usize::MAX));

    // Handle 0xFFFF_FFFF (INVALID_HANDLE_VALUE on 32-bit / truncated) is invalid
    let u32_max_handle = LockedHandleInfo::new(1234, 0xFFFF_FFFF, "C:\\test\\file.txt", 0x100000);
    assert!(!u32_max_handle.is_valid());
    assert!(!LockedHandleInfo::is_valid_handle_value(0xFFFF_FFFF));
}

#[test]
fn test_critical_system_process_rejection() {
    // Critical system processes by PID
    assert!(LockedHandleInfo::is_critical_process("idle", 0));
    assert!(LockedHandleInfo::is_critical_process("system", 4));

    // Critical system processes by image name
    assert!(LockedHandleInfo::is_critical_process("csrss.exe", 500));
    assert!(LockedHandleInfo::is_critical_process("CSRSS.EXE", 500));
    assert!(LockedHandleInfo::is_critical_process("smss.exe", 300));
    assert!(LockedHandleInfo::is_critical_process("lsass.exe", 600));
    assert!(LockedHandleInfo::is_critical_process("services.exe", 700));
    assert!(LockedHandleInfo::is_critical_process("winlogon.exe", 800));

    // Standard non-critical processes
    assert!(!LockedHandleInfo::is_critical_process("notepad.exe", 1234));
    assert!(!LockedHandleInfo::is_critical_process("chrome.exe", 5678));
}

#[test]
fn test_locked_handle_info_serde_json_roundtrip() {
    let original = LockedHandleInfo::new(4321, 0x100, "D:\\data\\database.db", 0x80000000);
    let serialized = serde_json::to_string(&original).expect("Serialization failed");

    assert!(serialized.contains("\"process_id\":4321"));
    assert!(serialized.contains("\"handle_val\":256"));
    assert!(serialized.contains("\"file_path\":\"D:\\\\data\\\\database.db\""));
    assert!(serialized.contains("\"access_mask\":2147483648"));

    let deserialized: LockedHandleInfo = serde_json::from_str(&serialized).expect("Deserialization failed");
    assert_eq!(original, deserialized);
}

#[test]
fn test_locking_process_with_handles_serde_backwards_compatibility() {
    // 1. New model with handles
    let handle = LockedHandleInfo::new(1001, 0x40, "C:\\locked.txt", 0x100);
    let proc = LockingProcess {
        identity: None,
        pid: 1001,
        name: "worker.exe".into(),
        app_type: "Application".into(),
        is_service: false,
        handles: vec![handle.clone()],
    };

    let serialized = serde_json::to_string(&proc).expect("Serialize LockingProcess");
    assert!(serialized.contains("\"handles\":["));
    let deserialized: LockingProcess = serde_json::from_str(&serialized).expect("Deserialize LockingProcess");
    assert_eq!(proc, deserialized);
    assert_eq!(deserialized.handles.len(), 1);
    assert_eq!(deserialized.handles[0], handle);

    // 2. Legacy JSON without "handles" field deserializes cleanly with empty handles vector
    let legacy_json = r#"{
        "identity": null,
        "pid": 2002,
        "name": "legacy_app.exe",
        "app_type": "Service",
        "is_service": true
    }"#;
    let legacy_proc: LockingProcess = serde_json::from_str(legacy_json).expect("Deserialize legacy LockingProcess");
    assert_eq!(legacy_proc.pid, 2002);
    assert_eq!(legacy_proc.name, "legacy_app.exe");
    assert!(legacy_proc.handles.is_empty());
}

#[test]
fn test_file_lock_result_with_handles_serde() {
    let handle = LockedHandleInfo::new(1234, 0x10, "C:\\test\\file.txt", 0x100);
    let result = FileLockResult {
        path: "C:\\test\\file.txt".into(),
        kind: InspectionKind::File,
        processes: vec![LockingProcess {
            identity: None,
            pid: 1234,
            name: "test.exe".into(),
            app_type: "Application".into(),
            is_service: false,
            handles: vec![handle.clone()],
        }],
        error: None,
        files_scanned: 1,
        entries_skipped: 0,
        partial: false,
        cancelled: false,
        coverage: vec!["coverage info".into()],
        handles: vec![handle],
    };

    let json = serde_json::to_string(&result).expect("Serialize FileLockResult");
    assert!(json.contains("\"handles\":["));
    let deserialized: FileLockResult = serde_json::from_str(&json).expect("Deserialize FileLockResult");
    assert_eq!(result, deserialized);
    assert_eq!(deserialized.handles.len(), 1);
    assert_eq!(deserialized.kind, InspectionKind::File);
}

#[test]
fn test_close_remote_handle_stub_validations() {
    // Rejects PID 0
    let err_pid = close_remote_handle(0, 0x10);
    assert!(err_pid.is_err());
    assert!(err_pid.unwrap_err().contains("PID 0"));

    // Rejects invalid handle 0 (NULL)
    let err_null = close_remote_handle(1234, 0);
    assert!(err_null.is_err());
    assert!(err_null.unwrap_err().contains("Invalid handle value"));

    // Rejects invalid handle usize::MAX
    let err_max = close_remote_handle(1234, usize::MAX);
    assert!(err_max.is_err());
    assert!(err_max.unwrap_err().contains("Invalid handle value"));

    // Rejects critical process
    let err_crit = close_remote_handle(4, 0x10);
    assert!(err_crit.is_err());
    assert!(err_crit.unwrap_err().contains("critical"));

    // Stub returns not-implemented error for valid parameters
    let res = close_remote_handle(1234, 0x20);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Not implemented"));
}
