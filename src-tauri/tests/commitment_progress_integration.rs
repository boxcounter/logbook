/// Integration tests for get_commitment_progress command.
use std::collections::BTreeMap;
use std::fs;

use tauri_app_lib::models::CreateEntryInput;

fn setup(suffix: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("logbook_int_cp_{}", suffix));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    // Write template.yaml
    fs::write(
        root.join("template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();

    // Write _monthly.md for June 2026
    let monthly_dir = root.join("2026/06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Developer\n    allocation: 30\n    goals:\n      - Feature A\n      - Bug fixes\n  - role: VP\n    allocation: 15\n    goals:\n      - Strategy\n---\n",
    )
    .unwrap();

    root
}

fn teardown(root: &std::path::Path) {
    let _ = fs::remove_dir_all(root);
}

#[test]
fn test_progress_on_empty_month() {
    let root = setup("empty");
    let progress = tauri_app_lib::commands::get_commitment_progress(
        root.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    assert_eq!(progress.len(), 2);
    assert_eq!(progress[0].role, "Developer");
    assert_eq!(progress[0].allocation_minutes, 1800); // 30 * 60
    assert_eq!(progress[0].spent_minutes, 0);
    assert_eq!(progress[1].role, "VP");
    assert_eq!(progress[1].allocation_minutes, 900); // 15 * 60
    assert_eq!(progress[1].spent_minutes, 0);

    teardown(&root);
}

#[test]
fn test_progress_aggregates_across_multiple_days() {
    let root = setup("multi_day");

    // Add entries across multiple days
    let mut dims_a = BTreeMap::new();
    dims_a.insert("goal".to_string(), "Feature A".to_string());

    let mut dims_b = BTreeMap::new();
    dims_b.insert("goal".to_string(), "Bug fixes".to_string());

    let mut dims_s = BTreeMap::new();
    dims_s.insert("goal".to_string(), "Strategy".to_string());

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &CreateEntryInput {
            item: "Day 1 feature".into(),
            duration: "60".into(),
            dimensions: dims_a.clone(),
        },
    )
    .unwrap();

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &CreateEntryInput {
            item: "Day 1 strategy".into(),
            duration: "30".into(),
            dimensions: dims_s.clone(),
        },
    )
    .unwrap();

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-05",
        &CreateEntryInput {
            item: "Day 5 bugs".into(),
            duration: "45".into(),
            dimensions: dims_b.clone(),
        },
    )
    .unwrap();

    let progress = tauri_app_lib::commands::get_commitment_progress(
        root.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    let dev = progress.iter().find(|c| c.role == "Developer").unwrap();
    // Feature A: 60, Bug fixes: 45 = 105 total
    assert_eq!(dev.spent_minutes, 105);
    let fa = dev.goals.iter().find(|g| g.name == "Feature A").unwrap();
    assert_eq!(fa.spent_minutes, 60);
    let bf = dev.goals.iter().find(|g| g.name == "Bug fixes").unwrap();
    assert_eq!(bf.spent_minutes, 45);

    let vp = progress.iter().find(|c| c.role == "VP").unwrap();
    assert_eq!(vp.spent_minutes, 30);

    teardown(&root);
}

#[test]
fn test_progress_no_monthly_file_returns_empty() {
    let tmp = std::env::temp_dir().join("logbook_int_cp_nofile");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let progress = tauri_app_lib::commands::get_commitment_progress(
        tmp.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    assert!(progress.is_empty());

    teardown(&tmp);
}

// F4: the monthly dimension key need not be the literal "goal". validate_dimensions
// only requires source=="monthly" (key is free). If progress aggregation hardcodes
// "goal", a template using e.g. key "objective" silently reports zero spent.
#[test]
fn test_progress_with_non_goal_monthly_key() {
    let root = std::env::temp_dir().join("logbook_int_cp_objective");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    // Monthly dimension key is "objective", not "goal".
    fs::write(
        root.join("template.yaml"),
        "dimensions:\n  - name: Objective\n    key: objective\n    source: monthly\n",
    )
    .unwrap();
    let monthly_dir = root.join("2026/06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Developer\n    allocation: 30\n    goals:\n      - Feature A\n---\n",
    )
    .unwrap();

    // Entry tags its goal under the "objective" key.
    let mut dims = BTreeMap::new();
    dims.insert("objective".to_string(), "Feature A".to_string());
    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &CreateEntryInput { item: "work".into(), duration: "90".into(), dimensions: dims },
    )
    .unwrap();

    let progress = tauri_app_lib::commands::get_commitment_progress(
        root.to_string_lossy().into_owned(),
        2026,
        6,
    )
    .unwrap();

    // Must aggregate the 90 minutes, not silently report zero.
    assert_eq!(progress[0].role, "Developer");
    assert_eq!(progress[0].spent_minutes, 90, "non-'goal' monthly key was not aggregated");

    let _ = fs::remove_dir_all(&root);
}
