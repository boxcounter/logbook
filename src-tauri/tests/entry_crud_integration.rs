/// Integration test: append → read → update → delete entry roundtrip.
use std::collections::BTreeMap;
use std::fs;

use tauri_app_lib::models::{CreateEntryInput, UpdateEntryInput};

fn test_root(suffix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("logbook_integration_test_{}", suffix))
}

fn setup(suffix: &str) {
    let root = test_root(suffix);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    // Write minimal dimensions.template.yaml
    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
    )
    .unwrap();
    // Write commitments.yaml and dimensions.yaml for June 2026
    let monthly_dir = root.join("2026/06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("commitments.yaml"),
        "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it\n",
    )
    .unwrap();
    fs::write(
        monthly_dir.join("dimensions.yaml"),
        "[]\n",
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
    let mut dims = BTreeMap::new();
    dims.insert("goal".to_string(), "Ship it".to_string());

    let new_entry = CreateEntryInput {
        item: "Integration test entry".to_string(),
        duration: "45m".to_string(),
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
    let update = UpdateEntryInput {
        item: Some("Updated entry".to_string()),
        duration: Some("90m".to_string()),
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

    let df =
        tauri_app_lib::files::set_day_note_in_file(&root, date, "").expect("clear note failed");
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
    // Test that CreateEntryInput with various duration formats roundtrips correctly
    let suffix = "parse_dur";
    setup(suffix);
    let root = test_root(suffix);
    let date = "2026-06-12";

    let cases = vec![
        ("1.5h", 90),
        ("30m", 30),
        ("90m", 90),
        ("2h", 120),
        ("1h 30m", 90),
    ];

    for (input, expected) in &cases {
        let new_entry = CreateEntryInput {
            item: format!("Test {}", input),
            duration: input.to_string(),
            dimensions: BTreeMap::new(),
        };
        let entry = tauri_app_lib::files::append_new_entry(&root, date, &new_entry)
            .expect(&format!("append failed for '{}'", input));
        assert_eq!(
            entry.duration, *expected,
            "duration mismatch for '{}'",
            input
        );
    }

    let day_file = tauri_app_lib::files::read_day_file(&root, date).unwrap();
    assert_eq!(day_file.entries.len(), cases.len());

    teardown(suffix);
}

// --- Required dimension validation tests ---

#[test]
fn test_append_entry_rejects_missing_required_dimension() {
    let suffix = "req_missing";
    setup(suffix);
    let root = test_root(suffix);

    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values: [A, B]\n    required: true\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
    )
    .unwrap();

    let date = "2026-06-12";
    let new_entry = CreateEntryInput {
        item: "Missing required dim".to_string(),
        duration: "30m".to_string(),
        dimensions: BTreeMap::new(), // biz is required but missing
    };

    let result = tauri_app_lib::files::append_new_entry(&root, date, &new_entry);
    assert!(
        result.is_err(),
        "should reject entry with missing required dimension"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("Missing required dimension"),
        "error should mention missing required dimension, got: {}",
        err
    );

    teardown(suffix);
}

#[test]
fn test_append_entry_accepts_when_required_dimensions_present() {
    let suffix = "req_ok";
    setup(suffix);
    let root = test_root(suffix);

    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values: [A, B]\n    required: true\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
    )
    .unwrap();

    let date = "2026-06-12";
    let mut dims = BTreeMap::new();
    dims.insert("biz".to_string(), "A".to_string());

    let new_entry = CreateEntryInput {
        item: "Has required dim".to_string(),
        duration: "30m".to_string(),
        dimensions: dims,
    };

    let result = tauri_app_lib::files::append_new_entry(&root, date, &new_entry);
    assert!(
        result.is_ok(),
        "should accept entry with required dimensions present"
    );

    // Verify it was written
    let day_file = tauri_app_lib::files::read_day_file(&root, date).unwrap();
    assert_eq!(day_file.entries.len(), 1);
    assert_eq!(day_file.entries[0].dimensions.get("biz").unwrap(), "A");

    teardown(suffix);
}

#[test]
fn test_update_entry_rejects_clearing_required_dimension() {
    let suffix = "req_update";
    setup(suffix);
    let root = test_root(suffix);

    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values: [A, B]\n    required: true\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
    )
    .unwrap();

    let date = "2026-06-12";
    let mut dims = BTreeMap::new();
    dims.insert("biz".to_string(), "A".to_string());

    let entry = tauri_app_lib::files::append_new_entry(
        &root,
        date,
        &CreateEntryInput {
            item: "Original".into(),
            duration: "30m".into(),
            dimensions: dims,
        },
    )
    .unwrap();

    // Try to update with empty dimensions (clearing required dim)
    let update = UpdateEntryInput {
        item: None,
        duration: None,
        dimensions: Some(BTreeMap::new()), // clears biz
    };

    let result = tauri_app_lib::files::update_entry_in_file(&root, date, &entry.id, &update);
    assert!(
        result.is_err(),
        "should reject update that clears required dimension"
    );

    // Verify original entry unchanged
    let day_file = tauri_app_lib::files::read_day_file(&root, date).unwrap();
    assert_eq!(day_file.entries[0].dimensions.get("biz").unwrap(), "A");

    teardown(suffix);
}
