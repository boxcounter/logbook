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
    source: commitments:role:goals
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
            "entries:\n  - id: {}\n    item: test\n    duration: 30\n    dimensions:\n      biz: A\n",
            uuid::Uuid::new_v4()
        );
        let today = format!(
            "{}-{:02}-{:02}",
            now.year(),
            now.month(),
            now.day()
        );
        fs::write(month_dir.join(format!("{}.yaml", today)), valid_entry).unwrap();

        let month_dims = concat!(
            "- name: Biz\n",
            "  key: biz\n",
            "  source: static\n",
            "  values: [A, B]\n",
            "  required: true\n",
            "- name: Goal\n",
            "  key: goal\n",
            "  source: commitments:role:goals\n",
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
        fs::write(month_dir.join(format!("{}.yaml", bad_date)), "this is not valid yaml\n").unwrap();

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
            "entries:\n  - id: {}\n    item: bad\n    duration: 0\n",
            uuid::Uuid::new_v4()
        );
        fs::write(month_dir.join(format!("{}.yaml", bad_date)), bad_entry).unwrap();

        let issues = integrity::check_scoped_integrity(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "InvalidDuration");

        cleanup(&root);
    }

    #[test]
    fn set_compromised_then_check_denies_write() {
        integrity::reset();

        integrity::set_compromised(IntegrityIssue {
            path: "test.yaml".into(),
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
            path: "test.yaml".into(),
            message: "test".into(),
            kind: "Test".into(),
        });
        integrity::reset();

        assert!(integrity::check().is_ok());
    }

    #[test]
    fn startup_scan_detects_invalid_uuid() {
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
        let bad_entry = "entries:\n  - id: not-a-uuid\n    item: bad\n    duration: 30\n";
        fs::write(month_dir.join(format!("{}.yaml", bad_date)), bad_entry).unwrap();

        let issues = integrity::check_scoped_integrity(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "InvalidUuid");

        cleanup(&root);
    }

    #[test]
    fn startup_scan_detects_unknown_dimension_key() {
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
            "entries:\n  - id: {}\n    item: bad\n    duration: 30\n    dimensions:\n      biz: A\n      unknown_key: X\n",
            uuid::Uuid::new_v4()
        );
        fs::write(month_dir.join(format!("{}.yaml", bad_date)), bad_entry).unwrap();

        let issues = integrity::check_scoped_integrity(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "UnknownDimensionKey");

        cleanup(&root);
    }

    #[test]
    fn startup_scan_detects_empty_required_dimension() {
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
            "entries:\n  - id: {}\n    item: bad\n    duration: 30\n    dimensions:\n      biz: '   '\n",
            uuid::Uuid::new_v4()
        );
        fs::write(month_dir.join(format!("{}.yaml", bad_date)), bad_entry).unwrap();

        let issues = integrity::check_scoped_integrity(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "EmptyRequiredDimension");

        cleanup(&root);
    }

    #[test]
    fn startup_scan_detects_jsonl_parse_error() {
        let root = temp_root();
        setup_fixture(&root);

        use chrono::{Datelike, Local};
        let now = Local::now();
        let op_root = root.join(".logbook").join("operations");
        let op_dir = op_root
            .join(format!("{}", now.year()))
            .join(format!("{:02}", now.month()));
        fs::create_dir_all(&op_dir).unwrap();

        // day that is today-1 (or day 1 if today is 1), write a valid .yaml file
        // and a corrupt .jsonl — the YAML check passes, then JSONL fails
        let bad_date = format!(
            "{}-{:02}-{:02}",
            now.year(),
            now.month(),
            if now.day() > 1 { now.day() - 1 } else { 1 }
        );
        let month_dir = root
            .join(format!("{}", now.year()))
            .join(format!("{:02}", now.month()));
        let valid_entry = format!(
            "entries:\n  - id: {}\n    item: test\n    duration: 30\n    dimensions:\n      biz: A\n",
            uuid::Uuid::new_v4()
        );
        fs::write(month_dir.join(format!("{}.yaml", bad_date)), valid_entry).unwrap();

        // Write invalid JSON (no closing brace) as a JSONL line
        fs::write(op_dir.join(format!("{}.jsonl", bad_date)), "{\"ts\":\"x\",\n").unwrap();

        let issues = integrity::check_scoped_integrity(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "JsonlParseError");

        cleanup(&root);
    }
}
