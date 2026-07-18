/// Integration tests for data version checking in load_root_state and init flow.
use std::fs;
use std::path::PathBuf;
use tauri_app_lib::commands::{check_data_version, load_root_state};
use tauri_app_lib::files;
use tauri_app_lib::models::{InitResult, CURRENT_DATA_VERSION};

fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!("logbook_data_version_{}", uuid::Uuid::new_v4()))
}

#[test]
fn load_root_state_with_no_version_file_works() {
    // load_root_state does NOT check the version — that's init's job.
    // This test confirms load_root_state is unchanged and still works
    // with valid config despite no version.txt.
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
    )
    .unwrap();
    let result = load_root_state(&root);
    assert!(matches!(result, InitResult::Ready { .. }), "got {:?}", result);
    fs::remove_dir_all(&root).unwrap();
    tauri_app_lib::integrity::reset();
}

#[test]
fn check_data_version_returns_ok_when_current() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    files::write_version_file(&root, CURRENT_DATA_VERSION).unwrap();
    let result = check_data_version(&root, CURRENT_DATA_VERSION);
    assert!(result.is_ok(), "expected ok, got {:?}", result);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn check_data_version_returns_not_found_when_missing() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    let result = check_data_version(&root, CURRENT_DATA_VERSION);
    assert!(matches!(result, Err(InitResult::DataVersionNotFound { .. })));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn check_data_version_returns_mismatch_when_wrong() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    files::write_version_file(&root, 99).unwrap();
    let result = check_data_version(&root, CURRENT_DATA_VERSION);
    assert!(matches!(result, Err(InitResult::DataVersionMismatch { .. })));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn write_version_file_and_read_back() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    files::write_version_file(&root, 3).unwrap();
    let version = files::read_version_file(&root).unwrap();
    assert_eq!(version, Some(3));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn create_starter_files_writes_version_txt_and_template() {
    // RecoveryScreen "Start fresh" → create_starter_files → reload → init.
    // Without version.txt, init's check_data_version lands on DataVersionNotFound
    // and the user is stuck on a dead-end screen.
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();

    tauri_app_lib::commands::create_starter_files(root.to_string_lossy().to_string()).unwrap();

    // version.txt must exist with the current data version.
    let version = files::read_version_file(&root).unwrap();
    assert_eq!(
        version,
        Some(CURRENT_DATA_VERSION),
        "create_starter_files must stamp version.txt"
    );

    // The starter template must have the expected content.
    let template = fs::read_to_string(root.join("dimensions.template.yaml")).unwrap();
    assert!(template.contains("key: goal"), "template content wrong: {}", template);
    assert!(template.contains("key: role"), "template content wrong: {}", template);

    // No tmp residue from the writes.
    let leftover: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(files::is_tmp_file_name)
                .unwrap_or(false)
        })
        .collect();
    assert!(leftover.is_empty(), "tmp residue: {:?}", leftover);

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn create_starter_files_does_not_overwrite_existing_version() {
    // Idempotency: a directory that already has a version.txt (e.g. user
    // re-runs Start fresh on a stamped dir) keeps its version.
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    files::write_version_file(&root, CURRENT_DATA_VERSION).unwrap();

    tauri_app_lib::commands::create_starter_files(root.to_string_lossy().to_string()).unwrap();

    assert_eq!(files::read_version_file(&root).unwrap(), Some(CURRENT_DATA_VERSION));
    fs::remove_dir_all(&root).unwrap();
}

// --- stamp_or_check_version (set_root_path version guard) ---

use tauri_app_lib::commands::stamp_or_check_version;

#[test]
fn stamp_or_check_version_stamps_brand_new_empty_dir() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();

    let result = stamp_or_check_version(&root);
    assert!(result.is_ok(), "empty dir should be stamped, got {:?}", result);
    assert_eq!(files::read_version_file(&root).unwrap(), Some(CURRENT_DATA_VERSION));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn stamp_or_check_version_stamps_dir_with_only_non_data_files() {
    // Finder-created metadata (e.g. .DS_Store) or unrelated notes must not
    // block stamping a directory that has no actual data content.
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(".DS_Store"), "junk").unwrap();
    fs::write(root.join("notes.txt"), "unrelated").unwrap();

    let result = stamp_or_check_version(&root);
    assert!(result.is_ok(), "non-data files should not block stamping, got {:?}", result);
    assert_eq!(files::read_version_file(&root).unwrap(), Some(CURRENT_DATA_VERSION));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn stamp_or_check_version_rejects_v1_dir_without_overwriting() {
    // The P0 bug: an old v1 data dir must NOT have its version.txt silently
    // rewritten to the current version.
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    files::write_version_file(&root, 1).unwrap();

    let result = stamp_or_check_version(&root);
    assert!(
        matches!(result, Err(InitResult::DataVersionMismatch { expected: CURRENT_DATA_VERSION, found: 1, .. })),
        "expected DataVersionMismatch, got {:?}",
        result
    );
    assert_eq!(
        files::read_version_file(&root).unwrap(),
        Some(1),
        "version.txt must not be overwritten"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn stamp_or_check_version_rejects_legacy_md_tree() {
    // v1 tree: no version.txt, day files as .md under YYYY/MM/.
    let root = temp_root();
    let month_dir = root.join("2024/01");
    fs::create_dir_all(&month_dir).unwrap();
    fs::write(month_dir.join("2024-01-02.md"), "---\nnote: legacy\n---\n").unwrap();

    let result = stamp_or_check_version(&root);
    assert!(
        matches!(result, Err(InitResult::DataVersionNotFound { .. })),
        "expected DataVersionNotFound, got {:?}",
        result
    );
    assert!(!files::version_path(&root).exists(), "no version.txt may be created");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn stamp_or_check_version_rejects_yaml_config_without_version() {
    // A dir that has data-relevant yaml (e.g. dimensions.template.yaml) but no
    // version.txt is not "brand new" — it must go through the version flow.
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("dimensions.template.yaml"), "dimensions: []\n").unwrap();

    let result = stamp_or_check_version(&root);
    assert!(
        matches!(result, Err(InitResult::DataVersionNotFound { .. })),
        "expected DataVersionNotFound, got {:?}",
        result
    );
    assert!(!files::version_path(&root).exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn stamp_or_check_version_accepts_current_version() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    files::write_version_file(&root, CURRENT_DATA_VERSION).unwrap();

    let result = stamp_or_check_version(&root);
    assert!(result.is_ok(), "current version should pass, got {:?}", result);
    assert_eq!(files::read_version_file(&root).unwrap(), Some(CURRENT_DATA_VERSION));
    fs::remove_dir_all(&root).unwrap();
}
