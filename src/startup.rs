//! Startup Manager: diagnostics, enrichment, scoring, and actions.

use serde::{Deserialize, Serialize};

// ─── Data Models ─────────────────────────────────────────────

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ImpactTier { Low, Medium, High, Unknown }

impl ImpactTier {
    pub fn label(&self) -> &str {
        match self { Self::Low => "LOW", Self::Medium => "MED", Self::High => "HIGH", Self::Unknown => "?" }
    }
    pub fn sort_key(&self) -> u8 {
        match self { Self::High => 0, Self::Medium => 1, Self::Unknown => 2, Self::Low => 3 }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Recommendation { Keep, Review, Disable, Cleanup }

impl Recommendation {
    pub fn label(&self) -> &str {
        match self { Self::Keep => "Keep", Self::Review => "Review", Self::Disable => "Disable", Self::Cleanup => "Cleanup" }
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

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BootDiagnostics {
    pub boot_duration_ms: Option<u64>,
    pub main_path_boot_ms: Option<u64>,
    pub post_boot_ms: Option<u64>,
    pub collected_at: String,
    pub degrading_items: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
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
pub enum StartupSortColumn { Name, Impact, Source, Publisher }

// ─── Path Parsing ────────────────────────────────────────────

pub fn parse_exe_from_command(cmd: &str) -> Option<String> {
    let t = cmd.trim();
    if t.is_empty() { return None; }

    // "C:\path\app.exe" --args
    if t.starts_with('"') {
        if let Some(end) = t[1..].find('"') {
            let p = &t[1..=end];
            if !p.is_empty() { return Some(p.to_string()); }
        }
    }

    // rundll32 handling
    let lower = t.to_lowercase();
    if lower.starts_with("rundll32") {
        let skip = if lower.starts_with("rundll32.exe") { 12 } else { 8 };
        let after = t[skip..].trim().trim_start_matches('"');
        if let Some(comma) = after.find(',') {
            let dll = after[..comma].trim().trim_end_matches('"');
            if !dll.is_empty() { return Some(dll.to_string()); }
        }
    }

    // Find known extensions
    for ext in &[".exe", ".cmd", ".bat", ".vbs"] {
        if let Some(pos) = lower.find(ext) {
            return Some(t[..pos + ext.len()].to_string());
        }
    }

    t.split_whitespace().next().map(|s| s.to_string())
}

// ─── Collection ──────────────────────────────────────────────

fn new_item(name: String, command: String, enabled: bool, source: String) -> StartupItem {
    StartupItem {
        name, command, enabled, source,
        exe_path: None, exe_exists: false, publisher: None, is_signed: None,
        impact_tier: ImpactTier::Unknown, recommendation: Recommendation::Review,
        reason: String::new(),
    }
}

#[cfg(target_os = "windows")]
fn ps_run(script: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("powershell")
        .creation_flags(0x08000000)
        .arg("-Command")
        .arg(script)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}

#[cfg(target_os = "windows")]
fn collect_registry_items(items: &mut Vec<StartupItem>, path: &str, source: &str) {
    let script = format!(
        r#"Get-ItemProperty -Path '{}' -ErrorAction SilentlyContinue | ForEach-Object {{ $_.PSObject.Properties | Where-Object {{ $_.Name -notlike 'PS*' }} | ForEach-Object {{ "$($_.Name)|$($_.Value)" }} }}"#,
        path
    );
    if let Some(text) = ps_run(&script) {
        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() == 2 {
                items.push(new_item(
                    parts[0].trim().to_string(),
                    parts[1].trim().to_string(),
                    true,
                    source.to_string(),
                ));
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn get_startup_items() -> Vec<StartupItem> {
    let mut items = Vec::new();

    collect_registry_items(&mut items, r"HKCU:\Software\Microsoft\Windows\CurrentVersion\Run", "Registry (HKCU)");
    collect_registry_items(&mut items, r"HKLM:\Software\Microsoft\Windows\CurrentVersion\Run", "Registry (HKLM)");

    // Startup folder
    if let Some(text) = ps_run(r#"Get-ChildItem "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup" -ErrorAction SilentlyContinue | ForEach-Object { "$($_.BaseName)|$($_.FullName)" }"#) {
        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() == 2 {
                items.push(new_item(parts[0].trim().into(), parts[1].trim().into(), true, "Startup Folder".into()));
            }
        }
    }

    // Task Scheduler (logon triggers)
    if let Some(text) = ps_run(r#"Get-ScheduledTask -ErrorAction SilentlyContinue | Where-Object { $_.Triggers | Where-Object { $_ -is [Microsoft.Management.Infrastructure.CimInstance] -and $_.CimClass.CimClassName -eq 'MSFT_TaskLogonTrigger' } } | ForEach-Object { $a = ($_.Actions | Select-Object -First 1).Execute; "$($_.TaskName)|$a|$($_.State)" }"#) {
        for line in text.lines() {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() >= 2 && !parts[0].trim().is_empty() {
                let state = if parts.len() == 3 { parts[2].trim() } else { "Ready" };
                items.push(new_item(
                    parts[0].trim().into(), parts[1].trim().into(),
                    state != "Disabled", "Task Scheduler".into(),
                ));
            }
        }
    }

    enrich_startup_items(&mut items);

    // Collect boot diagnostics for degrading item cross-ref
    let degrading = get_boot_diagnostics()
        .map(|b| b.degrading_items.clone())
        .unwrap_or_default();
    score_startup_items(&mut items, &degrading);

    items
}

#[cfg(not(target_os = "windows"))]
pub fn get_startup_items() -> Vec<StartupItem> { Vec::new() }

// ─── Enrichment ──────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn enrich_startup_items(items: &mut Vec<StartupItem>) {
    // Resolve paths & existence (no PowerShell needed)
    for item in items.iter_mut() {
        item.exe_path = parse_exe_from_command(&item.command);
        if let Some(ref p) = item.exe_path {
            item.exe_exists = std::path::Path::new(p).exists();
        } else if item.source == "Startup Folder" {
            item.exe_exists = std::path::Path::new(&item.command).exists();
            item.exe_path = Some(item.command.clone());
        }
    }

    // Batch publisher lookup
    let paths: Vec<(usize, String)> = items.iter().enumerate()
        .filter_map(|(i, it)| if it.exe_exists { it.exe_path.as_ref().map(|p| (i, p.clone())) } else { None })
        .collect();

    if !paths.is_empty() {
        let mut ps = String::from("$paths = @(\n");
        for (_, p) in &paths { ps.push_str(&format!("  '{}'\n", p.replace('\'', "''"))); }
        ps.push_str(")\nforeach ($p in $paths) { try { $vi = (Get-Item $p -EA SilentlyContinue).VersionInfo; $c = if($vi.CompanyName){$vi.CompanyName}elseif($vi.FileDescription){$vi.FileDescription}else{'Unknown'}; \"$p|$c\" } catch { \"$p|Unknown\" } }\n");

        if let Some(text) = ps_run(&ps) {
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                if parts.len() == 2 {
                    let (path, pub_name) = (parts[0].trim(), parts[1].trim());
                    for (idx, p) in &paths {
                        if p.eq_ignore_ascii_case(path) && pub_name != "Unknown" && !pub_name.is_empty() {
                            items[*idx].publisher = Some(pub_name.to_string());
                        }
                    }
                }
            }
        }

        // Batch signature check
        let mut ps = String::from("$paths = @(\n");
        for (_, p) in &paths { ps.push_str(&format!("  '{}'\n", p.replace('\'', "''"))); }
        ps.push_str(")\nforeach ($p in $paths) { try { $s = Get-AuthenticodeSignature $p -EA SilentlyContinue; $r = if($s.Status -eq 'Valid'){'Signed'}else{'Unsigned'}; \"$p|$r\" } catch { \"$p|Unknown\" } }\n");

        if let Some(text) = ps_run(&ps) {
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                if parts.len() == 2 {
                    let (path, status) = (parts[0].trim(), parts[1].trim());
                    for (idx, p) in &paths {
                        if p.eq_ignore_ascii_case(path) {
                            items[*idx].is_signed = match status {
                                "Signed" => Some(true), "Unsigned" => Some(false), _ => None,
                            };
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn enrich_startup_items(_items: &mut Vec<StartupItem>) {}

// ─── Impact Scoring ──────────────────────────────────────────

fn score_startup_items(items: &mut Vec<StartupItem>, degrading: &[String]) {
    let ms_keywords = ["microsoft", "windows", "microsoft corporation", ".net"];

    for item in items.iter_mut() {
        let pub_lower = item.publisher.as_ref().map(|p| p.to_lowercase()).unwrap_or_default();
        let is_ms = ms_keywords.iter().any(|k| pub_lower.contains(k));
        let is_degrading = degrading.iter().any(|d| d.eq_ignore_ascii_case(&item.name));

        if !item.exe_exists && item.exe_path.is_some() {
            item.impact_tier = ImpactTier::High;
            item.recommendation = Recommendation::Cleanup;
            item.reason = format!("File not found — broken startup entry");
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
            item.reason = format!("Verified Microsoft component");
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
pub fn get_boot_diagnostics() -> Option<BootDiagnostics> { None }

// ─── Actions ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub fn remove_startup_item(name: &str, source: &str) -> bool {
    if source.contains("HKCU") {
        ps_run(&format!(
            "Remove-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -Name '{}' -EA SilentlyContinue",
            name.replace('\'', "''")
        )).is_some()
    } else if source.contains("Startup Folder") {
        ps_run(&format!(
            "Remove-Item \"$env:APPDATA\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\{}*\" -EA SilentlyContinue",
            name.replace('"', "")
        )).is_some()
    } else {
        false
    }
}

/// Disable by renaming the registry value (reversible). Returns true on success.
#[cfg(target_os = "windows")]
pub fn disable_startup_item(name: &str, source: &str, command: &str) -> bool {
    if source.contains("HKCU") {
        let escaped_name = name.replace('\'', "''");
        let escaped_cmd = command.replace('\'', "''");
        let script = format!(
            r#"$path = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'; Remove-ItemProperty -Path $path -Name '{}' -EA SilentlyContinue; $disabled = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run_Disabled'; if (!(Test-Path $disabled)) {{ New-Item -Path $disabled -Force | Out-Null }}; Set-ItemProperty -Path $disabled -Name '{}' -Value '{}'"#,
            escaped_name, escaped_name, escaped_cmd
        );
        ps_run(&script).is_some()
    } else if source.contains("Startup Folder") {
        // Move the shortcut to a _disabled subfolder
        let script = format!(
            r#"$src = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup"; $dst = "$src\_disabled"; if (!(Test-Path $dst)) {{ New-Item -Path $dst -ItemType Directory -Force | Out-Null }}; Move-Item "$src\{}*" $dst -Force -EA SilentlyContinue"#,
            name.replace('"', "")
        );
        ps_run(&script).is_some()
    } else {
        false
    }
}

/// Re-enable a previously disabled item. Returns true on success.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn reenable_startup_item(name: &str, source: &str) -> bool {
    if source.contains("HKCU") {
        let escaped = name.replace('\'', "''");
        let script = format!(
            r#"$disabled = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run_Disabled'; $val = (Get-ItemProperty -Path $disabled -Name '{}' -EA SilentlyContinue).'{0}'; if ($val) {{ Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name '{0}' -Value $val; Remove-ItemProperty -Path $disabled -Name '{0}' -EA SilentlyContinue; $true }} else {{ $false }}"#,
            escaped
        );
        ps_run(&script).map(|s| s.trim().contains("True")).unwrap_or(false)
    } else {
        false
    }
}

#[cfg(target_os = "windows")]
pub fn open_file_location(path: &str) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("explorer.exe")
        .creation_flags(0x08000000)
        .arg(format!("/select,\"{}\"", path))
        .spawn();
}

#[cfg(target_os = "windows")]
pub fn search_online(name: &str) {
    use std::os::windows::process::CommandExt;
    let query = format!("https://www.google.com/search?q=what+is+{}", urlenccode(name));
    let _ = std::process::Command::new("powershell")
        .creation_flags(0x08000000)
        .arg("-Command")
        .arg(format!("Start-Process '{}'", query))
        .spawn();
}

fn urlenccode(s: &str) -> String {
    s.chars().map(|c| {
        if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c.to_string() }
        else { format!("%{:02X}", c as u8) }
    }).collect()
}

#[cfg(not(target_os = "windows"))]
pub fn remove_startup_item(_name: &str, _source: &str) -> bool { false }
#[cfg(not(target_os = "windows"))]
pub fn disable_startup_item(_name: &str, _source: &str, _command: &str) -> bool { false }
#[cfg(not(target_os = "windows"))]
pub fn reenable_startup_item(_name: &str, _source: &str) -> bool { false }
#[cfg(not(target_os = "windows"))]
pub fn open_file_location(_path: &str) {}
#[cfg(not(target_os = "windows"))]
pub fn search_online(_name: &str) {}

// ─── Sorting / Filtering helpers ─────────────────────────────

#[allow(dead_code)]
pub fn sort_items(items: &mut Vec<StartupItem>, col: StartupSortColumn, ascending: bool) {
    items.sort_by(|a, b| {
        let cmp = match col {
            StartupSortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            StartupSortColumn::Impact => a.impact_tier.sort_key().cmp(&b.impact_tier.sort_key()),
            StartupSortColumn::Source => a.source.cmp(&b.source),
            StartupSortColumn::Publisher => {
                let pa = a.publisher.as_deref().unwrap_or("zzz").to_lowercase();
                let pb = b.publisher.as_deref().unwrap_or("zzz").to_lowercase();
                pa.cmp(&pb)
            }
        };
        if ascending { cmp } else { cmp.reverse() }
    });
}

pub fn high_impact_count(items: &[StartupItem]) -> usize {
    items.iter().filter(|i| i.impact_tier == ImpactTier::High).count()
}
