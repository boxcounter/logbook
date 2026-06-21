use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri_app_lib::files;
use tauri_app_lib::models::{Commitment, CreateEntryInput};

fn fresh_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_template(root: &PathBuf, body: &str) {
    fs::write(root.join("template.yaml"), body).unwrap();
}

const TPL_BIZ_GOAL: &str =
    "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values: [产品, 市场]\n  - name: Goal\n    key: goal\n    source: monthly\n";

// 1. Pure read of a fresh month returns template dims, from_template=true, no file written.
#[test]
fn fresh_month_reads_template_without_writing() {
    let root = fresh_root("logbook_md_fresh_read");
    write_template(&root, TPL_BIZ_GOAL);

    let dims = files::resolve_month_dimensions(&root, 2026, 7);
    assert_eq!(dims.len(), 2);
    assert_eq!(dims[0].key, "biz");

    // resolve must NOT have created _monthly.md
    let monthly = files::monthly_path(&root, 2026, 7);
    assert!(!monthly.exists(), "resolve must not write _monthly.md");

    let _ = fs::remove_dir_all(&root);
}

// 2. First append instantiates a snapshot; later template edits don't change it.
#[test]
fn first_append_snapshots_template() {
    let root = fresh_root("logbook_md_snapshot");
    write_template(&root, TPL_BIZ_GOAL);

    let input = CreateEntryInput {
        item: "task".into(),
        duration: "30".into(),
        dimensions: HashMap::new(),
    };
    files::append_new_entry(&root, "2026-07-15", &input).unwrap();

    // _monthly.md now carries the dimensions block
    let monthly = files::read_monthly_file(&root, 2026, 7).unwrap();
    assert_eq!(monthly.dimensions.len(), 2);

    // Change the template; the month keeps its snapshot.
    write_template(
        &root,
        "dimensions:\n  - name: Other\n    key: other\n    source: static\n    values: [x]\n",
    );
    let dims = files::resolve_month_dimensions(&root, 2026, 7);
    assert_eq!(dims.len(), 2, "snapshot must not follow template changes");
    assert_eq!(dims[0].key, "biz");

    let _ = fs::remove_dir_all(&root);
}

// 3. A hand-written month block overrides the template.
#[test]
fn month_block_overrides_template() {
    let root = fresh_root("logbook_md_override");
    write_template(&root, TPL_BIZ_GOAL);
    let month_dir = root.join("2026").join("08");
    fs::create_dir_all(&month_dir).unwrap();
    fs::write(
        month_dir.join("_monthly.md"),
        "---\ndimensions:\n  - name: Client\n    key: client\n    source: static\n    values: [甲]\n---\n",
    )
    .unwrap();

    let dims = files::resolve_month_dimensions(&root, 2026, 8);
    assert_eq!(dims.len(), 1);
    assert_eq!(dims[0].key, "client");

    let _ = fs::remove_dir_all(&root);
}

// 4. ensure_month_instantiated preserves existing commitments (merge, not overwrite).
#[test]
fn instantiate_preserves_commitments() {
    let root = fresh_root("logbook_md_preserve");
    write_template(&root, TPL_BIZ_GOAL);
    let month_dir = root.join("2026").join("09");
    fs::create_dir_all(&month_dir).unwrap();
    fs::write(
        month_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it\n---\n",
    )
    .unwrap();

    files::ensure_month_instantiated(&root, 2026, 9).unwrap();

    let monthly = files::read_monthly_file(&root, 2026, 9).unwrap();
    assert_eq!(monthly.dimensions.len(), 2, "dims snapshotted");
    assert_eq!(monthly.commitments.len(), 1, "commitments preserved");
    assert_eq!(monthly.commitments[0].role, "Dev");

    let _ = fs::remove_dir_all(&root);
}

// 6. set_commitments (the command) must NOT wipe an existing dimensions block.
#[test]
fn set_commitments_preserves_dimensions_block() {
    let root = fresh_root("logbook_md_setcommit");
    write_template(&root, TPL_BIZ_GOAL);

    // Instantiate the month (dims block now present, no commitments yet).
    files::ensure_month_instantiated(&root, 2026, 10).unwrap();
    assert_eq!(files::read_monthly_file(&root, 2026, 10).unwrap().dimensions.len(), 2);

    // Set commitments via the command.
    let commitments = vec![Commitment {
        role: "Dev".into(),
        allocation: 40,
        goals: vec!["Ship it".into()],
    }];
    tauri_app_lib::commands::set_commitments(
        root.to_string_lossy().into_owned(),
        2026,
        10,
        commitments,
    )
    .unwrap();

    // Both the dims block AND the new commitments must be present.
    let monthly = files::read_monthly_file(&root, 2026, 10).unwrap();
    assert_eq!(monthly.dimensions.len(), 2, "set_commitments must preserve dims block");
    assert_eq!(monthly.commitments.len(), 1);
    assert_eq!(monthly.commitments[0].role, "Dev");

    let _ = fs::remove_dir_all(&root);
}

// 5. Missing template → resolve is lenient (empty), ensure is a no-op.
#[test]
fn missing_template_is_lenient() {
    let root = fresh_root("logbook_md_notpl");
    // no template.yaml written
    let dims = files::resolve_month_dimensions(&root, 2026, 7);
    assert!(dims.is_empty());
    files::ensure_month_instantiated(&root, 2026, 7).unwrap(); // no panic, no-op
    assert!(!files::monthly_path(&root, 2026, 7).exists());
    let _ = fs::remove_dir_all(&root);
}
