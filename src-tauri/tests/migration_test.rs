/// Integration tests for migrate_monthly_file.
/// Uses temporary directories (std::env::temp_dir) and cleans up after each test.
use std::fs;

// Import the library crate so we can call crate::files::*.
// Integration tests in tests/ use the library crate as an external dependency.
use tauri_app_lib::files;

/// Helper: create a _monthly.md with given YAML body in temp_dir/YYYY/MM/_monthly.md.
fn setup_monthly_file(
    root: &std::path::Path,
    year: i32,
    month: u32,
    yaml_body: &str,
) {
    let dir = root
        .join(year.to_string())
        .join(format!("{:02}", month));
    fs::create_dir_all(&dir).unwrap();
    let content = format!("---\n{}---\n", yaml_body);
    fs::write(dir.join("_monthly.md"), content).unwrap();
}

/// Helper: assert a file exists and its content contains the given substring.
fn assert_file_contains(path: &std::path::Path, expected: &str) {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    assert!(
        content.contains(expected),
        "Expected {} to contain '{}', but got:\n{}",
        path.display(),
        expected,
        content
    );
}

#[test]
fn test_migrate_monthly_file_both() {
    let tmp = std::env::temp_dir().join("logbook_test_migrate_both");
    let _ = fs::remove_dir_all(&tmp);

    setup_monthly_file(
        &tmp,
        2026,
        6,
        "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values:\n      - A\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it",
    );

    let result = files::migrate_monthly_file(&tmp, 2026, 6);
    assert!(result.is_ok(), "migration failed: {:?}", result.err());
    let (had_dims, had_commits) = result.unwrap();
    assert!(had_dims, "expected had_dims=true");
    assert!(had_commits, "expected had_commits=true");

    // Assert dimensions.yaml exists with correct content
    let dims_path = files::dimensions_path(&tmp, 2026, 6);
    assert!(dims_path.exists(), "dimensions.yaml should exist");
    assert_file_contains(&dims_path, "biz");
    assert_file_contains(&dims_path, "Biz");

    // Assert commitments.yaml exists with correct content
    let cmts_path = files::commitments_path(&tmp, 2026, 6);
    assert!(cmts_path.exists(), "commitments.yaml should exist");
    assert_file_contains(&cmts_path, "Dev");
    assert_file_contains(&cmts_path, "Ship it");

    // Assert _monthly.md was renamed to .bak
    let old_path = files::monthly_path(&tmp, 2026, 6);
    assert!(!old_path.exists(), "_monthly.md should be renamed");
    let bak_path = old_path.with_extension("md.bak");
    assert!(bak_path.exists(), "_monthly.md.bak should exist");
    assert_file_contains(&bak_path, "biz");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_migrate_monthly_file_dimensions_only() {
    let tmp = std::env::temp_dir().join("logbook_test_migrate_dims");
    let _ = fs::remove_dir_all(&tmp);

    setup_monthly_file(
        &tmp,
        2026,
        7,
        "dimensions:\n  - name: Cat\n    key: cat\n    source: static\n    values:\n      - X",
    );

    let result = files::migrate_monthly_file(&tmp, 2026, 7);
    assert!(result.is_ok());
    let (had_dims, had_commits) = result.unwrap();
    assert!(had_dims, "expected had_dims=true");
    assert!(!had_commits, "expected had_commits=false");

    // Assert dimensions.yaml exists
    let dims_path = files::dimensions_path(&tmp, 2026, 7);
    assert!(dims_path.exists());
    assert_file_contains(&dims_path, "cat");

    // Assert commitments.yaml does NOT exist
    let cmts_path = files::commitments_path(&tmp, 2026, 7);
    assert!(!cmts_path.exists(), "commitments.yaml should not exist when no commitments");

    // Assert _monthly.md was renamed to .bak
    let old_path = files::monthly_path(&tmp, 2026, 7);
    assert!(!old_path.exists());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_migrate_monthly_file_commitments_only() {
    let tmp = std::env::temp_dir().join("logbook_test_migrate_cmts");
    let _ = fs::remove_dir_all(&tmp);

    setup_monthly_file(
        &tmp,
        2026,
        8,
        "commitments:\n  - role: PM\n    allocation: 20\n    goals:\n      - Planning",
    );

    let result = files::migrate_monthly_file(&tmp, 2026, 8);
    assert!(result.is_ok());
    let (had_dims, had_commits) = result.unwrap();
    assert!(!had_dims, "expected had_dims=false");
    assert!(had_commits, "expected had_commits=true");

    // Assert dimensions.yaml does NOT exist
    let dims_path = files::dimensions_path(&tmp, 2026, 8);
    assert!(!dims_path.exists());

    // Assert commitments.yaml exists
    let cmts_path = files::commitments_path(&tmp, 2026, 8);
    assert!(cmts_path.exists());
    assert_file_contains(&cmts_path, "PM");
    assert_file_contains(&cmts_path, "Planning");

    // Assert _monthly.md was renamed to .bak
    let old_path = files::monthly_path(&tmp, 2026, 8);
    assert!(!old_path.exists());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_migrate_monthly_file_no_monthly_returns_false() {
    let tmp = std::env::temp_dir().join("logbook_test_migrate_no_file");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    // No _monthly.md created — month dir doesn't even exist
    let result = files::migrate_monthly_file(&tmp, 2026, 12);
    assert!(result.is_ok());
    let (had_dims, had_commits) = result.unwrap();
    assert!(!had_dims);
    assert!(!had_commits);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_migrate_monthly_file_empty_monthly() {
    let tmp = std::env::temp_dir().join("logbook_test_migrate_empty");
    let _ = fs::remove_dir_all(&tmp);

    // _monthly.md with empty dimensions and commitments
    setup_monthly_file(&tmp, 2026, 6, "");

    let result = files::migrate_monthly_file(&tmp, 2026, 6);
    assert!(result.is_ok());
    let (had_dims, had_commits) = result.unwrap();
    assert!(!had_dims, "empty dimensions should yield false");
    assert!(!had_commits, "empty commitments should yield false");

    // _monthly.md should still be renamed even if empty
    let old_path = files::monthly_path(&tmp, 2026, 6);
    assert!(!old_path.exists());
    let bak_path = old_path.with_extension("md.bak");
    assert!(bak_path.exists());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_migrate_monthly_file_idempotent() {
    let tmp = std::env::temp_dir().join("logbook_test_migrate_idempotent");
    let _ = fs::remove_dir_all(&tmp);

    setup_monthly_file(
        &tmp,
        2026,
        6,
        "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values:\n      - A\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it",
    );

    // First migration succeeds
    let r1 = files::migrate_monthly_file(&tmp, 2026, 6);
    assert!(r1.is_ok());
    let (hd1, hc1) = r1.unwrap();
    assert!(hd1);
    assert!(hc1);

    // Second migration: _monthly.md is gone, so it should return (false, false)
    // and not overwrite the existing .yaml files
    let r2 = files::migrate_monthly_file(&tmp, 2026, 6);
    assert!(r2.is_ok());
    let (hd2, hc2) = r2.unwrap();
    assert!(!hd2, "second migration should find no _monthly.md");
    assert!(!hc2, "second migration should find no _monthly.md");

    // Existing files should still be intact
    let dims_path = files::dimensions_path(&tmp, 2026, 6);
    assert!(dims_path.exists());
    assert_file_contains(&dims_path, "Biz");

    let cmts_path = files::commitments_path(&tmp, 2026, 6);
    assert!(cmts_path.exists());
    assert_file_contains(&cmts_path, "Dev");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_migrate_monthly_file_partial_migration_dimensions_only() {
    // Simulate a scenario where dimensions.yaml already exists but commitments.yaml doesn't.
    // Migration should still write commitments.yaml (if commitments exist) and rename .bak.
    let tmp = std::env::temp_dir().join("logbook_test_migrate_partial");
    let _ = fs::remove_dir_all(&tmp);

    // Pre-create dimensions.yaml (as if from a prior incomplete migration)
    let dims_dir = tmp.join("2026").join("06");
    fs::create_dir_all(&dims_dir).unwrap();
    fs::write(
        dims_dir.join("dimensions.yaml"),
        "- name: Pre-existing\n  key: pre\n  source: static\n  values:\n    - X\n",
    )
    .unwrap();

    // Create _monthly.md with dimensions AND commitments
    setup_monthly_file(
        &tmp,
        2026,
        6,
        "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values:\n      - A\ncommitments:\n  - role: TL\n    allocation: 10\n    goals:\n      - Review",
    );

    let result = files::migrate_monthly_file(&tmp, 2026, 6);
    assert!(result.is_ok());
    let (had_dims, had_commits) = result.unwrap();
    assert!(had_dims);
    assert!(had_commits);

    // dimensions.yaml should NOT have been overwritten (idempotent)
    let dims_path = files::dimensions_path(&tmp, 2026, 6);
    assert!(dims_path.exists());
    let dims_content = fs::read_to_string(&dims_path).unwrap();
    assert!(
        dims_content.contains("Pre-existing"),
        "dimensions.yaml should not be overwritten; got:\n{}",
        dims_content
    );

    // commitments.yaml SHOULD have been written (didn't exist before)
    let cmts_path = files::commitments_path(&tmp, 2026, 6);
    assert!(cmts_path.exists());
    assert_file_contains(&cmts_path, "TL");

    // _monthly.md should be renamed to .bak
    let old_path = files::monthly_path(&tmp, 2026, 6);
    assert!(!old_path.exists());
    let bak_path = old_path.with_extension("md.bak");
    assert!(bak_path.exists());

    let _ = fs::remove_dir_all(&tmp);
}
