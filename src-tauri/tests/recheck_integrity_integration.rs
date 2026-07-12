use tauri_app_lib::integrity;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("logbook_recheck_test_{}", uuid::Uuid::new_v4()))
    }

    fn setup_clean_fixture(root: &std::path::PathBuf) {
        fs::create_dir_all(root).unwrap();
        fs::write(
            root.join("dimensions.template.yaml"),
            "dimensions:\n  - name: Cat\n    key: cat\n    source: static\n    values: [A, B]\n    required: false\n",
        )
        .unwrap();

        use chrono::{Datelike, Local};
        let now = Local::now();
        let month_dir = root
            .join(format!("{}", now.year()))
            .join(format!("{:02}", now.month()));
        fs::create_dir_all(&month_dir).unwrap();

        let today = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());
        let entry = format!(
            "entries:\n  - id: {}\n    item: test\n    duration: 30\n",
            uuid::Uuid::new_v4()
        );
        fs::write(month_dir.join(format!("{}.yaml", today)), entry).unwrap();
    }

    fn setup_corrupt_fixture(root: &std::path::PathBuf) {
        setup_clean_fixture(root);

        use chrono::{Datelike, Local};
        let now = Local::now();
        let month_dir = root
            .join(format!("{}", now.year()))
            .join(format!("{:02}", now.month()));

        let bad_date = format!(
            "{}-{:02}-{:02}",
            now.year(),
            now.month(),
            if now.day() > 1 { now.day() - 1 } else { 1 }
        );
        let path = month_dir.join(format!("{}.yaml", bad_date));
        if !path.exists() {
            fs::write(&path, "not valid yaml: [[").unwrap();
        }
    }

    fn cleanup(root: &std::path::PathBuf) {
        integrity::reset();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recheck_on_clean_data_returns_no_issues() {
        let root = temp_root();
        setup_clean_fixture(&root);

        integrity::reset();
        let issues = integrity::check_scoped_integrity(&root);
        assert!(issues.is_empty(), "expected no issues, got {:?}", issues);

        let status = integrity::status();
        assert!(!status.compromised);

        cleanup(&root);
    }

    #[test]
    fn recheck_on_corrupt_data_reports_issues() {
        let root = temp_root();
        setup_corrupt_fixture(&root);

        integrity::reset();
        let issues = integrity::check_scoped_integrity(&root);
        assert!(!issues.is_empty(), "expected issues on corrupt data");

        cleanup(&root);
    }

    #[test]
    fn recheck_after_fix_clears_compromised_state() {
        let root = temp_root();
        setup_corrupt_fixture(&root);

        integrity::reset();
        let issues = integrity::check_scoped_integrity(&root);
        assert!(!issues.is_empty());

        for issue in &issues {
            integrity::set_compromised(issue.clone());
        }
        assert!(integrity::check().is_err(), "should be compromised after scan");

        // Simulate the recheck command: reset then rescan. With the corrupt
        // file still present, integrity should re-report issues.
        integrity::reset();
        let issues2 = integrity::check_scoped_integrity(&root);
        assert!(!issues2.is_empty(), "issues should persist while file is corrupt");

        cleanup(&root);
    }

    #[test]
    fn recheck_after_repair_allows_writes() {
        let root = temp_root();
        setup_corrupt_fixture(&root);

        integrity::reset();
        let issues = integrity::check_scoped_integrity(&root);
        assert!(!issues.is_empty());

        for issue in &issues {
            integrity::set_compromised(issue.clone());
        }
        assert!(integrity::check().is_err());

        // Fix the corrupt file
        let path = issues.iter().find(|i| i.kind == "YamlParseError");
        if let Some(issue) = path {
            let full_path = root.join(&issue.path);
            if full_path.exists() {
                fs::write(
                    &full_path,
                    format!(
                        "entries:\n  - id: {}\n    item: fixed\n    duration: 30\n",
                        uuid::Uuid::new_v4()
                    ),
                )
                .unwrap();
            }
        }

        // Recheck: reset + rescan
        integrity::reset();
        let issues2 = integrity::check_scoped_integrity(&root);
        assert!(issues2.is_empty(), "should pass after repair, got {:?}", issues2);
        assert!(integrity::check().is_ok(), "writes should be allowed after repair");

        cleanup(&root);
    }
}
