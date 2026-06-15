use crate::models::Entry;
use serde::Serialize;
use std::io::Write;
use std::path::Path;

/// An operation to be logged before mutation.
pub enum Operation {
    Append {
        date: String,
        entry_id: String,
        params: serde_json::Value,
    },
    Update {
        date: String,
        entry_id: String,
        before: Entry,
        params: serde_json::Value,
    },
    Delete {
        date: String,
        entry_id: String,
        before: Entry,
    },
    SetDayNote {
        date: String,
        before: Option<String>,
        params: String,
    },
}

/// JSONL log line structure (flattened for grep-ability)
#[derive(Serialize)]
struct LogLine {
    ts: String,
    op: String,
    date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    entry_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

/// Log file path: {root_path}/.logbook/operations/{year}/{month:02}/{date}.jsonl
fn log_path(root: &Path, date: &str) -> Result<std::path::PathBuf, String> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}", date, e))?;
    let parts: Vec<&str> = date.split('-').collect();
    let year = parts[0];
    let month = parts[1];
    Ok(root
        .join(".logbook")
        .join("operations")
        .join(year)
        .join(month)
        .join(format!("{}.jsonl", date)))
}

/// Append an operation to the log file.
/// Creates directories lazily. Writes one compact JSON line.
pub fn append(root_path: &str, op: Operation) -> Result<(), String> {
    let root = Path::new(root_path);

    let (op_name, date, entry_id, before, params) = match op {
        Operation::Append {
            date,
            entry_id,
            params,
        } => ("append", date, Some(entry_id), None, Some(params)),
        Operation::Update {
            date,
            entry_id,
            before,
            params,
        } => (
            "update",
            date,
            Some(entry_id),
            Some(serde_json::to_value(&before).map_err(|e| format!("Serialize before: {}", e))?),
            Some(params),
        ),
        Operation::Delete {
            date,
            entry_id,
            before,
        } => (
            "delete",
            date,
            Some(entry_id),
            Some(serde_json::to_value(&before).map_err(|e| format!("Serialize before: {}", e))?),
            None,
        ),
        Operation::SetDayNote {
            date,
            before,
            params,
        } => (
            "set_day_note",
            date,
            None,
            before.map(|b| serde_json::Value::String(b)),
            Some(serde_json::Value::String(params)),
        ),
    };

    let log_line = LogLine {
        ts: chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%:z")
            .to_string(),
        op: op_name.to_string(),
        date: date.clone(),
        entry_id,
        before,
        params,
    };

    let json =
        serde_json::to_string(&log_line).map_err(|e| format!("Serialize log: {}", e))?;

    let path = log_path(root, &date)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create log directory: {}", e))?;
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open log file {}: {}", path.display(), e))?;

    writeln!(file, "{}", json)
        .map_err(|e| format!("Failed to write log file {}: {}", path.display(), e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Entry;
    use std::collections::HashMap;
    use std::fs;

    fn test_root(suffix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("logbook_oplog_test_{}", suffix))
    }

    fn sample_entry() -> Entry {
        Entry {
            id: "test-id-123".to_string(),
            item: "Test entry".to_string(),
            duration: 30,
            dimensions: HashMap::new(),
        }
    }

    #[test]
    fn test_log_path_structure() {
        let root = std::path::Path::new("/data");
        let p = log_path(root, "2026-06-14").unwrap();
        assert_eq!(
            p,
            std::path::PathBuf::from("/data/.logbook/operations/2026/06/2026-06-14.jsonl")
        );
    }

    #[test]
    fn test_log_path_invalid_date() {
        assert!(log_path(std::path::Path::new("/data"), "bad-date").is_err());
    }

    #[test]
    fn test_append_creates_file() {
        let tmp = test_root("append_creates");
        let _ = fs::remove_dir_all(&tmp);

        let root_path = tmp.to_string_lossy().to_string();
        append(
            &root_path,
            Operation::Append {
                date: "2026-06-14".into(),
                entry_id: "e1".into(),
                params: serde_json::json!({"item": "Test", "duration": "30", "dimensions": {}}),
            },
        )
        .unwrap();

        let log_file = tmp.join(".logbook/operations/2026/06/2026-06-14.jsonl");
        assert!(log_file.exists());

        let content = fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("\"op\":\"append\""));
        assert!(content.contains("\"entry_id\":\"e1\""));
        assert!(content.contains("\"date\":\"2026-06-14\""));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_append_update_with_before() {
        let tmp = test_root("append_update");
        let _ = fs::remove_dir_all(&tmp);

        let root_path = tmp.to_string_lossy().to_string();
        let before = sample_entry();
        append(
            &root_path,
            Operation::Update {
                date: "2026-06-14".into(),
                entry_id: before.id.clone(),
                before: before.clone(),
                params: serde_json::json!({"item": "Updated", "duration": "60"}),
            },
        )
        .unwrap();

        let content = fs::read_to_string(
            tmp.join(".logbook/operations/2026/06/2026-06-14.jsonl"),
        )
        .unwrap();
        assert!(content.contains("\"op\":\"update\""));
        assert!(content.contains("\"before\":{"));
        assert!(content.contains("\"item\":\"Test entry\""));
        assert!(content.contains("\"duration\":30"));
        assert!(content.contains("\"params\":{"));
        assert!(content.contains("\"item\":\"Updated\""));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_append_delete_with_before() {
        let tmp = test_root("append_delete");
        let _ = fs::remove_dir_all(&tmp);

        let root_path = tmp.to_string_lossy().to_string();
        let before = sample_entry();
        append(
            &root_path,
            Operation::Delete {
                date: "2026-06-14".into(),
                entry_id: before.id.clone(),
                before,
            },
        )
        .unwrap();

        let content = fs::read_to_string(
            tmp.join(".logbook/operations/2026/06/2026-06-14.jsonl"),
        )
        .unwrap();
        assert!(content.contains("\"op\":\"delete\""));
        // delete has before but no params
        assert!(content.contains("\"before\":{"));
        assert!(!content.contains("\"params\""));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_append_set_day_note() {
        let tmp = test_root("append_note");
        let _ = fs::remove_dir_all(&tmp);

        let root_path = tmp.to_string_lossy().to_string();
        append(
            &root_path,
            Operation::SetDayNote {
                date: "2026-06-14".into(),
                before: Some("旧笔记".into()),
                params: "新笔记".into(),
            },
        )
        .unwrap();

        let content = fs::read_to_string(
            tmp.join(".logbook/operations/2026/06/2026-06-14.jsonl"),
        )
        .unwrap();
        assert!(content.contains("\"op\":\"set_day_note\""));
        assert!(content.contains("\"before\":\"旧笔记\""));
        assert!(content.contains("\"params\":\"新笔记\""));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_append_set_day_note_no_before() {
        let tmp = test_root("append_note_none");
        let _ = fs::remove_dir_all(&tmp);

        let root_path = tmp.to_string_lossy().to_string();
        // before is None (first time setting note)
        append(
            &root_path,
            Operation::SetDayNote {
                date: "2026-06-14".into(),
                before: None,
                params: "新笔记".into(),
            },
        )
        .unwrap();

        let content = fs::read_to_string(
            tmp.join(".logbook/operations/2026/06/2026-06-14.jsonl"),
        )
        .unwrap();
        // No "before" field when None (skip_serializing_if)
        assert!(!content.contains("\"before\""));
        assert!(content.contains("\"params\":\"新笔记\""));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_append_multiple_ops_same_file() {
        let tmp = test_root("append_multi");
        let _ = fs::remove_dir_all(&tmp);

        let root_path = tmp.to_string_lossy().to_string();
        let date = "2026-06-14";

        append(
            &root_path,
            Operation::Append {
                date: date.into(),
                entry_id: "e1".into(),
                params: serde_json::json!({"item": "First", "duration": "30", "dimensions": {}}),
            },
        )
        .unwrap();
        append(
            &root_path,
            Operation::Append {
                date: date.into(),
                entry_id: "e2".into(),
                params: serde_json::json!({"item": "Second", "duration": "45", "dimensions": {}}),
            },
        )
        .unwrap();

        let content = fs::read_to_string(
            tmp.join(".logbook/operations/2026/06/2026-06-14.jsonl"),
        )
        .unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2, "should have exactly 2 lines");
        assert!(lines[0].contains("\"item\":\"First\""));
        assert!(lines[1].contains("\"item\":\"Second\""));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_verify_op_log_empty_dir_returns_ok() {
        let tmp = std::env::temp_dir().join(format!("logbook_verify_empty_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let result = verify_op_log(&tmp.to_string_lossy().to_string());
        assert!(result.is_ok(), "Expected Ok for empty dir");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_verify_op_log_consistent_after_append() {
        let tmp = std::env::temp_dir().join(format!("logbook_verify_consistent_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // Set up config
        std::fs::write(
            tmp.join("config.yaml"),
            "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
        ).unwrap();

        let tmp_str = tmp.to_string_lossy().to_string();

        // Write a day file via append_new_entry (does NOT write op log)
        let entry = crate::files::append_new_entry(
            &tmp,
            "2026-06-15",
            &crate::models::NewEntry {
                item: "test entry".into(),
                duration: "30".into(),
                dimensions: std::collections::HashMap::new(),
            },
        ).unwrap();

        // Manually write an op log entry (simulates what commands::append_entry does)
        crate::operation_log::append(
            &tmp_str,
            crate::operation_log::Operation::Append {
                date: "2026-06-15".into(),
                entry_id: entry.id.clone(),
                params: serde_json::json!({
                    "item": "test entry",
                    "duration": "30",
                    "dimensions": {}
                }),
            },
        ).unwrap();

        // Verify — should be consistent
        let result = verify_op_log(&tmp_str);
        assert!(
            result.is_ok(),
            "Expected consistent, got mismatches: {:?}",
            result.err()
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
