# Operation Log Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add append-only JSONL operation logging for all four mutation commands (append_entry, update_entry, delete_entry, set_day_note). Log before-mutation snapshots so manual recovery is possible when code bugs corrupt data.

**Architecture:** New `operation_log` module with a single public `append(root_path, op) -> Result<(), String>` function. Each mutation command reads the before-snapshot, writes the log line, then executes the mutation. Directory created lazily on first write.

**Tech Stack:** Rust, `serde_json` (already in Cargo.toml), `chrono` (already in Cargo.toml)

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src-tauri/src/operation_log.rs` | Create | `Operation` enum, `append()` function, `day_path` → log file path helper |
| `src-tauri/src/lib.rs` | Modify (+1 line) | `pub mod operation_log;` |
| `src-tauri/src/commands.rs` | Modify (~20 lines) | Add before-snapshot reads + `operation_log::append()` calls in 4 mutation commands |
| `src-tauri/tests/operation_log_integration.rs` | Create | Integration tests verifying log file written correctly after each mutation |

---

### Task 1: Create `operation_log.rs` module

**Files:**
- Create: `src-tauri/src/operation_log.rs`

- [ ] **Step 1: Write the module with `Operation` enum and `append()` function**

```rust
use crate::models::Entry;
use chrono::Local;
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
    // Validate date format before constructing path
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
        ts: Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string(),
        op: op_name.to_string(),
        date: date.clone(),
        entry_id,
        before,
        params,
    };

    let json = serde_json::to_string(&log_line).map_err(|e| format!("Serialize log: {}", e))?;

    let path = log_path(root, &date)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create log directory: {}", e))?;
    }

    // Open (or create) and append one line
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
}
```

- [ ] **Step 2: Run unit tests to verify they pass**

```bash
cd src-tauri && cargo test operation_log
```

Expected: 7 tests pass (test_log_path_structure, test_log_path_invalid_date, test_append_creates_file, test_append_update_with_before, test_append_delete_with_before, test_append_set_day_note, test_append_set_day_note_no_before, test_append_multiple_ops_same_file)

---

### Task 2: Register module in `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs:1`

- [ ] **Step 1: Add module declaration**

In `src-tauri/src/lib.rs`, after line 1 (`pub mod commands;`), insert:

```rust
pub mod operation_log;
```

The top of `lib.rs` should read:

```rust
pub mod commands;
pub mod config;
mod error_log;
pub mod files;
pub mod models;
pub mod operation_log;
mod window_state;
```

- [ ] **Step 2: Verify compilation**

```bash
cd src-tauri && cargo check
```

Expected: clean compile, no warnings.

---

### Task 3: Integrate operation log into mutation commands

**Files:**
- Modify: `src-tauri/src/commands.rs:308-379` (four mutation commands)

- [ ] **Step 1: Add `use crate::operation_log;` to commands.rs imports**

In `src-tauri/src/commands.rs`, after line 1 (`use crate::config::{validate_config, validate_monthly};`), insert:

```rust
use crate::operation_log;
```

- [ ] **Step 2: Modify `append_entry` to log before mutation**

Replace the `append_entry` function body (lines 308-328) with:

```rust
#[tauri::command]
pub fn append_entry(root_path: String, date: String, entry: NewEntry) -> Result<Entry, String> {
    error_log::log_command_enter(
        "append_entry",
        &format!("date={} item={} dur={}", date, entry.item, entry.duration),
    );
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let duration = parse_duration(&entry.duration)?;
    let config = files::read_config(root)?;
    validate_required_dimensions(&config, &entry.dimensions)?;

    let entry_id = uuid::Uuid::new_v4().to_string();

    // Log before mutation
    let params = serde_json::json!({
        "item": entry.item,
        "duration": entry.duration,
        "dimensions": entry.dimensions,
    });
    operation_log::append(
        &root_path,
        operation_log::Operation::Append {
            date: date.clone(),
            entry_id: entry_id.clone(),
            params,
        },
    )?;

    let entry = Entry {
        id: entry_id,
        item: entry.item,
        duration,
        dimensions: entry.dimensions,
    };
    let result = files::append_to_day_file(root, &date, &entry);
    let ok = result.is_ok();
    error_log::log_command_exit("append_entry", ok, &format!("id={}", entry.id));
    result
}
```

Key change: `entry_id` is generated before the log call so it can be recorded. The `params` JSON captures the original NewEntry fields.

- [ ] **Step 3: Modify `update_entry` to log before mutation**

Replace the `update_entry` function body (lines 330-358) with:

```rust
#[tauri::command]
pub fn update_entry(
    root_path: String,
    date: String,
    entry_id: String,
    update: UpdateEntry,
) -> Result<DayFile, String> {
    error_log::log_command_enter("update_entry", &format!("date={} id={}", date, entry_id));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    if let Some(ref dur_str) = update.duration {
        parse_duration(dur_str)?;
    }
    if let Some(ref dims) = update.dimensions {
        let config = files::read_config(root)?;
        validate_required_dimensions(&config, dims)?;
    }

    // Read before snapshot
    let day_file = files::read_day_file(root, &date)?;
    let before = day_file
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .cloned()
        .ok_or_else(|| format!("Entry {} not found", entry_id))?;

    // Log before mutation
    let params = serde_json::json!({
        "item": update.item,
        "duration": update.duration,
        "dimensions": update.dimensions,
    });
    operation_log::append(
        &root_path,
        operation_log::Operation::Update {
            date: date.clone(),
            entry_id: entry_id.clone(),
            before,
            params,
        },
    )?;

    let result = files::update_entry_in_file(root, &date, &entry_id, &update);
    let ok = result.is_ok();
    error_log::log_command_exit(
        "update_entry",
        ok,
        &format!(
            "{} entries",
            result.as_ref().map(|d| d.entries.len()).unwrap_or(0)
        ),
    );
    result
}
```

- [ ] **Step 4: Modify `delete_entry` to log before mutation**

Replace the `delete_entry` function body (lines 360-369) with:

```rust
#[tauri::command]
pub fn delete_entry(root_path: String, date: String, entry_id: String) -> Result<DayFile, String> {
    error_log::log_command_enter("delete_entry", &format!("date={} id={}", date, entry_id));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;

    // Read before snapshot
    let day_file = files::read_day_file(root, &date)?;
    let before = day_file
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .cloned()
        .ok_or_else(|| format!("Entry {} not found", entry_id))?;

    // Log before mutation
    operation_log::append(
        &root_path,
        operation_log::Operation::Delete {
            date: date.clone(),
            entry_id: entry_id.clone(),
            before,
        },
    )?;

    let result = files::delete_entry_from_file(root, &date, &entry_id);
    let ok = result.is_ok();
    error_log::log_command_exit("delete_entry", ok, "");
    result
}
```

- [ ] **Step 5: Modify `set_day_note` to log before mutation**

Replace the `set_day_note` function body (lines 371-380) with:

```rust
#[tauri::command]
pub fn set_day_note(root_path: String, date: String, note: String) -> Result<DayFile, String> {
    error_log::log_command_enter("set_day_note", &format!("date={}", date));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;

    // Read before snapshot
    let day_file = files::read_day_file(root, &date)?;
    let before = day_file.note.clone();

    // Log before mutation
    operation_log::append(
        &root_path,
        operation_log::Operation::SetDayNote {
            date: date.clone(),
            before,
            params: note.clone(),
        },
    )?;

    let result = files::set_day_note_in_file(root, &date, &note);
    let ok = result.is_ok();
    error_log::log_command_exit("set_day_note", ok, "");
    result
}
```

- [ ] **Step 6: Verify compilation and all existing tests still pass**

```bash
cd src-tauri && cargo check && cargo test
```

Expected: clean compile, all 44+ existing tests pass + 7 new operation_log unit tests.

---

### Task 4: Write integration tests

**Files:**
- Create: `src-tauri/tests/operation_log_integration.rs`

Note: The integration tests call `operation_log::append()` directly (not through the commands layer), because commands require a running Tauri app. The unit tests in Task 1 already cover the `append()` function internals; these integration tests verify the module works correctly when called from outside the crate (public API), including real file I/O with temp directories.

- [ ] **Step 1: Write integration test file**

```rust
/// Integration test: verify operation_log module works through its public API,
/// including real file I/O on temp directories.
use std::collections::HashMap;
use std::fs;

use tauri_app_lib::models::Entry;
use tauri_app_lib::operation_log;

fn test_root(suffix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("logbook_oplog_int_{}", suffix))
}

fn read_log_lines(root: &std::path::Path, date: &str) -> Vec<String> {
    let parts: Vec<&str> = date.split('-').collect();
    let log_file = root
        .join(".logbook")
        .join("operations")
        .join(parts[0])
        .join(parts[1])
        .join(format!("{}.jsonl", date));
    if !log_file.exists() {
        return vec![];
    }
    let content = fs::read_to_string(&log_file).unwrap();
    content.lines().map(|l| l.to_string()).collect()
}

fn sample_entry(id: &str, item: &str, duration: u32) -> Entry {
    Entry {
        id: id.to_string(),
        item: item.to_string(),
        duration,
        dimensions: HashMap::new(),
    }
}

#[test]
fn test_append_operation_writes_valid_jsonl() {
    let tmp = test_root("int_append");
    let _ = fs::remove_dir_all(&tmp);
    let root_path = tmp.to_string_lossy().to_string();
    let date = "2026-06-14";

    operation_log::append(
        &root_path,
        operation_log::Operation::Append {
            date: date.into(),
            entry_id: "e1".into(),
            params: serde_json::json!({"item": "Test", "duration": "30", "dimensions": {}}),
        },
    )
    .unwrap();

    let lines = read_log_lines(&tmp, date);
    assert_eq!(lines.len(), 1);

    let log: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(log["op"], "append");
    assert_eq!(log["entry_id"], "e1");
    assert_eq!(log["date"], date);
    assert!(log["ts"].as_str().unwrap().len() > 0);
    assert_eq!(log["params"]["item"], "Test");
    assert_eq!(log["params"]["duration"], "30");
    assert!(log.get("before").is_none());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_update_operation_writes_before_and_params() {
    let tmp = test_root("int_update");
    let _ = fs::remove_dir_all(&tmp);
    let root_path = tmp.to_string_lossy().to_string();
    let date = "2026-06-14";

    let before = sample_entry("e1", "Original", 30);
    operation_log::append(
        &root_path,
        operation_log::Operation::Update {
            date: date.into(),
            entry_id: "e1".into(),
            before,
            params: serde_json::json!({"item": "Modified", "duration": "60"}),
        },
    )
    .unwrap();

    let lines = read_log_lines(&tmp, date);
    assert_eq!(lines.len(), 1);

    let log: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(log["op"], "update");
    assert_eq!(log["before"]["item"], "Original");
    assert_eq!(log["before"]["duration"], 30);
    assert_eq!(log["params"]["item"], "Modified");
    assert_eq!(log["params"]["duration"], "60");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_delete_operation_writes_before_no_params() {
    let tmp = test_root("int_delete");
    let _ = fs::remove_dir_all(&tmp);
    let root_path = tmp.to_string_lossy().to_string();
    let date = "2026-06-14";

    let before = sample_entry("e1", "To delete", 45);
    operation_log::append(
        &root_path,
        operation_log::Operation::Delete {
            date: date.into(),
            entry_id: "e1".into(),
            before,
        },
    )
    .unwrap();

    let lines = read_log_lines(&tmp, date);
    assert_eq!(lines.len(), 1);

    let log: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(log["op"], "delete");
    assert_eq!(log["before"]["item"], "To delete");
    assert!(log.get("params").is_none());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_set_day_note_with_and_without_before() {
    let tmp = test_root("int_note");
    let _ = fs::remove_dir_all(&tmp);
    let root_path = tmp.to_string_lossy().to_string();
    let date = "2026-06-14";

    // First note (no before)
    operation_log::append(
        &root_path,
        operation_log::Operation::SetDayNote {
            date: date.into(),
            before: None,
            params: "First note".into(),
        },
    )
    .unwrap();

    // Second note (has before)
    operation_log::append(
        &root_path,
        operation_log::Operation::SetDayNote {
            date: date.into(),
            before: Some("First note".into()),
            params: "Second note".into(),
        },
    )
    .unwrap();

    let lines = read_log_lines(&tmp, date);
    assert_eq!(lines.len(), 2);

    let first: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(first["op"], "set_day_note");
    assert!(first.get("before").is_none());
    assert_eq!(first["params"], "First note");

    let second: serde_json::Value = serde_json::from_str(&lines[1]).unwrap();
    assert_eq!(second["op"], "set_day_note");
    assert_eq!(second["before"], "First note");
    assert_eq!(second["params"], "Second note");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_multiple_ops_same_file_append_only() {
    let tmp = test_root("int_multi");
    let _ = fs::remove_dir_all(&tmp);
    let root_path = tmp.to_string_lossy().to_string();
    let date = "2026-06-14";

    // Simulate a real workflow: append → update → delete
    operation_log::append(
        &root_path,
        operation_log::Operation::Append {
            date: date.into(),
            entry_id: "e1".into(),
            params: serde_json::json!({"item": "Entry", "duration": "30", "dimensions": {}}),
        },
    )
    .unwrap();
    operation_log::append(
        &root_path,
        operation_log::Operation::Update {
            date: date.into(),
            entry_id: "e1".into(),
            before: sample_entry("e1", "Entry", 30),
            params: serde_json::json!({"item": "Entry (edited)"}),
        },
    )
    .unwrap();
    operation_log::append(
        &root_path,
        operation_log::Operation::Delete {
            date: date.into(),
            entry_id: "e1".into(),
            before: sample_entry("e1", "Entry (edited)", 30),
        },
    )
    .unwrap();

    let lines = read_log_lines(&tmp, date);
    assert_eq!(lines.len(), 3, "should have 3 lines: append, update, delete");

    // Verify order and types
    let ops: Vec<String> = lines
        .iter()
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            v["op"].as_str().unwrap().to_string()
        })
        .collect();
    assert_eq!(ops, vec!["append", "update", "delete"]);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_log_does_not_exist_before_any_mutation() {
    let tmp = test_root("int_empty");
    let _ = fs::remove_dir_all(&tmp);
    // Create the root dir but perform no mutations
    fs::create_dir_all(&tmp).unwrap();

    let log_dir = tmp.join(".logbook/operations");
    assert!(!log_dir.exists(), "log dir should not exist before first mutation");

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Run integration tests**

```bash
cd src-tauri && cargo test operation_log
```

Expected: all 5 integration tests pass alongside the 7 unit tests.

---

### Task 5: Full test suite and verification

**Files:** (none new — verification only)

- [ ] **Step 1: Run full Rust test suite**

```bash
cd src-tauri && cargo test
```

Expected: all tests pass (44 existing + 7 operation_log unit + 6 integration = ~57 tests).

- [ ] **Step 2: Run cargo check for warnings**

```bash
cd src-tauri && cargo check
```

Expected: clean, no warnings.

- [ ] **Step 3: Run frontend check**

```bash
pnpm vue-tsc --noEmit && pnpm test
```

Expected: clean (no frontend changes in this feature).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/operation_log.rs src-tauri/src/lib.rs src-tauri/src/commands.rs src-tauri/tests/operation_log_integration.rs
git commit -m "feat: add append-only operation log for mutation recovery"
```
