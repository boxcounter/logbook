/// Integration test: verify operation_log module works through its public API,
/// including real file I/O on temp directories.
use std::collections::BTreeMap;
use std::fs;

use tauri_app_lib::models::Entry;
use tauri_app_lib::operation_log;

fn test_root(suffix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("logbook_oplog_int_{}", suffix))
}

fn read_log_lines(root: &std::path::Path, date: &str) -> Vec<String> {
    let parts: Vec<&str> = date.split('-').collect();
    let log_file = root
        .join(".logbook")
        .join("operations")
        .join(parts[0])
        .join(parts[1])
        .join(format!("{}.jsonl", date));
    if !log_file.exists() {
        return vec![];
    }
    let content = fs::read_to_string(&log_file).unwrap();
    content.lines().map(|l| l.to_string()).collect()
}

fn sample_entry(id: &str, item: &str, duration: u32) -> Entry {
    Entry {
        id: id.to_string(),
        item: item.to_string(),
        duration,
        dimensions: BTreeMap::new(),
        attribution: tauri_app_lib::models::Attribution::default(),
    }
}

#[test]
fn test_append_operation_writes_valid_jsonl() {
    let tmp = test_root("int_append");
    let _ = fs::remove_dir_all(&tmp);
    let root_path = tmp.to_string_lossy().to_string();
    let date = "2026-06-14";

    operation_log::append(
        &root_path,
        operation_log::Operation::Append {
            date: date.into(),
            entry_id: "e1".into(),
            params: serde_json::json!({"item": "Test", "duration": "30m", "dimensions": {}}),
        },
    )
    .unwrap();

    let lines = read_log_lines(&tmp, date);
    assert_eq!(lines.len(), 1);

    let log: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(log["op"], "append");
    assert_eq!(log["entry_id"], "e1");
    assert_eq!(log["date"], date);
    assert!(log["ts"].as_str().unwrap().len() > 0);
    assert_eq!(log["params"]["item"], "Test");
    assert_eq!(log["params"]["duration"], "30m");
    assert!(log.get("before").is_none());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_update_operation_writes_before_and_params() {
    let tmp = test_root("int_update");
    let _ = fs::remove_dir_all(&tmp);
    let root_path = tmp.to_string_lossy().to_string();
    let date = "2026-06-14";

    let before = sample_entry("e1", "Original", 30);
    operation_log::append(
        &root_path,
        operation_log::Operation::Update {
            date: date.into(),
            entry_id: "e1".into(),
            before,
            params: serde_json::json!({"item": "Modified", "duration": "60m"}),
        },
    )
    .unwrap();

    let lines = read_log_lines(&tmp, date);
    assert_eq!(lines.len(), 1);

    let log: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(log["op"], "update");
    assert_eq!(log["before"]["item"], "Original");
    assert_eq!(log["before"]["duration"], 30);
    assert_eq!(log["params"]["item"], "Modified");
    assert_eq!(log["params"]["duration"], "60m");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_delete_operation_writes_before_no_params() {
    let tmp = test_root("int_delete");
    let _ = fs::remove_dir_all(&tmp);
    let root_path = tmp.to_string_lossy().to_string();
    let date = "2026-06-14";

    let before = sample_entry("e1", "To delete", 45);
    operation_log::append(
        &root_path,
        operation_log::Operation::Delete {
            date: date.into(),
            entry_id: "e1".into(),
            before,
        },
    )
    .unwrap();

    let lines = read_log_lines(&tmp, date);
    assert_eq!(lines.len(), 1);

    let log: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(log["op"], "delete");
    assert_eq!(log["before"]["item"], "To delete");
    assert!(log.get("params").is_none());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_set_day_note_with_and_without_before() {
    let tmp = test_root("int_note");
    let _ = fs::remove_dir_all(&tmp);
    let root_path = tmp.to_string_lossy().to_string();
    let date = "2026-06-14";

    // First note (no before)
    operation_log::append(
        &root_path,
        operation_log::Operation::SetDayNote {
            date: date.into(),
            before: None,
            params: "First note".into(),
        },
    )
    .unwrap();

    // Second note (has before)
    operation_log::append(
        &root_path,
        operation_log::Operation::SetDayNote {
            date: date.into(),
            before: Some("First note".into()),
            params: "Second note".into(),
        },
    )
    .unwrap();

    let lines = read_log_lines(&tmp, date);
    assert_eq!(lines.len(), 2);

    let first: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(first["op"], "set_day_note");
    assert!(first.get("before").is_none());
    assert_eq!(first["params"], "First note");

    let second: serde_json::Value = serde_json::from_str(&lines[1]).unwrap();
    assert_eq!(second["op"], "set_day_note");
    assert_eq!(second["before"], "First note");
    assert_eq!(second["params"], "Second note");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_multiple_ops_same_file_append_only() {
    let tmp = test_root("int_multi");
    let _ = fs::remove_dir_all(&tmp);
    let root_path = tmp.to_string_lossy().to_string();
    let date = "2026-06-14";

    // Simulate a real workflow: append -> update -> delete
    operation_log::append(
        &root_path,
        operation_log::Operation::Append {
            date: date.into(),
            entry_id: "e1".into(),
            params: serde_json::json!({"item": "Entry", "duration": "30m", "dimensions": {}}),
        },
    )
    .unwrap();
    operation_log::append(
        &root_path,
        operation_log::Operation::Update {
            date: date.into(),
            entry_id: "e1".into(),
            before: sample_entry("e1", "Entry", 30),
            params: serde_json::json!({"item": "Entry (edited)"}),
        },
    )
    .unwrap();
    operation_log::append(
        &root_path,
        operation_log::Operation::Delete {
            date: date.into(),
            entry_id: "e1".into(),
            before: sample_entry("e1", "Entry (edited)", 30),
        },
    )
    .unwrap();

    let lines = read_log_lines(&tmp, date);
    assert_eq!(lines.len(), 3, "should have 3 lines: append, update, delete");

    // Verify order and types
    let ops: Vec<String> = lines
        .iter()
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            v["op"].as_str().unwrap().to_string()
        })
        .collect();
    assert_eq!(ops, vec!["append", "update", "delete"]);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_log_does_not_exist_before_any_mutation() {
    let tmp = test_root("int_empty");
    let _ = fs::remove_dir_all(&tmp);
    // Create the root dir but perform no mutations
    fs::create_dir_all(&tmp).unwrap();

    let log_dir = tmp.join(".logbook/operations");
    assert!(!log_dir.exists(), "log dir should not exist before first mutation");

    let _ = fs::remove_dir_all(&tmp);
}
