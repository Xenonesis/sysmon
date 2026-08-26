//! Safe storage space reclaimer for temporary files, shader caches, and crash dumps.

use serde::{Deserialize, Serialize};
use std::fs::{read_dir, remove_file};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReclaimCategory {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub paths: Vec<PathBuf>,
    pub size_bytes: u64,
    pub file_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ReclaimCategoryDef {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

pub const RECLAIM_CATEGORIES: &[ReclaimCategoryDef] = &[
    ReclaimCategoryDef {
        id: "shader_cache",
        label: "DirectX & GPU Shader Caches",
        description: "Compiled graphics shaders that will be automatically recreated when games launch.",
    },
    ReclaimCategoryDef {
        id: "crash_dumps",
        label: "Windows & Application Crash Dumps",
        description: "Kernel and user-mode minidump files left behind by past application or OS crashes.",
    },
    ReclaimCategoryDef {
        id: "user_temp",
        label: "User Temporary Files (%TEMP%)",
        description: "Scratch files, extractors, and cached installers that can be safely discarded.",
    },
    ReclaimCategoryDef {
        id: "windows_update",
        label: "Windows Update Download Cache",
        description: "Completed Windows Update delivery packages that have already been staged or installed.",
    },
];

pub fn resolve_category_paths(id: &str) -> Option<Vec<PathBuf>> {
    match id {
        "shader_cache" => {
            let mut paths = Vec::new();
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                let p1 = PathBuf::from(&local).join("D3DSCache");
                let p2 = PathBuf::from(&local).join("NVIDIA").join("DXCache");
                let p3 = PathBuf::from(&local).join("AMD").join("DxCache");
                if p1.exists() {
                    paths.push(p1);
                }
                if p2.exists() {
                    paths.push(p2);
                }
                if p3.exists() {
                    paths.push(p3);
                }
            }
            Some(paths)
        }
        "crash_dumps" => {
            let mut paths = Vec::new();
            let win_dir = std::env::var("SystemRoot")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("C:\\Windows"));
            let minidump_dir = win_dir.join("Minidump");
            if minidump_dir.exists() {
                paths.push(minidump_dir);
            }
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                let crash_dumps = PathBuf::from(&local).join("CrashDumps");
                if crash_dumps.exists() {
                    paths.push(crash_dumps);
                }
            }
            Some(paths)
        }
        "user_temp" => {
            let mut paths = Vec::new();
            let temp_dir = std::env::temp_dir();
            if temp_dir.exists() {
                paths.push(temp_dir);
            }
            Some(paths)
        }
        "windows_update" => {
            let mut paths = Vec::new();
            let win_dir = std::env::var("SystemRoot")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("C:\\Windows"));
            let update_staging = win_dir.join("SoftwareDistribution").join("Download");
            if update_staging.exists() {
                paths.push(update_staging);
            }
            Some(paths)
        }
        _ => None,
    }
}

pub fn calculate_dir_size(path: &Path) -> (u64, usize) {
    let mut total_bytes = 0u64;
    let mut total_files = 0usize;

    if !path.exists() {
        return (0, 0);
    }

    if let Ok(entries) = read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(ft) = entry.file_type() {
                if ft.is_symlink() {
                    // Reparse point / junction / symlink: do NOT follow or recurse!
                    if let Ok(meta) = std::fs::symlink_metadata(&p) {
                        total_bytes += meta.len();
                        total_files += 1;
                    }
                } else if ft.is_file() {
                    if let Ok(meta) = entry.metadata() {
                        total_bytes += meta.len();
                        total_files += 1;
                    }
                } else if ft.is_dir() {
                    let (sub_bytes, sub_files) = calculate_dir_size(&p);
                    total_bytes += sub_bytes;
                    total_files += sub_files;
                }
            }
        }
    }

    (total_bytes, total_files)
}

pub fn scan_reclaimable_caches() -> Vec<ReclaimCategory> {
    RECLAIM_CATEGORIES
        .iter()
        .map(|def| {
            let paths = resolve_category_paths(def.id).unwrap_or_default();
            let mut size_bytes = 0u64;
            let mut file_count = 0usize;
            for p in &paths {
                let (b, c) = calculate_dir_size(p);
                size_bytes += b;
                file_count += c;
            }
            ReclaimCategory {
                id: def.id,
                label: def.label,
                description: def.description,
                paths,
                size_bytes,
                file_count,
            }
        })
        .collect()
}

pub fn clean_reclaimable_category(id: &str) -> Result<(u64, usize), String> {
    let paths = resolve_category_paths(id)
        .ok_or_else(|| format!("Unknown reclaim category '{id}'"))?;

    let mut reclaimed_bytes = 0u64;
    let mut deleted_count = 0usize;

    for dir in &paths {
        clean_dir_contents(dir, &mut reclaimed_bytes, &mut deleted_count);
    }

    Ok((reclaimed_bytes, deleted_count))
}

fn clean_dir_contents(dir: &Path, reclaimed_bytes: &mut u64, deleted_count: &mut usize) {
    if let Ok(entries) = read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(ft) = entry.file_type() {
                if ft.is_symlink() {
                    // Reparse point / junction / symlink:
                    // NEVER recurse into its target directory!
                    // Remove only the link/junction itself.
                    let len = std::fs::symlink_metadata(&p).map(|m| m.len()).unwrap_or(0);
                    if remove_file(&p).is_ok() || std::fs::remove_dir(&p).is_ok() {
                        *reclaimed_bytes += len;
                        *deleted_count += 1;
                    }
                } else if ft.is_file() {
                    if let Ok(meta) = entry.metadata() {
                        let len = meta.len();
                        let mut permissions = meta.permissions();
                        if permissions.readonly() {
                            permissions.set_readonly(false);
                            let _ = std::fs::set_permissions(&p, permissions);
                        }
                        if remove_file(&p).is_ok() {
                            *reclaimed_bytes += len;
                            *deleted_count += 1;
                        }
                    }
                } else if ft.is_dir() {
                    clean_dir_contents(&p, reclaimed_bytes, deleted_count);
                    let _ = std::fs::remove_dir(&p); // Try remove empty directory
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, File};
    use std::io::Write;

    #[test]
    fn test_calculate_dir_size_with_sample_files() {
        let temp_dir = std::env::temp_dir().join("sysmon_reclaim_test_calc");
        let _ = std::fs::remove_dir_all(&temp_dir);
        create_dir_all(&temp_dir).expect("create test dir");

        let f1 = temp_dir.join("test1.bin");
        let f2 = temp_dir.join("test2.bin");
        let sub = temp_dir.join("subdir");
        create_dir_all(&sub).expect("create subdir");
        let f3 = sub.join("test3.bin");
        {
            let mut file1 = File::create(&f1).unwrap();
            file1.write_all(&[0u8; 1024]).unwrap(); // 1 KB
            let mut file2 = File::create(&f2).unwrap();
            file2.write_all(&[0u8; 2048]).unwrap(); // 2 KB
            let mut file3 = File::create(&f3).unwrap();
            file3.write_all(&[0u8; 512]).unwrap(); // 512 B
        }

        let (bytes, count) = calculate_dir_size(&temp_dir);
        assert_eq!(bytes, 3584);
        assert_eq!(count, 3);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_clean_dir_contents_removes_unlocked_and_skips_locked() {
        let temp_dir = std::env::temp_dir().join("sysmon_reclaim_test_lock");
        let _ = std::fs::remove_dir_all(&temp_dir);
        create_dir_all(&temp_dir).expect("create test dir");

        let unlocked_file = temp_dir.join("unlocked.bin");
        let locked_file = temp_dir.join("locked.bin");

        {
            let mut f = File::create(&unlocked_file).unwrap();
            f.write_all(&[1u8; 1000]).unwrap();
        }
        {
            let mut f = File::create(&locked_file).unwrap();
            f.write_all(&[2u8; 2000]).unwrap();
        }

        // Open locked_file with share_mode(0) (no FILE_SHARE_DELETE) to simulate an active locked file
        #[cfg(windows)]
        use std::os::windows::fs::OpenOptionsExt;

        let mut opts = std::fs::OpenOptions::new();
        opts.read(true).write(true);
        #[cfg(windows)]
        opts.share_mode(0); // Exclusive lock: prevents deletion while open
        let _held_file = opts.open(&locked_file).expect("open locked file");
        let mut reclaimed_bytes = 0u64;
        let mut deleted_count = 0usize;

        // Clean directory: unlocked should be deleted, locked should be gracefully skipped
        clean_dir_contents(&temp_dir, &mut reclaimed_bytes, &mut deleted_count);

        assert_eq!(reclaimed_bytes, 1000);
        assert_eq!(deleted_count, 1);
        assert!(!unlocked_file.exists(), "unlocked file should have been deleted");
        assert!(locked_file.exists(), "locked file should still exist and be gracefully skipped");

        drop(_held_file);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_scan_reclaimable_caches_structure() {
        let categories = scan_reclaimable_caches();
        assert_eq!(categories.len(), 4);

        let ids: Vec<&str> = categories.iter().map(|c| c.id).collect();
        assert!(ids.contains(&"shader_cache"));
        assert!(ids.contains(&"crash_dumps"));
        assert!(ids.contains(&"user_temp"));
        assert!(ids.contains(&"windows_update"));

        for cat in categories {
            assert!(!cat.label.is_empty());
            assert!(!cat.description.is_empty());
        }
    }

    #[test]
    fn test_resolve_category_paths_decoupled() {
        let shader = resolve_category_paths("shader_cache");
        assert!(shader.is_some());

        let temp = resolve_category_paths("user_temp");
        assert!(temp.is_some());
        assert!(!temp.unwrap().is_empty());

        let unknown = resolve_category_paths("unknown_xyz");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_clean_reclaimable_category_unknown() {
        let result = clean_reclaimable_category("non_existent_category_id_123");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Unknown reclaim category 'non_existent_category_id_123'"
        );
    }

    #[test]
    fn test_clean_dir_contents_does_not_recurse_into_symlinks() {
        let temp_dir = std::env::temp_dir().join("sysmon_reclaim_test_symlink");
        let _ = std::fs::remove_dir_all(&temp_dir);
        create_dir_all(&temp_dir).expect("create test dir");

        let target_dir = temp_dir.join("target_dir");
        create_dir_all(&target_dir).expect("create target dir");
        let target_file = target_dir.join("precious_file.txt");
        {
            let mut f = File::create(&target_file).unwrap();
            f.write_all(b"must not be deleted").unwrap();
        }

        let clean_root = temp_dir.join("clean_root");
        create_dir_all(&clean_root).expect("create clean root");

        // If symlink can be created, ensure target is not traversed
        #[cfg(windows)]
        let symlink_created = std::os::windows::fs::symlink_dir(&target_dir, clean_root.join("link_to_target")).is_ok();
        #[cfg(not(windows))]
        let symlink_created = false;

        if symlink_created {
            let mut reclaimed_bytes = 0u64;
            let mut deleted_count = 0usize;
            clean_dir_contents(&clean_root, &mut reclaimed_bytes, &mut deleted_count);

            // Target file in target_dir MUST still exist!
            assert!(target_file.exists(), "Target file in target_dir was deleted through symlink recursion!");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
