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
        "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
    )
    .unwrap();
    let result = load_root_state(&root);
    assert!(matches!(result, InitResult::Ready { .. }), "got {:?}", result);
    fs::remove_dir_all(&root).unwrap();
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
