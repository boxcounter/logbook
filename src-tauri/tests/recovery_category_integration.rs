//! Integration tests for load_root_state error classification.

use std::fs;
use std::path::PathBuf;
use tauri_app_lib::commands::load_root_state;
use tauri_app_lib::models::{InitResult, RecoveryCategory};

const VALID_CONFIG: &str = "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n";

fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!("logbook_recovery_{}", uuid::Uuid::new_v4()))
}

fn category_of(result: &InitResult) -> RecoveryCategory {
    match result {
        InitResult::ConfigError { category, .. } => *category,
        other => panic!("expected ConfigError, got {:?}", other),
    }
}

#[test]
fn root_missing_when_dir_absent() {
    let root = temp_root(); // never created
    let result = load_root_state(&root);
    assert_eq!(category_of(&result), RecoveryCategory::RootMissing);
    match result {
        InitResult::ConfigError { root_path, scan_warnings, .. } => {
            assert_eq!(root_path, root.to_string_lossy());
            assert!(scan_warnings.is_empty(), "no scan on a missing root");
        }
        _ => unreachable!(),
    }
}

#[test]
fn config_missing_when_dir_present_but_no_config() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    let result = load_root_state(&root);
    assert_eq!(category_of(&result), RecoveryCategory::ConfigMissing);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn in_place_when_config_present_but_malformed() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("template.yaml"), "this: is: not: valid: yaml: : :").unwrap();
    let result = load_root_state(&root);
    assert_eq!(category_of(&result), RecoveryCategory::InPlace);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn in_place_when_config_valid_but_invalid_values() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    // parses fine, but source is not static/monthly → validate_dimensions error
    fs::write(
        root.join("template.yaml"),
        "dimensions:\n  - name: X\n    key: x\n    source: bogus\n",
    )
    .unwrap();
    let result = load_root_state(&root);
    assert_eq!(category_of(&result), RecoveryCategory::InPlace);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn ready_when_everything_valid() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("template.yaml"), VALID_CONFIG).unwrap();
    let result = load_root_state(&root);
    assert!(matches!(result, InitResult::Ready { .. }), "got {:?}", result);
    fs::remove_dir_all(&root).unwrap();
}
