/// Integration test: append → read → update → delete entry roundtrip.
use std::collections::HashMap;
use std::fs;

use tauri_app_lib::models::{NewEntry, UpdateEntry};

fn test_root(suffix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("logbook_integration_test_{}", suffix))
}

fn setup(suffix: &str) {
    let root = test_root(suffix);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    // Write minimal config.yaml
    fs::write(
        root.join("config.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();
    // Write minimal _monthly.md for June 2026
    let monthly_dir = root.join("2026/06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it\n---\n",
    )
    .unwrap();
}

fn teardown(suffix: &str) {
    let _ = fs::remove_dir_all(test_root(suffix));
}

#[test]
fn test_append_read_update_delete_roundtrip() {
    let suffix = "roundtrip";
    setup(suffix);
    let root = test_root(suffix);
    let date = "2026-06-12";

    // Append entry
    let mut dims = HashMap::new();
    dims.insert("goal".to_string(), "Ship it".to_string());

    let new_entry = NewEntry {
        item: "Integration test entry".to_string(),
        duration: "45".to_string(),
        dimensions: dims,
    };

    let entry = tauri_app_lib::files::append_new_entry(&root, date, &new_entry);
    assert!(entry.is_ok(), "append failed: {:?}", entry.err());
    let entry = entry.unwrap();
    assert_eq!(entry.item, "Integration test entry");
    assert_eq!(entry.duration, 45);

    // Read back
    let day_file = tauri_app_lib::files::read_day_file(&root, date);
    assert!(day_file.is_ok(), "read failed: {:?}", day_file.err());
    let day_file = day_file.unwrap();
    assert_eq!(day_file.entries.len(), 1);
    assert_eq!(day_file.entries[0].id, entry.id);

    // Update entry
    let update = UpdateEntry {
        item: Some("Updated entry".to_string()),
        duration: Some("90".to_string()),
        dimensions: None,
    };
    let updated = tauri_app_lib::files::update_entry_in_file(&root, date, &entry.id, &update);
    assert!(updated.is_ok(), "update failed: {:?}", updated.err());
    let updated = updated.unwrap();
    assert_eq!(updated.entries[0].item, "Updated entry");
    assert_eq!(updated.entries[0].duration, 90);

    // Delete entry
    let deleted = tauri_app_lib::files::delete_entry_from_file(&root, date, &entry.id);
    assert!(deleted.is_ok(), "delete failed: {:?}", deleted.err());
    let deleted = deleted.unwrap();
    assert!(deleted.entries.is_empty());

    teardown(suffix);
}

#[test]
fn test_set_and_clear_day_note() {
    let suffix = "note";
    setup(suffix);
    let root = test_root(suffix);
    let date = "2026-06-12";

    let df = tauri_app_lib::files::set_day_note_in_file(&root, date, "测试笔记")
        .expect("set_day_note failed");
    assert_eq!(df.note, Some("测试笔记".to_string()));

    let df = tauri_app_lib::files::read_day_file(&root, date).unwrap();
    assert_eq!(df.note, Some("测试笔记".to_string()));

    let df = tauri_app_lib::files::set_day_note_in_file(&root, date, "")
        .expect("clear note failed");
    assert_eq!(df.note, None);

    teardown(suffix);
}

#[test]
fn test_read_nonexistent_date_returns_empty() {
    let suffix = "empty";
    setup(suffix);
    let root = test_root(suffix);
    let df = tauri_app_lib::files::read_day_file(&root, "2026-06-15").unwrap();
    assert!(df.entries.is_empty());
    assert!(df.note.is_none());
    teardown(suffix);
}

#[test]
fn test_parse_duration_via_append() {
    // Test that NewEntry with various duration formats roundtrips correctly
    let suffix = "parse_dur";
    setup(suffix);
    let root = test_root(suffix);
    let date = "2026-06-12";

    let cases = vec![
        ("1.5h", 90),
        ("30m", 30),
        ("90", 90),
        ("2h", 120),
        ("1h 30m", 90),
    ];

    for (input, expected) in &cases {
        let new_entry = NewEntry {
            item: format!("Test {}", input),
            duration: input.to_string(),
            dimensions: HashMap::new(),
        };
        let entry = tauri_app_lib::files::append_new_entry(&root, date, &new_entry)
            .expect(&format!("append failed for '{}'", input));
        assert_eq!(entry.duration, *expected, "duration mismatch for '{}'", input);
    }

    let day_file = tauri_app_lib::files::read_day_file(&root, date).unwrap();
    assert_eq!(day_file.entries.len(), cases.len());

    teardown(suffix);
}
