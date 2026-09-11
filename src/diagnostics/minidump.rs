//! Bounded header inspection, not stack unwinding or root-cause attribution.
use serde::{Deserialize, Serialize};
use std::fs::{File, read_dir};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrashKind {
    Kernel32,
    Kernel64,
    Application,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinidumpCrashReport {
    pub file_name: String,
    pub timestamp: Option<String>,
    pub kind: CrashKind,
    pub code: u32,
    pub code_name: String,
    pub parameters: Vec<u64>,
    pub explanation: String,
    /// Module containing the exception address; not proof of blame.
    pub address_module: Option<String>,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DumpError {
    Unsupported(String),
    Invalid(String),
    Io(String),
}
impl std::fmt::Display for DumpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(s) => write!(f, "Unsupported: {s}"),
            Self::Invalid(s) => write!(f, "Invalid dump: {s}"),
            Self::Io(s) => write!(f, "Read error: {s}"),
        }
    }
}
impl From<std::io::Error> for DumpError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

pub fn lookup_bugcheck_info(code: u32) -> (&'static str, &'static str) {
    match code {
        0xA => (
            "IRQL_NOT_LESS_OR_EQUAL",
            "Kernel memory was accessed at an invalid address or IRQL.",
        ),
        0x1E => (
            "KMODE_EXCEPTION_NOT_HANDLED",
            "A kernel-mode exception was not handled.",
        ),
        0x3B => (
            "SYSTEM_SERVICE_EXCEPTION",
            "An exception occurred in a system service routine.",
        ),
        0x50 => ("PAGE_FAULT_IN_NONPAGED_AREA", "Invalid system memory was referenced."),
        0x7E => (
            "SYSTEM_THREAD_EXCEPTION_NOT_HANDLED",
            "A system thread exception was not handled.",
        ),
        0x9F => ("DRIVER_POWER_STATE_FAILURE", "A driver power-state transition failed."),
        0xD1 => (
            "DRIVER_IRQL_NOT_LESS_OR_EQUAL",
            "A driver accessed invalid memory at elevated IRQL.",
        ),
        0x116 => ("VIDEO_TDR_FAILURE", "Recovery from a display timeout failed."),
        0x124 => (
            "WHEA_UNCORRECTABLE_ERROR",
            "Windows reported an uncorrectable hardware error.",
        ),
        0x133 => (
            "DPC_WATCHDOG_VIOLATION",
            "The DPC watchdog detected excessive execution time.",
        ),
        0x139 => (
            "KERNEL_SECURITY_CHECK_FAILURE",
            "The kernel detected critical data corruption.",
        ),
        _ => (
            "UNKNOWN_BUGCHECK",
            "A kernel bugcheck was recorded; this code is not in the local dictionary.",
        ),
    }
}

fn invalid(reason: &str) -> DumpError {
    DumpError::Invalid(reason.into())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("validated fixed structure"))
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("validated fixed structure"))
}
fn bounds(offset: u64, size: u64, length: u64) -> Result<(), DumpError> {
    if offset.checked_add(size).is_none_or(|end| end > length) {
        return Err(invalid("location extends beyond the file"));
    }
    Ok(())
}
fn read_at(file: &mut File, length: u64, offset: u64, size: usize) -> Result<Vec<u8>, DumpError> {
    bounds(offset, size as u64, length)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = vec![0; size];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}
fn location(bytes: &[u8], offset: usize, length: u64) -> Result<(), DumpError> {
    bounds(u32_at(bytes, offset + 4) as u64, u32_at(bytes, offset) as u64, length)
}

pub fn parse_minidump_file(path: &Path) -> Result<MinidumpCrashReport, DumpError> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let signature = read_at(&mut file, length, 0, 8)?;
    let file_name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    if &signature[..4] == b"PAGE" {
        let (kind, size, machine_offset, code_offset, parameter_offset, width, machine) = match &signature[4..] {
            b"DUMP" => (CrashKind::Kernel32, 4096, 0x20, 0x28, 0x2c, 4, 0x14c),
            b"DU64" => (CrashKind::Kernel64, 8192, 0x30, 0x38, 0x40, 8, 0x8664),
            _ => return Err(DumpError::Unsupported("unrecognized PAGE kernel-header variant".into())),
        };
        let header = read_at(&mut file, length, 0, size)?;
        if u32_at(&header, machine_offset) != machine {
            return Err(DumpError::Unsupported("kernel-header machine architecture".into()));
        }
        let code = u32_at(&header, code_offset);
        if code == 0 {
            return Err(invalid("kernel header has no bugcheck"));
        }
        let parameters = (0..4)
            .map(|index| {
                let offset = parameter_offset + index * width;
                if width == 8 {
                    u64_at(&header, offset)
                } else {
                    u32_at(&header, offset) as u64
                }
            })
            .collect();
        let (name, explanation) = lookup_bugcheck_info(code);
        return Ok(MinidumpCrashReport {
            file_name, timestamp: None, kind, code, code_name: name.into(), parameters,
            explanation: explanation.into(), address_module: None,
            recommendation: "Header metadata only. Open the dump in WinDbg with matching symbols to inspect parameters and stacks; no module or hardware cause has been established.".into(),
        });
    }
    if &signature[..4] != b"MDMP" {
        return Err(DumpError::Unsupported(
            "not PAGE/DUMP, PAGE/DU64 or application MDMP".into(),
        ));
    }
    let header = read_at(&mut file, length, 0, 32)?;
    if u32_at(&header, 4) & 0xffff != 0xa793 {
        return Err(invalid("invalid MDMP version"));
    }
    let count = u32_at(&header, 8) as usize;
    if count == 0 {
        return Err(DumpError::Unsupported("MDMP has no exception streams".into()));
    }
    if count > 4096 {
        return Err(invalid("excessive stream count"));
    }
    let directory_rva = u32_at(&header, 12) as u64;
    if directory_rva < 32 {
        return Err(invalid("stream directory overlaps header"));
    }
    let directory = read_at(&mut file, length, directory_rva, count * 12)?;
    let mut exception = None;
    let mut modules = Vec::new();
    for entry in directory.as_chunks::<12>().0 {
        let kind = u32_at(entry, 0);
        let size = u32_at(entry, 4) as usize;
        let rva = u32_at(entry, 8) as u64;
        bounds(rva, size as u64, length)?;
        if size > 0 && rva < 32 {
            return Err(invalid("stream overlaps header"));
        }
        match kind {
            6 => {
                if exception.is_some() {
                    return Err(invalid("duplicate exception stream"));
                }
                if size < 168 {
                    return Err(invalid("truncated exception stream"));
                }
                let bytes = read_at(&mut file, length, rva, 168)?;
                if u32_at(&bytes, 32) > 15 {
                    return Err(invalid("exception parameter count exceeds 15"));
                }
                location(&bytes, 160, length)?;
                exception = Some((u32_at(&bytes, 8), u64_at(&bytes, 24)));
            }
            4 => {
                if size < 4 {
                    return Err(invalid("truncated module count"));
                }
                let n = u32_at(&read_at(&mut file, length, rva, 4)?, 0) as usize;
                if n > 65536
                    || n.checked_mul(108)
                        .and_then(|v| v.checked_add(4))
                        .is_none_or(|v| v > size)
                {
                    return Err(invalid("module records exceed their stream bounds"));
                }
                for index in 0..n {
                    let bytes = read_at(&mut file, length, rva + 4 + index as u64 * 108, 108)?;
                    location(&bytes, 76, length)?;
                    location(&bytes, 84, length)?;
                    let base = u64_at(&bytes, 0);
                    let end = base
                        .checked_add(u32_at(&bytes, 8) as u64)
                        .ok_or_else(|| invalid("module address overflow"))?;
                    let name_rva = u32_at(&bytes, 20) as u64;
                    let string_length = u32_at(&read_at(&mut file, length, name_rva, 4)?, 0) as usize;
                    if string_length > 32768 || !string_length.is_multiple_of(2) {
                        return Err(invalid("invalid UTF-16 module name length"));
                    }
                    let raw = read_at(&mut file, length, name_rva + 4, string_length)?;
                    let units: Vec<_> = raw
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|b| u16::from_le_bytes([b[0], b[1]]))
                        .collect();
                    let name = String::from_utf16(&units).map_err(|_| invalid("invalid UTF-16 module name"))?;
                    modules.push((base, end, name));
                }
            }
            // Other streams are not interpreted. Their advertised extent was validated above.
            _ => {}
        }
    }
    let (code, address) =
        exception.ok_or_else(|| DumpError::Unsupported("application MDMP without exception evidence".into()))?;
    let address_module = modules
        .into_iter()
        .find(|(base, end, _)| address >= *base && address < *end)
        .map(|(_, _, name)| name.rsplit(['\\', '/']).next().unwrap_or(&name).to_owned());
    let (name, explanation) = match code {
        0xc0000005 => (
            "STATUS_ACCESS_VIOLATION",
            "The application accessed memory without the required permissions.",
        ),
        0xc0000409 => (
            "STATUS_FAIL_FAST_EXCEPTION",
            "The application terminated with a fail-fast exception.",
        ),
        0xe0434352 => (
            "CLR_EXCEPTION",
            "A .NET exception was recorded; inspect managed exception details.",
        ),
        _ => (
            "UNKNOWN_APPLICATION_EXCEPTION",
            "An application exception was recorded. This is not a kernel bugcheck.",
        ),
    };
    let timestamp = match u32_at(&header, 20) {
        0 => None,
        seconds => {
            chrono::DateTime::from_timestamp(seconds as i64, 0).map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        }
    };
    Ok(MinidumpCrashReport {
        file_name, timestamp, kind: CrashKind::Application, code, code_name: name.into(), parameters: Vec::new(),
        explanation: explanation.into(), address_module,
        recommendation: "Inspect the exception and stack in a debugger. The address-containing module is location evidence, not proof of the cause.".into(),
    })
}

#[derive(Debug, Clone, Default)]
pub struct CrashScanOutcome {
    pub discovered: usize,
    pub parsed: usize,
    pub unsupported: usize,
    pub failed: usize,
    pub truncated: usize,
    pub reports: Vec<MinidumpCrashReport>,
    pub issues: Vec<String>,
}
impl CrashScanOutcome {
    fn issue(&mut self, message: String) {
        if self.issues.len() < 100 {
            self.issues.push(message);
        }
    }
    fn scan_file(&mut self, path: &Path) {
        self.discovered += 1;
        if self.discovered > 1000 {
            self.truncated += 1;
            return;
        }
        match parse_minidump_file(path) {
            Ok(report) => {
                self.parsed += 1;
                self.reports.push(report);
            }
            Err(error @ DumpError::Unsupported(_)) => {
                self.unsupported += 1;
                self.issue(format!("{}: {error}", path.display()));
            }
            Err(error) => {
                self.failed += 1;
                self.issue(format!("{}: {error}", path.display()));
            }
        }
    }
    fn scan_dir(&mut self, dir: &Path) {
        let entries = match read_dir(dir) {
            Ok(entries) => entries,
            Err(error) => {
                self.failed += 1;
                self.issue(format!("{}: {error}", dir.display()));
                return;
            }
        };
        for entry in entries {
            match entry {
                Ok(entry)
                    if entry
                        .path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("dmp")) =>
                {
                    self.scan_file(&entry.path())
                }
                Ok(_) => {}
                Err(error) => {
                    self.failed += 1;
                    self.issue(error.to_string());
                }
            }
        }
    }
}
#[cfg(test)]
pub fn scan_crash_dumps_in_dir(dir: &Path) -> CrashScanOutcome {
    let mut outcome = CrashScanOutcome::default();
    outcome.scan_dir(dir);
    outcome
}
pub fn scan_recent_crashes() -> CrashScanOutcome {
    let mut outcome = CrashScanOutcome::default();
    if let Some(windows) = std::env::var_os("SystemRoot") {
        let root = PathBuf::from(windows);
        outcome.scan_dir(&root.join("Minidump"));
        let memory = root.join("MEMORY.DMP");
        match memory.try_exists() {
            Ok(true) => outcome.scan_file(&memory),
            Ok(false) => {}
            Err(error) => {
                outcome.failed += 1;
                outcome.issue(format!("{}: {error}", memory.display()));
            }
        }
    } else {
        outcome.failed += 1;
        outcome.issue("SystemRoot unavailable".into());
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        outcome.scan_dir(&PathBuf::from(local).join("CrashDumps"));
    } else {
        outcome.failed += 1;
        outcome.issue("LOCALAPPDATA unavailable".into());
    }
    outcome.reports.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    outcome.truncated += outcome.reports.len().saturating_sub(50);
    outcome.reports.truncate(50);
    outcome
}

#[derive(Default)]
pub(crate) struct CrashScanState {
    pub(crate) outcome: Option<CrashScanOutcome>,
    pending: Option<Receiver<CrashScanOutcome>>,
}
impl CrashScanState {
    pub(crate) fn request(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        if let Err(error) = std::thread::Builder::new().name("dump-scan".into()).spawn(move || {
            let _ = tx.send(scan_recent_crashes());
        }) {
            self.pending = None;
            self.outcome = Some(CrashScanOutcome {
                failed: 1,
                issues: vec![error.to_string()],
                ..Default::default()
            });
        }
    }
    pub(crate) fn poll(&mut self) {
        let Some(rx) = &self.pending else {
            return;
        };
        match rx.try_recv() {
            Ok(outcome) => {
                self.outcome = Some(outcome);
                self.pending = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                self.outcome = Some(CrashScanOutcome {
                    failed: 1,
                    issues: vec!["Dump worker disconnected".into()],
                    ..Default::default()
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(bytes: &[u8]) -> Result<MinidumpCrashReport, DumpError> {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut file, bytes).unwrap();
        parse_minidump_file(file.path())
    }
    fn application() -> Vec<u8> {
        let mut bytes = vec![0; 212];
        bytes[..4].copy_from_slice(b"MDMP");
        bytes[4..8].copy_from_slice(&0xa793u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&32u32.to_le_bytes());
        bytes[32..36].copy_from_slice(&6u32.to_le_bytes());
        bytes[36..40].copy_from_slice(&168u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&44u32.to_le_bytes());
        bytes[52..56].copy_from_slice(&0xc0000005u32.to_le_bytes());
        bytes
    }
    #[test]
    fn genuine_kernel_header_layouts_are_distinct_from_application_exceptions() {
        for (signature, size, machine_offset, machine, code_offset, parameter_offset, kind) in [
            (b"DUMP", 4096, 0x20, 0x14cu32, 0x28, 0x2c, CrashKind::Kernel32),
            (b"DU64", 8192, 0x30, 0x8664u32, 0x38, 0x40, CrashKind::Kernel64),
        ] {
            let mut bytes = vec![0; size];
            bytes[..4].copy_from_slice(b"PAGE");
            bytes[4..8].copy_from_slice(signature);
            bytes[machine_offset..machine_offset + 4].copy_from_slice(&machine.to_le_bytes());
            bytes[code_offset..code_offset + 4].copy_from_slice(&0x116u32.to_le_bytes());
            bytes[parameter_offset..parameter_offset + 4].copy_from_slice(&123u32.to_le_bytes());
            let report = parse(&bytes).unwrap();
            assert_eq!(report.kind, kind);
            assert_eq!(report.code, 0x116);
            assert_eq!(report.parameters[0], 123);
            assert!(report.address_module.is_none());
            assert!(matches!(parse(&bytes[..64]), Err(DumpError::Invalid(_))));
        }
        let report = parse(&application()).unwrap();
        assert_eq!(report.kind, CrashKind::Application);
        assert_eq!(report.code, 0xc0000005);
    }
    #[test]
    fn corrupt_directories_and_stream_local_records_are_rejected() {
        let mut bytes = application();
        bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(parse(&bytes), Err(DumpError::Invalid(_))));
        let mut bytes = application();
        bytes[12..16].copy_from_slice(&999u32.to_le_bytes());
        assert!(matches!(parse(&bytes), Err(DumpError::Invalid(_))));
        let mut bytes = application();
        bytes[36..40].copy_from_slice(&32u32.to_le_bytes());
        assert!(matches!(parse(&bytes), Err(DumpError::Invalid(_))));
        let mut bytes = application();
        bytes[32..36].copy_from_slice(&4u32.to_le_bytes());
        bytes[36..40].copy_from_slice(&4u32.to_le_bytes());
        bytes[44..48].copy_from_slice(&1u32.to_le_bytes());
        assert!(matches!(parse(&bytes), Err(DumpError::Invalid(_))));
    }
    #[test]
    fn scans_disclose_invalid_unsupported_and_directory_failures() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("invalid.dmp"), b"MDMP").unwrap();
        std::fs::write(root.path().join("unsupported.dmp"), b"NOTADUMP").unwrap();
        std::fs::write(root.path().join("app.dmp"), application()).unwrap();
        let outcome = scan_crash_dumps_in_dir(root.path());
        assert_eq!(
            (outcome.discovered, outcome.parsed, outcome.failed, outcome.unsupported),
            (3, 1, 1, 1)
        );
        assert_eq!(scan_crash_dumps_in_dir(&root.path().join("missing")).failed, 1);
    }
}
