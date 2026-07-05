use std::fs;
use std::path::Path;

use crate::models::ScanWarning;

/// Walk the data directory tree and collect integrity warnings.
///
/// - Recurse into year/month subdirectories.
/// - `.yaml` files (except `_monthly.yaml`): validate date stem and YAML.
/// - `.tmp` files: report as orphaned temp files.
/// - Non-`.yaml`, non-`.tmp` files: silently skipped.
/// - Directories that cannot be read are reported as warnings.
pub fn scan_data_dir(root: &Path) -> Vec<ScanWarning> {
    let mut warnings = Vec::new();
    scan_dir(root, root, &mut warnings);
    warnings
}

fn scan_dir(root: &Path, dir: &Path, warnings: &mut Vec<ScanWarning>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            warnings.push(ScanWarning {
                kind: "UnreadableDir".to_string(),
                path: relative_path(root, dir),
                message: format!("Cannot read directory: {}", e),
            });
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warnings.push(ScanWarning {
                    kind: "UnreadableEntry".to_string(),
                    path: relative_path(root, dir),
                    message: format!("Cannot read directory entry: {}", e),
                });
                continue;
            }
        };

        let path = entry.path();

        if path.is_dir() {
            scan_dir(root, &path, warnings);
            continue;
        }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        if file_name.ends_with(".tmp") {
            warnings.push(ScanWarning {
                kind: "OrphanedTemp".to_string(),
                path: relative_path(root, &path),
                message: "orphaned temporary file".to_string(),
            });
            continue;
        }

        if !file_name.ends_with(".yaml") {
            continue;
        }

        // Only scan .yaml files inside YYYY/MM/ directories.
        // Root-level .yaml files (dimensions.template.yaml, etc.) are silently skipped.
        {
            let parent = path.parent();
            let grandparent = parent.and_then(|p| p.parent());
            let is_in_month_dir = parent
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|name| {
                    name.len() == 2
                        && name
                            .parse::<u32>()
                            .map_or(false, |m| (1..=12).contains(&m))
                })
                .unwrap_or(false)
                && grandparent
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .map(|name| name.len() == 4 && name.parse::<u32>().is_ok())
                    .unwrap_or(false);
            if !is_in_month_dir {
                continue;
            }
        }

        let stem = &file_name[..file_name.len() - 5]; // strip ".yaml"

        // Validate date stem (YYYY-MM-DD)
        if chrono::NaiveDate::parse_from_str(stem, "%Y-%m-%d").is_err() {
            warnings.push(ScanWarning {
                kind: "SkippedFile".to_string(),
                path: relative_path(root, &path),
                message: format!("invalid date filename: {}", file_name),
            });
            continue;
        }

        // Validate frontmatter via read_day_file
        if let Err(e) = crate::files::read_day_file(root, stem) {
            warnings.push(ScanWarning {
                kind: "CorruptedFile".to_string(),
                path: relative_path(root, &path),
                message: e,
            });
        }
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!("logbook_scan_test_{}", uuid::Uuid::new_v4()))
    }

    /// Create directory tree and all intermediate dirs.
    fn mkdirs(p: &PathBuf) {
        fs::create_dir_all(p).expect("mkdirs");
    }

    /// Write a file, creating parent directories as needed.
    fn write_file(path: &PathBuf, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dirs");
        }
        fs::write(path, content).expect("write file");
    }

    // ---------------------------------------------------------------------------
    // 1. empty dir
    // ---------------------------------------------------------------------------

    #[test]
    fn test_empty_dir_no_warnings() {
        let root = temp_root();
        mkdirs(&root);

        let warnings = scan_data_dir(&root);
        assert!(
            warnings.is_empty(),
            "expected no warnings in empty dir, got {:?}",
            warnings
        );

        fs::remove_dir_all(&root).expect("cleanup");
    }

    // ---------------------------------------------------------------------------
    // 2. nonexistent dir
    // ---------------------------------------------------------------------------

    #[test]
    fn test_nonexistent_dir_reports_unreadable() {
        let root = temp_root();
        // Do NOT create the directory

        let warnings = scan_data_dir(&root);
        assert_eq!(warnings.len(), 1, "expected 1 warning for nonexistent dir, got {:?}", warnings);
        assert_eq!(warnings[0].kind, "UnreadableDir");
    }

    // ---------------------------------------------------------------------------
    // 3. valid day file
    // ---------------------------------------------------------------------------

    #[test]
    fn test_valid_day_file_no_warnings() {
        let root = temp_root();
        let day_file = root.join("2026/06/2026-06-15.yaml");
        write_file(
            &day_file,
            "note: A valid note\nentries: []\n",
        );

        let warnings = scan_data_dir(&root);
        assert!(
            warnings.is_empty(),
            "expected no warnings for valid day file, got {:?}",
            warnings
        );

        fs::remove_dir_all(&root).expect("cleanup");
    }

    // ---------------------------------------------------------------------------
    // 4. invalid filename
    // ---------------------------------------------------------------------------

    #[test]
    fn test_invalid_filename_reported() {
        let root = temp_root();
        let bad_file = root.join("2026/06/not-a-date.yaml");
        write_file(&bad_file, "note: ok\n");

        let warnings = scan_data_dir(&root);
        assert_eq!(warnings.len(), 1, "expected 1 warning, got {:?}", warnings);

        assert_eq!(warnings[0].kind, "SkippedFile");
        assert!(warnings[0].path.contains("not-a-date.yaml"));
        assert!(!warnings[0].message.is_empty());

        fs::remove_dir_all(&root).expect("cleanup");
    }

    // ---------------------------------------------------------------------------
    // 5. corrupt frontmatter
    // ---------------------------------------------------------------------------

    #[test]
    fn test_corrupt_frontmatter_reported() {
        let root = temp_root();
        let day_file = root.join("2026/06/2026-06-15.yaml");
        write_file(&day_file, "\tindented: not valid yaml\n");

        let warnings = scan_data_dir(&root);
        assert_eq!(warnings.len(), 1, "expected 1 warning, got {:?}", warnings);

        assert_eq!(warnings[0].kind, "CorruptedFile");
        assert!(warnings[0].path.contains("2026-06-15.yaml"));
        assert!(!warnings[0].message.is_empty());

        fs::remove_dir_all(&root).expect("cleanup");
    }

    // ---------------------------------------------------------------------------
    // 6. orphaned .tmp
    // ---------------------------------------------------------------------------

    #[test]
    fn test_orphaned_tmp_reported() {
        let root = temp_root();
        let tmp_file = root.join("2026/06/2026-06-15.yaml.tmp");
        write_file(&tmp_file, "leftover temp content\n");

        let warnings = scan_data_dir(&root);
        assert_eq!(warnings.len(), 1, "expected 1 warning, got {:?}", warnings);

        assert_eq!(warnings[0].kind, "OrphanedTemp");
        assert!(warnings[0].path.contains(".tmp"));
        assert!(!warnings[0].message.is_empty());

        fs::remove_dir_all(&root).expect("cleanup");
    }

    // ---------------------------------------------------------------------------
    // 8. non-.yaml files skipped
    // ---------------------------------------------------------------------------

    #[test]
    fn test_non_yaml_files_skipped() {
        let root = temp_root();
        let txt = root.join("2026/06/notes.txt");
        write_file(&txt, "some text file\n");

        let warnings = scan_data_dir(&root);
        assert!(
            warnings.is_empty(),
            "non-.yaml files should be skipped, got {:?}",
            warnings
        );

        fs::remove_dir_all(&root).expect("cleanup");
    }

    // ---------------------------------------------------------------------------
    // 9. multiple issues accumulated
    // ---------------------------------------------------------------------------

    #[test]
    fn test_multiple_issues_accumulated() {
        let root = temp_root();

        // bad filename
        write_file(&root.join("2026/06/not-a-date.yaml"), "note: ok\n");
        // corrupt frontmatter
        write_file(
            &root.join("2026/06/2026-06-15.yaml"),
            "\tbad yaml here\n",
        );
        // orphaned tmp
        write_file(&root.join("2026/06/2026-06-16.yaml.tmp"), "orphan\n");

        let warnings = scan_data_dir(&root);
        assert_eq!(warnings.len(), 3, "expected 3 warnings, got {:?}", warnings);

        let kinds: Vec<&str> = warnings.iter().map(|w| w.kind.as_str()).collect();
        assert!(kinds.contains(&"SkippedFile"));
        assert!(kinds.contains(&"CorruptedFile"));
        assert!(kinds.contains(&"OrphanedTemp"));

        fs::remove_dir_all(&root).expect("cleanup");
    }
}
