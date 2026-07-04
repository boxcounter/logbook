use crate::models::Entry;
use serde::Serialize;
use std::fs;
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
    use chrono::Datelike;
    // Derive from the parsed date so a lenient input ("2026-6-5") still maps to
    // the canonical zero-padded path (consistent with files::day_path).
    let d = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}", date, e))?;
    Ok(root
        .join(".logbook")
        .join("operations")
        .join(format!("{:04}", d.year()))
        .join(format!("{:02}", d.month()))
        .join(format!("{:04}-{:02}-{:02}.jsonl", d.year(), d.month(), d.day())))
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

/// Describes a difference found during operation log verification.
#[derive(Debug)]
pub struct OpLogMismatch {
    pub date: String,
    pub description: String,
}

fn copy_month_config(root: &Path, replay_root: &Path) -> Result<(), Vec<OpLogMismatch>> {
    let year_dirs = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for year_entry in year_dirs {
        let year_entry = match year_entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !year_entry.path().is_dir() {
            continue;
        }
        let year_name = year_entry.file_name().to_string_lossy().to_string();
        if year_name.parse::<u32>().is_err() {
            continue;
        }
        let month_dirs = match std::fs::read_dir(year_entry.path()) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for month_entry in month_dirs {
            let month_entry = match month_entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !month_entry.path().is_dir() {
                continue;
            }
            let month_conf = month_entry.path().join("commitments.yaml");
            if month_conf.exists() {
                let dest_dir = replay_root
                    .join(&year_name)
                    .join(month_entry.file_name());
                std::fs::create_dir_all(&dest_dir).map_err(|e| {
                    vec![OpLogMismatch {
                        date: "".to_string(),
                        description: format!("create replay dir {}: {}", dest_dir.display(), e),
                    }]
                })?;
                std::fs::copy(&month_conf, dest_dir.join("commitments.yaml"))
                    .map_err(|e| {
                        vec![OpLogMismatch {
                            date: "".to_string(),
                            description: format!("copy commitments: {}", e),
                        }]
                    })?;
            }
        }
    }
    Ok(())
}

/// Verify that replaying the operation log produces the same data as currently on disk.
/// Returns Ok(()) if consistent, or Err(Vec<OpLogMismatch>) describing each difference.
pub fn verify_op_log(root_path: &str) -> Result<(), Vec<OpLogMismatch>> {
    let root = Path::new(root_path);
    let mut mismatches = Vec::new();

    // 1. Collect all op log entries
    let log_dir = root.join(".logbook").join("operations");
    if !log_dir.exists() {
        return Ok(());
    }

    let mut log_entries: Vec<(String, serde_json::Value)> = Vec::new();
    collect_log_entries(&log_dir, &mut log_entries).map_err(|e| {
        vec![OpLogMismatch {
            date: "".to_string(),
            description: format!("Failed to collect log entries: {}", e),
        }]
    })?;

    if log_entries.is_empty() {
        return Ok(());
    }

    // 2. Replay to temp directory
    let replay_root = std::env::temp_dir()
        .join(format!("logbook_oplog_replay_{}", uuid::Uuid::new_v4()));

    // Copy template and commitments to replay dir so validation can read them
    let template_src = root.join("dimensions.template.yaml");
    if template_src.exists() {
        fs::create_dir_all(&replay_root).map_err(|e| {
            vec![OpLogMismatch {
                date: "".to_string(),
                description: format!("create replay dir: {}", e),
            }]
        })?;
        fs::copy(&template_src, replay_root.join("dimensions.template.yaml"))
            .map_err(|e| {
                vec![OpLogMismatch {
                    date: "".to_string(),
                    description: format!("copy template: {}", e),
                }]
            })?;
    }
    // Copy month-level commitments.yaml files so dimension validation works.
    copy_month_config(root, &replay_root)?;

    for (_idx, (_ts, log_line)) in log_entries.iter().enumerate() {
        let op = log_line["op"].as_str().unwrap_or("");
        let date = log_line["date"].as_str().unwrap_or("");
        let result: Result<(), String> = match op {
            "append" => {
                let entry_id = log_line["entry_id"].as_str().unwrap_or("");
                let params = &log_line["params"];
                let duration_str = params["duration"].as_str().unwrap_or("0");
                let duration =
                    crate::commands::parse_duration(duration_str).unwrap_or_else(
                        |_| {
                            crate::error_log::log_error(
                                "oplog_replay",
                                &format!("Failed to parse duration '{}' in log entry, defaulting to 0", duration_str),
                            );
                            0
                        },
                    );
                let item = params["item"].as_str().unwrap_or("").to_string();
                let dimensions = params["dimensions"]
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| {
                                (k.clone(), v.as_str().unwrap_or("").to_string())
                            })
                            .collect::<std::collections::BTreeMap<_, _>>()
                    })
                    .unwrap_or_default();
                let entry = crate::models::Entry {
                    id: entry_id.to_string(),
                    item,
                    duration,
                    dimensions,
                    
                };
                crate::files::append_to_day_file(&replay_root, date, &entry)
                    .map(|_| ())
            }
            "update" => {
                let entry_id = log_line["entry_id"].as_str().unwrap_or("");
                let params = &log_line["params"];
                let update = crate::models::UpdateEntryInput {
                    item: params["item"].as_str().map(String::from),
                    duration: params["duration"].as_str().map(String::from),
                    dimensions: params["dimensions"].as_object().map(|obj| {
                        obj.iter()
                            .map(|(k, v)| {
                                (k.clone(), v.as_str().unwrap_or("").to_string())
                            })
                            .collect::<std::collections::BTreeMap<_, _>>()
                    }),
                };
                crate::files::update_entry_in_file(
                    &replay_root,
                    date,
                    entry_id,
                    &update,
                )
                .map(|_| ())
            }
            "delete" => {
                let entry_id = log_line["entry_id"].as_str().unwrap_or("");
                crate::files::delete_entry_from_file(
                    &replay_root,
                    date,
                    entry_id,
                )
                .map(|_| ())
            }
            "set_day_note" => {
                let note = log_line["params"].as_str().unwrap_or("");
                crate::files::set_day_note_in_file(&replay_root, date, note)
                    .map(|_| ())
            }
            _ => Ok(()),
        };
        if let Err(e) = result {
            mismatches.push(OpLogMismatch {
                date: date.to_string(),
                description: format!("Replay error at op {}: {}", op, e),
            });
        }
    }

    // 3. Compare replay dir with original root
    let original_files = collect_md_files(root)
        .map_err(|e| {
            vec![OpLogMismatch {
                date: "".to_string(),
                description: format!("Failed to collect original files: {}", e),
            }]
        })?;
    let replay_files = collect_md_files(&replay_root)
        .map_err(|e| {
            vec![OpLogMismatch {
                date: "".to_string(),
                description: format!("Failed to collect replay files: {}", e),
            }]
        })?;

    for (rel_path, _orig_content) in &original_files {
        let replay_path = replay_root.join(rel_path);
        let orig_path = root.join(rel_path);
        if !replay_path.exists() {
            mismatches.push(OpLogMismatch {
                date: rel_path.clone(),
                description: "File exists in original but not in replay".to_string(),
            });
        } else {
            let orig_content = fs::read_to_string(&orig_path).unwrap_or_else(
                |e| {
                    crate::error_log::log_error(
                        "oplog_verify",
                        &format!("Failed to read original file {}: {:?}", rel_path, e),
                    );
                    String::new()
                },
            );
            let replay_content = fs::read_to_string(&replay_path).unwrap_or_else(
                |e| {
                    crate::error_log::log_error(
                        "oplog_verify",
                        &format!("Failed to read replay file {}: {:?}", rel_path, e),
                    );
                    String::new()
                },
            );
            if orig_content.trim() != replay_content.trim() {
                mismatches.push(OpLogMismatch {
                    date: rel_path.clone(),
                    description: format!(
                        "Content mismatch: original and replay differ for {}",
                        rel_path
                    ),
                });
            }
        }
    }

    for (rel_path, _) in &replay_files {
        if !original_files.contains_key(rel_path) {
            mismatches.push(OpLogMismatch {
                date: rel_path.clone(),
                description: "File exists in replay but not in original".to_string(),
            });
        }
    }

    // Cleanup
    if let Err(e) = fs::remove_dir_all(&replay_root) {
        crate::error_log::log_error(
            "oplog_verify",
            &format!("Failed to remove replay temp dir: {:?}", e),
        );
    }

    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches)
    }
}

/// Recursively collect all JSONL log entries from the operations directory,
/// sorted by timestamp.
fn collect_log_entries(
    dir: &Path,
    entries: &mut Vec<(String, serde_json::Value)>,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in
        fs::read_dir(dir).map_err(|e| format!("read_dir: {}", e))?
    {
        let entry =
            entry.map_err(|e| format!("dir entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_log_entries(&path, entries)?;
        } else if path.extension().map_or(false, |ext| ext == "jsonl") {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("read {}: {}", path.display(), e))?;
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(val) =
                    serde_json::from_str::<serde_json::Value>(line)
                {
                    let ts = val["ts"].as_str().unwrap_or("").to_string();
                    entries.push((ts, val));
                } else {
                    crate::error_log::log_error(
                        "oplog_collect",
                        &format!("Skipping invalid JSON line in {}: {}", path.display(), &line[..line.len().min(200)]),
                    );
                }
            }
        }
    }
    // Sort by timestamp
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(())
}

/// Collect all .md files (excluding _monthly.md and .logbook/) from a root
/// directory, returning relative path -> content.
fn collect_md_files(
    root: &Path,
) -> Result<std::collections::HashMap<String, String>, String> {
    let mut files = std::collections::HashMap::new();
    if !root.exists() {
        return Ok(files);
    }
    collect_md_files_recursive(root, root, &mut files)?;
    Ok(files)
}

fn collect_md_files_recursive(
    base: &Path,
    dir: &Path,
    files: &mut std::collections::HashMap<String, String>,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in
        fs::read_dir(dir).map_err(|e| format!("read_dir: {}", e))?
    {
        let entry =
            entry.map_err(|e| format!("dir entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if dir_name == ".logbook" {
                continue;
            }
            collect_md_files_recursive(base, &path, files)?;
        } else if path.extension().map_or(false, |ext| ext == "md") {
            let rel_path = path
                .strip_prefix(base)
                .map_err(|e| format!("strip_prefix: {}", e))?
                .to_string_lossy()
                .to_string();
            let content = fs::read_to_string(&path).unwrap_or_else(|e| {
                crate::error_log::log_error(
                    "oplog_verify",
                    &format!("Failed to read original file during collection {}: {:?}", path.display(), e),
                );
                String::new()
            });
            files.insert(rel_path, content);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Entry;
    use std::collections::BTreeMap;
    use std::fs;

    fn test_root(suffix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("logbook_oplog_test_{}", suffix))
    }

    fn sample_entry() -> Entry {
        Entry {
            id: "test-id-123".to_string(),
            item: "Test entry".to_string(),
            duration: 30,
            dimensions: BTreeMap::new(),
            
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
    fn test_log_path_normalizes_unpadded_date() {
        // Lenient "2026-6-5" must map to the canonical zero-padded path.
        let p = log_path(std::path::Path::new("/data"), "2026-6-5").unwrap();
        assert_eq!(
            p,
            std::path::PathBuf::from("/data/.logbook/operations/2026/06/2026-06-05.jsonl")
        );
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
                params: serde_json::json!({"item": "Test", "duration": "30m", "dimensions": {}}),
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
                params: serde_json::json!({"item": "Updated", "duration": "60m"}),
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
                params: serde_json::json!({"item": "First", "duration": "30m", "dimensions": {}}),
            },
        )
        .unwrap();
        append(
            &root_path,
            Operation::Append {
                date: date.into(),
                entry_id: "e2".into(),
                params: serde_json::json!({"item": "Second", "duration": "45m", "dimensions": {}}),
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
            tmp.join("dimensions.template.yaml"),
            "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
        ).unwrap();

        let tmp_str = tmp.to_string_lossy().to_string();

        // Write a day file via append_new_entry (does NOT write op log)
        let entry = crate::files::append_new_entry(
            &tmp,
            "2026-06-15",
            &crate::models::CreateEntryInput {
                item: "test entry".into(),
                duration: "30m".into(),
                dimensions: std::collections::BTreeMap::new(),
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
                    "duration": "30m",
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
