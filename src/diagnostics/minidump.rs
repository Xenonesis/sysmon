//! Native Windows Minidump (.dmp) parser and crash explanation dictionary.

use serde::{Deserialize, Serialize};
use std::fs::{File, read_dir};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Detailed crash report parsed from a Windows minidump file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinidumpCrashReport {
    pub file_name: String,
    pub timestamp: String,
    pub bugcheck_code: u32,
    pub bugcheck_name: String,
    pub explanation: String,
    pub faulting_module: Option<String>,
    pub recommendation: String,
}

/// Look up human-readable bugcheck name, explanation, and recommendation for a bugcheck code.
pub fn lookup_bugcheck_info(code: u32, module: Option<&str>) -> (&'static str, &'static str, &'static str) {
    match code {
        0x0000000A => (
            "IRQL_NOT_LESS_OR_EQUAL",
            "A kernel-mode process attempted to access memory at an invalid address or illegal IRQL.",
            "Usually caused by faulty device drivers or corrupted RAM. Check recently installed drivers.",
        ),
        0x0000001E => (
            "KMODE_EXCEPTION_NOT_HANDLED",
            "A kernel-mode program generated an exception that the error handler did not catch.",
            "Often caused by faulty hardware, incompatible drivers, or system service corruption.",
        ),
        0x0000003B => (
            "SYSTEM_SERVICE_EXCEPTION",
            "An exception occurred while executing a system service routine.",
            "Commonly triggered by graphics driver bugs, anti-cheat drivers, or system file corruption.",
        ),
        0x00000050 => (
            "PAGE_FAULT_IN_NONPAGED_AREA",
            "Invalid system memory was referenced by the kernel.",
            "Check for defective physical RAM modules or an overheating memory controller.",
        ),
        0x0000007E => (
            "SYSTEM_THREAD_EXCEPTION_NOT_HANDLED",
            "A system thread generated an exception which the error handler did not handle.",
            "Look at the faulting driver. Update or roll back the driver specified in the report.",
        ),
        0x0000009F => (
            "DRIVER_POWER_STATE_FAILURE",
            "A driver is in an inconsistent or invalid power state during sleep or wake.",
            "Check SSD firmware updates and chipset/ACPI power management drivers.",
        ),
        0x000000D1 => (
            "DRIVER_IRQL_NOT_LESS_OR_EQUAL",
            "A kernel driver accessed pageable memory at an elevated interrupt level.",
            "Predominantly caused by network adapter (Wi-Fi/Ethernet) or display driver bugs.",
        ),
        0x00000116 => (
            "VIDEO_TDR_ERROR",
            "The display adapter driver failed to respond within the allocated timeout period and Windows reset it.",
            "Perform a clean re-installation of graphics drivers using Display Driver Uninstaller (DDU) or check GPU temperatures.",
        ),
        0x00000124 => (
            "WHEA_UNCORRECTABLE_ERROR",
            "Windows Hardware Error Architecture caught a fatal hardware fault.",
            "Usually caused by unstable CPU/RAM overclocking, insufficient VCore voltage, or failing SSDs.",
        ),
        0x00000133 => (
            "DPC_WATCHDOG_VIOLATION",
            "A Deferred Procedure Call (DPC) ran longer than the watchdog threshold allowed.",
            "Check for high DPC latency in storage controller or network interface drivers.",
        ),
        0x00000139 => (
            "KERNEL_SECURITY_CHECK_FAILURE",
            "The kernel detected corruption of a critical data structure.",
            "Run 'sfc /scannow' and check system files or verify recently updated third-party drivers.",
        ),
        0xC0000005 => (
            "STATUS_ACCESS_VIOLATION",
            "The instruction at the fault address referenced memory without proper access permissions.",
            "Check for application memory corruption, null pointer dereference, or faulty memory modules.",
        ),
        0xC0000409 => (
            "STATUS_STACK_BUFFER_OVERRUN",
            "The system detected an overrun of a stack-based buffer in the application (fail-fast exception).",
            "Update the application or check for software security patches addressing buffer overflow vulnerabilities.",
        ),
        0xE0434352 => (
            "CLR_EXCEPTION",
            "An unhandled Microsoft .NET Common Language Runtime (CLR) exception was thrown.",
            "Check the Windows Application Event Log or application logs for .NET exception stack traces.",
        ),
        _ => {
            if let Some(m) = module {
                let lower = m.to_ascii_lowercase();
                if lower.contains("nvld") || lower.contains("amdk") || lower.contains("atik") || lower.contains("igdk")
                {
                    (
                        "GPU_KERNEL_CRASH",
                        "A crash occurred inside the graphics card kernel-mode driver.",
                        "Reinstall the graphics driver or reset GPU core/memory clock offsets to factory defaults.",
                    )
                } else {
                    (
                        "KERNEL_BUGCHECK",
                        "Windows encountered an unrecoverable kernel stop error.",
                        "Inspect recent Windows updates, driver installations, and system memory integrity.",
                    )
                }
            } else {
                (
                    "KERNEL_BUGCHECK",
                    "Windows encountered an unrecoverable kernel stop error.",
                    "Inspect recent Windows updates, driver installations, and system memory integrity.",
                )
            }
        }
    }
}

/// Parse a Windows Minidump file into a structured crash report.
///
/// Implements bounded binary parsing to guarantee zero panics on truncated or corrupted files.
pub fn parse_minidump_file(path: &Path) -> Result<MinidumpCrashReport, String> {
    let mut f = File::open(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    let file_len = f.metadata().map(|m| m.len()).unwrap_or(0);
    if file_len < 32 {
        return Err(format!(
            "Minidump file is truncated ({} bytes, expected at least 32)",
            file_len
        ));
    }

    // MINIDUMP_HEADER is 32 bytes:
    // Signature: u32 (0x504d444d / 'MDMP')
    // Version: u32
    // NumberOfStreams: u32
    // StreamDirectoryRva: u32
    // CheckSum: u32
    // TimeDateStamp: u32
    // Flags: u64
    let mut header = [0u8; 32];
    f.read_exact(&mut header)
        .map_err(|e| format!("Failed to read header: {e}"))?;

    let sig = u32::from_le_bytes(header[0..4].try_into().unwrap());
    if sig != 0x504d444d {
        return Err(format!(
            "Not a valid Windows Minidump file (signature 0x{sig:08X} does not match 0x504D444D)"
        ));
    }

    let num_streams = u32::from_le_bytes(header[8..12].try_into().unwrap());
    let stream_rva = u32::from_le_bytes(header[12..16].try_into().unwrap());
    let timestamp_u32 = u32::from_le_bytes(header[20..24].try_into().unwrap());

    let date_str = chrono::DateTime::from_timestamp(timestamp_u32 as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Unknown Date".into());

    let mut bugcheck_code = 0u32;
    let mut faulting_module: Option<String> = None;
    let mut exception_address: Option<u64> = None;

    // Safety check: bound stream count to prevent OOM allocations on corrupted files
    const MAX_STREAMS: u32 = 1024;
    if num_streams > 0 && num_streams <= MAX_STREAMS {
        let dir_bytes = (num_streams as u64) * 12;
        if (stream_rva as u64).saturating_add(dir_bytes) <= file_len {
            if f.seek(SeekFrom::Start(stream_rva as u64)).is_ok() {
                let mut dir_entries = vec![0u8; dir_bytes as usize];
                if f.read_exact(&mut dir_entries).is_ok() {
                    // First pass: locate ExceptionStream (StreamType = 6)
                    for i in 0..num_streams as usize {
                        let offset = i * 12;
                        let stream_type = u32::from_le_bytes(dir_entries[offset..offset + 4].try_into().unwrap());
                        let data_size = u32::from_le_bytes(dir_entries[offset + 4..offset + 8].try_into().unwrap());
                        let rva = u32::from_le_bytes(dir_entries[offset + 8..offset + 12].try_into().unwrap());

                        if stream_type == 6 && data_size >= 12 {
                            let read_len = (data_size as usize).min(1024);
                            if (rva as u64).saturating_add(read_len as u64) <= file_len {
                                if f.seek(SeekFrom::Start(rva as u64)).is_ok() {
                                    let mut exc_buf = vec![0u8; read_len];
                                    if f.read_exact(&mut exc_buf).is_ok() {
                                        // ExceptionCode is at offset 8 in MINIDUMP_EXCEPTION_STREAM
                                        bugcheck_code = u32::from_le_bytes(exc_buf[8..12].try_into().unwrap());
                                        // ExceptionAddress is at offset 24 (u64)
                                        if exc_buf.len() >= 32 {
                                            exception_address =
                                                Some(u64::from_le_bytes(exc_buf[24..32].try_into().unwrap()));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Second pass: if we have an exception address, search ModuleListStream (StreamType = 4)
                    if let Some(exc_addr) = exception_address {
                        for i in 0..num_streams as usize {
                            let offset = i * 12;
                            let stream_type = u32::from_le_bytes(dir_entries[offset..offset + 4].try_into().unwrap());
                            let data_size = u32::from_le_bytes(dir_entries[offset + 4..offset + 8].try_into().unwrap());
                            let rva = u32::from_le_bytes(dir_entries[offset + 8..offset + 12].try_into().unwrap());

                            if stream_type == 4 && data_size >= 4 {
                                if (rva as u64).saturating_add(4) <= file_len {
                                    if f.seek(SeekFrom::Start(rva as u64)).is_ok() {
                                        let mut count_buf = [0u8; 4];
                                        if f.read_exact(&mut count_buf).is_ok() {
                                            let num_modules = u32::from_le_bytes(count_buf).min(512);
                                            for m in 0..num_modules as usize {
                                                let mod_offset = (rva as u64) + 4 + (m as u64) * 108;
                                                if mod_offset.saturating_add(108) > file_len {
                                                    break;
                                                }
                                                if f.seek(SeekFrom::Start(mod_offset)).is_ok() {
                                                    let mut mod_buf = [0u8; 108];
                                                    if f.read_exact(&mut mod_buf).is_ok() {
                                                        let base =
                                                            u64::from_le_bytes(mod_buf[0..8].try_into().unwrap());
                                                        let size =
                                                            u32::from_le_bytes(mod_buf[8..12].try_into().unwrap());
                                                        let name_rva =
                                                            u32::from_le_bytes(mod_buf[20..24].try_into().unwrap());

                                                        if exc_addr >= base
                                                            && exc_addr < base.saturating_add(size as u64)
                                                        {
                                                            // Read MINIDUMP_STRING at name_rva
                                                            if (name_rva as u64).saturating_add(4) <= file_len {
                                                                if f.seek(SeekFrom::Start(name_rva as u64)).is_ok() {
                                                                    let mut str_hdr = [0u8; 4];
                                                                    if f.read_exact(&mut str_hdr).is_ok() {
                                                                        let str_len =
                                                                            u32::from_le_bytes(str_hdr).min(512);
                                                                        if (name_rva as u64) + 4 + (str_len as u64)
                                                                            <= file_len
                                                                        {
                                                                            let mut str_bytes =
                                                                                vec![0u8; str_len as usize];
                                                                            if f.read_exact(&mut str_bytes).is_ok() {
                                                                                let u16_chars: Vec<u16> = str_bytes
                                                                                    .chunks_exact(2)
                                                                                    .map(|c| {
                                                                                        u16::from_le_bytes([c[0], c[1]])
                                                                                    })
                                                                                    .collect();
                                                                                let full_str = String::from_utf16_lossy(
                                                                                    &u16_chars,
                                                                                );
                                                                                let file_part = Path::new(&full_str)
                                                                                    .file_name()
                                                                                    .and_then(|n| n.to_str())
                                                                                    .unwrap_or(&full_str);
                                                                                faulting_module =
                                                                                    Some(file_part.to_string());
                                                                                break;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("crash.dmp")
        .to_string();
    let (name, expl, rec) = lookup_bugcheck_info(bugcheck_code, faulting_module.as_deref());

    Ok(MinidumpCrashReport {
        file_name,
        timestamp: date_str,
        bugcheck_code,
        bugcheck_name: name.to_string(),
        explanation: expl.to_string(),
        faulting_module,
        recommendation: rec.to_string(),
    })
}

/// Scan a specific directory for `.dmp` files and parse each one.
pub fn scan_crash_dumps_in_dir(dir: &Path) -> Vec<MinidumpCrashReport> {
    let mut reports = Vec::new();
    if let Ok(entries) = read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("dmp")) {
                if let Ok(rep) = parse_minidump_file(&p) {
                    reports.push(rep);
                }
            }
        }
    }
    reports
}

/// Scan standard Windows crash minidump locations (`%SystemRoot%\Minidump` and `%LOCALAPPDATA%\CrashDumps`).
/// Returns reports sorted by timestamp descending (newest first).
pub fn scan_recent_crashes() -> Vec<MinidumpCrashReport> {
    let mut reports = Vec::new();

    // 1. Kernel BSOD minidump directory
    let win_dir = std::env::var("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Windows"));
    let minidump_dir = win_dir.join("Minidump");
    reports.extend(scan_crash_dumps_in_dir(&minidump_dir));

    // 2. Application user-mode crash dumps
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let user_dumps = PathBuf::from(local).join("CrashDumps");
        reports.extend(scan_crash_dumps_in_dir(&user_dumps));
    }

    // Sort newest crashes first
    reports.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    reports.truncate(50);
    reports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_bugcheck_info_known_codes() {
        let (name, expl, rec) = lookup_bugcheck_info(0x00000116, Some("nvlddmkm.sys"));
        assert_eq!(name, "VIDEO_TDR_ERROR");
        assert!(expl.contains("display adapter"));
        assert!(rec.contains("GPU") || rec.contains("graphics"));

        let (name2, _, _) = lookup_bugcheck_info(0x00000124, None);
        assert_eq!(name2, "WHEA_UNCORRECTABLE_ERROR");

        let (name3, _, _) = lookup_bugcheck_info(0x0000000A, None);
        assert_eq!(name3, "IRQL_NOT_LESS_OR_EQUAL");

        let (name4, _, _) = lookup_bugcheck_info(0x0000003B, None);
        assert_eq!(name4, "SYSTEM_SERVICE_EXCEPTION");

        let (name5, _, _) = lookup_bugcheck_info(0x000000D1, None);
        assert_eq!(name5, "DRIVER_IRQL_NOT_LESS_OR_EQUAL");

        let (name6, _, _) = lookup_bugcheck_info(0xDEADBEEF, Some("amdkmdag.sys"));
        assert_eq!(name6, "GPU_KERNEL_CRASH");

        let (name7, _, _) = lookup_bugcheck_info(0xDEADBEEF, None);
        assert_eq!(name7, "KERNEL_BUGCHECK");

        let (name8, _, _) = lookup_bugcheck_info(0xC0000005, None);
        assert_eq!(name8, "STATUS_ACCESS_VIOLATION");
    }

    fn create_test_synthetic_minidump() -> Vec<u8> {
        let mut buf = vec![0u8; 240];

        // Header: 32 bytes
        buf[0..4].copy_from_slice(&0x504d444du32.to_le_bytes()); // 'MDMP'
        buf[4..8].copy_from_slice(&0x0000a793u32.to_le_bytes()); // Version
        buf[8..12].copy_from_slice(&2u32.to_le_bytes()); // NumberOfStreams = 2
        buf[12..16].copy_from_slice(&32u32.to_le_bytes()); // StreamDirectoryRva = 32
        buf[16..20].copy_from_slice(&0u32.to_le_bytes()); // CheckSum
        buf[20..24].copy_from_slice(&1700000000u32.to_le_bytes()); // TimeDateStamp: 2023-11-14 22:13:20 UTC
        buf[24..32].copy_from_slice(&0u64.to_le_bytes()); // Flags

        // Stream Directory: 2 entries * 12 bytes = 24 bytes (offset 32..56)
        // Entry 0: ExceptionStream (StreamType = 6)
        buf[32..36].copy_from_slice(&6u32.to_le_bytes()); // StreamType = 6
        buf[36..40].copy_from_slice(&32u32.to_le_bytes()); // DataSize = 32
        buf[40..44].copy_from_slice(&56u32.to_le_bytes()); // Rva = 56

        // Entry 1: ModuleListStream (StreamType = 4)
        buf[44..48].copy_from_slice(&4u32.to_le_bytes()); // StreamType = 4
        buf[48..52].copy_from_slice(&112u32.to_le_bytes()); // DataSize = 112
        buf[52..56].copy_from_slice(&88u32.to_le_bytes()); // Rva = 88

        // ExceptionStream: 32 bytes (offset 56..88)
        buf[56..60].copy_from_slice(&1234u32.to_le_bytes()); // ThreadId
        buf[60..64].copy_from_slice(&0u32.to_le_bytes()); // Alignment
        buf[64..68].copy_from_slice(&0x00000116u32.to_le_bytes()); // ExceptionCode: VIDEO_TDR_ERROR
        buf[68..72].copy_from_slice(&0u32.to_le_bytes()); // ExceptionFlags
        buf[72..80].copy_from_slice(&0u64.to_le_bytes()); // ExceptionRecord
        buf[80..88].copy_from_slice(&0x7fff00001000u64.to_le_bytes()); // ExceptionAddress

        // ModuleListStream: 112 bytes (offset 88..200)
        buf[88..92].copy_from_slice(&1u32.to_le_bytes()); // NumberOfModules = 1
        // Module 0: 108 bytes (offset 92..200)
        buf[92..100].copy_from_slice(&0x7fff00000000u64.to_le_bytes()); // BaseOfImage
        buf[100..104].copy_from_slice(&0x10000u32.to_le_bytes()); // SizeOfImage = 64KB
        buf[104..108].copy_from_slice(&0u32.to_le_bytes()); // CheckSum
        buf[108..112].copy_from_slice(&0u32.to_le_bytes()); // TimeDateStamp
        buf[112..116].copy_from_slice(&200u32.to_le_bytes()); // ModuleNameRva = 200

        // MINIDUMP_STRING: offset 200..232
        let name = "nvlddmkm.sys";
        let u16_name: Vec<u16> = name.encode_utf16().collect();
        let name_bytes_len = (u16_name.len() * 2) as u32;
        buf[200..204].copy_from_slice(&name_bytes_len.to_le_bytes());
        for (i, ch) in u16_name.iter().enumerate() {
            buf[204 + i * 2..204 + (i + 1) * 2].copy_from_slice(&ch.to_le_bytes());
        }

        buf
    }

    #[test]
    fn test_parse_synthetic_minidump() {
        let bytes = create_test_synthetic_minidump();
        let temp_dir = std::env::temp_dir().join(format!("test_minidump_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let dump_path = temp_dir.join("synthetic_crash.dmp");
        std::fs::write(&dump_path, &bytes).expect("Failed to write synthetic dump file");

        let report = parse_minidump_file(&dump_path).expect("Failed to parse valid synthetic minidump");
        assert_eq!(report.file_name, "synthetic_crash.dmp");
        assert_eq!(report.bugcheck_code, 0x00000116);
        assert_eq!(report.bugcheck_name, "VIDEO_TDR_ERROR");
        assert!(report.explanation.contains("display adapter"));
        assert_eq!(report.faulting_module, Some("nvlddmkm.sys".to_string()));
        assert!(report.timestamp.contains("2023-11-14"));

        let _ = std::fs::remove_file(&dump_path);
        let _ = std::fs::remove_dir(&temp_dir);
    }

    #[test]
    fn test_corrupted_minidump_handling() {
        let temp_dir = std::env::temp_dir().join(format!("test_minidump_corrupted_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        // 1. Completely empty file
        let empty_path = temp_dir.join("empty.dmp");
        std::fs::write(&empty_path, b"").unwrap();
        let res = parse_minidump_file(&empty_path);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("truncated"));

        // 2. Truncated header (< 32 bytes)
        let trunc_path = temp_dir.join("trunc.dmp");
        std::fs::write(&trunc_path, b"MDMP_TRUNCATED").unwrap();
        let res = parse_minidump_file(&trunc_path);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("truncated"));

        // 3. Wrong signature
        let bad_sig_path = temp_dir.join("bad_sig.dmp");
        let mut bad_header = [0u8; 32];
        bad_header[0..4].copy_from_slice(b"BAD!");
        std::fs::write(&bad_sig_path, &bad_header).unwrap();
        let res = parse_minidump_file(&bad_sig_path);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("signature"));

        // 4. Excessive stream count (potential allocation attack)
        let oom_path = temp_dir.join("excessive_streams.dmp");
        let mut oom_header = [0u8; 32];
        oom_header[0..4].copy_from_slice(&0x504d444du32.to_le_bytes());
        oom_header[8..12].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // 4 billion streams
        oom_header[12..16].copy_from_slice(&32u32.to_le_bytes());
        std::fs::write(&oom_path, &oom_header).unwrap();
        let res = parse_minidump_file(&oom_path);
        assert!(res.is_ok()); // Successfully produces a safe fallback report without panicking or OOM

        // 5. Out of bounds stream directory RVA
        let oob_path = temp_dir.join("oob_rva.dmp");
        let mut oob_header = [0u8; 32];
        oob_header[0..4].copy_from_slice(&0x504d444du32.to_le_bytes());
        oob_header[8..12].copy_from_slice(&2u32.to_le_bytes());
        oob_header[12..16].copy_from_slice(&0xFFFF0000u32.to_le_bytes()); // StreamDirectoryRva far past EOF
        std::fs::write(&oob_path, &oob_header).unwrap();
        let res = parse_minidump_file(&oob_path);
        assert!(res.is_ok()); // Safely skips invalid stream directory without panicking

        // 6. Non-existent file
        let missing_path = temp_dir.join("does_not_exist.dmp");
        let res = parse_minidump_file(&missing_path);
        assert!(res.is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_scan_crash_dumps_in_dir_filters_and_parses() {
        let temp_dir = std::env::temp_dir().join(format!("test_minidump_scan_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        // Valid dmp
        let valid_path = temp_dir.join("valid.dmp");
        std::fs::write(&valid_path, create_test_synthetic_minidump()).unwrap();

        // Non-dmp file
        let txt_path = temp_dir.join("notes.txt");
        std::fs::write(&txt_path, b"some text file").unwrap();

        // Corrupted dmp file
        let corrupt_path = temp_dir.join("broken.dmp");
        std::fs::write(&corrupt_path, b"not a dmp").unwrap();

        let reports = scan_crash_dumps_in_dir(&temp_dir);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].bugcheck_code, 0x00000116);
        assert_eq!(reports[0].bugcheck_name, "VIDEO_TDR_ERROR");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_scan_recent_crashes_does_not_panic() {
        // Must execute cleanly on any Windows system
        let crashes = scan_recent_crashes();
        // Just verify it doesn't panic and returns a valid slice
        for crash in &crashes {
            assert!(!crash.file_name.is_empty());
            assert!(!crash.bugcheck_name.is_empty());
        }
    }

    #[test]
    fn test_parse_real_crash_dump_if_present() {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let crash_dir = PathBuf::from(local).join("CrashDumps");
            if let Ok(entries) = read_dir(&crash_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("dmp")) {
                        // Should either succeed or return a clean Err; must never panic
                        if let Ok(rep) = parse_minidump_file(&path) {
                            assert!(!rep.file_name.is_empty());
                            assert!(!rep.timestamp.is_empty());
                            assert!(!rep.bugcheck_name.is_empty());
                        }
                        break; // Testing one real dump is sufficient
                    }
                }
            }
        }
    }
}
