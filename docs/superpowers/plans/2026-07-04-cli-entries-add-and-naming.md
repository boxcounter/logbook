# CLI — entries add + 动词命名约定 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `entries add` command (stdin JSON, reuses existing `append_entry`) and standardise CLI verb naming (`dimensions get` → `list`, `entries` flat → subcommand).

**Architecture:** CLI layer (`cli/mod.rs`, `cli/entries.rs`, `cli/dimensions.rs`) handles argument parsing and output formatting. All business logic reuses existing `commands::append_entry` via `crate::commands` — no new Rust business logic needed.

**Tech Stack:** Rust, clap 4 (derive), serde_json, existing `tauri_app_lib` modules.

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/src/cli/mod.rs` | Modify | Restructure `Commands::Entries` to subcommand; route `EntryAction::Add` |
| `src-tauri/src/cli/entries.rs` | Modify | Add `add()` function; make `list()` public |
| `src-tauri/src/cli/dimensions.rs` | Modify | Rename `DimensionsCommands::Get` → `List` |
| `src-tauri/tests/cli_integration.rs` | Modify | Add `entries add` tests; update `entries list` tests for new syntax; add `dimensions list` test |
| `docs/naming-conventions.md` | Modify | Add CLI 动词约定 chapter |
| `docs/superpowers/specs/2026-06-15-cli-design.md` | Modify | Update command清单 for new syntax + entries add |

---

### Task 1: Rename `dimensions get` → `dimensions list`

**Files:**
- Modify: `src-tauri/src/cli/dimensions.rs`
- Modify: `src-tauri/tests/cli_integration.rs` (add `dimensions list` test)

- [ ] **Step 1: Rename variant and update doc string**

In `src-tauri/src/cli/dimensions.rs`, rename variant `Get` to `List` and update doc string:

```rust
#[derive(Subcommand)]
pub enum DimensionsCommands {
    /// List dimensions for a month or the template
    List {
```

And rename the match arm label in `handle_dimensions`:

```rust
pub fn handle_dimensions(cmd: DimensionsCommands, root: &Path) -> Result<(), String> {
    match cmd {
        DimensionsCommands::List { year, month, template, json } => {
```

- [ ] **Step 2: Verify compiles**

```bash
cd src-tauri && cargo check
```

Expected: no errors.

- [ ] **Step 3: Add `dimensions list` integration test**

Append to `src-tauri/tests/cli_integration.rs`:

```rust
#[test]
fn test_dimensions_list() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_dims_list");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "dimensions", "list", "--year", "2026", "--month", "6",
    ]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"Goal\""), "stdout: {}", stdout);
    assert!(stdout.contains("\"Role\""), "stdout: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 4: Run test**

```bash
cd src-tauri && cargo test test_dimensions_list
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cli/dimensions.rs src-tauri/tests/cli_integration.rs
git commit -m "refactor: rename dimensions get → list"
```

---

### Task 2: Restructure `entries` to subcommand (`list` / `add`)

**Files:**
- Modify: `src-tauri/src/cli/mod.rs`
- Modify: `src-tauri/src/cli/entries.rs`
- Modify: `src-tauri/tests/cli_integration.rs` (update test syntax)

- [ ] **Step 1: Define `EntryAction` enum and update `Commands::Entries`**

In `src-tauri/src/cli/mod.rs`, replace:

```rust
#[derive(Subcommand)]
pub enum Commands {
    /// List, view progress, or set monthly commitments
    Commitments {
        #[command(subcommand)]
        action: CommitmentAction,
    },
    /// List entries for a date
    Entries {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
    },
    /// Get or set dimensions for a month or the template
    #[command(subcommand)]
    Dimensions(DimensionsCommands),
}
```

With:

```rust
#[derive(Subcommand)]
pub enum Commands {
    /// List, view progress, or set monthly commitments
    Commitments {
        #[command(subcommand)]
        action: CommitmentAction,
    },
    /// List or add entries
    Entries {
        #[command(subcommand)]
        action: EntryAction,
    },
    /// List or set dimensions for a month or the template
    #[command(subcommand)]
    Dimensions(DimensionsCommands),
}
```

And after `CommitmentAction`, add:

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
}
```

- [ ] **Step 2: Route `EntryAction` in `run()`**

In the `match cli.command` block within `run()`, replace:

```rust
        Commands::Entries { date } => {
            entries::list(&root, &date, cli.json);
        }
```

With:

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

- [ ] **Step 3: Make `entries::list` public and add `entries::add`**

In `src-tauri/src/cli/entries.rs`, change `fn list` to `pub fn list` (was `pub` already — verify). Then add the `add` function after `list`:

```rust
pub fn add(root: &Path, date: &str, json: bool) {
    use std::io::Read;

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| {
            output::print_error(&format!("Failed to read stdin: {}", e));
            std::process::exit(1);
        });

    let entry_input: crate::models::CreateEntryInput =
        serde_json::from_str(&input).unwrap_or_else(|e| {
            output::print_error(&format!(
                "Failed to parse input as CreateEntryInput JSON.\n\
                 Expected: {{\"item\":\"...\",\"duration\":\"...\",\"dimensions\":{{...}}}}\n\
                 Error: {}",
                e
            ));
            std::process::exit(1);
        });

    let entry = crate::commands::append_entry(
        root.to_string_lossy().into_owned(),
        date.to_string(),
        entry_input,
    )
    .unwrap_or_else(|e| {
        output::print_error(&e);
        std::process::exit(1);
    });

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&entry).expect("Failed to serialize entry")
        );
    } else {
        let dims: Vec<String> = entry
            .dimensions
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        println!(
            "Added: \"{}\" | {}m | {}",
            entry.item,
            entry.duration,
            dims.join(", ")
        );
    }
}
```

- [ ] **Step 4: Verify compiles**

```bash
cd src-tauri && cargo check
```

Expected: no errors.

- [ ] **Step 5: Update existing `entries` tests for new syntax**

In `src-tauri/tests/cli_integration.rs`, update `test_entries_list` (line 389):

Change `"entries", "--date", "2026-06-15",` to `"entries", "list", "--date", "2026-06-15",`.

And `test_entries_list_human` (line 413):

Change `"entries", "--date", "2026-06-15",` to `"entries", "list", "--date", "2026-06-15",`.

- [ ] **Step 6: Run existing entries tests**

```bash
cd src-tauri && cargo test test_entries
```

Expected: PASS for `test_entries_list` and `test_entries_list_human`.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/cli/mod.rs src-tauri/src/cli/entries.rs src-tauri/tests/cli_integration.rs
git commit -m "refactor: restructure entries to subcommand with list/add"
```

---

### Task 3: Add `entries add` integration tests

**Files:**
- Modify: `src-tauri/tests/cli_integration.rs`

- [ ] **Step 1: Add test — valid entry with dimensions**

Append to `src-tauri/tests/cli_integration.rs`:

```rust
#[test]
fn test_entries_add_valid_with_dimensions() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"Code review","duration":"30m","dimensions":{"role":"Dev","goal":"Review"}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Code review"), "stdout: {}", stdout);
    assert!(stdout.contains("30m"), "stdout: {}", stdout);

    // Verify day file was written
    let day_path = tmp.join("2026").join("06").join("2026-06-15.md");
    assert!(day_path.exists(), "day file not created");
    let content = fs::read_to_string(&day_path).unwrap();
    assert!(content.contains("Code review"));

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Add test — valid entry with minimal fields (no dimensions)**

```rust
#[test]
fn test_entries_add_minimal() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_min");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"Coffee break","duration":"15m"}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Coffee break"), "stdout: {}", stdout);
    assert!(stdout.contains("15m"), "stdout: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 3: Add test — invalid JSON**

```rust
#[test]
fn test_entries_add_bad_json() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_bad_json");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], "not json");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to parse"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 4: Add test — invalid date**

```rust
#[test]
fn test_entries_add_bad_date() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_bad_date");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"x","duration":"10m"}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "not-a-date",
    ], input);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid date format") || stderr.contains("Error"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 5: Add test — empty stdin**

```rust
#[test]
fn test_entries_add_empty_stdin() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_empty");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], "");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to parse"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 6: Add test — JSON output mode returns Entry**

```rust
#[test]
fn test_entries_add_json_output() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_json");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"Write docs","duration":"60m","dimensions":{"role":"Dev"}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "add", "--date", "2026-06-15",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"id\""), "stdout: {}", stdout);
    assert!(stdout.contains("\"Write docs\""), "stdout: {}", stdout);
    assert!(stdout.contains("\"duration\""), "stdout: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 7: Run all new tests**

```bash
cd src-tauri && cargo test test_entries_add
```

Expected: 6 tests PASS.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/tests/cli_integration.rs
git commit -m "test: add entries add integration tests"
```

---

### Task 4: Add CLI 动词约定 to `docs/naming-conventions.md`

**Files:**
- Modify: `docs/naming-conventions.md`

- [ ] **Step 1: Append new chapter**

Add after the existing chapter 5 (after the "常见误判" block):

```markdown
## 6. CLI 动词约定

### 动词矩阵

| 动词 | 操作 | 适用 |
|------|------|------|
| `list` | 取集合 | `entries list`, `commitments list`, `dimensions list` |
| `get` | 取单体 | 暂无，未来 `entries get --id <uuid>` |
| `add` | 集合内新增 | `entries add` |
| `update` | 修改单体 | 未来 `entries update --id <uuid>` |
| `delete` | 删除单体 | 未来 `entries delete --id <uuid>` |
| `set` | 整体替换 | `commitments set`, `dimensions set` |
| `progress` | 衍生计算视图 | `commitments progress`（领域专用名，非 CRUD） |

### 区分规则

- **`list` vs `get`**：资源是集合用 `list`，是单件用 `get`。不互为别名。
- **`set` vs `update`**：`set` 操作资源本身（整体替换），`update` 操作集合中单个成员（需要 `--id` 或 `--key` 定位）。
- **stdin 约定**：所有写入命令（`add`、`set`）统一从 stdin 读取 JSON 或 YAML，不用 CLI flags 分散传参。
```

- [ ] **Step 2: Verify file is valid markdown**

```bash
head -5 docs/naming-conventions.md
```

(Visual check — no broken syntax.)

- [ ] **Step 3: Commit**

```bash
git add docs/naming-conventions.md
git commit -m "docs: add CLI 动词约定 to naming-conventions"
```

---

### Task 5: Update CLI design spec

**Files:**
- Modify: `docs/superpowers/specs/2026-06-15-cli-design.md`

- [ ] **Step 1: Update command list and add `entries add` section**

In `docs/superpowers/specs/2026-06-15-cli-design.md`, update the "命令清单" section:

Replace:
```
logbook-cli entries list         --date 2026-06-15
```
With:
```
logbook-cli entries list         --date 2026-06-15
logbook-cli entries add          --date 2026-06-15    # 从 stdin 读 JSON
```

And update the dimensions line:
```
logbook-cli dimensions list      --year 2026 --month 6
logbook-cli dimensions set       --year 2026 --month 7    # 从 stdin 读 YAML/JSON
```

After the `entries list` section, append:

```markdown
### entries add

从 stdin 读 `CreateEntryInput` JSON，复用 `append_entry` 创建条目。

```
$ echo '{"item":"Code review","duration":"30m","dimensions":{"role":"Dev"}}' | logbook-cli entries add --date 2026-06-15
Added: "Code review" | 30m | role=Dev
```

`duration` 支持 `parse_duration` 的所有格式（`30m`、`1h30m`、`120` 等）。dimensions 可选，省略时为 `{}`。
```

Update the "不在 scope" section:
```
- 不支持 entry CRUD（append/update/delete）——按需后续加
```
With:
```
- `entries add` 已实现（2026-07-04）
- 不支持 entry update/delete——按需后续加
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/2026-06-15-cli-design.md
git commit -m "docs: update CLI spec for entries add and dimensions list"
```

---

### Task 6: Run full test suite

- [ ] **Step 1: Run all tests**

```bash
cd src-tauri && cargo test
```

Expected: all tests PASS (lib tests + integration tests).

- [ ] **Step 2: Run frontend tests**

```bash
pnpm test
```

Expected: all vitest tests PASS.

---

### Task 7: Verify --help output reflects changes

- [ ] **Step 1: Build the CLI binary**

```bash
cd src-tauri && cargo build --bin logbook-cli
```

- [ ] **Step 2: Check help output**

```bash
./target/debug/logbook-cli --help
```

Expected: `entries` shows `list` and `add` subcommands.

```bash
./target/debug/logbook-cli entries --help
```

Expected: shows `list` and `add` subcommands with `--date` argument.

```bash
./target/debug/logbook-cli dimensions --help
```

Expected: shows `list` (not `get`) and `set`.

- [ ] **Step 3: Commit if no issues**

(No code to commit — verification-only step.)
