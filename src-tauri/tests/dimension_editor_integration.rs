/// Integration tests for save_dimensions and save_dimensions_template commands.
use std::fs;

use tauri_app_lib::files;
use tauri_app_lib::models::Dimension;

fn fresh_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn teardown(root: &std::path::Path) {
    let _ = fs::remove_dir_all(root);
}

fn make_dimension(name: &str, key: &str, source: &str, values: Option<Vec<&str>>, required: bool) -> Dimension {
    Dimension {
        name: name.to_string(),
        key: key.to_string(),
        source: source.to_string(),
        values: values.map(|v| v.into_iter().map(|s| s.to_string()).collect()),
        required,
        deleted: false,
    }
}

fn biz_goal_dims() -> Vec<Dimension> {
    vec![
        make_dimension("Biz", "biz", "static", Some(vec!["产品", "市场"]), false),
        make_dimension("Goal", "goal", "monthly", None, false),
    ]
}

// ── save_dimensions ──────────────────────────────────────────────────

#[test]
fn test_save_dimensions_writes_and_reads_back() {
    let root = fresh_root("logbook_sd_write_read");
    // Need a dimensions.template.yaml so `resolve_month_dimensions` has a fallback
    // for the Biz dimension (used by entry CRUD). For save_dimensions itself,
    // only the dimensions.yaml matters.
    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:goals\n",
    )
    .unwrap();

    let dims = biz_goal_dims();
    let result = tauri_app_lib::commands::save_dimensions(
        root.to_string_lossy().into_owned(),
        2026,
        7,
        dims,
    )
    .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].key, "biz");
    assert_eq!(result[1].key, "goal");

    // Verify file was written
    let read_back = files::read_dimensions_file(&root, 2026, 7).unwrap();
    assert_eq!(read_back.len(), 2);
    assert_eq!(read_back[0].key, "biz");
    assert_eq!(read_back[1].key, "goal");

    teardown(&root);
}

#[test]
fn test_save_dimensions_creates_month_dir() {
    let root = fresh_root("logbook_sd_create_dir");
    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:goals\n",
    )
    .unwrap();

    // Month directory does not exist yet
    let month_dir = root.join("2026").join("08");
    assert!(!month_dir.exists());

    let dims = biz_goal_dims();
    tauri_app_lib::commands::save_dimensions(
        root.to_string_lossy().into_owned(),
        2026,
        8,
        dims,
    )
    .unwrap();

    // Month directory and dimensions.yaml must exist
    assert!(month_dir.exists());
    assert!(month_dir.join("dimensions.yaml").exists());

    teardown(&root);
}

#[test]
fn test_save_dimensions_root_missing_rejected() {
    let root = fresh_root("logbook_sd_root_missing");
    teardown(&root); // remove it

    let err = tauri_app_lib::commands::save_dimensions(
        root.to_string_lossy().into_owned(),
        2026,
        7,
        biz_goal_dims(),
    )
    .unwrap_err();

    assert!(err.contains("Root path does not exist"));
}

#[test]
fn test_save_dimensions_validates_before_write() {
    let root = fresh_root("logbook_sd_validate");
    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:goals\n",
    )
    .unwrap();

    // Invalid: static dimension without values
    let dims = vec![Dimension {
        name: "Bad".into(),
        key: "bad".into(),
        source: "static".into(),
        values: None,
        required: false,
        deleted: false,
    }];

    let err = tauri_app_lib::commands::save_dimensions(
        root.to_string_lossy().into_owned(),
        2026,
        7,
        dims,
    )
    .unwrap_err();

    assert!(err.contains("values is not set"), "unexpected error: {}", err);

    // Verify no file was written
    assert!(!files::dimensions_path(&root, 2026, 7).exists());

    teardown(&root);
}

#[test]
fn test_save_dimensions_invalid_key_rejected() {
    let root = fresh_root("logbook_sd_bad_key");
    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:goals\n",
    )
    .unwrap();

    let dims = vec![Dimension {
        name: "Bad Key".into(),
        key: "bad key!".into(),
        source: "static".into(),
        values: Some(vec!["x".into()]),
        required: false,
        deleted: false,
    }];

    let err = tauri_app_lib::commands::save_dimensions(
        root.to_string_lossy().into_owned(),
        2026,
        7,
        dims,
    )
    .unwrap_err();

    assert!(err.contains("invalid characters"), "unexpected error: {}", err);
    assert!(!files::dimensions_path(&root, 2026, 7).exists());

    teardown(&root);
}

#[test]
fn test_save_dimensions_roundtrip() {
    let root = fresh_root("logbook_sd_roundtrip");
    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:goals\n",
    )
    .unwrap();

    let original = biz_goal_dims();

    // Write
    tauri_app_lib::commands::save_dimensions(
        root.to_string_lossy().into_owned(),
        2026,
        9,
        original.clone(),
    )
    .unwrap();

    // Read back
    let read_back = files::read_dimensions_file(&root, 2026, 9).unwrap();
    assert_eq!(read_back.len(), original.len());
    for (a, b) in original.iter().zip(read_back.iter()) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.key, b.key);
        assert_eq!(a.source, b.source);
        assert_eq!(a.values, b.values);
        assert_eq!(a.required, b.required);
        assert_eq!(a.deleted, b.deleted);
    }

    teardown(&root);
}

#[test]
fn test_save_dimensions_overwrites_existing() {
    let root = fresh_root("logbook_sd_overwrite");
    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:goals\n",
    )
    .unwrap();

    // Write initial dimensions
    let initial = vec![make_dimension("Biz", "biz", "static", Some(vec!["产品"]), false)];
    tauri_app_lib::commands::save_dimensions(
        root.to_string_lossy().into_owned(),
        2026,
        10,
        initial,
    )
    .unwrap();

    assert_eq!(files::read_dimensions_file(&root, 2026, 10).unwrap().len(), 1);

    // Overwrite with different dimensions
    let updated = biz_goal_dims();
    tauri_app_lib::commands::save_dimensions(
        root.to_string_lossy().into_owned(),
        2026,
        10,
        updated.clone(),
    )
    .unwrap();

    let read_back = files::read_dimensions_file(&root, 2026, 10).unwrap();
    assert_eq!(read_back.len(), 2);

    teardown(&root);
}

// ── save_dimensions_template ─────────────────────────────────────────

#[test]
fn test_save_dimensions_template_writes_file() {
    let root = fresh_root("logbook_sdt_write");

    let dims = biz_goal_dims();
    tauri_app_lib::commands::save_dimensions_template(
        root.to_string_lossy().into_owned(),
        dims,
    )
    .unwrap();

    let template_path = root.join("dimensions.template.yaml");
    assert!(template_path.exists());

    // Read back and verify
    let template = files::read_dimensions_template(&root).unwrap();
    assert_eq!(template.dimensions.len(), 2);
    assert_eq!(template.dimensions[0].key, "biz");
    assert_eq!(template.dimensions[1].key, "goal");

    // Verify file content has the Template wrapper
    let content = fs::read_to_string(&template_path).unwrap();
    assert!(content.contains("dimensions:"), "file must contain top-level 'dimensions:' key, got: {}", content);

    teardown(&root);
}

#[test]
fn test_save_dimensions_template_validates() {
    let root = fresh_root("logbook_sdt_validate");

    let dims = vec![Dimension {
        name: "Bad".into(),
        key: "bad".into(),
        source: "static".into(),
        values: None, // missing values for static
        required: false,
        deleted: false,
    }];

    let err = tauri_app_lib::commands::save_dimensions_template(
        root.to_string_lossy().into_owned(),
        dims,
    )
    .unwrap_err();

    assert!(err.contains("values is not set"), "unexpected error: {}", err);
    assert!(!root.join("dimensions.template.yaml").exists());

    teardown(&root);
}

#[test]
fn test_save_dimensions_template_overwrites_existing() {
    let root = fresh_root("logbook_sdt_overwrite");

    // Write initial template
    let initial = vec![make_dimension("Biz", "biz", "static", Some(vec!["产品"]), false)];
    tauri_app_lib::commands::save_dimensions_template(
        root.to_string_lossy().into_owned(),
        initial,
    )
    .unwrap();

    let t1 = files::read_dimensions_template(&root).unwrap();
    assert_eq!(t1.dimensions.len(), 1);
    assert_eq!(t1.dimensions[0].key, "biz");

    // Overwrite with different template
    let updated = biz_goal_dims();
    tauri_app_lib::commands::save_dimensions_template(
        root.to_string_lossy().into_owned(),
        updated,
    )
    .unwrap();

    let t2 = files::read_dimensions_template(&root).unwrap();
    assert_eq!(t2.dimensions.len(), 2);

    teardown(&root);
}
