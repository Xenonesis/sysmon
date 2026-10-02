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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartupRegistryHive {
    CurrentUser,
    LocalMachine,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartupLocator {
    Registry {
        hive: StartupRegistryHive,
        value_path: String,
        enabled_value_path: String,
        approved_path: String,
        value_name: String,
    },
    StartupFolder {
        enabled_path: String,
        disabled_path: String,
        approved_hive: StartupRegistryHive,
        approved_path: String,
        approved_name: String,
    },
    ScheduledTask {
        task_path: String,
        task_name: String,
    },
}

impl Default for StartupLocator {
    fn default() -> Self {
        Self::ScheduledTask {
            task_path: "\\".into(),
            task_name: String::new(),
        }
    }
}

impl StartupLocator {
    pub fn requires_admin(&self) -> bool {
        match self {
            Self::Registry { hive, .. } => *hive == StartupRegistryHive::LocalMachine,
            Self::StartupFolder { approved_hive, .. } => *approved_hive == StartupRegistryHive::LocalMachine,
            Self::ScheduledTask { .. } => true,
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
    pub locator: StartupLocator,
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

mod actions;
mod boot_diagnostics;
mod collect;
mod enrich;
mod parsing;

pub use actions::*;
pub use boot_diagnostics::*;
pub use collect::*;
pub use parsing::*;

// ─── Sorting / Filtering helpers ─────────────────────────────

pub fn high_impact_count(items: &[StartupItem]) -> usize {
    items
        .iter()
        .filter(|i| i.impact_tier == ImpactTier::High && i.enabled)
        .count()
}

#[cfg(test)]
mod tests {
    use super::actions::valid_quarantine_id;
    use super::collect::is_approved_disabled;
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
        assert_eq!(
            parse_exe_from_command(r#"C:\Users\Юрий\Programs\launcher.exe --user-data-dir="C:\Data""#),
            Some(r#"C:\Users\Юрий\Programs\launcher.exe"#.to_string())
        );
        assert_eq!(
            parse_exe_from_command(r#"C:\Users\André\app.bat"#),
            Some(r#"C:\Users\André\app.bat"#.to_string())
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
    fn startup_locators_keep_duplicate_names_isolated() {
        let first = StartupLocator::ScheduledTask {
            task_path: r"\VendorA\".into(),
            task_name: "Updater".into(),
        };
        let second = StartupLocator::ScheduledTask {
            task_path: r"\VendorB\".into(),
            task_name: "Updater".into(),
        };
        assert_ne!(first, second);
    }

    #[test]
    fn quarantine_ids_reject_path_traversal() {
        assert!(valid_quarantine_id("1234-5678"));
        assert!(!valid_quarantine_id("..\\record"));
        assert!(!valid_quarantine_id("../record"));
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
