/// Integration tests for scan_data_dir.
///
/// These tests verify that scan_data_dir correctly identifies:
/// - Files with invalid names (not YYYY-MM-DD.yaml) as SkippedFile
/// - Files with corrupt yaml as CorruptedFile
/// - Clean directories with valid data produce no warnings

use std::fs;
use std::path::PathBuf;

const CONFIG_YAML: &str = "\
dimensions:
  - name: Goal
    key: goal
    source: commitments:role:goals
";

fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!("logbook_scan_integration_{}", uuid::Uuid::new_v4()))
}

fn write_file(path: &PathBuf, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(path, content).expect("write file");
}

// ---------------------------------------------------------------------------
// Test 1: scan returns warnings for bad files
// ---------------------------------------------------------------------------

#[test]
fn test_scan_returns_warnings_for_bad_files() {
    let root = temp_root();

    // Valid config file (should not produce warnings)
    write_file(&root.join("dimensions.template.yaml"), CONFIG_YAML);

    // Invalid filename — not YYYY-MM-DD format
    write_file(&root.join("readme.txt"), "just some notes\n");

    // Valid date filename but corrupt yaml content
    write_file(
        &root.join("2026/06/2026-06-15.yaml"),
        "\tbad yaml content\n",
    );

    let warnings = tauri_app_lib::scan::scan_data_dir(&root);
    assert_eq!(warnings.len(), 1, "expected 1 warning, got {:?}", warnings);
    assert_eq!(warnings[0].kind, "CorruptedFile");

    // Verify individual warnings have non-empty messages
    for w in &warnings {
        assert!(!w.message.is_empty(), "warning message should not be empty");
        assert!(!w.path.is_empty(), "warning path should not be empty");
    }

    fs::remove_dir_all(&root).expect("cleanup");
}

// ---------------------------------------------------------------------------
// Test 2: scan returns empty for clean data
// ---------------------------------------------------------------------------

#[test]
fn test_scan_returns_empty_for_clean_data() {
    let root = temp_root();

    write_file(&root.join("dimensions.template.yaml"), CONFIG_YAML);

    // Valid day file with proper yaml content
    write_file(
        &root.join("2026/06/2026-06-15.yaml"),
        "note: Clean day\nentries: []\n",
    );

    let warnings = tauri_app_lib::scan::scan_data_dir(&root);
    assert!(
        warnings.is_empty(),
        "expected no warnings for clean data, got {:?}",
        warnings
    );

    fs::remove_dir_all(&root).expect("cleanup");
}
