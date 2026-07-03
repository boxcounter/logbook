use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use tauri_app_lib::files;
use tauri_app_lib::models::{Commitment, CreateEntryInput, Dimension};

fn fresh_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_template(root: &PathBuf, body: &str) {
    fs::write(root.join("dimensions.template.yaml"), body).unwrap();
}

const TPL: &str =
    "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values: [产品, 市场]\n  - name: Goal\n    key: goal\n    source: commitments:goals\n  - name: Role\n    key: role\n    source: commitments:role\n";

// 1. Pure read of a fresh month returns template dims, no file written.
#[test]
fn fresh_month_reads_template_without_writing() {
    let root = fresh_root("logbook_md_fresh_read");
    write_template(&root, TPL);

    let dims = files::resolve_month_dimensions(&root, 2026, 7).unwrap();
    assert_eq!(dims.len(), 3);
    assert_eq!(dims[0].key, "biz");

    // resolve must NOT have created dimensions.yaml
    assert!(
        !files::dimensions_path(&root, 2026, 7).exists(),
        "resolve must not write dimensions.yaml"
    );

    let _ = fs::remove_dir_all(&root);
}

// 1b. A dimensions.template.yaml saved with a leading UTF-8 BOM must still parse.
#[test]
fn template_with_utf8_bom_still_parses() {
    let root = fresh_root("logbook_md_bom_template");
    write_template(&root, &format!("\u{feff}{}", TPL));

    let tpl = files::read_dimensions_template(&root).expect("read_dimensions_template must tolerate a leading BOM");
    assert_eq!(tpl.dimensions.len(), 3);
    assert_eq!(tpl.dimensions[0].key, "biz");

    let dims = files::resolve_month_dimensions(&root, 2026, 7).unwrap();
    assert_eq!(dims.len(), 3, "BOM template must not collapse to empty dims");

    let _ = fs::remove_dir_all(&root);
}

// 2. First append instantiates a snapshot; later template edits don't change it.
#[test]
fn first_append_snapshots_template() {
    let root = fresh_root("logbook_md_snapshot");
    write_template(&root, TPL);

    let input = CreateEntryInput {
        item: "task".into(),
        duration: "30m".into(),
        dimensions: BTreeMap::new(),
    };
    files::append_new_entry(&root, "2026-07-15", &input).unwrap();

    // dimensions.yaml now carries the template snapshot
    let dims = files::read_dimensions_file(&root, 2026, 7).unwrap();
    assert_eq!(dims.len(), 3, "append must snapshot template to dimensions.yaml");

    // Change the template; the month keeps its snapshot.
    write_template(&root,
        "dimensions:\n  - name: Other\n    key: other\n    source: static\n    values: [x]\n  - name: Goal\n    key: goal\n    source: commitments:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
    );
    let resolved = files::resolve_month_dimensions(&root, 2026, 7).unwrap();
    assert_eq!(resolved.len(), 3, "snapshot must not follow template changes");
    assert_eq!(resolved[0].key, "biz");

    let _ = fs::remove_dir_all(&root);
}

// 3. A hand-written month dimensions.yaml overrides the template.
#[test]
fn month_block_overrides_template() {
    let root = fresh_root("logbook_md_override");
    write_template(&root, TPL);
    let client_dim = Dimension {
        name: "Client".into(),
        key: "client".into(),
        source: "static".into(),
        values: Some(vec!["甲".into()]),
        required: false,
        deleted: false,
    };
    files::write_dimensions_file(&root, 2026, 8, &[client_dim]).unwrap();

    let dims = files::resolve_month_dimensions(&root, 2026, 8).unwrap();
    assert_eq!(dims.len(), 1);
    assert_eq!(dims[0].key, "client");

    let _ = fs::remove_dir_all(&root);
}

// 4. set_commitments must NOT wipe an existing dimensions.yaml.
#[test]
fn set_commitments_preserves_dimensions_yaml() {
    let root = fresh_root("logbook_md_setcommit");
    write_template(&root, TPL);

    // Instantiate the month (dimensions.yaml now present, no commitments yet).
    files::create_dimensions_if_missing(&root, 2026, 10).unwrap();
    assert_eq!(files::read_dimensions_file(&root, 2026, 10).unwrap().len(), 3);

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

    // Both dimensions.yaml AND commitments.yaml must be present.
    let dims = files::read_dimensions_file(&root, 2026, 10).unwrap();
    assert_eq!(dims.len(), 3, "set_commitments must preserve dimensions");
    let comms = files::read_commitments_file(&root, 2026, 10).unwrap();
    assert_eq!(comms.len(), 1);
    assert_eq!(comms[0].role, "Dev");

    let _ = fs::remove_dir_all(&root);
}

// 5. get_month_dimensions reports usingDefaultDimensions: true before instantiation, false after.
#[test]
fn get_month_dimensions_reports_usingDefaultDimensions_flag() {
    let root = fresh_root("logbook_md_fromtemplate");
    write_template(&root, TPL);
    let root_str = root.to_string_lossy().into_owned();

    // Fresh month: serves the template, flagged as not-yet-customized.
    let md = tauri_app_lib::commands::get_month_dimensions(root_str.clone(), 2026, 11).unwrap();
    assert!(md.usingDefaultDimensions, "fresh month must report usingDefaultDimensions = true");
    assert_eq!(md.dimensions.len(), 3);

    // After instantiation: own snapshot, flag flips.
    files::create_dimensions_if_missing(&root, 2026, 11).unwrap();
    let md2 = tauri_app_lib::commands::get_month_dimensions(root_str, 2026, 11).unwrap();
    assert!(!md2.usingDefaultDimensions, "instantiated month must report usingDefaultDimensions = false");
    assert_eq!(md2.dimensions.len(), 3);

    let _ = fs::remove_dir_all(&root);
}

// 6. A day note must NOT instantiate the month.
#[test]
fn set_day_note_does_not_instantiate() {
    let root = fresh_root("logbook_md_noteonly");
    write_template(&root, TPL);
    let root_str = root.to_string_lossy().into_owned();

    tauri_app_lib::commands::set_day_note(root_str.clone(), "2026-12-05".into(), "a note".into())
        .unwrap();

    // dimensions.yaml must NOT exist.
    assert!(
        !files::dimensions_path(&root, 2026, 12).exists(),
        "set_day_note must not create dimensions.yaml"
    );

    let md = tauri_app_lib::commands::get_month_dimensions(root_str, 2026, 12).unwrap();
    assert!(md.usingDefaultDimensions, "note-only month must remain usingDefaultDimensions = true");

    let _ = fs::remove_dir_all(&root);
}

// 7. Missing template → resolve is lenient (empty), create_dimensions is a no-op.
#[test]
fn missing_template_is_lenient() {
    let root = fresh_root("logbook_md_notpl");
    // no dimensions.template.yaml written
    let dims = files::resolve_month_dimensions(&root, 2026, 7).unwrap();
    assert!(dims.is_empty());
    files::create_dimensions_if_missing(&root, 2026, 7).unwrap(); // no panic, no-op
    assert!(!files::dimensions_path(&root, 2026, 7).exists());
    let _ = fs::remove_dir_all(&root);
}

// 8. A MALFORMED template must surface an error, not return empty dims.
#[test]
fn malformed_template_surfaces_error_not_empty() {
    let root = fresh_root("logbook_md_badtpl");
    write_template(&root, "dimensions: not-a-list\n");

    // resolve must surface the parse error, not return empty dims.
    let resolved = files::resolve_month_dimensions(&root, 2026, 7);
    assert!(
        resolved.is_err(),
        "malformed template must surface an error, got: {:?}",
        resolved
    );

    // Appending an entry must be rejected.
    let input = CreateEntryInput {
        item: "leak".to_string(),
        duration: "30m".to_string(),
        dimensions: BTreeMap::new(),
    };
    assert!(
        files::append_new_entry(&root, "2026-07-15", &input).is_err(),
        "append must fail when the template is unparseable, not bypass validation"
    );

    let _ = fs::remove_dir_all(&root);
}
