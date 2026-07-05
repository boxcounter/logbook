# Backend Validation Completeness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three missing backend validation checks (unknown dimension key rejection, empty item rejection, empty static value rejection) and extract a unified `validate_entry_input` function to eliminate duplication between `append_entry` and `update_entry`.

**Architecture:** Add `validate_entry_input` to `commands.rs` as a single pre-write entry validation gate. Refactor `append_entry` to call it. Add conditional checks to `update_entry` (optional fields). Add individual value emptiness check to `config.rs::validate_dimensions`.

**Tech Stack:** Rust, Cargo, `commands.rs`/`config.rs`/`cli_integration.rs`

## Global Constraints

- 后端是数据合法性检查的唯一权威源：所有写入路径必须在落盘前自行完成完整校验（`src-tauri/AGENTS.md` 关键约定）
- 三个缺口均为**硬拒绝**，不静默丢弃
- `validate_entry_input` 的 key 集合包含 `deleted: true` 的维度 key
- TDD: 先写测试，确认失败，再实现

---

### Task 1: Add `validate_entry_input` function to `commands.rs`

**Files:**
- Modify: `src-tauri/src/commands.rs` (after line 172, after `validate_cross_dimension_constraints`)

**Interfaces:**
- Consumes: `Dimension`, `BTreeMap`, `HashMap`, `validate_required_dimensions`, `validate_cross_dimension_constraints`, `parse_duration`
- Produces: `pub fn validate_entry_input(item: &str, duration_str: &str, dimensions: &BTreeMap<String, String>, dimension_config: &[Dimension], role_key: &str, goal_key: &str, role_to_goals: &std::collections::HashMap<String, Vec<String>>) -> Result<u32, String>`

- [ ] **Step 1: Write failing unit tests for `validate_entry_input`**

Add to the end of the `#[cfg(test)] mod tests` block in `commands.rs` (before the closing `}` at line 2703):

```rust
    // --- validate_entry_input tests ---

    fn make_dim_config() -> Vec<Dimension> {
        vec![
            Dimension {
                name: "Biz".into(),
                key: "biz".into(),
                source: "static".into(),
                values: Some(vec!["A".into()]),
                required: false,
                deleted: false,
            },
            Dimension {
                name: "Goal".into(),
                key: "goal".into(),
                source: "commitments:role:goals".into(),
                values: None,
                required: false,
                deleted: false,
            },
            Dimension {
                name: "Role".into(),
                key: "role".into(),
                source: "commitments:role".into(),
                values: None,
                required: false,
                deleted: false,
            },
            // Deleted dimension — key still valid for unknown key check
            Dimension {
                name: "Old".into(),
                key: "old".into(),
                source: "static".into(),
                values: Some(vec!["X".into()]),
                required: false,
                deleted: true,
            },
        ]
    }

    #[test]
    fn test_validate_entry_input_rejects_empty_item() {
        let config = make_dim_config();
        let dims = BTreeMap::new();
        let role_to_goals = std::collections::HashMap::new();
        let err = validate_entry_input("", "1h", &dims, &config, "role", "goal", &role_to_goals).unwrap_err();
        assert!(err.contains("Entry item cannot be empty"));
    }

    #[test]
    fn test_validate_entry_input_rejects_whitespace_item() {
        let config = make_dim_config();
        let dims = BTreeMap::new();
        let role_to_goals = std::collections::HashMap::new();
        let err = validate_entry_input("   ", "1h", &dims, &config, "role", "goal", &role_to_goals).unwrap_err();
        assert!(err.contains("Entry item cannot be empty"));
    }

    #[test]
    fn test_validate_entry_input_rejects_unknown_dim_key() {
        let config = make_dim_config();
        let mut dims = BTreeMap::new();
        dims.insert("nonexistent".to_string(), "x".to_string());
        let role_to_goals = std::collections::HashMap::new();
        let err = validate_entry_input("Item", "1h", &dims, &config, "role", "goal", &role_to_goals).unwrap_err();
        assert!(err.contains("Unknown dimension key"));
        assert!(err.contains("nonexistent"));
    }

    #[test]
    fn test_validate_entry_input_allows_deleted_dim_key() {
        let config = make_dim_config();
        let mut dims = BTreeMap::new();
        dims.insert("old".to_string(), "legacy_value".to_string());
        let role_to_goals = std::collections::HashMap::new();
        // Should NOT reject—deleted dimension key is still a known key
        assert!(validate_entry_input("Item", "1h", &dims, &config, "role", "goal", &role_to_goals).is_ok());
    }

    #[test]
    fn test_validate_entry_input_ok() {
        let config = make_dim_config();
        let mut dims = BTreeMap::new();
        dims.insert("biz".to_string(), "A".to_string());
        let role_to_goals = std::collections::HashMap::new();
        let result = validate_entry_input("Test item", "1h 30m", &dims, &config, "role", "goal", &role_to_goals).unwrap();
        assert_eq!(result, 90);
    }

    #[test]
    fn test_validate_entry_input_duration_fail() {
        let config = make_dim_config();
        let dims = BTreeMap::new();
        let role_to_goals = std::collections::HashMap::new();
        let err = validate_entry_input("Item", "no duration", &dims, &config, "role", "goal", &role_to_goals).unwrap_err();
        assert!(err.contains("Could not parse duration"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test test_validate_entry_input_rejects_empty_item test_validate_entry_input_rejects_whitespace_item test_validate_entry_input_rejects_unknown_dim_key test_validate_entry_input_allows_deleted_dim_key test_validate_entry_input_ok test_validate_entry_input_duration_fail -- --nocapture 2>&1 | head -30
```

Expected: All 6 tests FAIL with "function `validate_entry_input` not found" or similar.

- [ ] **Step 3: Implement `validate_entry_input`**

Insert after `validate_cross_dimension_constraints` (after line 172, before `load_root_state`):

```rust
/// Unified pre-write validation for entry input (append + update paths).
/// Returns parsed duration (u32 minutes) on success.
pub fn validate_entry_input(
    item: &str,
    duration_str: &str,
    dimensions: &BTreeMap<String, String>,
    dimension_config: &[Dimension],
    role_key: &str,
    goal_key: &str,
    role_to_goals: &std::collections::HashMap<String, Vec<String>>,
) -> Result<u32, String> {
    if item.trim().is_empty() {
        return Err("Entry item cannot be empty".to_string());
    }

    let duration = parse_duration(duration_str)?;

    let known_keys: std::collections::HashSet<&str> = dimension_config
        .iter()
        .map(|d| d.key.as_str())
        .collect();
    for key in dimensions.keys() {
        if !known_keys.contains(key.as_str()) {
            return Err(format!("Unknown dimension key '{}'", key));
        }
    }

    validate_required_dimensions(dimension_config, dimensions)?;

    validate_cross_dimension_constraints(dimensions, role_key, goal_key, role_to_goals)?;

    Ok(duration)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd src-tauri && cargo test test_validate_entry_input_rejects_empty_item test_validate_entry_input_rejects_whitespace_item test_validate_entry_input_rejects_unknown_dim_key test_validate_entry_input_allows_deleted_dim_key test_validate_entry_input_ok test_validate_entry_input_duration_fail -- --nocapture
```

Expected: All 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add validate_entry_input unified entry validation function"
```

---

### Task 2: Refactor `append_entry` to use `validate_entry_input`

**Files:**
- Modify: `src-tauri/src/commands.rs:520-580` (`append_entry`)

**Interfaces:**
- Consumes: `validate_entry_input` (from Task 1), `goal_dim_key`, `role_dim_key`, `build_commitment_maps`
- Produces: refactored `append_entry` that delegates to `validate_entry_input`

- [ ] **Step 1: Replace `append_entry` validation calls with `validate_entry_input`**

Replace lines 528-541 of `src-tauri/src/commands.rs`:

Old:
```rust
    let duration = parse_duration(&entry.duration)?;
    let (year, month) = files::year_month_from_date(&date)?;
    files::create_dimensions_if_missing(root, year, month)?;
    let dims = files::resolve_month_dimensions(root, year, month)?;
    validate_required_dimensions(&dims, &entry.dimensions)?;

    // Cross-dimension validation
    {
        let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
        let goal_key = goal_dim_key(root, year, month)?;
        let role_key = role_dim_key(root, year, month)?;
        let (_, role_to_goals) = build_commitment_maps(&commitments);
        validate_cross_dimension_constraints(&entry.dimensions, &role_key, &goal_key, &role_to_goals)?;
    }
```

New:
```rust
    let (year, month) = files::year_month_from_date(&date)?;
    files::create_dimensions_if_missing(root, year, month)?;
    let dims = files::resolve_month_dimensions(root, year, month)?;
    let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
    let goal_key = goal_dim_key(root, year, month)?;
    let role_key = role_dim_key(root, year, month)?;
    let (_, role_to_goals) = build_commitment_maps(&commitments);
    let duration = validate_entry_input(
        &entry.item,
        &entry.duration,
        &entry.dimensions,
        &dims,
        &role_key,
        &goal_key,
        &role_to_goals,
    )?;
```

Note: `duration` is now obtained as the return value of `validate_entry_input` instead of directly from `parse_duration`. The `validate_required_dimensions` and `validate_cross_dimension_constraints` calls are now inside `validate_entry_input`. The variable `duration` is still declared as `let duration = ...` and used later on line 571.

- [ ] **Step 2: Run existing tests to verify no regression**

```bash
cd src-tauri && cargo test -p tauri_app_lib
```

Expected: All existing tests PASS.

- [ ] **Step 3: Run the CLI integration tests**

```bash
cd src-tauri && cargo test --test cli_integration -- --nocapture
```

Expected: All existing CLI integration tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "refactor: delegate append_entry validation to validate_entry_input"
```

---

### Task 3: Add validation checks to `update_entry`

**Files:**
- Modify: `src-tauri/src/commands.rs:583-617` (`update_entry`)

**Interfaces:**
- Consumes: `parse_duration`, `validate_required_dimensions`, `validate_cross_dimension_constraints`, `validate_entry_input` (from Task 1)
- Produces: `update_entry` with added empty item, unknown key, required dim, and cross-dimension checks

- [ ] **Step 1: Add item, unknown key, required dim, and cross-dimension checks to `update_entry`**

In `update_entry`, replace the existing `parse_duration` and `validate_required_dimensions`/`validate_cross_dimension_constraints` blocks (lines 595-607) with:

```rust
    if let Some(ref item) = update.item {
        if item.trim().is_empty() {
            return Err("Entry item cannot be empty".to_string());
        }
    }
    if let Some(ref dur_str) = update.duration {
        parse_duration(dur_str)?;
    }
    if let Some(ref dims) = update.dimensions {
        let effective = files::resolve_month_dimensions(root, year, month)?;
        let known_keys: std::collections::HashSet<&str> = effective
            .iter()
            .map(|d| d.key.as_str())
            .collect();
        for key in dims.keys() {
            if !known_keys.contains(key.as_str()) {
                return Err(format!("Unknown dimension key '{}'", key));
            }
        }
        validate_required_dimensions(&effective, dims)?;
        {
            let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
            let goal_key = goal_dim_key(root, year, month)?;
            let role_key = role_dim_key(root, year, month)?;
            let (_, role_to_goals) = build_commitment_maps(&commitments);
            validate_cross_dimension_constraints(dims, &role_key, &goal_key, &role_to_goals)?;
        }
    }
```

- [ ] **Step 2: Run existing tests to verify no regression**

```bash
cd src-tauri && cargo test -p tauri_app_lib && cargo test --test cli_integration -- --nocapture
```

Expected: All existing tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add empty item and unknown dimension key validation to update_entry"
```

---

### Task 4: Add empty value string check to `validate_dimensions` in `config.rs`

**Files:**
- Modify: `src-tauri/src/config.rs:105-112` (`validate_dimensions`)

**Interfaces:**
- Consumes: `Dimension`
- Produces: `validate_dimensions` now rejects `values` containing empty/whitespace-only strings

- [ ] **Step 1: Write failing unit test for empty value string**

Add to the `#[cfg(test)] mod tests` block in `config.rs` (after `test_validate_dimensions_empty_values` at line 565):

```rust
    #[test]
    fn test_validate_dimensions_empty_value_string() {
        let config = Template {
            dimensions: vec![Dimension {
                name: "Cat".into(),
                key: "cat".into(),
                source: "static".into(),
                values: Some(vec!["ok".into(), "".into()]),
                required: false,
                deleted: false,
            }],
        };
        let errors = validate_dimensions(&config.dimensions);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "ValuesEmpty");
    }

    #[test]
    fn test_validate_dimensions_whitespace_only_value_string() {
        let config = Template {
            dimensions: vec![Dimension {
                name: "Cat".into(),
                key: "cat".into(),
                source: "static".into(),
                values: Some(vec!["   ".into()]),
                required: false,
                deleted: false,
            }],
        };
        let errors = validate_dimensions(&config.dimensions);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "ValuesEmpty");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd src-tauri && cargo test test_validate_dimensions_empty_value_string test_validate_dimensions_whitespace_only_value_string -- --nocapture
```

Expected: Both tests FAIL (empty list passes current check, empty string in values passes current check).

- [ ] **Step 3: Implement empty value string check**

In `config.rs`, replace the existing `Some(vals) if vals.is_empty()` arm (lines 105-111):

Old:
```rust
                Some(vals) if vals.is_empty() => errors.push(ConfigErrorDetail {
                    kind: "ValuesEmpty".to_string(),
                    message: format!(
                        "Dimension '{}' (key: {}): values list is empty",
                        dim.name, dim.key
                    ),
                }),
                _ => {}
```

New:
```rust
                Some(vals) if vals.is_empty() => errors.push(ConfigErrorDetail {
                    kind: "ValuesEmpty".to_string(),
                    message: format!(
                        "Dimension '{}' (key: {}): values list is empty",
                        dim.name, dim.key
                    ),
                }),
                Some(vals) if vals.iter().any(|v| v.trim().is_empty()) => {
                    errors.push(ConfigErrorDetail {
                        kind: "ValuesEmpty".to_string(),
                        message: format!(
                            "Dimension '{}' (key: {}): values list contains an empty or whitespace-only entry",
                            dim.name, dim.key
                        ),
                    });
                }
                _ => {}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd src-tauri && cargo test test_validate_dimensions_empty_value_string test_validate_dimensions_whitespace_only_value_string -- --nocapture
```

Expected: Both tests PASS.

- [ ] **Step 5: Run all existing tests to verify no regression**

```bash
cd src-tauri && cargo test -p tauri_app_lib
```

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "feat: reject empty/whitespace-only values in static dimension values"
```

---

### Task 5: Add CLI integration tests for new rejection paths

**Files:**
- Modify: `src-tauri/tests/cli_integration.rs` (after line 581)

**Interfaces:**
- Consumes: `setup_fixture`, `run_with_stdin` (existing test helpers)
- Produces: Three new tests verifying CLI rejection behavior

- [ ] **Step 1: Add CLI rejection tests**

Add after line 581 (end of file):

```rust
#[test]
fn test_entries_add_rejects_empty_item() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_empty_item");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"","duration":"1h","dimensions":{}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], input);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Entry item cannot be empty"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_add_rejects_unknown_dim_key() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_unknown_key");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"Work","duration":"1h","dimensions":{"nonexistent":"x"}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], input);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown dimension key"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_dimensions_set_rejects_empty_value_string() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_dims_set_empty_val");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"[{"name":"X","key":"x","source":"static","values":["a",""]}]"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "dimensions", "set", "--year", "2026", "--month", "6",
    ], input);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ValuesEmpty") || stderr.contains("empty"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Run the new CLI tests**

```bash
cd src-tauri && cargo test --test cli_integration test_entries_add_rejects_empty_item test_entries_add_rejects_unknown_dim_key test_dimensions_set_rejects_empty_value_string -- --nocapture
```

Expected: All 3 tests PASS.

- [ ] **Step 3: Run all CLI integration tests to verify no regression**

```bash
cd src-tauri && cargo test --test cli_integration -- --nocapture
```

Expected: All existing + new CLI tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/cli_integration.rs
git commit -m "test: add CLI rejection tests for empty item, unknown key, empty static value"
```

---

### Task 6: Final verification

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

```bash
cd src-tauri && cargo test
```

Expected: All tests PASS (lib unit tests + integration tests + CLI integration tests).

- [ ] **Step 2: Run `cargo check` for type errors**

```bash
cd src-tauri && cargo check
```

Expected: No errors, no warnings.

- [ ] **Step 3: Run frontend type check**

```bash
pnpm vue-tsc --noEmit
```

Expected: No errors (frontend unchanged, but verify nothing broke).

- [ ] **Step 4: Commit (if any changes from verification)**

```bash
git status
```
