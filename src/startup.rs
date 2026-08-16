//! Startup Manager: diagnostics, enrichment, scoring, and actions.

use serde::{Deserialize, Serialize};

// ─── Data Models ─────────────────────────────────────────────

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ImpactTier {
    Low,
    Medium,
    High,
    Unknown,
}

impl ImpactTier {
    pub fn label(&self) -> &str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MED",
            Self::High => "HIGH",
            Self::Unknown => "?",
        }
    }
    pub fn sort_key(&self) -> u8 {
        match self {
            Self::High => 0,
            Self::Medium => 1,
            Self::Unknown => 2,
            Self::Low => 3,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Recommendation {
    Keep,
    Review,
    Disable,
    Cleanup,
}

impl Recommendation {
    pub fn label(&self) -> &str {
        match self {
            Self::Keep => "Keep",
            Self::Review => "Review",
            Self::Disable => "Disable",
            Self::Cleanup => "Cleanup",
        }
    }
}

#[derive(Clone)]
pub struct StartupItem {
    pub name: String,
    pub command: String,
    #[allow(dead_code)]
    pub enabled: bool,
    pub source: String,
    pub exe_path: Option<String>,
    pub exe_exists: bool,
    pub publisher: Option<String>,
    pub is_signed: Option<bool>,
    pub impact_tier: ImpactTier,
    pub recommendation: Recommendation,
    pub reason: String,
}

#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct BootDiagnostics {
    pub boot_duration_ms: Option<u64>,
    pub main_path_boot_ms: Option<u64>,
    pub post_boot_ms: Option<u64>,
    pub collected_at: String,
    pub degrading_items: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StartupOptimizationEntry {
    pub timestamp: String,
    pub action: String,
    pub item_name: String,
    pub item_source: String,
    pub impact_tier_before: String,
    pub high_impact_count_before: usize,
    pub high_impact_count_after: usize,
}

#[derive(PartialEq, Clone, Copy)]
pub enum StartupSortColumn {
    Name,
    Impact,
    Source,
    Publisher,
}

// ─── Path Parsing ────────────────────────────────────────────

pub fn expand_env_vars(path: &str) -> String {
    let mut result = String::new();
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let mut var_name = String::new();
            let mut found_end = false;
            while let Some(&nc) = chars.peek() {
                if nc == '%' {
                    chars.next();
                    found_end = true;
                    break;
                }
                if let Some(ch) = chars.next() {
                    var_name.push(ch);
                }
            }
            if found_end {
                if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                } else {
                    result.push('%');
                    result.push_str(&var_name);
                    result.push('%');
                }
            } else {
                result.push('%');
                result.push_str(&var_name);
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn parse_exe_from_command(cmd: &str) -> Option<String> {
    let t = cmd.trim();
    if t.is_empty() {
        return None;
    }

    // 1. Quoted string: "C:\path\app.exe" ...
    if let Some(stripped) = t.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            let p = &stripped[..end];
            if !p.is_empty() {
                return Some(p.to_string());
            }
        }
    }

    // 2. rundll32 handling (case-insensitive, char-boundary safe)
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("rundll32.exe") {
        let after = t.get(12..).unwrap_or("").trim().trim_start_matches('"');
        if let Some(comma) = after.find(',') {
            let dll = after[..comma].trim().trim_end_matches('"');
            if !dll.is_empty() {
                return Some(dll.to_string());
            }
        }
    } else if lower.starts_with("rundll32") {
        let after = t.get(8..).unwrap_or("").trim().trim_start_matches('"');
        if let Some(comma) = after.find(',') {
            let dll = after[..comma].trim().trim_end_matches('"');
            if !dll.is_empty() {
                return Some(dll.to_string());
            }
        }
    }

    // 3. Search for known executable extensions safely
    for ext in &[".exe", ".cmd", ".bat", ".vbs", ".ps1"] {
        let mut start = 0;
        while let Some(pos) = lower[start..].find(ext) {
            let abs_pos = start + pos + ext.len();
            if abs_pos <= t.len() && t.is_char_boundary(abs_pos) {
                let is_end = abs_pos == t.len()
                    || t[abs_pos..].starts_with(' ')
                    || t[abs_pos..].starts_with('"')
                    || t[abs_pos..].starts_with('/');
                if is_end {
                    return Some(t[..abs_pos].trim_matches('"').to_string());
                }
            }
            start += pos + 1;
            if start >= t.len() {
                break;
            }
        }
    }

    // 4. Default: first whitespace-separated token
    t.split_whitespace().next().map(|s| s.trim_matches('"').to_string())
}

// ─── Collection ──────────────────────────────────────────────

fn new_item(name: String, command: String, enabled: bool, source: String) -> StartupItem {
    StartupItem {
        name,
        command,
        enabled,
        source,
        exe_path: None,
        exe_exists: false,
        publisher: None,
        is_signed: None,
        impact_tier: ImpactTier::Unknown,
        recommendation: Recommendation::Review,
        reason: String::new(),
    }
}

#[cfg(target_os = "windows")]
fn ps_run(script: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("powershell")
        .creation_flags(0x08000000)
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(script)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}

#[cfg(target_os = "windows")]
fn is_approved_disabled(bytes: &[u8]) -> bool {
    // Windows StartupApproved binary structure:
    // First byte:
    // 0x02, 0x06 = Enabled
    // 0x01, 0x03, 0x07 = Disabled by Task Manager or Windows Settings
    if let Some(&first) = bytes.first() {
        first % 2 != 0 || first == 0x03 || first == 0x01
    } else {
        false
    }
}

#[cfg(target_os = "windows")]
fn get_approved_map(root: winreg::HKEY, subpath: &str) -> std::collections::HashMap<String, bool> {
    let mut map = std::collections::HashMap::new();
    let key = winreg::RegKey::predef(root);
    if let Ok(approved_key) = key.open_subkey_with_flags(subpath, winreg::enums::KEY_READ) {
        for (name, val) in approved_key.enum_values().flatten() {
            let disabled = is_approved_disabled(&val.bytes);
            map.insert(name.to_lowercase(), !disabled);
        }
    }
    map
}

#[cfg(target_os = "windows")]
fn string_from_reg_value(val: &winreg::RegValue) -> String {
    match val.vtype {
        winreg::enums::REG_SZ | winreg::enums::REG_EXPAND_SZ => {
            let words: Vec<u16> = val
                .bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&w| w != 0)
                .collect();
            String::from_utf16_lossy(&words)
        }
        _ => val.to_string(),
    }
}

#[cfg(target_os = "windows")]
fn collect_registry_native(
    items: &mut Vec<StartupItem>,
    root: winreg::HKEY,
    run_subpath: &str,
    approved_subpath: &str,
    source: &str,
) {
    let approved_map = get_approved_map(root, approved_subpath);
    let key = winreg::RegKey::predef(root);
    if let Ok(run_key) = key.open_subkey_with_flags(run_subpath, winreg::enums::KEY_READ) {
        for (name, val) in run_key.enum_values().flatten() {
            let cmd = string_from_reg_value(&val);
            if !name.is_empty() && !cmd.is_empty() {
                let enabled = approved_map.get(&name.to_lowercase()).copied().unwrap_or(true);
                items.push(new_item(name, cmd, enabled, source.to_string()));
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn collect_registry_disabled_stash(items: &mut Vec<StartupItem>, root: winreg::HKEY, subpath: &str, source: &str) {
    let key = winreg::RegKey::predef(root);
    if let Ok(disabled_key) = key.open_subkey_with_flags(subpath, winreg::enums::KEY_READ) {
        for (name, val) in disabled_key.enum_values().flatten() {
            let cmd = string_from_reg_value(&val);
            if !name.is_empty() && !cmd.is_empty() && !items.iter().any(|it| it.name.eq_ignore_ascii_case(&name)) {
                items.push(new_item(name, cmd, false, source.to_string()));
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn collect_folder_items(
    items: &mut Vec<StartupItem>,
    folder_path: &std::path::Path,
    source: &str,
    is_disabled_folder: bool,
    approved_map: &std::collections::HashMap<String, bool>,
) {
    if let Ok(entries) = std::fs::read_dir(folder_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.eq_ignore_ascii_case("desktop.ini") {
                        continue;
                    }
                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(file_name);
                    let cmd = path.to_string_lossy().to_string();
                    let enabled = if is_disabled_folder {
                        false
                    } else {
                        approved_map
                            .get(&file_name.to_lowercase())
                            .or_else(|| approved_map.get(&stem.to_lowercase()))
                            .copied()
                            .unwrap_or(true)
                    };
                    items.push(new_item(stem.to_string(), cmd, enabled, source.to_string()));
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn collect_task_scheduler_items(items: &mut Vec<StartupItem>) {
    let script = r#"Get-ScheduledTask -ErrorAction SilentlyContinue | Where-Object { $_.TaskPath -notlike '\Microsoft\Windows\*' -and ($_.Triggers | Where-Object { $_ -is [Microsoft.Management.Infrastructure.CimInstance] -and $_.CimClass.CimClassName -eq 'MSFT_TaskLogonTrigger' }) } | ForEach-Object { $a = ($_.Actions | Select-Object -First 1).Execute; if ($a) { "$($_.TaskName)|$a|$($_.State)" } }"#;
    if let Some(text) = ps_run(script) {
        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() >= 2 && !parts[0].trim().is_empty() {
                let state = if parts.len() == 3 { parts[2].trim() } else { "Ready" };
                items.push(new_item(
                    parts[0].trim().into(),
                    parts[1].trim().into(),
                    state != "Disabled",
                    "Task Scheduler".into(),
                ));
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn get_startup_data() -> (Vec<StartupItem>, Option<BootDiagnostics>) {
    use winreg::enums::*;

    let diag = get_boot_diagnostics();
    let degrading = diag.as_ref().map(|d| d.degrading_items.clone()).unwrap_or_default();
    let mut items = Vec::new();

    // 1. HKCU Run
    collect_registry_native(
        &mut items,
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
        "Registry (HKCU)",
    );

    // 2. HKLM Run (64-bit)
    collect_registry_native(
        &mut items,
        HKEY_LOCAL_MACHINE,
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
        "Registry (HKLM)",
    );

    // 3. HKLM Run (32-bit WOW6432Node)
    collect_registry_native(
        &mut items,
        HKEY_LOCAL_MACHINE,
        r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run",
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32",
        "Registry (HKLM 32-bit)",
    );

    // 4. Disabled stash keys
    collect_registry_disabled_stash(
        &mut items,
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Run_Disabled",
        "Registry (HKCU)",
    );
    collect_registry_disabled_stash(
        &mut items,
        HKEY_LOCAL_MACHINE,
        r"Software\Microsoft\Windows\CurrentVersion\Run_Disabled",
        "Registry (HKLM)",
    );

    // 5. User Startup Folder
    let hkcu_folder_approved = get_approved_map(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
    );
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let mut user_startup = std::path::PathBuf::from(appdata);
        user_startup.push(r"Microsoft\Windows\Start Menu\Programs\Startup");
        collect_folder_items(
            &mut items,
            &user_startup,
            "Startup Folder (User)",
            false,
            &hkcu_folder_approved,
        );

        let mut user_disabled = user_startup.clone();
        user_disabled.push("_disabled");
        collect_folder_items(
            &mut items,
            &user_disabled,
            "Startup Folder (User)",
            true,
            &hkcu_folder_approved,
        );
    }

    // 6. Common / All Users Startup Folder
    let hklm_folder_approved = get_approved_map(
        HKEY_LOCAL_MACHINE,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
    );
    if let Some(programdata) = std::env::var_os("ProgramData") {
        let mut common_startup = std::path::PathBuf::from(programdata);
        common_startup.push(r"Microsoft\Windows\Start Menu\Programs\Startup");
        collect_folder_items(
            &mut items,
            &common_startup,
            "Startup Folder (Common)",
            false,
            &hklm_folder_approved,
        );
    }

    // 7. Task Scheduler (Logon triggers, excluding OS maintenance tasks)
    collect_task_scheduler_items(&mut items);

    // 8. Enrich items (executable verification, publisher, Authenticode signature)
    enrich_startup_items(&mut items);

    // 9. Score impact
    score_startup_items(&mut items, &degrading);

    (items, diag)
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn get_startup_items() -> Vec<StartupItem> {
    get_startup_data().0
}

#[cfg(not(target_os = "windows"))]
pub fn get_startup_data() -> (Vec<StartupItem>, Option<BootDiagnostics>) {
    (Vec::new(), None)
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn get_startup_items() -> Vec<StartupItem> {
    Vec::new()
}

// ─── Enrichment ──────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn enrich_startup_items(items: &mut [StartupItem]) {
    // 1. Resolve paths & existence (instant, zero PowerShell)
    for item in items.iter_mut() {
        let expanded_cmd = expand_env_vars(&item.command);
        item.exe_path = parse_exe_from_command(&expanded_cmd);
        if let Some(p) = &item.exe_path {
            item.exe_exists = std::path::Path::new(p).exists();
        } else if item.source.contains("Startup Folder") {
            let p = std::path::Path::new(&expanded_cmd);
            item.exe_exists = p.exists();
            item.exe_path = Some(expanded_cmd);
        }
    }

    // 2. Collect unique existing executable paths for batch lookup
    let mut unique_paths: Vec<String> = Vec::new();
    for item in items.iter() {
        if item.exe_exists {
            if let Some(p) = &item.exe_path {
                if !unique_paths.iter().any(|up| up.eq_ignore_ascii_case(p)) {
                    unique_paths.push(p.clone());
                }
            }
        }
    }

    if unique_paths.is_empty() {
        return;
    }

    // 3. Batch lookup for VersionInfo and Authenticode in a single fast PowerShell call
    let mut script = String::from("$paths = @(\n");
    for p in &unique_paths {
        script.push_str(&format!("  '{}'\n", p.replace('\'', "''")));
    }
    script.push_str(
        r#")
foreach ($p in $paths) {
    try {
        $vi = (Get-Item -LiteralPath $p -ErrorAction SilentlyContinue).VersionInfo
        $pub = if ($vi.CompanyName) { $vi.CompanyName } elseif ($vi.FileDescription) { $vi.FileDescription } else { 'Unknown' }
        $sig = (Get-AuthenticodeSignature -LiteralPath $p -ErrorAction SilentlyContinue).Status
        $signed = if ($sig -eq 'Valid') { 'Signed' } elseif ($sig) { 'Unsigned' } else { 'Unknown' }
        "$p|$pub|$signed"
    } catch {
        "$p|Unknown|Unknown"
    }
}
"#,
    );

    if let Some(text) = ps_run(&script) {
        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() == 3 {
                let path = parts[0].trim();
                let pub_name = parts[1].trim();
                let signed_status = parts[2].trim();

                for item in items.iter_mut() {
                    if let Some(ip) = &item.exe_path {
                        if ip.eq_ignore_ascii_case(path) {
                            if pub_name != "Unknown" && !pub_name.is_empty() {
                                item.publisher = Some(pub_name.to_string());
                            }
                            item.is_signed = match signed_status {
                                "Signed" => Some(true),
                                "Unsigned" => Some(false),
                                _ => None,
                            };
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn enrich_startup_items(_items: &mut [StartupItem]) {}

// ─── Impact Scoring ──────────────────────────────────────────

fn score_startup_items(items: &mut [StartupItem], degrading: &[String]) {
    let ms_keywords = ["microsoft", "windows", "microsoft corporation", ".net"];

    for item in items.iter_mut() {
        let pub_lower = item.publisher.as_ref().map(|p| p.to_lowercase()).unwrap_or_default();
        let is_ms = ms_keywords.iter().any(|k| pub_lower.contains(k));
        let is_degrading = degrading.iter().any(|d| d.eq_ignore_ascii_case(&item.name));

        if !item.exe_exists && item.exe_path.is_some() {
            item.impact_tier = ImpactTier::High;
            item.recommendation = Recommendation::Cleanup;
            item.reason = "File not found — broken startup entry".to_string();
        } else if is_degrading {
            item.impact_tier = ImpactTier::High;
            item.recommendation = Recommendation::Review;
            item.reason = "Flagged by Windows boot diagnostics as slowing startup".into();
        } else if is_ms && item.source.contains("HKLM") {
            item.impact_tier = ImpactTier::Low;
            item.recommendation = Recommendation::Keep;
            item.reason = "Windows system component".into();
        } else if item.is_signed == Some(true) && is_ms {
            item.impact_tier = ImpactTier::Low;
            item.recommendation = Recommendation::Keep;
            item.reason = "Verified Microsoft component".to_string();
        } else if item.is_signed == Some(true) && !pub_lower.is_empty() && pub_lower != "unknown" {
            item.impact_tier = ImpactTier::Medium;
            item.recommendation = Recommendation::Review;
            item.reason = format!("Signed by {}", item.publisher.as_deref().unwrap_or("Unknown"));
        } else if item.is_signed == Some(false) {
            item.impact_tier = ImpactTier::High;
            item.recommendation = Recommendation::Disable;
            item.reason = "Unsigned program — review for necessity".into();
        } else {
            item.impact_tier = ImpactTier::Medium;
            item.recommendation = Recommendation::Review;
            item.reason = "Review for necessity".into();
        }
    }
}

// ─── Boot Diagnostics ────────────────────────────────────────

#[cfg(target_os = "windows")]
pub fn get_boot_diagnostics() -> Option<BootDiagnostics> {
    let script = r#"
try {
    $e = Get-WinEvent -LogName 'Microsoft-Windows-Diagnostics-Performance/Operational' -FilterXPath "*[System[EventID=100]]" -MaxEvents 1 -EA Stop
    $xml = [xml]$e.ToXml()
    $ns = New-Object Xml.XmlNamespaceManager($xml.NameTable)
    $ns.AddNamespace('e','http://www.microsoft.com/Windows/Diagnosis/PerfDiag/Events')
    $bt = $xml.SelectSingleNode('//e:BootTime',$ns).'#text'
    $mp = $xml.SelectSingleNode('//e:MainPathBootTime',$ns).'#text'
    $pb = $xml.SelectSingleNode('//e:BootPostBootTime',$ns).'#text'
    "BOOT|$bt|$mp|$pb"
} catch { "BOOT|||" }
try {
    $evts = Get-WinEvent -LogName 'Microsoft-Windows-Diagnostics-Performance/Operational' -FilterXPath "*[System[EventID>=101 and EventID<=110]]" -MaxEvents 20 -EA Stop
    foreach ($ev in $evts) {
        $x = [xml]$ev.ToXml()
        $ns2 = New-Object Xml.XmlNamespaceManager($x.NameTable)
        $ns2.AddNamespace('e','http://www.microsoft.com/Windows/Diagnosis/PerfDiag/Events')
        $n = $x.SelectSingleNode('//e:Name',$ns2).'#text'
        if ($n) { "DEGRADE|$n" }
    }
} catch {}
"#;

    let text = ps_run(script)?;
    let mut diag = BootDiagnostics {
        collected_at: chrono::Local::now().to_rfc3339(),
        ..Default::default()
    };

    for line in text.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.first() == Some(&"BOOT") && parts.len() >= 4 {
            diag.boot_duration_ms = parts[1].trim().parse().ok();
            diag.main_path_boot_ms = parts[2].trim().parse().ok();
            diag.post_boot_ms = parts[3].trim().parse().ok();
        } else if parts.first() == Some(&"DEGRADE") && parts.len() >= 2 {
            let name = parts[1].trim().to_string();
            if !name.is_empty() && !diag.degrading_items.contains(&name) {
                diag.degrading_items.push(name);
            }
        }
    }

    Some(diag)
}

#[cfg(not(target_os = "windows"))]
pub fn get_boot_diagnostics() -> Option<BootDiagnostics> {
    None
}

// ─── Actions ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub fn remove_startup_item(name: &str, source: &str) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    if source.contains("HKCU") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu
            .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
            .and_then(|k| k.delete_value(name));
        let _ = hkcu
            .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run_Disabled", KEY_WRITE)
            .and_then(|k| k.delete_value(name));
        let _ = hkcu
            .open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
                KEY_WRITE,
            )
            .and_then(|k| k.delete_value(name));
        true
    } else if source.contains("HKLM") {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let _ = hklm
            .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
            .and_then(|k| k.delete_value(name));
        let _ = hklm
            .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run_Disabled", KEY_WRITE)
            .and_then(|k| k.delete_value(name));
        let _ = hklm
            .open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
                KEY_WRITE,
            )
            .and_then(|k| k.delete_value(name));
        let _ = hklm
            .open_subkey_with_flags(r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run", KEY_WRITE)
            .and_then(|k| k.delete_value(name));
        true
    } else if source.contains("Startup Folder") {
        let mut deleted = false;
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut p = std::path::PathBuf::from(appdata);
            p.push(r"Microsoft\Windows\Start Menu\Programs\Startup");
            if let Ok(entries) = std::fs::read_dir(&p) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    let stem = ep.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if stem.eq_ignore_ascii_case(name) {
                        deleted = std::fs::remove_file(ep).is_ok() || deleted;
                    }
                }
            }
            let mut dis_p = p.clone();
            dis_p.push("_disabled");
            if let Ok(entries) = std::fs::read_dir(&dis_p) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    let stem = ep.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if stem.eq_ignore_ascii_case(name) {
                        deleted = std::fs::remove_file(ep).is_ok() || deleted;
                    }
                }
            }
        }
        deleted
    } else if source.contains("Task Scheduler") {
        let safe_name = name.replace('\'', "''");
        let script = format!(
            "Unregister-ScheduledTask -TaskName '{}' -Confirm:$false -EA SilentlyContinue; if ($?) {{ 'SUCCESS' }}",
            safe_name
        );
        if let Some(out) = ps_run(&script) {
            out.contains("SUCCESS")
        } else {
            false
        }
    } else {
        false
    }
}

/// Disable a startup item using native Windows StartupApproved binary flags (reversible).
#[cfg(target_os = "windows")]
pub fn disable_startup_item(name: &str, source: &str, _command: &str) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let disabled_bytes: [u8; 12] = [0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    if source.contains("HKCU") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
        if let Ok((app_key, _)) = hkcu.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&disabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            return app_key.set_raw_value(name, &reg_val).is_ok();
        }
        false
    } else if source.contains("HKLM") {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let path = if source.contains("32-bit") {
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32"
        } else {
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
        };
        if let Ok((app_key, _)) = hklm.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&disabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            return app_key.set_raw_value(name, &reg_val).is_ok();
        }
        false
    } else if source.contains("Startup Folder") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder";
        if let Ok((app_key, _)) = hkcu.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&disabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            let _ = app_key.set_raw_value(name, &reg_val);
        }

        // Also move file to _disabled if present in User Startup
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut src_path = std::path::PathBuf::from(appdata);
            src_path.push(r"Microsoft\Windows\Start Menu\Programs\Startup");
            let mut dst_path = src_path.clone();
            dst_path.push("_disabled");
            let _ = std::fs::create_dir_all(&dst_path);

            if let Ok(entries) = std::fs::read_dir(&src_path) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    let stem = ep.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if stem.eq_ignore_ascii_case(name) {
                        if let Some(fname) = ep.file_name() {
                            let mut dest = dst_path.clone();
                            dest.push(fname);
                            return std::fs::rename(&ep, &dest).is_ok();
                        }
                    }
                }
            }
        }
        true
    } else if source.contains("Task Scheduler") {
        let safe_name = name.replace('\'', "''");
        let script = format!(
            "Disable-ScheduledTask -TaskName '{}' -EA SilentlyContinue; if ($?) {{ 'SUCCESS' }}",
            safe_name
        );
        if let Some(out) = ps_run(&script) {
            out.contains("SUCCESS")
        } else {
            false
        }
    } else {
        false
    }
}

/// Re-enable a previously disabled item. Returns true on success.
#[cfg(target_os = "windows")]
pub fn reenable_startup_item(name: &str, source: &str) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let enabled_bytes: [u8; 12] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    if source.contains("HKCU") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
        if let Ok((app_key, _)) = hkcu.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&enabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            let _ = app_key.set_raw_value(name, &reg_val);
        }

        // Also restore from Run_Disabled if it was stored there
        let disabled_path = r"Software\Microsoft\Windows\CurrentVersion\Run_Disabled";
        let run_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
        if let Ok(dis_key) = hkcu.open_subkey_with_flags(disabled_path, KEY_READ) {
            if let Ok(val) = dis_key.get_raw_value(name) {
                if let Ok(run_key) = hkcu.open_subkey_with_flags(run_path, KEY_WRITE) {
                    let _ = run_key.set_raw_value(name, &val);
                    if let Ok(dis_key_w) = hkcu.open_subkey_with_flags(disabled_path, KEY_WRITE) {
                        let _ = dis_key_w.delete_value(name);
                    }
                }
            }
        }
        true
    } else if source.contains("HKLM") {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let path = if source.contains("32-bit") {
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32"
        } else {
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"
        };
        if let Ok((app_key, _)) = hklm.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&enabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            let _ = app_key.set_raw_value(name, &reg_val);
        }
        true
    } else if source.contains("Startup Folder") {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder";
        if let Ok((app_key, _)) = hkcu.create_subkey(path) {
            let reg_val = winreg::RegValue {
                bytes: (&enabled_bytes[..]).into(),
                vtype: winreg::enums::REG_BINARY,
            };
            let _ = app_key.set_raw_value(name, &reg_val);
        }

        // Also restore from _disabled folder back to Startup
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut startup_path = std::path::PathBuf::from(appdata);
            startup_path.push(r"Microsoft\Windows\Start Menu\Programs\Startup");
            let mut disabled_path = startup_path.clone();
            disabled_path.push("_disabled");

            if let Ok(entries) = std::fs::read_dir(&disabled_path) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    let stem = ep.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if stem.eq_ignore_ascii_case(name) {
                        if let Some(fname) = ep.file_name() {
                            let mut dest = startup_path.clone();
                            dest.push(fname);
                            return std::fs::rename(&ep, &dest).is_ok();
                        }
                    }
                }
            }
        }
        true
    } else if source.contains("Task Scheduler") {
        let safe_name = name.replace('\'', "''");
        let script = format!(
            "Enable-ScheduledTask -TaskName '{}' -EA SilentlyContinue; if ($?) {{ 'SUCCESS' }}",
            safe_name
        );
        if let Some(out) = ps_run(&script) {
            out.contains("SUCCESS")
        } else {
            false
        }
    } else {
        false
    }
}

#[cfg(target_os = "windows")]
pub fn open_file_location(path: &str) {
    let clean_path = path.trim().trim_matches('"');
    let _ = std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", clean_path))
        .spawn();
}

#[cfg(target_os = "windows")]
pub fn search_online(name: &str) {
    use std::os::windows::process::CommandExt;
    let query = format!("https://www.google.com/search?q=what+is+{}", urlenccode(name));
    let _ = std::process::Command::new("cmd")
        .creation_flags(0x08000000)
        .args(["/c", "start", "", &query])
        .spawn();
}

fn urlenccode(s: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

#[cfg(not(target_os = "windows"))]
pub fn remove_startup_item(_name: &str, _source: &str) -> bool {
    false
}
#[cfg(not(target_os = "windows"))]
pub fn disable_startup_item(_name: &str, _source: &str, _command: &str) -> bool {
    false
}
#[cfg(not(target_os = "windows"))]
pub fn reenable_startup_item(_name: &str, _source: &str) -> bool {
    false
}
#[cfg(not(target_os = "windows"))]
pub fn open_file_location(_path: &str) {}
#[cfg(not(target_os = "windows"))]
pub fn search_online(_name: &str) {}

// ─── Sorting / Filtering helpers ─────────────────────────────

pub fn high_impact_count(items: &[StartupItem]) -> usize {
    items
        .iter()
        .filter(|i| i.impact_tier == ImpactTier::High && i.enabled)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exe_from_command() {
        assert_eq!(
            parse_exe_from_command(r#""C:\Program Files\App\app.exe" --arg"#),
            Some(r#"C:\Program Files\App\app.exe"#.to_string())
        );
        assert_eq!(
            parse_exe_from_command(r#"C:\Windows\System32\cmd.exe /c start"#),
            Some(r#"C:\Windows\System32\cmd.exe"#.to_string())
        );
        assert_eq!(
            parse_exe_from_command(r#"rundll32.exe "C:\Program Files\Realtek\Audio.dll",Entry"#),
            Some(r#"C:\Program Files\Realtek\Audio.dll"#.to_string())
        );
        assert_eq!(
            parse_exe_from_command(r#"C:\Günlük\日本語\my_app.exe -silent"#),
            Some(r#"C:\Günlük\日本語\my_app.exe"#.to_string())
        );
        assert_eq!(parse_exe_from_command(r#""#), None);
    }

    #[test]
    fn test_expand_env_vars() {
        let expanded = expand_env_vars("%SystemDrive%\\Windows");
        assert!(!expanded.contains("%SystemDrive%"));
        assert!(expanded.ends_with("\\Windows"));
    }

    #[test]
    fn test_approved_disabled_logic() {
        assert!(is_approved_disabled(&[0x03, 0x00, 0x00, 0x00]));
        assert!(is_approved_disabled(&[0x01, 0x00, 0x00, 0x00]));
        assert!(is_approved_disabled(&[0x07, 0x00, 0x00, 0x00]));
        assert!(!is_approved_disabled(&[0x02, 0x00, 0x00, 0x00]));
        assert!(!is_approved_disabled(&[0x06, 0x00, 0x00, 0x00]));
        assert!(!is_approved_disabled(&[]));
    }

    #[test]
    fn test_get_startup_items_live() {
        let items = get_startup_items();
        println!("Collected {} startup items", items.len());
        for it in &items {
            println!("Item: {} ({}) - enabled={}", it.name, it.source, it.enabled);
        }
        assert!(!items.is_empty());
    }

    #[test]
    fn test_notify_rust_windows() {
        let res = notify_rust::Notification::new()
            .summary("SysMon Test")
            .body("Testing notification")
            .show();
        println!("Notify result: {:?}", res);
    }
}
