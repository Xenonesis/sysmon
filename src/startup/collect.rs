use super::enrich::{enrich_startup_items, score_startup_items};
use super::*;

// ─── Collection ──────────────────────────────────────────────

fn new_item(name: String, command: String, enabled: bool, source: String, locator: StartupLocator) -> StartupItem {
    StartupItem {
        name,
        command,
        enabled,
        source,
        locator,
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
pub(crate) fn ps_run(script: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("powershell")
        .creation_flags(0x08000000)
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(script)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}

#[cfg(target_os = "windows")]
pub(crate) fn is_approved_disabled(bytes: &[u8]) -> bool {
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
                .as_chunks::<2>()
                .0
                .iter()
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
    hive: StartupRegistryHive,
) {
    let approved_map = get_approved_map(root, approved_subpath);
    let key = winreg::RegKey::predef(root);
    if let Ok(run_key) = key.open_subkey_with_flags(run_subpath, winreg::enums::KEY_READ) {
        for (name, val) in run_key.enum_values().flatten() {
            let cmd = string_from_reg_value(&val);
            if !name.is_empty() && !cmd.is_empty() {
                let enabled = approved_map.get(&name.to_lowercase()).copied().unwrap_or(true);
                let locator = StartupLocator::Registry {
                    hive: hive.clone(),
                    value_path: run_subpath.to_string(),
                    enabled_value_path: run_subpath.to_string(),
                    approved_path: approved_subpath.to_string(),
                    value_name: name.clone(),
                };
                items.push(new_item(name, cmd, enabled, source.to_string(), locator));
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn collect_registry_disabled_stash(
    items: &mut Vec<StartupItem>,
    root: winreg::HKEY,
    subpath: &str,
    enabled_subpath: &str,
    approved_subpath: &str,
    source: &str,
    hive: StartupRegistryHive,
) {
    let key = winreg::RegKey::predef(root);
    if let Ok(disabled_key) = key.open_subkey_with_flags(subpath, winreg::enums::KEY_READ) {
        for (name, val) in disabled_key.enum_values().flatten() {
            let cmd = string_from_reg_value(&val);
            if !name.is_empty() && !cmd.is_empty() && !items.iter().any(|it| it.name.eq_ignore_ascii_case(&name)) {
                let locator = StartupLocator::Registry {
                    hive: hive.clone(),
                    value_path: subpath.to_string(),
                    enabled_value_path: enabled_subpath.to_string(),
                    approved_path: approved_subpath.to_string(),
                    value_name: name.clone(),
                };
                items.push(new_item(name, cmd, false, source.to_string(), locator));
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
    approved_hive: StartupRegistryHive,
    approved_path: &str,
) {
    if let Ok(entries) = std::fs::read_dir(folder_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && let Some(file_name) = path.file_name().and_then(|n| n.to_str())
            {
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
                let (enabled_path, disabled_path) = if is_disabled_folder {
                    let enabled_path = folder_path.parent().unwrap_or(folder_path).join(file_name);
                    (enabled_path, path.clone())
                } else {
                    (path.clone(), folder_path.join("_disabled").join(file_name))
                };
                let locator = StartupLocator::StartupFolder {
                    enabled_path: enabled_path.to_string_lossy().into_owned(),
                    disabled_path: disabled_path.to_string_lossy().into_owned(),
                    approved_hive: approved_hive.clone(),
                    approved_path: approved_path.to_string(),
                    approved_name: file_name.to_string(),
                };
                items.push(new_item(stem.to_string(), cmd, enabled, source.to_string(), locator));
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn collect_task_scheduler_items(items: &mut Vec<StartupItem>) {
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct TaskItem {
        task_name: String,
        task_path: String,
        command: String,
        state: String,
    }

    let script = r#"Get-ScheduledTask -ErrorAction SilentlyContinue | Where-Object { $_.TaskPath -notlike '\Microsoft\Windows\*' -and ($_.Triggers | Where-Object { $_ -is [Microsoft.Management.Infrastructure.CimInstance] -and $_.CimClass.CimClassName -eq 'MSFT_TaskLogonTrigger' }) } | ForEach-Object { $a = ($_.Actions | Select-Object -First 1).Execute; if ($a) { [pscustomobject]@{ TaskName=$_.TaskName; TaskPath=$_.TaskPath; Command=$a; State=[string]$_.State } | ConvertTo-Json -Compress } }"#;
    if let Some(text) = ps_run(script) {
        for line in text.lines() {
            if let Ok(task) = serde_json::from_str::<TaskItem>(line) {
                if task.task_name.trim().is_empty() {
                    continue;
                }
                let locator = StartupLocator::ScheduledTask {
                    task_path: task.task_path.clone(),
                    task_name: task.task_name.clone(),
                };
                items.push(new_item(
                    task.task_name,
                    task.command,
                    task.state != "Disabled",
                    "Task Scheduler".into(),
                    locator,
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
        StartupRegistryHive::CurrentUser,
    );

    // 2. HKLM Run (64-bit)
    collect_registry_native(
        &mut items,
        HKEY_LOCAL_MACHINE,
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
        "Registry (HKLM)",
        StartupRegistryHive::LocalMachine,
    );

    // 3. HKLM Run (32-bit WOW6432Node)
    collect_registry_native(
        &mut items,
        HKEY_LOCAL_MACHINE,
        r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run",
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32",
        "Registry (HKLM 32-bit)",
        StartupRegistryHive::LocalMachine,
    );

    // 4. Disabled stash keys
    collect_registry_disabled_stash(
        &mut items,
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Run_Disabled",
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
        "Registry (HKCU)",
        StartupRegistryHive::CurrentUser,
    );
    collect_registry_disabled_stash(
        &mut items,
        HKEY_LOCAL_MACHINE,
        r"Software\Microsoft\Windows\CurrentVersion\Run_Disabled",
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
        "Registry (HKLM)",
        StartupRegistryHive::LocalMachine,
    );

    // 5. User Startup Folder
    let hkcu_folder_approved = get_approved_map(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
    );
    if let Ok(user_startup) = crate::app_paths::startup_folder(false) {
        collect_folder_items(
            &mut items,
            &user_startup,
            "Startup Folder (User)",
            false,
            &hkcu_folder_approved,
            StartupRegistryHive::CurrentUser,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
        );

        let mut user_disabled = user_startup.clone();
        user_disabled.push("_disabled");
        collect_folder_items(
            &mut items,
            &user_disabled,
            "Startup Folder (User)",
            true,
            &hkcu_folder_approved,
            StartupRegistryHive::CurrentUser,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
        );
    }

    // 6. Common / All Users Startup Folder
    let hklm_folder_approved = get_approved_map(
        HKEY_LOCAL_MACHINE,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
    );
    if let Ok(common_startup) = crate::app_paths::startup_folder(true) {
        collect_folder_items(
            &mut items,
            &common_startup,
            "Startup Folder (Common)",
            false,
            &hklm_folder_approved,
            StartupRegistryHive::LocalMachine,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
        );
        collect_folder_items(
            &mut items,
            &common_startup.join("_disabled"),
            "Startup Folder (Common)",
            true,
            &hklm_folder_approved,
            StartupRegistryHive::LocalMachine,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
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
