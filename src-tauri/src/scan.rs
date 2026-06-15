use std::path::Path;

use crate::models::ScanWarning;

// TODO: implement
pub fn scan_data_dir(_root: &Path) -> Vec<ScanWarning> {
    vec![]
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
    fn test_nonexistent_dir_no_warnings() {
        let root = temp_root();
        // Do NOT create the directory

        let warnings = scan_data_dir(&root);
        assert!(
            warnings.is_empty(),
            "expected no warnings for nonexistent dir, got {:?}",
            warnings
        );
        // nothing to clean up — dir was never created
    }

    // ---------------------------------------------------------------------------
    // 3. valid day file
    // ---------------------------------------------------------------------------

    #[test]
    fn test_valid_day_file_no_warnings() {
        let root = temp_root();
        let day_file = root.join("2026/06/2026-06-15.md");
        write_file(
            &day_file,
            "---\nnote: \"A valid note\"\n---\n\nSome body text.\n",
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
        let bad_file = root.join("2026/06/not-a-date.md");
        write_file(&bad_file, "---\nnote: ok\n---\n");

        let warnings = scan_data_dir(&root);
        assert_eq!(warnings.len(), 1, "expected 1 warning, got {:?}", warnings);

        assert_eq!(warnings[0].kind, "SkippedFile");
        assert!(warnings[0].path.contains("not-a-date.md"));
        assert!(!warnings[0].message.is_empty());

        fs::remove_dir_all(&root).expect("cleanup");
    }

    // ---------------------------------------------------------------------------
    // 5. corrupt frontmatter
    // ---------------------------------------------------------------------------

    #[test]
    fn test_corrupt_frontmatter_reported() {
        let root = temp_root();
        let day_file = root.join("2026/06/2026-06-15.md");
        write_file(&day_file, "this is not yaml\nno frontmatter markers\njust garbage\n");

        let warnings = scan_data_dir(&root);
        assert_eq!(warnings.len(), 1, "expected 1 warning, got {:?}", warnings);

        assert_eq!(warnings[0].kind, "CorruptedFile");
        assert!(warnings[0].path.contains("2026-06-15.md"));
        assert!(!warnings[0].message.is_empty());

        fs::remove_dir_all(&root).expect("cleanup");
    }

    // ---------------------------------------------------------------------------
    // 6. orphaned .tmp
    // ---------------------------------------------------------------------------

    #[test]
    fn test_orphaned_tmp_reported() {
        let root = temp_root();
        let tmp_file = root.join("2026/06/2026-06-15.md.tmp");
        write_file(&tmp_file, "leftover temp content\n");

        let warnings = scan_data_dir(&root);
        assert_eq!(warnings.len(), 1, "expected 1 warning, got {:?}", warnings);

        assert_eq!(warnings[0].kind, "OrphanedTemp");
        assert!(warnings[0].path.contains(".tmp"));
        assert!(!warnings[0].message.is_empty());

        fs::remove_dir_all(&root).expect("cleanup");
    }

    // ---------------------------------------------------------------------------
    // 7. _monthly.md skipped
    // ---------------------------------------------------------------------------

    #[test]
    fn test_monthly_file_skipped() {
        let root = temp_root();
        let monthly = root.join("2026/06/_monthly.md");
        write_file(&monthly, "garbage content that would be corrupt if scanned\n");

        let warnings = scan_data_dir(&root);
        assert!(
            warnings.is_empty(),
            "_monthly.md should be skipped, got {:?}",
            warnings
        );

        fs::remove_dir_all(&root).expect("cleanup");
    }

    // ---------------------------------------------------------------------------
    // 8. non-.md files skipped
    // ---------------------------------------------------------------------------

    #[test]
    fn test_non_md_files_skipped() {
        let root = temp_root();
        let txt = root.join("2026/06/notes.txt");
        write_file(&txt, "some text file\n");

        let warnings = scan_data_dir(&root);
        assert!(
            warnings.is_empty(),
            "non-.md files should be skipped, got {:?}",
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
        write_file(&root.join("2026/06/not-a-date.md"), "---\nok: true\n---\n");
        // corrupt frontmatter
        write_file(
            &root.join("2026/06/2026-06-15.md"),
            "no valid yaml here\n",
        );
        // orphaned tmp
        write_file(&root.join("2026/06/2026-06-16.md.tmp"), "orphan\n");

        let warnings = scan_data_dir(&root);
        assert_eq!(warnings.len(), 3, "expected 3 warnings, got {:?}", warnings);

        let kinds: Vec<&str> = warnings.iter().map(|w| w.kind.as_str()).collect();
        assert!(kinds.contains(&"SkippedFile"));
        assert!(kinds.contains(&"CorruptedFile"));
        assert!(kinds.contains(&"OrphanedTemp"));

        fs::remove_dir_all(&root).expect("cleanup");
    }
}
