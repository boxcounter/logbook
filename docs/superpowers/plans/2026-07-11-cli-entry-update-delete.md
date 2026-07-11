# CLI Entry Update/Delete Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `entries update` and `entries delete` CLI subcommands that wrap the existing `commands::update_entry` / `commands::delete_entry` backends, enabling AI Agents to modify entries programmatically.

**Architecture:** Thin CLI wrappers over existing Tauri commands — same pattern as `entries add` wrapping `commands::append_entry`. `update` reads `UpdateEntryInput` JSON from stdin, calls the backend, extracts the single updated Entry from the returned DayFile, and outputs it. `delete` calls the backend and outputs an operation confirmation (no data). Both are classified as write commands (require instance lock).

**Tech Stack:** Rust, clap 4 (derive), serde_json, existing `commands::update_entry` / `commands::delete_entry`.

## Global Constraints

- CLI write commands read input from stdin as JSON (consistent with `entries add`, `commitments set`, `dimensions set`).
- `is_read_only()` in `cli/mod.rs` is the single source of truth for read/write classification — new write commands must fall into `false`.
- Backend is the sole authority for data validation (`src-tauri/AGENTS.md:54`) — CLI layer does not re-validate; it parses stdin and forwards errors.
- YAML serialization uses `yaml_serde` 0.10 (not `serde_yml`).
- Integration tests isolate via `LOGBOOK_LOG_DIR` + `LOGBOOK_LOCK_DIR` env vars and clean up temp dirs.
- No hardcoded fallbacks (`src-tauri/AGENTS.md:53`).

**Spec:** `docs/superpowers/specs/2026-07-11-cli-entry-update-delete-design.md`

---

### Task 1: Add EntryAction variants, dispatch arms, and stub functions

This task wires up the clap subcommand structure so the CLI can parse `entries update` / `entries delete`. The actual logic is implemented in Tasks 2 and 3.

**Files:**
- Modify: `src-tauri/src/cli/mod.rs:93-107` (EntryAction enum), `src-tauri/src/cli/mod.rs:195-202` (dispatch), `src-tauri/src/cli/mod.rs:254-273` (unit tests)
- Modify: `src-tauri/src/cli/entries.rs` (add stub functions)

**Interfaces:**
- Produces: `EntryAction::Update { date, entry_id }`, `EntryAction::Delete { date, entry_id }` variants; `entries::update(root, date, entry_id, json)` and `entries::delete(root, date, entry_id, json)` function signatures (stub bodies, replaced in Tasks 2-3).

- [ ] **Step 1: Add stub functions to `cli/entries.rs`**

Add these two stub functions at the end of `src-tauri/src/cli/entries.rs` (after `format_entries_human`):

```rust
pub fn update(root: &Path, date: &str, entry_id: &str, json: bool) {
    let _ = (root, date, entry_id, json);
    todo!("implement in Task 2");
}

pub fn delete(root: &Path, date: &str, entry_id: &str, json: bool) {
    let _ = (root, date, entry_id, json);
    todo!("implement in Task 3");
}
```

These stubs exist so the dispatch in Step 3 compiles. They are replaced with real logic in Tasks 2 and 3.

- [ ] **Step 2: Write the failing unit tests in `cli/mod.rs`**

In `src-tauri/src/cli/mod.rs`, find the `test_write_commands` function (around line 254). Add these two assertions at the end, before the closing `}`:

```rust
        assert!(!Commands::Entries {
            action: EntryAction::Update {
                date: "2026-07-11".to_string(),
                entry_id: "test-id".to_string(),
            },
        }
        .is_read_only());
        assert!(!Commands::Entries {
            action: EntryAction::Delete {
                date: "2026-07-11".to_string(),
                entry_id: "test-id".to_string(),
            },
        }
        .is_read_only());
```

- [ ] **Step 3: Run tests to verify they fail (compile error)**

Run: `cd src-tauri && cargo test -p tauri_app_lib test_write_commands -- --exact`
Expected: COMPILE ERROR — `EntryAction::Update` / `EntryAction::Delete` do not exist yet.

- [ ] **Step 4: Add EntryAction enum variants**

In `src-tauri/src/cli/mod.rs`, replace the `EntryAction` enum (lines 93-107):

```rust
#[derive(Subcommand)]
pub enum EntryAction {
    /// List entries for a date
    List {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
    },
    /// Add an entry (read JSON from stdin)
    Add {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
    },
    /// Update an entry (read JSON from stdin)
    Update {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
        /// Entry ID to update
        #[arg(long)]
        entry_id: String,
    },
    /// Delete an entry
    Delete {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
        /// Entry ID to delete
        #[arg(long)]
        entry_id: String,
    },
}
```

- [ ] **Step 5: Add dispatch arms**

In `src-tauri/src/cli/mod.rs`, find the `Commands::Entries` match block (lines 195-202) and add two arms. Replace:

```rust
        Commands::Entries { action } => match action {
            EntryAction::List { date } => {
                entries::list(&root, &date, cli.json);
            }
            EntryAction::Add { date } => {
                entries::add(&root, &date, cli.json);
            }
        },
```

with:

```rust
        Commands::Entries { action } => match action {
            EntryAction::List { date } => {
                entries::list(&root, &date, cli.json);
            }
            EntryAction::Add { date } => {
                entries::add(&root, &date, cli.json);
            }
            EntryAction::Update { date, entry_id } => {
                entries::update(&root, &date, &entry_id, cli.json);
            }
            EntryAction::Delete { date, entry_id } => {
                entries::delete(&root, &date, &entry_id, cli.json);
            }
        },
```

- [ ] **Step 6: Run unit tests to verify they pass**

Run: `cd src-tauri && cargo test -p tauri_app_lib test_write_commands -- --exact`
Expected: PASS — both new assertions pass (Update/Delete are write commands → `is_read_only()` returns false).

- [ ] **Step 7: Verify CLI binary builds and parses new subcommands**

Run: `cd src-tauri && cargo build --bin logbook-cli`
Expected: builds successfully.

Run: `cd src-tauri && ./target/debug/logbook-cli entries --help`
Expected: help output lists `list`, `add`, `update`, `delete`.

Run: `cd src-tauri && ./target/debug/logbook-cli entries update --help`
Expected: help output shows `--date` and `--entry-id` flags.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/cli/mod.rs src-tauri/src/cli/entries.rs
git commit -m "feat(cli): wire up entries update/delete subcommands (stubs)"
```

---

### Task 2: Implement `entries update` — change item and/or dimensions

Implement the `update` function: read `UpdateEntryInput` JSON from stdin, call `commands::update_entry`, extract the single updated Entry from the returned DayFile, output it.

**Files:**
- Modify: `src-tauri/src/cli/entries.rs` (replace `update` stub)
- Test: `src-tauri/tests/cli_integration.rs`

**Interfaces:**
- Consumes: `commands::update_entry(root_path: String, date: String, entry_id: String, update: UpdateEntryInput) -> Result<DayFile, String>` (from `commands.rs:666`)
- Consumes: `models::UpdateEntryInput { item: Option<String>, duration: Option<String>, dimensions: Option<BTreeMap<String, String>> }` (from `models.rs:81`)
- Produces: working `entries update` CLI command.

- [ ] **Step 1: Write the failing integration test for updating item + dimensions**

In `src-tauri/tests/cli_integration.rs`, add this test at the end of the file:

```rust
#[test]
fn test_entries_update_changes_item_and_dimensions() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_update");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_commitments(&tmp, 2026, 6, "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it\n    - Review");
    setup_day_file(
        &tmp,
        "2026-06-15",
        "entries:\n  - id: e1\n    item: Old item\n    duration: 30\n    dimensions:\n      role: Dev\n      goal: Review",
    );

    // Update: change item text + switch goal from Review to Ship it
    let input = r#"{"item":"Updated item","dimensions":{"role":"Dev","goal":"Ship it"}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "update", "--date", "2026-06-15", "--entry-id", "e1",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated item"), "stdout: {}", stdout);
    assert!(stdout.contains("Ship it"), "stdout: {}", stdout);
    assert!(!stdout.contains("Review"), "old goal should be replaced: {}", stdout);

    // Verify the change persisted via entries list
    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "list", "--date", "2026-06-15",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated item"), "update did not persist: {}", stdout);
    assert!(stdout.contains("Ship it"), "dimension update did not persist: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Write the failing integration test for bad JSON input**

Add this test below the previous one:

```rust
#[test]
fn test_entries_update_bad_json() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_update_bad_json");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_day_file(
        &tmp,
        "2026-06-15",
        "entries:\n  - id: e1\n    item: Keep\n    duration: 30\n    dimensions: {}",
    );

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "update", "--date", "2026-06-15", "--entry-id", "e1",
    ], "not json");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to parse"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --test cli_integration test_entries_update -- --exact`
Expected: FAIL — the `update` stub calls `todo!()` which panics (non-zero exit).

- [ ] **Step 4: Implement the `update` function**

In `src-tauri/src/cli/entries.rs`, replace the `update` stub function with:

```rust
pub fn update(root: &Path, date: &str, entry_id: &str, json: bool) {
    if let Err(e) = crate::integrity::check() {
        output::print_error(&e);
        std::process::exit(1);
    }

    use std::io::Read;

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| {
            output::print_error(&format!("Failed to read stdin: {}", e));
            std::process::exit(1);
        });

    let update_input: crate::models::UpdateEntryInput =
        serde_json::from_str(&input).unwrap_or_else(|e| {
            output::print_error(&format!(
                "Failed to parse input as UpdateEntryInput JSON.\n\
                 Expected: {{\"item\":\"...\",\"duration\":\"...\",\"dimensions\":{{...}}}}\n\
                 Error: {}",
                e
            ));
            std::process::exit(1);
        });

    let day_file = crate::commands::update_entry(
        root.to_string_lossy().into_owned(),
        date.to_string(),
        entry_id.to_string(),
        update_input,
    )
    .unwrap_or_else(|e| {
        output::print_error(&e);
        std::process::exit(1);
    });

    // Extract the single updated entry from the returned DayFile.
    // update_entry succeeds only if the entry exists, so find() cannot fail.
    let entry = day_file
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .unwrap_or_else(|| {
            output::print_error("Updated entry not found in result");
            std::process::exit(1);
        });

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(entry).expect("Failed to serialize entry")
        );
    } else {
        let dims: Vec<String> = entry
            .dimensions
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        println!(
            "Updated: \"{}\" | {}m | {}",
            entry.item,
            entry.duration,
            dims.join(", ")
        );
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --test cli_integration test_entries_update`
Expected: PASS — both `test_entries_update_changes_item_and_dimensions` and `test_entries_update_bad_json` pass.

- [ ] **Step 6: Run full test suite to verify no regressions**

Run: `cd src-tauri && cargo test`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/cli/entries.rs src-tauri/tests/cli_integration.rs
git commit -m "feat(cli): implement entries update command"
```

---

### Task 3: Implement `entries delete` — remove an entry by ID

Implement the `delete` function: call `commands::delete_entry`, output an operation confirmation (no entry data, since the entry no longer exists).

**Files:**
- Modify: `src-tauri/src/cli/entries.rs` (replace `delete` stub)
- Test: `src-tauri/tests/cli_integration.rs`

**Interfaces:**
- Consumes: `commands::delete_entry(root_path: String, date: String, entry_id: String) -> Result<DayFile, String>` (from `commands.rs:748`)
- Produces: working `entries delete` CLI command.

- [ ] **Step 1: Write the failing integration test for deleting an entry**

In `src-tauri/tests/cli_integration.rs`, add this test at the end of the file:

```rust
#[test]
fn test_entries_delete_removes_entry() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_delete");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_day_file(
        &tmp,
        "2026-06-15",
        "entries:\n  - id: e1\n    item: Delete me\n    duration: 30\n    dimensions:\n      goal: Review\n  - id: e2\n    item: Keep me\n    duration: 20\n    dimensions:\n      goal: Review",
    );

    // Delete e1
    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "delete", "--date", "2026-06-15", "--entry-id", "e1",
    ]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"ok\": true"), "stdout: {}", stdout);
    assert!(stdout.contains("e1"), "stdout should echo entry_id: {}", stdout);

    // Verify e1 is gone and e2 remains via entries list
    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "list", "--date", "2026-06-15",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Delete me"), "deleted entry still present: {}", stdout);
    assert!(stdout.contains("Keep me"), "remaining entry missing: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Write the failing integration test for human-readable output**

Add this test below the previous one:

```rust
#[test]
fn test_entries_delete_human_output() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_delete_human");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_day_file(
        &tmp,
        "2026-06-15",
        "entries:\n  - id: abc123\n    item: Temp\n    duration: 10\n    dimensions: {}",
    );

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "delete", "--date", "2026-06-15", "--entry-id", "abc123",
    ]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Deleted: abc123"), "stdout: {}", stdout);
    assert!(stdout.contains("2026-06-15"), "stdout: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --test cli_integration test_entries_delete`
Expected: FAIL — the `delete` stub calls `todo!()` which panics (non-zero exit).

- [ ] **Step 4: Implement the `delete` function**

In `src-tauri/src/cli/entries.rs`, replace the `delete` stub function with:

```rust
pub fn delete(root: &Path, date: &str, entry_id: &str, json: bool) {
    if let Err(e) = crate::integrity::check() {
        output::print_error(&e);
        std::process::exit(1);
    }

    crate::commands::delete_entry(
        root.to_string_lossy().into_owned(),
        date.to_string(),
        entry_id.to_string(),
    )
    .unwrap_or_else(|e| {
        output::print_error(&e);
        std::process::exit(1);
    });

    if json {
        println!(
            "{}",
            serde_json::json!({"ok": true, "date": date, "entry_id": entry_id})
        );
    } else {
        println!("Deleted: {} from {}", entry_id, date);
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --test cli_integration test_entries_delete`
Expected: PASS — both `test_entries_delete_removes_entry` and `test_entries_delete_human_output` pass.

- [ ] **Step 6: Run full test suite to verify no regressions**

Run: `cd src-tauri && cargo test`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/cli/entries.rs src-tauri/tests/cli_integration.rs
git commit -m "feat(cli): implement entries delete command"
```

---

## Notes for the implementer

- **Why `update` returns a single Entry, not DayFile:** the backend `commands::update_entry` returns `DayFile` (the whole day). The CLI extracts the updated entry by `entry_id` so the output matches `entries add` (single Entry). This is a deliberate choice — see spec §输出格式.

- **Why `delete` returns no data:** the entry no longer exists after deletion. Returning a confirmation object (`{"ok": true, ...}`) is sufficient; the agent can verify via `entries list` if needed. See spec §输出格式.

- **`is_read_only()` needs no change:** the existing `matches!(action, EntryAction::List { .. })` at `cli/mod.rs:120` only matches `List`. `Update` and `Delete` automatically fall through to `false` (write command → requires instance lock). This is verified by the unit test in Task 1.

- **Integrity check:** both `update` and `delete` call `integrity::check()` at the start (same as `entries::add`). The backend commands also call it, but the CLI checks first for early failure with a clear error message before reading stdin.

- **Test isolation:** all integration tests use `LOGBOOK_LOG_DIR` + `LOGBOOK_LOCK_DIR` (set by the `run` / `run_with_stdin` helpers) to avoid polluting real product directories.
