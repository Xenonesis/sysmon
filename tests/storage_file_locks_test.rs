use system_monitor::storage::file_locks::{
    FileLockResult, InspectionKind, LockedHandleInfo, LockingProcess, close_all_handles_for_path, close_remote_handle,
    query_process_name, unlock_locking_processes,
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
fn test_close_remote_handle_validations() {
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

    #[cfg(windows)]
    {
        // On Windows, non-existent PID returns OS error from OpenProcess
        let res = close_remote_handle(99999999, 0x20);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("Failed to open process") || err.contains("error code"));
    }
}

#[test]
fn test_close_remote_handle_live_file() {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use std::os::windows::io::IntoRawHandle;

        let temp_dir = tempfile::tempdir().expect("Create temp dir");
        let file_path = temp_dir.path().join("exclusive_locked_file.txt");

        // Open file with exclusive lock (share_mode = 0: denies all sharing)
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .share_mode(0)
            .open(&file_path)
            .expect("Open locked file");

        // Extract raw Win32 handle without running File::drop
        let raw_handle = file.into_raw_handle() as usize;
        let pid = std::process::id();

        // While handle is held exclusively, deleting the file must fail with sharing violation
        let remove_attempt = std::fs::remove_file(&file_path);
        assert!(
            remove_attempt.is_err(),
            "File should be exclusively locked and not removable while handle is held"
        );

        // Close the remote handle in current process via DuplicateHandle(DUPLICATE_CLOSE_SOURCE)
        let res = close_remote_handle(pid, raw_handle);
        assert!(res.is_ok(), "close_remote_handle failed: {:?}", res);

        // Now that handle was closed by DuplicateHandle, file can be deleted immediately
        let remove_after = std::fs::remove_file(&file_path);
        assert!(
            remove_after.is_ok(),
            "File should be unlocked and removable after remote handle closure: {:?}",
            remove_after
        );
    }
}

#[test]
fn test_close_all_handles_for_path_unlocked_file() {
    let temp_dir = tempfile::tempdir().expect("Create temp dir");
    let file_path = temp_dir.path().join("unlocked_file.txt");
    std::fs::write(&file_path, b"test content").expect("Write file");

    let path_str = file_path.to_str().expect("Path to str");
    let res = close_all_handles_for_path(path_str);
    assert!(res.is_ok(), "close_all_handles_for_path failed: {:?}", res);
    assert_eq!(res.unwrap(), 0, "No handles should be closed on an unlocked file");
}

#[test]
fn test_close_all_handles_for_path_nonexistent_file() {
    let res = close_all_handles_for_path("C:\\nonexistent_dir_12345\\nonexistent_file.txt");
    #[cfg(windows)]
    assert!(res.is_err(), "Expected error on nonexistent file");
    #[cfg(not(windows))]
    assert!(res.is_ok() || res.is_err());
}

#[test]
fn test_critical_process_rejection_in_batch_unlock() {
    let proc = LockingProcess {
        identity: None,
        pid: 500,
        name: "csrss.exe".into(),
        app_type: "Application".into(),
        is_service: false,
        handles: Vec::new(),
    };
    let res = unlock_locking_processes("C:\\test\\locked.txt", &[proc]);
    assert!(res.is_err(), "Critical process should not be unlocked or terminated");
    let err = res.unwrap_err();
    assert!(
        err.contains("critical system process"),
        "Error message should explain critical process rejection: {err}"
    );
}

#[test]
fn test_critical_process_with_path_and_pid() {
    assert!(LockedHandleInfo::is_critical_process("C:\\Windows\\System32\\csrss.exe", 500));
    assert!(LockedHandleInfo::is_critical_process("C:\\Windows\\System32\\CSRSS.EXE", 500));
    assert!(LockedHandleInfo::is_critical_process("C:\\Windows\\System32\\lsass.exe", 600));
    assert!(LockedHandleInfo::is_critical_process("C:\\Windows\\System32\\services.exe", 700));
    assert!(LockedHandleInfo::is_critical_process("C:\\Windows\\System32\\winlogon.exe", 800));
    assert!(LockedHandleInfo::is_critical_process("C:\\Windows\\System32\\smss.exe", 300));
    // Any process with PID <= 4 is critical
    assert!(LockedHandleInfo::is_critical_process("random_name.exe", 4));
    assert!(LockedHandleInfo::is_critical_process("unknown.exe", 0));
}

#[test]
fn test_service_process_actionable_guidance_in_batch_unlock() {
    let proc = LockingProcess {
        identity: None,
        pid: 3456,
        name: "spoolsv.exe".into(),
        app_type: "Windows Service".into(),
        is_service: true,
        handles: Vec::new(),
    };
    let res = unlock_locking_processes("C:\\test\\printer_spool.dat", &[proc]);
    assert!(res.is_err(), "Service process with empty handles should not be blindly terminated");
    let err = res.unwrap_err();
    assert!(
        err.contains("Windows service process cannot be terminated via batch unlock")
            && err.contains("Services manager"),
        "Error message should give actionable guidance: {err}"
    );
}

#[test]
fn test_current_process_protected_from_termination() {
    let own_pid = std::process::id();
    let proc = LockingProcess {
        identity: None,
        pid: own_pid,
        name: "system-monitor.exe".into(),
        app_type: "Application".into(),
        is_service: false,
        handles: Vec::new(),
    };
    let res = unlock_locking_processes("C:\\test\\active_file.bin", &[proc]);
    assert!(res.is_err(), "Current process should never be self-terminated");
    let err = res.unwrap_err();
    assert!(
        err.contains("cannot terminate current system monitor process"),
        "Error message should indicate self-termination protection: {err}"
    );
}

#[test]
fn test_batch_unlock_empty_processes() {
    let res = unlock_locking_processes("C:\\test\\empty.txt", &[]);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 0);
}

#[test]
fn test_query_process_name_on_current_process() {
    #[cfg(windows)]
    {
        let own_pid = std::process::id();
        let name = query_process_name(own_pid);
        assert!(name.is_some(), "query_process_name should resolve for current process");
        let name_str = name.unwrap();
        assert!(name_str.ends_with(".exe") || !name_str.is_empty());
    }
}
