use super::*;

// ─── Enrichment ──────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub(crate) fn enrich_startup_items(items: &mut [StartupItem]) {
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
        if item.exe_exists
            && let Some(p) = &item.exe_path
            && !unique_paths.iter().any(|up| up.eq_ignore_ascii_case(p))
        {
            unique_paths.push(p.clone());
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
                    if let Some(ip) = &item.exe_path
                        && ip.eq_ignore_ascii_case(path)
                    {
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

#[cfg(not(target_os = "windows"))]
pub(crate) fn enrich_startup_items(_items: &mut [StartupItem]) {}

// ─── Impact Scoring ──────────────────────────────────────────

pub(crate) fn score_startup_items(items: &mut [StartupItem], degrading: &[String]) {
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
