/// Integration tests for set_commitments command.
use std::collections::HashMap;
use std::fs;

use tauri_app_lib::models::{Commitment, NewEntry};

fn setup(suffix: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("logbook_int_sc_{}", suffix));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    fs::write(
        root.join("config.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();

    let monthly_dir = root.join("2026/06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Developer\n    allocation: 40\n    goals:\n      - Feature A\n      - Code review\n  - role: VP\n    allocation: 10\n    goals:\n      - Strategy\n---\n",
    )
    .unwrap();

    root
}

fn teardown(root: &std::path::Path) {
    let _ = fs::remove_dir_all(root);
}

fn make_commitments(roles: Vec<(&str, u32, Vec<&str>)>) -> Vec<Commitment> {
    roles
        .into_iter()
        .map(|(role, alloc, goals)| Commitment {
            role: role.to_string(),
            allocation: alloc,
            goals: goals.into_iter().map(|g| g.to_string()).collect(),
        })
        .collect()
}

#[test]
fn test_set_commitments_write_and_read() {
    let root = setup("write_read");
    let new = make_commitments(vec![("Dev", 80, vec!["X", "Y"])]);

    let result =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].role, "Dev");
    assert_eq!(result[0].allocation, 80);
    assert_eq!(result[0].goals, vec!["X", "Y"]);

    // Verify file content
    let content = fs::read_to_string(root.join("2026/06/_monthly.md")).unwrap();
    assert!(content.contains("role: Dev"));
    assert!(content.contains("allocation: 80"));

    teardown(&root);
}

#[test]
fn test_set_commitments_empty_list_rejected() {
    let root = setup("empty_reject");
    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, vec![])
            .unwrap_err();
    assert!(err.contains("At least one role"));
    teardown(&root);
}

#[test]
fn test_set_commitments_empty_role_rejected() {
    let root = setup("empty_role");
    let commitments = make_commitments(vec![("", 40, vec!["A"])]);
    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, commitments)
            .unwrap_err();
    assert!(err.contains("Role name cannot be empty"));
    teardown(&root);
}

#[test]
fn test_set_commitments_zero_allocation_rejected() {
    let root = setup("zero_alloc");
    let commitments = make_commitments(vec![("Dev", 0, vec!["A"])]);
    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, commitments)
            .unwrap_err();
    assert!(err.contains("must be greater than 0"));
    teardown(&root);
}

#[test]
fn test_set_commitments_empty_goal_rejected() {
    let root = setup("empty_goal");
    let commitments = make_commitments(vec![("Dev", 40, vec![""])]);
    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, commitments)
            .unwrap_err();
    assert!(err.contains("Goal name cannot be empty"));
    teardown(&root);
}

#[test]
fn test_set_commitments_duplicate_goal_same_role_rejected() {
    let root = setup("dup_goal");
    let commitments = make_commitments(vec![("Dev", 40, vec!["A", "A"])]);
    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, commitments)
            .unwrap_err();
    assert!(err.contains("already exists"));
    teardown(&root);
}

#[test]
fn test_set_commitments_goal_rename_syncs_entries() {
    let root = setup("rename_sync");

    // Add entries with the old goal name
    let mut dims = HashMap::new();
    dims.insert("goal".to_string(), "Feature A".to_string());

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &NewEntry {
            item: "Coding".into(),
            duration: "60".into(),
            dimensions: dims.clone(),
        },
    )
    .unwrap();

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-02",
        &NewEntry {
            item: "More coding".into(),
            duration: "30".into(),
            dimensions: dims,
        },
    )
    .unwrap();

    // Rename "Feature A" → "Feature X"
    let new = make_commitments(vec![
        ("Developer", 40, vec!["Feature X", "Code review"]),
        ("VP", 10, vec!["Strategy"]),
    ]);

    let result =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap();

    assert_eq!(result[0].goals, vec!["Feature X", "Code review"]);

    // Verify entries were updated
    let day1 = tauri_app_lib::files::read_day_file(&root, "2026-06-01").unwrap();
    assert_eq!(day1.entries[0].dimensions.get("goal").unwrap(), "Feature X");

    let day2 = tauri_app_lib::files::read_day_file(&root, "2026-06-02").unwrap();
    assert_eq!(day2.entries[0].dimensions.get("goal").unwrap(), "Feature X");

    teardown(&root);
}

#[test]
fn test_set_commitments_delete_goal_rejected_when_entries_exist() {
    let root = setup("del_reject");

    let mut dims = HashMap::new();
    dims.insert("goal".to_string(), "Code review".to_string());

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &NewEntry {
            item: "Reviewing".into(),
            duration: "30".into(),
            dimensions: dims,
        },
    )
    .unwrap();

    // Try to remove "Code review" goal
    let new = make_commitments(vec![("Developer", 40, vec!["Feature A"])]);

    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap_err();

    assert!(err.contains("Cannot delete goal"));
    assert!(err.contains("Code review"));
    assert!(err.contains("used by 1 entries"));

    teardown(&root);
}

#[test]
fn test_set_commitments_delete_goal_allowed_when_no_entries() {
    let root = setup("del_allowed");

    // No entries — deleting "Code review" should succeed
    let new = make_commitments(vec![("Developer", 40, vec!["Feature A"])]);

    let result =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].goals, vec!["Feature A"]);

    teardown(&root);
}

#[test]
fn test_set_commitments_creates_new_monthly_file() {
    let root = setup("new_file");
    // Delete existing _monthly.md to simulate a month with no prior commitments
    fs::remove_file(root.join("2026/06/_monthly.md")).unwrap();

    let new = make_commitments(vec![("Dev", 20, vec!["Goal 1"])]);
    let result =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].role, "Dev");
    assert!(root.join("2026/06/_monthly.md").exists());

    teardown(&root);
}
