/// Integration tests for set_commitments command.
use std::collections::BTreeMap;
use std::fs;

use tauri_app_lib::models::{Commitment, CreateEntryInput};

fn setup(suffix: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("logbook_int_sc_{}", suffix));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();

    let monthly_dir = root.join("2026/06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("commitments.yaml"),
        "- role: Developer\n  allocation: 40\n  goals:\n    - Feature A\n    - Code review\n- role: VP\n  allocation: 10\n  goals:\n    - Strategy\n",
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
    let content = fs::read_to_string(root.join("2026/06/commitments.yaml")).unwrap();
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
    let mut dims = BTreeMap::new();
    dims.insert("goal".to_string(), "Feature A".to_string());

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &CreateEntryInput {
            item: "Coding".into(),
            duration: "60".into(),
            dimensions: dims.clone(),
        },
    )
    .unwrap();

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-02",
        &CreateEntryInput {
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

    let mut dims = BTreeMap::new();
    dims.insert("goal".to_string(), "Code review".to_string());

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &CreateEntryInput {
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

// F3: a stray non-date .md (or corrupt file) in the month dir must NOT abort the
// whole rename. Mirrors the tolerant scan in count_entries_with_goal.
#[test]
fn test_set_commitments_rename_tolerates_stray_md_file() {
    let root = setup("rename_stray");

    let mut dims = BTreeMap::new();
    dims.insert("goal".to_string(), "Feature A".to_string());
    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &CreateEntryInput { item: "Coding".into(), duration: "60".into(), dimensions: dims },
    )
    .unwrap();

    // A user-placed, non-date markdown file sits in the month directory.
    fs::write(root.join("2026/06/notes.md"), "# scratch notes\n").unwrap();

    // Rename "Feature A" → "Feature X": must succeed despite the stray file.
    let new = make_commitments(vec![
        ("Developer", 40, vec!["Feature X", "Code review"]),
        ("VP", 10, vec!["Strategy"]),
    ]);
    let result =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .expect("rename must not abort on a stray non-date .md file");
    assert_eq!(result[0].goals, vec!["Feature X", "Code review"]);

    let day1 = tauri_app_lib::files::read_day_file(&root, "2026-06-01").unwrap();
    assert_eq!(day1.entries[0].dimensions.get("goal").unwrap(), "Feature X");

    teardown(&root);
}

// F1: a corrupt valid-date day file must abort the rename BEFORE any write, so
// no day file is left half-renamed (read-all → then write-all).
#[test]
fn test_set_commitments_rename_aborts_before_write_on_corrupt_file() {
    let root = setup("rename_atomic");

    let mut dims = BTreeMap::new();
    dims.insert("goal".to_string(), "Feature A".to_string());
    for date in ["2026-06-01", "2026-06-02"] {
        tauri_app_lib::files::append_new_entry(
            &root,
            date,
            &CreateEntryInput { item: "Coding".into(), duration: "60".into(), dimensions: dims.clone() },
        )
        .unwrap();
    }

    // A corrupt file with a VALID date name (not a stray non-date file).
    fs::write(root.join("2026/06/2026-06-03.md"), "this is not valid frontmatter\n").unwrap();

    let new = make_commitments(vec![
        ("Developer", 40, vec!["Feature X", "Code review"]),
        ("VP", 10, vec!["Strategy"]),
    ]);
    let err = tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
        .expect_err("a corrupt valid-date file must abort the rename");
    assert!(err.contains("parse") || err.contains("frontmatter") || err.contains("2026-06-03"), "unexpected error: {}", err);

    // No partial rename: both good files still hold the OLD goal name.
    for date in ["2026-06-01", "2026-06-02"] {
        let df = tauri_app_lib::files::read_day_file(&root, date).unwrap();
        assert_eq!(df.entries[0].dimensions.get("goal").unwrap(), "Feature A", "{} was partially renamed", date);
    }

    teardown(&root);
}

#[test]
fn test_set_commitments_creates_new_commitments_file() {
    let root = setup("new_file");
    // Delete existing commitments.yaml to simulate a month with no prior commitments
    fs::remove_file(root.join("2026/06/commitments.yaml")).unwrap();

    let new = make_commitments(vec![("Dev", 20, vec!["Goal 1"])]);
    let result =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].role, "Dev");
    assert!(root.join("2026/06/commitments.yaml").exists());

    teardown(&root);
}
