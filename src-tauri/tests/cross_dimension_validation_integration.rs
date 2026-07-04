/// Integration test: cross-dimension validation at the Tauri command level.
///
/// Exercises `commands::append_entry` and `commands::update_entry` with
/// commitments.yaml to verify:
///   1. Rejection when goal is not declared under the submitted role.
///   2. Rejection when role is not declared in commitments at all.
///   3. Acceptance when role + goal match the commitments declaration.
use std::collections::BTreeMap;
use std::fs;

use tauri_app_lib::models::CreateEntryInput;
use tauri_app_lib::models::UpdateEntryInput;

fn test_root(suffix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("logbook_xdim_test_{}", suffix))
}

fn setup(suffix: &str) {
    let root = test_root(suffix);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    fs::write(
        root.join("dimensions.template.yaml"),
        concat!(
            "dimensions:\n",
            "  - name: Goal\n    key: goal\n    source: commitments:goals\n",
            "  - name: Role\n    key: role\n    source: commitments:role\n",
        ),
    )
    .unwrap();

    let month_dir = root.join("2026/06");
    fs::create_dir_all(&month_dir).unwrap();

    // Write commitments.yaml (current format — not _monthly.md frontmatter).
    fs::write(
        month_dir.join("commitments.yaml"),
        concat!(
            "- role: Dev\n",
            "  allocation: 40\n",
            "  goals:\n",
            "    - Ship it\n",
            "    - Code review\n",
            "- role: PM\n",
            "  allocation: 40\n",
            "  goals:\n",
            "    - Planning\n",
        ),
    )
    .unwrap();
}

fn teardown(suffix: &str) {
    let _ = fs::remove_dir_all(test_root(suffix));
}

fn make_entry(goal: &str, role: &str) -> CreateEntryInput {
    let mut dims = BTreeMap::new();
    dims.insert("goal".to_string(), goal.to_string());
    dims.insert("role".to_string(), role.to_string());
    CreateEntryInput {
        item: "Cross-dim test entry".to_string(),
        duration: "1h".to_string(),
        dimensions: dims,
    }
}

#[test]
fn append_rejects_goal_not_under_role() {
    let suffix = "append_goal_not_under_role";
    setup(suffix);
    let root = test_root(suffix);
    let root_path = root.to_string_lossy().to_string();
    let date = "2026-06-15".to_string();

    // "Bug fixes" is NOT under role "Dev" in commitments.
    let entry = make_entry("Bug fixes", "Dev");
    let result = tauri_app_lib::commands::append_entry(root_path, date, entry);
    assert!(result.is_err(), "Expected rejection but got: {:?}", result.ok()    );
    let err = result.unwrap_err();
    assert!(
        err.contains("Goal 'Bug fixes' is not declared under role 'Dev'"),
        "Wrong error message: {}",
        err
    );

    teardown(suffix);
}

#[test]
fn update_rejects_changing_goal_to_invalid_for_role() {
    let suffix = "update_goal_invalid";
    setup(suffix);
    let root = test_root(suffix);
    let root_path = root.to_string_lossy().to_string();
    let date = "2026-06-15".to_string();

    let entry = make_entry("Ship it", "Dev");
    let result = tauri_app_lib::commands::append_entry(root_path.clone(), date.clone(), entry).unwrap();

    // Update BOTH role and goal: cross-dim validation only runs on the dimensions
    // present in the update payload.
    let mut dims = BTreeMap::new();
    dims.insert("role".to_string(), "Dev".to_string());
    dims.insert("goal".to_string(), "Planning".to_string());
    let update = UpdateEntryInput {
        item: None,
        duration: None,
        dimensions: Some(dims),
    };
    let result = tauri_app_lib::commands::update_entry(root_path, date, result.id, update);
    assert!(result.is_err(), "Expected rejection but got: {:?}", result.ok());
    let err = result.unwrap_err();
    assert!(
        err.contains("Goal 'Planning' is not declared under role 'Dev'"),
        "Wrong error message: {}",
        err
    );

    teardown(suffix);
}

#[test]
fn update_accepts_changing_goal_to_valid_for_role() {
    let suffix = "update_goal_valid";
    setup(suffix);
    let root = test_root(suffix);
    let root_path = root.to_string_lossy().to_string();
    let date = "2026-06-15".to_string();

    let entry = make_entry("Ship it", "Dev");
    let result = tauri_app_lib::commands::append_entry(root_path.clone(), date.clone(), entry).unwrap();

    let mut dims = BTreeMap::new();
    dims.insert("role".to_string(), "Dev".to_string());
    dims.insert("goal".to_string(), "Code review".to_string());
    let update = UpdateEntryInput {
        item: None,
        duration: None,
        dimensions: Some(dims),
    };
    let result = tauri_app_lib::commands::update_entry(root_path, date, result.id, update);
    assert!(result.is_ok(), "Expected success but got: {:?}", result.err());

    teardown(suffix);
}

#[test]
fn update_rejects_changing_role_to_invalid() {
    let suffix = "update_role_invalid";
    setup(suffix);
    let root = test_root(suffix);
    let root_path = root.to_string_lossy().to_string();
    let date = "2026-06-15".to_string();

    let entry = make_entry("Ship it", "Dev");
    let result = tauri_app_lib::commands::append_entry(root_path.clone(), date.clone(), entry).unwrap();

    let mut dims = BTreeMap::new();
    dims.insert("role".to_string(), "Designer".to_string());
    let update = UpdateEntryInput {
        item: None,
        duration: None,
        dimensions: Some(dims),
    };
    let result = tauri_app_lib::commands::update_entry(root_path, date, result.id, update);
    assert!(result.is_err(), "Expected rejection but got: {:?}", result.ok());
    let err = result.unwrap_err();
    assert!(
        err.contains("Role 'Designer' is not declared in commitments"),
        "Wrong error message: {}",
        err
    );

    teardown(suffix);
}

#[test]
fn append_rejects_role_not_in_commitments() {
    let suffix = "append_role_unknown";
    setup(suffix);
    let root = test_root(suffix);
    let root_path = root.to_string_lossy().to_string();
    let date = "2026-06-15".to_string();

    // "Designer" is not a role in commitments.yaml.
    let entry = make_entry("Ship it", "Designer");
    let result = tauri_app_lib::commands::append_entry(root_path, date, entry);
    assert!(result.is_err(), "Expected rejection but got: {:?}", result.ok());
    let err = result.unwrap_err();
    assert!(
        err.contains("Role 'Designer' is not declared in commitments"),
        "Wrong error message: {}",
        err
    );

    teardown(suffix);
}

#[test]
fn append_accepts_valid_role_and_goal() {
    let suffix = "append_valid";
    setup(suffix);
    let root = test_root(suffix);
    let root_path = root.to_string_lossy().to_string();
    let date = "2026-06-15".to_string();

    // "Ship it" IS under role "Dev" in commitments.
    let entry = make_entry("Ship it", "Dev");
    let result = tauri_app_lib::commands::append_entry(root_path, date, entry);
    assert!(result.is_ok(), "Expected success but got: {:?}", result.err());
    let entry = result.unwrap();
    assert_eq!(entry.item, "Cross-dim test entry");
    assert_eq!(entry.duration, 60);

    teardown(suffix);
}

#[test]
fn append_accepts_when_no_commitments_file_exists() {
    let suffix = "append_no_commitments_file";
    let root = test_root(suffix);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    fs::write(
        root.join("dimensions.template.yaml"),
        concat!(
            "dimensions:\n",
            "  - name: Goal\n    key: goal\n    source: commitments:goals\n",
            "  - name: Role\n    key: role\n    source: commitments:role\n",
        ),
    )
    .unwrap();

    let month_dir = root.join("2026/06");
    fs::create_dir_all(&month_dir).unwrap();
    // No commitments.yaml — validation should be skipped.

    let root_path = root.to_string_lossy().to_string();
    let date = "2026-06-15".to_string();

    let entry = make_entry("Ship it", "Dev");
    let result = tauri_app_lib::commands::append_entry(root_path, date, entry);
    assert!(
        result.is_ok(),
        "Expected success without commitments but got: {:?}",
        result.err()
    );

    teardown(suffix);
}
