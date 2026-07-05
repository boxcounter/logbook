use tauri_app_lib::{integrity, models::*};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!("logbook_integrity_test_{}", uuid::Uuid::new_v4()))
    }

    fn setup_fixture(root: &PathBuf) {
        let dims = r#"dimensions:
  - name: Biz
    key: biz
    source: static
    values: [A, B]
    required: true
  - name: Goal
    key: goal
    source: commitments:goals
"#;
        fs::create_dir_all(root).unwrap();
        fs::write(root.join("dimensions.template.yaml"), dims).unwrap();

        use chrono::{Datelike, Local};
        let now = Local::now();
        let month_dir = root
            .join(format!("{}", now.year()))
            .join(format!("{:02}", now.month()));
        fs::create_dir_all(&month_dir).unwrap();

        let valid_entry = format!(
            "---\nentries:\n  - id: {}\n    item: test\n    duration: 30\n    dimensions:\n      biz: A\n---\n",
            uuid::Uuid::new_v4()
        );
        let today = format!(
            "{}-{:02}-{:02}",
            now.year(),
            now.month(),
            now.day()
        );
        fs::write(month_dir.join(format!("{}.md", today)), valid_entry).unwrap();

        let month_dims = concat!(
            "- name: Biz\n",
            "  key: biz\n",
            "  source: static\n",
            "  values: [A, B]\n",
            "  required: true\n",
            "- name: Goal\n",
            "  key: goal\n",
            "  source: commitments:goals\n",
        );
        fs::write(month_dir.join("dimensions.yaml"), month_dims).unwrap();
    }

    fn cleanup(root: &PathBuf) {
        integrity::reset();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn guard_starts_uncompromised() {
        integrity::reset();
        assert!(integrity::check().is_ok());
    }

    #[test]
    fn startup_scan_passes_on_valid_data() {
        let root = temp_root();
        setup_fixture(&root);

        let issues = integrity::check_scoped_integrity(&root);
        assert!(issues.is_empty(), "expected no issues, got {:?}", issues);

        cleanup(&root);
    }

    #[test]
    fn startup_scan_detects_corrupt_yaml() {
        let root = temp_root();
        setup_fixture(&root);

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
        fs::write(month_dir.join(format!("{}.md", bad_date)), "this is not valid yaml\n").unwrap();

        let issues = integrity::check_scoped_integrity(&root);
        assert_eq!(issues.len(), 1, "expected 1 issue, got {:?}", issues);
        assert_eq!(issues[0].kind, "YamlParseError");

        cleanup(&root);
    }

    #[test]
    fn startup_scan_detects_zero_duration() {
        let root = temp_root();
        setup_fixture(&root);

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
        let bad_entry = format!(
            "---\nentries:\n  - id: {}\n    item: bad\n    duration: 0\n---\n",
            uuid::Uuid::new_v4()
        );
        fs::write(month_dir.join(format!("{}.md", bad_date)), bad_entry).unwrap();

        let issues = integrity::check_scoped_integrity(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "InvalidDuration");

        cleanup(&root);
    }

    #[test]
    fn set_compromised_then_check_denies_write() {
        integrity::reset();

        integrity::set_compromised(IntegrityIssue {
            path: "test.md".into(),
            message: "test error".into(),
            kind: "Test".into(),
        });

        assert!(integrity::check().is_err());

        integrity::reset();
    }

    #[test]
    fn reset_after_compromised_allows_write() {
        integrity::reset();

        integrity::set_compromised(IntegrityIssue {
            path: "test.md".into(),
            message: "test".into(),
            kind: "Test".into(),
        });
        integrity::reset();

        assert!(integrity::check().is_ok());
    }
}
