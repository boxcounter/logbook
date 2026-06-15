# Commitment Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add inline editing capability to CommitmentsPanel — role CRUD, allocation editing, goal CRUD with rename sync and delete protection.

**Architecture:** New Rust `set_commitments` command validates, diffs old vs new commitments, syncs goal renames to entries, and atomically writes `_monthly.md`. CommitmentsPanel.vue gains display/edit dual-mode with local state, frontend pre-validation, and save/cancel semantics. MonthView passes `commitments` and `rootPath` as props, listens for `saved` event to refresh progress.

**Tech Stack:** Rust (Tauri 2.x command + `yaml_serde` + atomic file I/O), Vue 3 SFC + Composition API, Vitest + vue-test-utils (frontend), Rust `#[cfg(test)]` + integration tests (backend).

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src-tauri/src/commands.rs` | Modify | Add `set_commitments` command + helper functions |
| `src-tauri/src/files.rs` | Modify | Add `write_monthly_file` function |
| `src-tauri/src/lib.rs` | Modify | Register `set_commitments` in `invoke_handler` |
| `src-tauri/tests/commitment_editor_integration.rs` | Create | Integration tests for `set_commitments` |
| `src/components/CommitmentsPanel.vue` | Modify | Add edit mode with dual display/edit states |
| `src/components/MonthView.vue` | Modify | Pass `commitments`/`rootPath` props, handle `saved` event |
| `src/__tests__/components/CommitmentsPanel.test.ts` | Modify | Add edit mode tests |
| `src/__tests__/mocks/tauri.ts` | Modify | Add `set_commitments` mock |
| `src/__tests__/mocks/fixtures.ts` | Modify | Add `makeCommitment` export (already exists) |

---

### Task 1: Add `write_monthly_file` to `files.rs`

**Files:**
- Modify: `src-tauri/src/files.rs`

- [ ] **Step 1: Add `write_monthly_file` function**

Add after `read_monthly_file` (after line 183):

```rust
/// Write a monthly file (atomic: temp then rename).
pub fn write_monthly_file(
    root: &Path,
    year: i32,
    month: u32,
    monthly: &MonthlyFile,
) -> Result<(), String> {
    let path = monthly_path(root, year, month);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let yaml_body = yaml_serde::to_string(monthly)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    let content = format!("---\n{}---\n", yaml_body);
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &content)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}
```

- [ ] **Step 2: Run existing tests to verify no regression**

Run: `cd src-tauri && cargo test`
Expected: All existing tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/files.rs
git commit -m "feat: add write_monthly_file for atomic _monthly.md writes"
```

---

### Task 2: Add `set_commitments` command with validation (unit tests first)

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Write unit tests for validation logic**

Add inside the existing `#[cfg(test)] mod tests` block in `commands.rs`, after the last test:

```rust
// --- set_commitments validation tests ---

use crate::models::Commitment;

fn make_commitments(roles: Vec<(&str, u32, Vec<&str>)>) -> Vec<Commitment> {
    roles
        .into_iter()
        .map(|(role, alloc, goals)| Commitment {
            role: role.to_string(),
            allocation: alloc,
            goals: goals.into_iter().map(|g| g.to_string()).collect(),
        })
        .collect()
}

#[test]
fn test_validate_commitments_empty_list() {
    let result = validate_commitments(&[]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("At least one role"));
}

#[test]
fn test_validate_commitments_empty_role() {
    let c = make_commitments(vec![("", 40, vec!["Goal A"])]);
    let result = validate_commitments(&c);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Role name cannot be empty"));
}

#[test]
fn test_validate_commitments_whitespace_role() {
    let c = make_commitments(vec![("   ", 40, vec!["Goal A"])]);
    let result = validate_commitments(&c);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Role name cannot be empty"));
}

#[test]
fn test_validate_commitments_zero_allocation() {
    let c = make_commitments(vec![("Dev", 0, vec!["Goal A"])]);
    let result = validate_commitments(&c);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Allocation for 'Dev'"));
    assert!(result.unwrap_err().contains("must be greater than 0"));
}

#[test]
fn test_validate_commitments_empty_goal() {
    let c = make_commitments(vec![("Dev", 40, vec![""])]);
    let result = validate_commitments(&c);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Goal name cannot be empty"));
}

#[test]
fn test_validate_commitments_duplicate_goal_same_role() {
    let c = make_commitments(vec![("Dev", 40, vec!["Ship it", "Ship it"])]);
    let result = validate_commitments(&c);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
    assert!(result.unwrap_err().contains("Dev"));
}

#[test]
fn test_validate_commitments_valid() {
    let c = make_commitments(vec![
        ("Dev", 80, vec!["Ship it", "Review"]),
        ("TL", 40, vec!["1:1", "Architecture"]),
    ]);
    assert!(validate_commitments(&c).is_ok());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test validate_commitments`
Expected: 7 tests FAIL — `validate_commitments` function not found.

- [ ] **Step 3: Write the `validate_commitments` function**

Add above the `#[tauri::command]` for the new command (before or near other validation helpers), e.g., after `validate_date_format` on line 570:

```rust
/// Validate commitments before saving (no IO).
fn validate_commitments(commitments: &[Commitment]) -> Result<(), String> {
    if commitments.is_empty() {
        return Err("At least one role is required".to_string());
    }
    for c in commitments {
        if c.role.trim().is_empty() {
            return Err("Role name cannot be empty".to_string());
        }
        if c.allocation == 0 {
            return Err(format!(
                "Allocation for '{}' must be greater than 0",
                c.role
            ));
        }
        let mut goal_set = std::collections::HashSet::new();
        for g in &c.goals {
            if g.trim().is_empty() {
                return Err("Goal name cannot be empty".to_string());
            }
            if !goal_set.insert(g) {
                return Err(format!("Goal '{}' already exists in '{}'", g, c.role));
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test validate_commitments`
Expected: 7 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add validate_commitments with unit tests"
```

---

### Task 3: Add goal rename detection (unit tests first)

**Files:**
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Write unit tests for `detect_goal_changes`**

Add inside `#[cfg(test)] mod tests`:

```rust
// --- detect_goal_changes tests ---

#[test]
fn test_detect_goal_rename_single_role() {
    let old = make_commitments(vec![("Dev", 40, vec!["Old name"])]);
    let new = make_commitments(vec![("Dev", 40, vec!["New name"])]);
    let changes = detect_goal_changes(&old, &new);
    assert_eq!(changes.renames.len(), 1);
    assert_eq!(changes.renames[0], ("Old name".to_string(), "New name".to_string()));
    assert!(changes.deleted.is_empty());
}

#[test]
fn test_detect_goal_deleted() {
    let old = make_commitments(vec![("Dev", 40, vec!["Ship it", "Review"])]);
    let new = make_commitments(vec![("Dev", 40, vec!["Ship it"])]);
    let changes = detect_goal_changes(&old, &new);
    assert!(changes.renames.is_empty());
    assert_eq!(changes.deleted, vec!["Review"]);
}

#[test]
fn test_detect_goal_added_no_rename() {
    let old = make_commitments(vec![("Dev", 40, vec!["Ship it"])]);
    let new = make_commitments(vec![("Dev", 40, vec!["Ship it", "Review"])]);
    let changes = detect_goal_changes(&old, &new);
    assert!(changes.renames.is_empty());
    assert!(changes.deleted.is_empty());
}

#[test]
fn test_detect_goal_rename_when_count_matches() {
    let old = make_commitments(vec![("Dev", 40, vec!["A", "B", "C"])]);
    let new = make_commitments(vec![("Dev", 40, vec!["A", "B", "D"])]);
    let changes = detect_goal_changes(&old, &new);
    assert_eq!(changes.renames.len(), 1);
    assert_eq!(changes.renames[0], ("C".to_string(), "D".to_string()));
    assert!(changes.deleted.is_empty());
}

#[test]
fn test_detect_goal_delete_add_not_rename() {
    // Count differs: delete + add, NOT rename
    let old = make_commitments(vec![("Dev", 40, vec!["A", "B"])]);
    let new = make_commitments(vec![("Dev", 40, vec!["A", "B", "C"])]);
    let changes = detect_goal_changes(&old, &new);
    assert!(changes.renames.is_empty());
    // C is new, nothing deleted
    assert!(changes.deleted.is_empty());
}

#[test]
fn test_detect_goal_changes_role_removed() {
    let old = make_commitments(vec![
        ("Dev", 40, vec!["A"]),
        ("PM", 10, vec!["B"]),
    ]);
    let new = make_commitments(vec![
        ("Dev", 40, vec!["A"]),
    ]);
    let changes = detect_goal_changes(&old, &new);
    assert!(changes.renames.is_empty());
    // Goal "B" from removed role "PM" is a deletion
    assert_eq!(changes.deleted, vec!["B"]);
}

#[test]
fn test_detect_goal_changes_role_added() {
    let old = make_commitments(vec![("Dev", 40, vec!["A"])]);
    let new = make_commitments(vec![
        ("Dev", 40, vec!["A"]),
        ("PM", 10, vec!["B"]),
    ]);
    let changes = detect_goal_changes(&old, &new);
    assert!(changes.renames.is_empty());
    assert!(changes.deleted.is_empty());
}

#[test]
fn test_detect_goal_changes_no_diff() {
    let c = make_commitments(vec![("Dev", 40, vec!["A", "B"])]);
    let changes = detect_goal_changes(&c, &c);
    assert!(changes.renames.is_empty());
    assert!(changes.deleted.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test detect_goal_changes`
Expected: 8 tests FAIL — `detect_goal_changes` not found, `GoalChanges` struct not found.

- [ ] **Step 3: Write `GoalChanges` struct and `detect_goal_changes` function**

Add after `validate_commitments`:

```rust
struct GoalChanges {
    renames: Vec<(String, String)>, // (old_name, new_name)
    deleted: Vec<String>,
}

fn detect_goal_changes(old: &[Commitment], new: &[Commitment]) -> GoalChanges {
    use std::collections::HashSet;

    let old_goals: HashSet<String> = old
        .iter()
        .flat_map(|c| c.goals.iter().cloned())
        .collect();
    let new_goals: HashSet<String> = new
        .iter()
        .flat_map(|c| c.goals.iter().cloned())
        .collect();

    let deleted: Vec<String> = old_goals.difference(&new_goals).cloned().collect();

    // Detect renames: for each old role, if the new role exists with same
    // goal count and exactly one goal differs, it's a rename.
    let mut renames: Vec<(String, String)> = Vec::new();
    let mut matched_old_goals: HashSet<String> = HashSet::new();

    for old_c in old {
        if let Some(new_c) = new.iter().find(|c| c.role == old_c.role) {
            if old_c.goals.len() == new_c.goals.len() {
                let old_set: HashSet<_> = old_c.goals.iter().cloned().collect();
                let new_set: HashSet<_> = new_c.goals.iter().cloned().collect();

                let old_not_new: Vec<_> = old_set.difference(&new_set).cloned().collect();
                let new_not_old: Vec<_> = new_set.difference(&old_set).cloned().collect();

                if old_not_new.len() == 1 && new_not_old.len() == 1 {
                    renames.push((old_not_new[0].clone(), new_not_old[0].clone()));
                    matched_old_goals.insert(old_not_new[0].clone());
                }
            }
        }
    }

    // Remove renamed goals from the deleted list
    let deleted: Vec<String> = deleted
        .into_iter()
        .filter(|g| !matched_old_goals.contains(g))
        .collect();

    GoalChanges { renames, deleted }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test detect_goal_changes`
Expected: 8 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add detect_goal_changes for goal rename detection"
```

---

### Task 4: Add `set_commitments` command (integration tests first)

**Files:**
- Create: `src-tauri/tests/commitment_editor_integration.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write integration tests**

Create `src-tauri/tests/commitment_editor_integration.rs`:

```rust
/// Integration tests for set_commitments command.
use std::collections::HashMap;
use std::fs;

use tauri_app_lib::models::{Commitment, NewEntry};

fn setup(suffix: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("logbook_int_sc_{}", suffix));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    fs::write(
        root.join("config.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();

    let monthly_dir = root.join("2026/06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Developer\n    allocation: 40\n    goals:\n      - Feature A\n      - Code review\n  - role: VP\n    allocation: 10\n    goals:\n      - Strategy\n---\n",
    )
    .unwrap();

    root
}

fn teardown(root: &std::path::Path) {
    let _ = fs::remove_dir_all(root);
}

fn make_commitments(roles: Vec<(&str, u32, Vec<&str>)>) -> Vec<Commitment> {
    roles
        .into_iter()
        .map(|(role, alloc, goals)| Commitment {
            role: role.to_string(),
            allocation: alloc,
            goals: goals.into_iter().map(|g| g.to_string()).collect(),
        })
        .collect()
}

#[test]
fn test_set_commitments_write_and_read() {
    let root = setup("write_read");
    let new = make_commitments(vec![("Dev", 80, vec!["X", "Y"])]);

    let result =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].role, "Dev");
    assert_eq!(result[0].allocation, 80);
    assert_eq!(result[0].goals, vec!["X", "Y"]);

    // Verify file content
    let content = fs::read_to_string(root.join("2026/06/_monthly.md")).unwrap();
    assert!(content.contains("role: Dev"));
    assert!(content.contains("allocation: 80"));

    teardown(&root);
}

#[test]
fn test_set_commitments_empty_list_rejected() {
    let root = setup("empty_reject");
    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, vec![])
            .unwrap_err();
    assert!(err.contains("At least one role"));
    teardown(&root);
}

#[test]
fn test_set_commitments_empty_role_rejected() {
    let root = setup("empty_role");
    let commitments = make_commitments(vec![("", 40, vec!["A"])]);
    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, commitments)
            .unwrap_err();
    assert!(err.contains("Role name cannot be empty"));
    teardown(&root);
}

#[test]
fn test_set_commitments_zero_allocation_rejected() {
    let root = setup("zero_alloc");
    let commitments = make_commitments(vec![("Dev", 0, vec!["A"])]);
    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, commitments)
            .unwrap_err();
    assert!(err.contains("must be greater than 0"));
    teardown(&root);
}

#[test]
fn test_set_commitments_empty_goal_rejected() {
    let root = setup("empty_goal");
    let commitments = make_commitments(vec![("Dev", 40, vec![""])]);
    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, commitments)
            .unwrap_err();
    assert!(err.contains("Goal name cannot be empty"));
    teardown(&root);
}

#[test]
fn test_set_commitments_duplicate_goal_same_role_rejected() {
    let root = setup("dup_goal");
    let commitments = make_commitments(vec![("Dev", 40, vec!["A", "A"])]);
    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, commitments)
            .unwrap_err();
    assert!(err.contains("already exists"));
    teardown(&root);
}

#[test]
fn test_set_commitments_goal_rename_syncs_entries() {
    let root = setup("rename_sync");

    // Add entries with the old goal name
    let mut dims = HashMap::new();
    dims.insert("goal".to_string(), "Feature A".to_string());

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &NewEntry {
            item: "Coding".into(),
            duration: "60".into(),
            dimensions: dims.clone(),
        },
    )
    .unwrap();

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-02",
        &NewEntry {
            item: "More coding".into(),
            duration: "30".into(),
            dimensions: dims,
        },
    )
    .unwrap();

    // Rename "Feature A" → "Feature X"
    let new = make_commitments(vec![
        ("Developer", 40, vec!["Feature X", "Code review"]),
        ("VP", 10, vec!["Strategy"]),
    ]);

    let result =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap();

    assert_eq!(result[0].goals, vec!["Feature X", "Code review"]);

    // Verify entries were updated
    let day1 = tauri_app_lib::files::read_day_file(&root, "2026-06-01").unwrap();
    assert_eq!(day1.entries[0].dimensions.get("goal").unwrap(), "Feature X");

    let day2 = tauri_app_lib::files::read_day_file(&root, "2026-06-02").unwrap();
    assert_eq!(day2.entries[0].dimensions.get("goal").unwrap(), "Feature X");

    teardown(&root);
}

#[test]
fn test_set_commitments_delete_goal_rejected_when_entries_exist() {
    let root = setup("del_reject");

    let mut dims = HashMap::new();
    dims.insert("goal".to_string(), "Code review".to_string());

    tauri_app_lib::files::append_new_entry(
        &root,
        "2026-06-01",
        &NewEntry {
            item: "Reviewing".into(),
            duration: "30".into(),
            dimensions: dims,
        },
    )
    .unwrap();

    // Try to remove "Code review" goal
    let new = make_commitments(vec![("Developer", 40, vec!["Feature A"])]);

    let err =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap_err();

    assert!(err.contains("Cannot delete goal"));
    assert!(err.contains("Code review"));
    assert!(err.contains("used by 1 entries"));

    teardown(&root);
}

#[test]
fn test_set_commitments_delete_goal_allowed_when_no_entries() {
    let root = setup("del_allowed");

    // No entries — deleting "Code review" should succeed
    let new = make_commitments(vec![("Developer", 40, vec!["Feature A"])]);

    let result =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].goals, vec!["Feature A"]);

    teardown(&root);
}

#[test]
fn test_set_commitments_creates_new_monthly_file() {
    let root = setup("new_file");
    // Delete existing _monthly.md to simulate a month with no prior commitments
    fs::remove_file(root.join("2026/06/_monthly.md")).unwrap();

    let new = make_commitments(vec![("Dev", 20, vec!["Goal 1"])]);
    let result =
        tauri_app_lib::commands::set_commitments(root.to_string_lossy().into_owned(), 2026, 6, new)
            .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].role, "Dev");
    assert!(root.join("2026/06/_monthly.md").exists());

    teardown(&root);
}
```

- [ ] **Step 2: Run integration tests to verify they fail**

Run: `cd src-tauri && cargo test commitment_editor`
Expected: Tests FAIL — `set_commitments` function not found.

- [ ] **Step 3: Write the `set_commitments` command**

Add after `get_commitment_progress` in `commands.rs`:

```rust
#[tauri::command]
pub fn set_commitments(
    root_path: String,
    year: i32,
    month: u32,
    commitments: Vec<Commitment>,
) -> Result<Vec<Commitment>, String> {
    error_log::log_command_enter(
        "set_commitments",
        &format!("{}-{:02} {} roles", year, month, commitments.len()),
    );
    let root = std::path::Path::new(&root_path);

    // 1. Validate
    validate_commitments(&commitments)?;

    // 2. Read old state for diff
    let old = read_monthly_file_safe(root, year, month)?;

    // 3. Detect changes
    let changes = detect_goal_changes(&old.commitments, &commitments);

    // 4. Check deleted goals for existing entries
    for goal_name in &changes.deleted {
        let count = count_entries_with_goal(root, year, month, goal_name)?;
        if count > 0 {
            return Err(format!(
                "Cannot delete goal '{}': used by {} entries this month",
                goal_name, count
            ));
        }
    }

    // 5. Apply renames to all day files
    for (old_name, new_name) in &changes.renames {
        rename_goal_in_entries(root, year, month, old_name, new_name)?;
    }

    // 6. Write _monthly.md
    let monthly = MonthlyFile { commitments };
    files::write_monthly_file(root, year, month, &monthly)?;

    let ok = true;
    error_log::log_command_exit("set_commitments", ok, "");
    Ok(monthly.commitments)
}

/// Count entries in a month that reference a specific goal.
fn count_entries_with_goal(
    root: &std::path::Path,
    year: i32,
    month: u32,
    goal_name: &str,
) -> Result<usize, String> {
    let month_dir = root
        .join(year.to_string())
        .join(format!("{:02}", month));

    if !month_dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    let entries = match std::fs::read_dir(&month_dir) {
        Ok(e) => e,
        Err(e) => return Err(format!("Failed to read month dir: {}", e)),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "_monthly.md" || !file_name.ends_with(".md") {
            continue;
        }
        let date = file_name.trim_end_matches(".md");
        if let Ok(day_file) = files::read_day_file(root, date) {
            count += day_file
                .entries
                .iter()
                .filter(|e| e.dimensions.get("goal").map(|g| g == goal_name).unwrap_or(false))
                .count();
        }
    }
    Ok(count)
}

/// Rename a goal in all day files of a given month.
fn rename_goal_in_entries(
    root: &std::path::Path,
    year: i32,
    month: u32,
    old_name: &str,
    new_name: &str,
) -> Result<(), String> {
    let month_dir = root
        .join(year.to_string())
        .join(format!("{:02}", month));

    if !month_dir.exists() {
        return Ok(());
    }

    let entries = match std::fs::read_dir(&month_dir) {
        Ok(e) => e,
        Err(e) => return Err(format!("Failed to read month dir: {}", e)),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "_monthly.md" || !file_name.ends_with(".md") {
            continue;
        }
        let date = file_name.trim_end_matches(".md");
        let mut day_file = files::read_day_file(root, date)?;
        let mut changed = false;
        for e in &mut day_file.entries {
            if let Some(goal) = e.dimensions.get("goal") {
                if goal == old_name {
                    e.dimensions.insert("goal".to_string(), new_name.to_string());
                    changed = true;
                }
            }
        }
        if changed {
            files::write_day_file(root, date, &day_file)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Register command in `lib.rs`**

In `src-tauri/src/lib.rs`, add `commands::set_commitments` to the `invoke_handler` macro (insert after `commands::get_commitments` on line 56):

```rust
commands::set_commitments,
```

- [ ] **Step 5: Run integration tests to verify they pass**

Run: `cd src-tauri && cargo test commitment_editor`
Expected: 10 tests PASS.

- [ ] **Step 6: Run all tests to verify no regressions**

Run: `cd src-tauri && cargo test`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/tests/commitment_editor_integration.rs
git commit -m "feat: add set_commitments command with rename sync and delete protection"
```

---

### Task 5: Update Tauri mock and fixtures for frontend

**Files:**
- Modify: `src/__tests__/mocks/tauri.ts`

- [ ] **Step 1: Add `set_commitments` to mock**

In `src/__tests__/mocks/tauri.ts`, inside the `defaultInvoke` function's switch statement, add after `case "get_commitment_progress":` (or before the `default` case):

```typescript
case "set_commitments":
  return args?.commitments ?? []; // echo back commitments
```

- [ ] **Step 2: Verify existing frontend tests still pass**

Run: `npx vitest run`
Expected: All existing tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/__tests__/mocks/tauri.ts
git commit -m "test: add set_commitments mock to tauri mocks"
```

---

### Task 6: Add edit mode to CommitmentsPanel (component tests first)

**Files:**
- Modify: `src/__tests__/components/CommitmentsPanel.test.ts`
- Modify: `src/components/CommitmentsPanel.vue`
- Modify: `src/components/MonthView.vue`

- [ ] **Step 1: Write tests for edit mode**

Add to `src/__tests__/components/CommitmentsPanel.test.ts` after the existing tests (inside the `describe("CommitmentsPanel", () => { ... })` block):

```typescript
import { makeCommitment } from "../mocks/fixtures";
import type { Commitment } from "../../types";
import { setupTauriMocks } from "../mocks/tauri";

function makeCommitmentObj(overrides?: Partial<Commitment>): Commitment {
  return {
    role: "Developer",
    allocation: 40,
    goals: ["Ship feature X", "Code review"],
    ...overrides,
  };
}

function mountPanelWithEdit(
  commitments: Commitment[],
  progress = commitments.map(c => ({
    role: c.role,
    allocation_minutes: c.allocation * 60,
    spent_minutes: 0,
    goals: c.goals.map(g => ({ name: g, spent_minutes: 0 })),
  })),
  rootPath = "/test/root",
) {
  return mount(CommitmentsPanel, {
    props: {
      progress,
      commitments,
      rootPath,
      selectedYear: 2026,
      selectedMonth: 6,
    },
  });
}

describe("CommitmentsPanel edit mode", () => {
  it("shows edit button when commitments provided", () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);
    const editBtn = wrapper.find("button").find((b) => b.text().includes("编辑"));
    expect(editBtn.exists()).toBe(true);
  });

  it("clicking edit button enters edit mode", async () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);
    const editBtn = wrapper.find("button").find((b) => b.text().includes("编辑"));
    await editBtn.trigger("click");
    // Should see save/cancel buttons
    expect(wrapper.text()).toContain("保存");
    expect(wrapper.text()).toContain("取消");
  });

  it("edit mode shows role and allocation inputs", async () => {
    const commitments = [makeCommitmentObj({ role: "Developer", allocation: 40 })];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    // Role name input should be populated
    const roleInputs = wrapper.findAll("input[type='text']");
    const roleInput = roleInputs.find((i) => (i.element as HTMLInputElement).value === "Developer");
    expect(roleInput).toBeTruthy();

    // Allocation input should be populated
    const allocInput = wrapper.find("input[type='number']");
    expect((allocInput.element as HTMLInputElement).value).toBe("40");
  });

  it("edits show goal names as inputs with delete buttons", async () => {
    const commitments = [makeCommitmentObj({ goals: ["Goal A"] })];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    // Goal input should show "Goal A"
    expect(wrapper.text()).toContain("Goal A");
    // Should have delete buttons for goals
    expect(wrapper.findAll("button").some((b) => b.text().includes("✕"))).toBe(true);
  });

  it("can add a new goal to a role", async () => {
    const commitments = [makeCommitmentObj({ goals: ["Goal A"] })];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    const addGoalBtns = wrapper.findAll("button").filter((b) => b.text().includes("添加 Goal"));
    expect(addGoalBtns.length).toBe(1);
    await addGoalBtns[0].trigger("click");

    // Should now have 2 goal inputs (one existing + one new empty)
    // Verify component state updated
    expect(wrapper.vm.editingCommitments[0].goals.length).toBe(2);
  });

  it("can delete a goal from a role", async () => {
    const commitments = [makeCommitmentObj({ goals: ["A", "B"] })];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    // Click delete on first goal
    const deleteBtns = wrapper.findAll("button").filter((b) => b.text().includes("✕"));
    await deleteBtns[0].trigger("click");

    expect(wrapper.vm.editingCommitments[0].goals.length).toBe(1);
    expect(wrapper.vm.editingCommitments[0].goals[0]).toBe("B");
  });

  it("can add a new role", async () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    const addRoleBtn = wrapper.find("button").find((b) => b.text().includes("添加 Role"));
    expect(addRoleBtn.exists()).toBe(true);
    await addRoleBtn.trigger("click");

    expect(wrapper.vm.editingCommitments.length).toBe(2);
  });

  it("can remove a role if more than one", async () => {
    const commitments = [
      makeCommitmentObj({ role: "Dev" }),
      makeCommitmentObj({ role: "PM" }),
    ];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    // Delete buttons for roles should be visible
    const roleDeleteBtns = wrapper.findAll("button").filter((b) => b.text().includes("删除 Role"));
    expect(roleDeleteBtns.length).toBe(2);

    await roleDeleteBtns[0].trigger("click");
    expect(wrapper.vm.editingCommitments.length).toBe(1);
  });

  it("last role has no delete button", async () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    const roleDeleteBtns = wrapper.findAll("button").filter((b) => b.text().includes("删除 Role"));
    expect(roleDeleteBtns.length).toBe(0);
  });

  it("cancel restores snapshot and returns to display mode", async () => {
    const commitments = [makeCommitmentObj({ allocation: 40 })];
    const wrapper = mountPanelWithEdit(commitments);
    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    // Modify allocation
    const allocInput = wrapper.find("input[type='number']");
    await allocInput.setValue(99);

    // Cancel
    const cancelBtn = wrapper.find("button").find((b) => b.text().includes("取消"));
    await cancelBtn.trigger("click");

    // Should be back in display mode showing original values
    expect(wrapper.text()).toContain("40.0h"); // 40h displayed in display mode
  });

  it("frontend pre-validation: empty role name blocked", async () => {
    const commitments = [makeCommitmentObj({ role: "Dev" })];
    const wrapper = mountPanelWithEdit(commitments);
    const mocks = setupTauriMocks();

    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    // Clear role name
    const roleInput = wrapper.find("input[type='text']");
    await roleInput.setValue("");

    // Try to save
    const saveBtn = wrapper.find("button").find((b) => b.text().includes("保存"));
    await saveBtn.trigger("click");

    // Should show error, not call invoke
    expect(wrapper.text()).toContain("Role name cannot be empty");
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_commitments", expect.anything());
  });

  it("frontend pre-validation: zero allocation blocked", async () => {
    const commitments = [makeCommitmentObj({ allocation: 40 })];
    const wrapper = mountPanelWithEdit(commitments);
    const mocks = setupTauriMocks();

    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    const allocInput = wrapper.find("input[type='number']");
    await allocInput.setValue(0);

    const saveBtn = wrapper.find("button").find((b) => b.text().includes("保存"));
    await saveBtn.trigger("click");

    expect(wrapper.text()).toContain("must be greater than 0");
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_commitments", expect.anything());
  });

  it("frontend pre-validation: empty goal name blocked", async () => {
    const commitments = [makeCommitmentObj({ goals: ["A"] })];
    const wrapper = mountPanelWithEdit(commitments);
    const mocks = setupTauriMocks();

    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    // Add an empty goal
    const addGoalBtn = wrapper.find("button").find((b) => b.text().includes("添加 Goal"));
    await addGoalBtn.trigger("click");

    const saveBtn = wrapper.find("button").find((b) => b.text().includes("保存"));
    await saveBtn.trigger("click");

    expect(wrapper.text()).toContain("Goal name cannot be empty");
    expect(mocks.invoke).not.toHaveBeenCalledWith("set_commitments", expect.anything());
  });

  it("save calls invoke and emits saved event on success", async () => {
    const commitments = [makeCommitmentObj({ allocation: 80 })];
    const wrapper = mountPanelWithEdit(commitments);

    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    const saveBtn = wrapper.find("button").find((b) => b.text().includes("保存"));
    await saveBtn.trigger("click");

    // Check that the saved event was emitted
    expect(wrapper.emitted("saved")).toBeTruthy();
  });

  it("save button shows loading state during save", async () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);

    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    // Make invoke delay to test loading state
    const mocks = setupTauriMocks();
    let resolveInvoke: (value: unknown) => void;
    mocks.invoke.mockImplementation(
      () => new Promise((resolve) => { resolveInvoke = resolve; })
    );

    const saveBtn = wrapper.find("button").find((b) => b.text().includes("保存"));
    await saveBtn.trigger("click");

    // Button should be disabled during save
    expect(saveBtn.attributes("disabled")).toBeDefined();

    // Resolve and clean up
    resolveInvoke!(commitments);
    await wrapper.vm.$nextTick();
    await wrapper.vm.$nextTick();
  });

  it("displays backend error as toast", async () => {
    const commitments = [makeCommitmentObj()];
    const wrapper = mountPanelWithEdit(commitments);
    const mocks = setupTauriMocks();
    mocks.invoke.mockRejectedValueOnce("Cannot delete goal 'X': used by 3 entries this month");

    await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");

    const saveBtn = wrapper.find("button").find((b) => b.text().includes("保存"));
    await saveBtn.trigger("click");
    await wrapper.vm.$nextTick();
    await wrapper.vm.$nextTick();

    expect(wrapper.text()).toContain("Cannot delete goal");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npx vitest run src/__tests__/components/CommitmentsPanel.test.ts`
Expected: New edit mode tests FAIL — CommitmentsPanel has no edit mode yet.

- [ ] **Step 3: Implement edit mode in CommitmentsPanel.vue**

Replace `src/components/CommitmentsPanel.vue` entirely:

```vue
<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Commitment, CommitmentProgress } from "../types";
import { formatDuration } from "../utils/format";

const props = defineProps<{
  progress: CommitmentProgress[];
  commitments?: Commitment[];
  rootPath?: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{
  saved: [];
}>();

// ---- Display mode helpers ----

function pct(spent: number, alloc: number): string {
  if (alloc === 0) return "0%";
  return Math.min(100, Math.round((spent / alloc) * 100)) + "%";
}

function barColor(spent: number, alloc: number): string {
  if (alloc === 0) return "bg-gray-300";
  const spentRatio = spent / alloc;
  if (spentRatio > 1) return "bg-red-500";
  const elapsed = elapsedRatio();
  if (spentRatio < elapsed * 0.6) return "bg-orange-500";
  if (spentRatio > elapsed * 1.4) return "bg-yellow-500";
  return "bg-green-500";
}

function elapsedRatio(): number {
  const now = new Date();
  const isCurrentMonth =
    props.selectedYear === now.getFullYear() &&
    props.selectedMonth === now.getMonth() + 1;
  if (isCurrentMonth) {
    const daysInMonth = new Date(props.selectedYear, props.selectedMonth, 0).getDate();
    return now.getDate() / daysInMonth;
  }
  return 1.0;
}

// ---- Edit mode ----

const isEditing = ref(false);
const editingCommitments = ref<Commitment[]>([]);
const editError = ref("");
const isSaving = ref(false);
const lastSavedCommitments = ref<Commitment[]>([]);

function enterEdit() {
  if (!props.commitments || props.commitments.length === 0) return;
  const snapshot = JSON.parse(JSON.stringify(props.commitments)) as Commitment[];
  editingCommitments.value = snapshot;
  editError.value = "";
  isEditing.value = true;
}

function cancelEdit() {
  isEditing.value = false;
  editingCommitments.value = [];
  editError.value = "";
}

function addGoal(roleIndex: number) {
  editingCommitments.value[roleIndex].goals.push("");
}

function removeGoal(roleIndex: number, goalIndex: number) {
  editingCommitments.value[roleIndex].goals.splice(goalIndex, 1);
}

function addRole() {
  editingCommitments.value.push({ role: "", allocation: 0, goals: [] });
}

function removeRole(roleIndex: number) {
  if (editingCommitments.value.length <= 1) return;
  editingCommitments.value.splice(roleIndex, 1);
}

// ---- Frontend pre-validation ----

function preValidate(): string | null {
  if (editingCommitments.value.length === 0) {
    return "At least one role is required";
  }
  for (const c of editingCommitments.value) {
    if (!c.role.trim()) {
      return "Role name cannot be empty";
    }
    if (c.allocation === 0 || !c.allocation) {
      return `Allocation for '${c.role || "unnamed"}' must be greater than 0`;
    }
    for (const g of c.goals) {
      if (!g.trim()) {
        return "Goal name cannot be empty";
      }
    }
  }
  return null;
}

// ---- Save ----

async function save() {
  const err = preValidate();
  if (err) {
    editError.value = err;
    return;
  }

  if (!props.rootPath) return;

  isSaving.value = true;
  editError.value = "";

  try {
    const saved = (await invoke("set_commitments", {
      rootPath: props.rootPath,
      year: props.selectedYear,
      month: props.selectedMonth,
      commitments: editingCommitments.value.map((c) => ({
        role: c.role.trim(),
        allocation: c.allocation,
        goals: c.goals.map((g) => g.trim()).filter((g) => g !== ""),
      })),
    })) as Commitment[];

    lastSavedCommitments.value = JSON.parse(JSON.stringify(saved)) as Commitment[];
    isEditing.value = false;
    editingCommitments.value = [];
    emit("saved");
  } catch (e) {
    editError.value = typeof e === "string" ? e : String(e);
  } finally {
    isSaving.value = false;
  }
}
</script>

<template>
  <div v-if="progress.length > 0 || (commitments && commitments.length > 0) || isEditing" class="bg-white rounded-lg shadow-sm p-4">
    <div class="flex justify-between items-center mb-3">
      <h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Commitments</h3>
      <button
        v-if="!isEditing && commitments && commitments.length > 0"
        class="text-xs text-gray-400 hover:text-gray-600 transition-colors cursor-pointer"
        @click="enterEdit"
      >
        ✏️ 编辑
      </button>
    </div>

    <!-- Display mode -->
    <template v-if="!isEditing">
      <div v-for="s in progress" :key="s.role" class="mb-4 last:mb-0">
        <div class="flex justify-between items-center text-sm mb-1">
          <span class="font-semibold text-gray-700">{{ s.role }}</span>
          <span class="text-gray-500 text-xs">
            {{ formatDuration(s.spent_minutes) }} / {{ (s.allocation_minutes / 60).toFixed(1) }}h
          </span>
        </div>
        <div class="h-1.5 bg-gray-100 rounded-full overflow-hidden mb-2">
          <div
            :class="barColor(s.spent_minutes, s.allocation_minutes)"
            class="h-full rounded-full transition-all"
            :style="{ width: pct(s.spent_minutes, s.allocation_minutes) }"
          />
        </div>
        <div class="ml-2 flex flex-col gap-0.5 text-xs">
          <div
            v-for="g in s.goals"
            :key="g.name"
            class="flex justify-between"
            :class="g.spent_minutes > 0 ? 'text-gray-600' : 'text-gray-300'"
          >
            <span>{{ g.name }}</span>
            <span v-if="g.spent_minutes > 0" class="font-medium text-gray-700">{{ formatDuration(g.spent_minutes) }}</span>
            <span v-else>0m</span>
          </div>
        </div>
      </div>
    </template>

    <!-- Edit mode -->
    <template v-else>
      <div v-if="editError" class="mb-3 p-2 bg-red-50 border border-red-200 rounded text-xs text-red-700">
        {{ editError }}
      </div>

      <div v-for="(c, ri) in editingCommitments" :key="ri" class="mb-4 last:mb-0">
        <div class="flex items-center gap-2 mb-2">
          <input
            v-model="c.role"
            type="text"
            placeholder="Role"
            class="flex-1 px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <label class="text-xs text-gray-500 whitespace-nowrap">Alloc:</label>
          <input
            v-model.number="c.allocation"
            type="number"
            min="1"
            placeholder="hours"
            class="w-16 px-2 py-1 text-sm border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <span class="text-xs text-gray-400">h</span>
          <button
            v-if="editingCommitments.length > 1"
            class="text-xs text-red-400 hover:text-red-600 cursor-pointer ml-1"
            @click="removeRole(ri)"
          >
            删除 Role
          </button>
        </div>

        <div class="ml-4 flex flex-col gap-1.5">
          <div v-for="(g, gi) in c.goals" :key="gi" class="flex items-center gap-1">
            <input
              v-model="c.goals[gi]"
              type="text"
              placeholder="Goal name"
              class="flex-1 px-2 py-0.5 text-xs border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <button
              class="text-gray-400 hover:text-red-500 text-xs cursor-pointer px-1"
              @click="removeGoal(ri, gi)"
            >
              ✕
            </button>
          </div>
          <button
            class="text-xs text-blue-500 hover:text-blue-700 cursor-pointer self-start"
            @click="addGoal(ri)"
          >
            + 添加 Goal
          </button>
        </div>

        <hr v-if="ri < editingCommitments.length - 1" class="my-3 border-gray-100" />
      </div>

      <button
        class="text-xs text-blue-500 hover:text-blue-700 cursor-pointer mt-2"
        @click="addRole"
      >
        + 添加 Role
      </button>

      <div class="flex justify-end gap-2 mt-4 pt-3 border-t border-gray-100">
        <button
          class="px-3 py-1 text-xs text-gray-600 bg-gray-100 rounded hover:bg-gray-200 cursor-pointer"
          @click="cancelEdit"
        >
          取消
        </button>
        <button
          class="px-3 py-1 text-xs text-white bg-blue-500 rounded hover:bg-blue-600 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="isSaving"
          @click="save"
        >
          {{ isSaving ? "保存中…" : "保存" }}
        </button>
      </div>
    </template>
  </div>
</template>
```

- [ ] **Step 4: Update MonthView.vue to pass new props and handle `saved` event**

In `src/components/MonthView.vue`, find the CommitmentsPanel usage in the template and update from:

```vue
<CommitmentsPanel
  :progress="store.commitmentProgress"
  :selectedYear="selectedYear"
  :selectedMonth="selectedMonth"
/>
```

To:

```vue
<CommitmentsPanel
  :progress="store.commitmentProgress"
  :commitments="store.commitments"
  :rootPath="store.rootPath"
  :selectedYear="selectedYear"
  :selectedMonth="selectedMonth"
  @saved="loadCommitmentProgress(selectedYear, selectedMonth)"
/>
```

- [ ] **Step 5: Run component tests to verify they pass**

Run: `npx vitest run src/__tests__/components/CommitmentsPanel.test.ts`
Expected: All tests PASS (existing display tests + new edit mode tests).

- [ ] **Step 6: Run all frontend tests to verify no regressions**

Run: `npx vitest run`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/components/CommitmentsPanel.vue src/components/MonthView.vue src/__tests__/components/CommitmentsPanel.test.ts
git commit -m "feat: add inline edit mode to CommitmentsPanel"
```

---

### Task 7: External modification conflict handling

**Files:**
- Modify: `src/components/CommitmentsPanel.vue`

- [ ] **Step 1: Write test for external modification detection**

Add to the edit mode test block in `CommitmentsPanel.test.ts`:

```typescript
it("exits edit mode when commitments prop changes externally", async () => {
  const commitments = [makeCommitmentObj()];
  const wrapper = mountPanelWithEdit(commitments);

  await wrapper.find("button").find((b) => b.text().includes("编辑")).trigger("click");
  expect(wrapper.vm.isEditing).toBe(true);

  // Simulate external change: update the commitments prop with different data
  const changedCommitments = [makeCommitmentObj({ allocation: 99 })];
  await wrapper.setProps({ commitments: changedCommitments });
  await wrapper.vm.$nextTick();

  // Should exit edit mode
  expect(wrapper.vm.isEditing).toBe(false);
  // Should be back in display mode showing new values
  expect(wrapper.text()).toContain("99.0h");
});
```

- [ ] **Step 2: Add watch for external changes during edit**

In `CommitmentsPanel.vue` script, add after the imports (modify the existing `import { ref, computed }` line to include `watch`):

Change:
```typescript
import { ref, computed } from "vue";
```
To:
```typescript
import { ref, watch } from "vue";
```

Then add the watch after the `isEditing` / `editingCommitments` declarations but before `enterEdit`:

```typescript
// Watch for external changes while editing (file watcher pushes new data)
watch(
  () => props.commitments,
  (newVal, oldVal) => {
    if (!isEditing.value) return;
    if (!newVal || !oldVal) return;
    if (JSON.stringify(newVal) === JSON.stringify(oldVal)) return;
    if (JSON.stringify(newVal) === JSON.stringify(lastSavedCommitments.value)) return;

    // External modification detected — exit edit mode, display refreshes
    isEditing.value = false;
    editingCommitments.value = [];
    editError.value = "";
  }
);
```

- [ ] **Step 3: Run tests**

Run: `npx vitest run src/__tests__/components/CommitmentsPanel.test.ts`
Expected: All tests pass.

- [ ] **Step 4: Full test suite**

Run: `cd src-tauri && cargo test && cd .. && npx vitest run`
Expected: All backend and frontend tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/CommitmentsPanel.vue src/__tests__/components/CommitmentsPanel.test.ts
git commit -m "feat: detect external commitment changes during edit mode"
```

---

### Task 8: End-to-end manual verification

- [ ] **Step 1: Build and launch the app**

Run: `pnpm tauri dev`
Expected: App launches without errors.

- [ ] **Step 2: Smoke test the edit flow**

1. Navigate to current month
2. Click "✏️ 编辑" on CommitmentsPanel
3. Add a new role with allocation and goals
4. Click "保存"
5. Verify CommitmentsPanel display updates
6. Click "✏️ 编辑" again
7. Modify a goal name
8. Click "保存"
9. Verify entries with old goal name were updated
10. Try to delete a goal that has entries — verify rejection message
11. Click "取消" — verify edit mode exits without changes

- [ ] **Step 3: Commit any fixes if needed**

```bash
git add -A
git commit -m "chore: manual verification fixes"
```
