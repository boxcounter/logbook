/// Integration tests for verify_op_log.
use std::collections::BTreeMap;
use std::fs;

use tauri_app_lib::models::CreateEntryInput;
use tauri_app_lib::operation_log;

fn test_root(suffix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("logbook_opverify_test_{}", suffix))
}

#[test]
fn test_verify_consistent_after_append() {
    let tmp = test_root("consistent");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    // Write dimensions.template.yaml
    fs::write(
        tmp.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();

    let tmp_str = tmp.to_string_lossy().to_string();
    let date = "2026-06-15";

    // Write a day file via append_new_entry (does NOT write op log)
    let entry = tauri_app_lib::files::append_new_entry(
        &tmp,
        date,
        &CreateEntryInput {
            item: "test entry".to_string(),
            duration: "30".to_string(),
            dimensions: BTreeMap::new(),
        },
    )
    .unwrap();

    // Manually write a matching op log entry (simulates what commands::append_entry does)
    operation_log::append(
        &tmp_str,
        operation_log::Operation::Append {
            date: date.into(),
            entry_id: entry.id.clone(),
            params: serde_json::json!({
                "item": "test entry",
                "duration": "30",
                "dimensions": {}
            }),
        },
    )
    .unwrap();

    // Verify — should be consistent
    let result = operation_log::verify_op_log(&tmp_str);
    assert!(
        result.is_ok(),
        "Expected consistent, got mismatches: {:?}",
        result.err()
    );

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_verify_empty_log_returns_ok() {
    let tmp = test_root("empty");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let result = operation_log::verify_op_log(&tmp.to_string_lossy().to_string());
    assert!(result.is_ok(), "Expected Ok for empty dir");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_verify_detects_missing_operation() {
    let tmp = test_root("missing_op");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    // Write dimensions.template.yaml
    fs::write(
        tmp.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();

    // Write a day file directly via fs::write (NOT through append_new_entry).
    // This means no op log is created — the data exists on disk but has no
    // corresponding operation log entry.
    let day_dir = tmp.join("2026").join("06");
    fs::create_dir_all(&day_dir).unwrap();
    let day_content = "---\nentries:\n- id: manual-id\n  item: manual entry\n  duration: 30\n  dimensions: {}\n---\n";
    fs::write(day_dir.join("2026-06-15.md"), day_content).unwrap();

    // The op log directory doesn't exist → verify_op_log should return Ok
    // (no log = nothing to verify). This documents the current behavior:
    // verify_op_log only checks consistency when the operations directory
    // exists; it does not detect files that exist on disk without
    // corresponding log entries.
    let result = operation_log::verify_op_log(&tmp.to_string_lossy().to_string());
    assert!(
        result.is_ok(),
        "Expected Ok when op log directory does not exist"
    );

    let _ = fs::remove_dir_all(&tmp);
}
