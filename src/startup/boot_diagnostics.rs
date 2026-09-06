use super::collect::ps_run;
use super::*;

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
