# Monthly Dimension Templates — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let every month have its own dimension set and values: a global `template.yaml` is snapshotted into a month's `_monthly.md` on the month's first write; thereafter that month is self-contained and template edits don't affect it.

**Architecture:** Rename global `config.yaml` → `template.yaml` (the default dimension set). `MonthlyFile` gains a `dimensions` block. A month's *effective dimensions* = its own block if present, else the template. On first write (`append_entry`/`update_entry`/`delete_entry`/`set_day_note`/`set_commitments`) a single `ensure_month_instantiated` helper snapshots the template into the month, preserving any existing commitments. Pure reads never write. The frontend's `dimensions` becomes month-scoped state refreshed on month navigation, instead of a global injected once at init.

**Tech Stack:** Rust (Tauri 2.x commands, `yaml_serde`, `notify`), Vue 3 `<script setup>` + TypeScript, `reactive()` store via provide/inject.

**Spec:** `docs/superpowers/specs/2026-06-21-monthly-dimension-templates-design.md`

**Conventions to obey:**
- Rust tests: pure/no-IO → `#[cfg(test)] mod tests` in `src/`; touches filesystem → `tests/` integration. (`src-tauri/CLAUDE.md`)
- Atomic writes (temp + rename) already handled by `files::write_*`.
- Frontend design tokens: spacing `--spacing-*` (`gap-sm`/`p-md`), text `text-secondary/micro`, line-height follows tier (`docs/.../design-system-consolidation`). No raw px for spacing, no `text-sm`.
- Verify gate (`pnpm run verify` / Stop hook): `pnpm vue-tsc --noEmit && cd src-tauri && cargo check && cargo test`.

**Commands (run from these dirs):**
- Rust: `cd src-tauri && cargo test`
- A single Rust test: `cd src-tauri && cargo test <name> -- --exact --nocapture` (omit `--exact` for substring match)
- Frontend typecheck: `pnpm vue-tsc --noEmit` (repo root)
- Frontend unit tests: `pnpm test` (vitest)

---

## File Structure

**Rust (`src-tauri/src/`):**
- `models.rs` — `MonthlyFile.dimensions` field; rename `Config`→`Template`; new `MonthDimensions`; reshape `InitResult::Ready`.
- `files.rs` — `config_path`→`template_path`, `read_config`→`read_template`; new `resolve_month_dimensions`, `ensure_month_instantiated`, `year_month_from_date`.
- `config.rs` — `validate_config`→`validate_dimensions(&[Dimension])`; `validate_monthly` also validates the month's dimensions; watcher watches `template.yaml`.
- `commands.rs` — `validate_required_dimensions(&[Dimension], …)`; wire `ensure_month_instantiated`+resolve into 5 write commands; new `get_month_dimensions`; `create_starter_files` writes `template.yaml`; `init`/`set_root_path` return month-scoped dims.
- `operation_log.rs` — replay copies `template.yaml`.
- `lib.rs` — register `get_month_dimensions`.

**Frontend (`src/`):**
- `types.ts` — reshape `InitResult` Ready; add `MonthDimensions`; drop `Config` usage in store.
- `stores/useStore.ts` — `config` → `dimensions: Dimension[]` + `fromTemplate: boolean`.
- `App.vue`, `components/SetupScreen.vue` — set `store.dimensions`/`fromTemplate` from init result.
- `components/MonthView.vue` — `loadMonth` fetches `get_month_dimensions`; replace `store.config?.dimensions`; render preview indicator.
- `components/composite/EntryRow.vue` — `store.config?.dimensions` → `store.dimensions`.

**Tests / fixtures:**
- `src-tauri/tests/fixtures/config.yaml` → `template.yaml`; `tests/fixtures/2026/06/_monthly.md` gains a `dimensions:` block (optional, for a fixture-based test).
- Integration tests writing `config.yaml`: `op_log_verify_integration.rs`, `entry_crud_integration.rs`, `cli_integration.rs`, `commitment_progress_integration.rs`, `commitment_editor_integration.rs`, `scan_integration.rs`.
- Contract `tests/contracts/create_starter_files.yaml`.
- New `tests/monthly_dimensions_integration.rs`.
- Frontend vitest fixtures `src/__tests__/mocks/fixtures.ts` (`makeConfig`→`makeDimensions`) + 6 spec files consuming it.

---

## Phase 1 — Rename `config.yaml` → `template.yaml` (mechanical, behavior-preserving)

> No semantic change. The `Config` *type* stays for now (renamed in Phase 2). End state: `cargo test` green, app reads/writes `template.yaml` everywhere `config.yaml` was used.

### Task 1.1: Rename path helper + reader in `files.rs`

**Files:**
- Modify: `src-tauri/src/files.rs:42-45` (`config_path`), `:205-212` (`read_config`), `:317-322` (test)

- [ ] **Step 1: Update the failing test first**

In `src-tauri/src/files.rs`, replace the `test_config_path` test (lines 317-322):

```rust
    #[test]
    fn test_template_path() {
        let root = Path::new("/data");
        let p = template_path(root);
        assert_eq!(p, PathBuf::from("/data/template.yaml"));
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd src-tauri && cargo test test_template_path`
Expected: FAIL — `cannot find function template_path`.

- [ ] **Step 3: Rename the function and reader**

In `src-tauri/src/files.rs`, replace lines 42-45:

```rust
/// Template path: {root}/template.yaml
pub fn template_path(root: &Path) -> PathBuf {
    root.join("template.yaml")
}
```

Replace `read_config` (lines 205-212):

```rust
/// Read template.yaml. Returns error if file missing.
pub fn read_template(root: &Path) -> Result<Config, String> {
    let path = template_path(root);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    yaml_serde::from_str::<Config>(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}
```

- [ ] **Step 4: Update the internal caller in `files.rs`**

`append_new_entry` (line 96) and `update_entry_in_file` (line 131) call `read_config(root)`. Replace both `read_config(root)` → `read_template(root)`. (Two occurrences.)

- [ ] **Step 5: Run the file's tests**

Run: `cd src-tauri && cargo test --lib files::`
Expected: compile errors in OTHER files (`commands.rs`, `config.rs`, `operation_log.rs`) still referencing `read_config`/`config_path`. That's fine — fixed in 1.2–1.4. Do NOT commit yet.

### Task 1.2: Update `commands.rs` call sites + `create_starter_files`

**Files:**
- Modify: `src-tauri/src/commands.rs:345,391` (`read_config`), `:932-947` (`create_starter_files`)

- [ ] **Step 1: Replace `read_config` call sites**

In `src-tauri/src/commands.rs`, replace `files::read_config(root)` → `files::read_template(root)` at line 345 (`append_entry`) and line 391 (`update_entry`). (Two occurrences.)

- [ ] **Step 2: Rewrite `create_starter_files` to write `template.yaml`**

Replace the body (lines 933-947):

```rust
pub fn create_starter_files(path: String) -> Result<(), String> {
    let root = std::path::Path::new(&path);
    if !root.exists() {
        std::fs::create_dir_all(root).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let template_path = root.join("template.yaml");
    if !template_path.exists() {
        std::fs::write(
            &template_path,
            "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
        )
        .map_err(|e| format!("Failed to write template.yaml: {}", e))?;
    }
    Ok(())
}
```

- [ ] **Step 3: Build**

Run: `cd src-tauri && cargo build`
Expected: errors remain only in `config.rs` (watcher) and `operation_log.rs`. Proceed.

### Task 1.3: Update watcher in `config.rs`

**Files:**
- Modify: `src-tauri/src/config.rs:144-145` (comment), `:170-171` (watch filename + reader)

- [ ] **Step 1: Update the watched filename and reader**

In `src-tauri/src/config.rs`, line 170 change the condition `if file_name == "config.yaml"` → `if file_name == "template.yaml"`. On line 171 change `files::read_config(&root_path)` → `files::read_template(&root_path)`. Update the comment at line 144 `catch config.yaml` → `catch template.yaml`.

- [ ] **Step 2: Build**

Run: `cd src-tauri && cargo build`
Expected: only `operation_log.rs` errors remain (if any reference `read_config`). Proceed.

### Task 1.4: Update `operation_log.rs` replay copy

**Files:**
- Modify: `src-tauri/src/operation_log.rs:176-192` (replay copy), `:660-661` (test fixture write)

- [ ] **Step 1: Rename the copied file in replay**

In `src-tauri/src/operation_log.rs` around lines 176-186, replace the three `config.yaml` literals so replay copies the template:

```rust
    // Copy template to replay dir so dimension validation can read it
    let template_src = root.join("template.yaml");
    if template_src.exists() {
        fs::create_dir_all(&replay_root).map_err(|e| {
            vec![OpLogMismatch {
                date: "".to_string(),
                description: format!("create replay dir: {}", e),
            }]
        })?;
        fs::copy(&template_src, replay_root.join("template.yaml"))
            .map_err(|e| {
                vec![OpLogMismatch {
                    date: "".to_string(),
                    description: format!("copy template: {}", e),
                }]
            })?;
    }
```

- [ ] **Step 2: Update the test fixture write at line 660-661**

Find the test that writes `tmp.join("config.yaml")` (line 660) with content `"dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n"`. Change the path literal `"config.yaml"` → `"template.yaml"`.

- [ ] **Step 3: Build the whole crate**

Run: `cd src-tauri && cargo build`
Expected: PASS (no more `read_config`/`config_path` references).

### Task 1.5: Sweep integration tests + contract fixture + fixtures

**Files:**
- Modify: `src-tauri/tests/fixtures/config.yaml` (rename), `tests/contracts/create_starter_files.yaml`, and every integration test writing `config.yaml`.

- [ ] **Step 1: Rename the fixture file**

Run:
```bash
git mv src-tauri/tests/fixtures/config.yaml src-tauri/tests/fixtures/template.yaml
```

- [ ] **Step 2: Find every remaining `config.yaml` reference under tests**

Run: `grep -rn "config.yaml" src-tauri/tests`
Expected list (paths writing a starter config or asserting the created file):
`op_log_verify_integration.rs`, `entry_crud_integration.rs`, `cli_integration.rs`, `commitment_progress_integration.rs`, `commitment_editor_integration.rs`, `scan_integration.rs`, `contracts/create_starter_files.yaml`.

- [ ] **Step 3: Replace each `config.yaml` literal with `template.yaml`**

In each file above, replace string literals `"config.yaml"` / `config.yaml` (in paths like `root.join("config.yaml")` and any asserted output path) with `template.yaml`. The file *contents* written (the `dimensions:` YAML) stay the same. For `contracts/create_starter_files.yaml`, change the expected created filename to `template.yaml`.

- [ ] **Step 4: Run the full Rust suite**

Run: `cd src-tauri && cargo test`
Expected: PASS. If a test asserts a specific filename in an error message (e.g. `"config.yaml not found"`), update that assertion to `template.yaml`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/
git commit -m "refactor: rename config.yaml to template.yaml (no behavior change)"
```

---

## Phase 2 — `MonthlyFile.dimensions` + validation refactor

> Adds the per-month dimensions field and a reusable dimension validator. Still no behavior change: dimension blocks are empty everywhere, effective dims = template.

### Task 2.1: Rename `Config` → `Template` and add `MonthlyFile.dimensions`

**Files:**
- Modify: `src-tauri/src/models.rs:6-9` (`Config`), `:29-33` (`MonthlyFile`)

- [ ] **Step 1: Rename the struct and add the field**

In `src-tauri/src/models.rs`, replace lines 6-9:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub dimensions: Vec<Dimension>,
}
```

Replace `MonthlyFile` (lines 29-33):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyFile {
    #[serde(default)]
    pub dimensions: Vec<Dimension>,
    #[serde(default)]
    pub commitments: Vec<Commitment>,
}
```

- [ ] **Step 2: Global rename `Config` → `Template`**

Replace every remaining identifier `Config` (the struct) with `Template` across the crate. Affected: `files.rs` (`read_template` return type, import), `config.rs` (`validate_config` signature + tests + import), `commands.rs` (`validate_required_dimensions` param + `make_config` test helper + imports), `models.rs` tests (lines 178, 213, 310 construct `Config { dimensions: vec![] }`).

Run to find them: `grep -rn "\bConfig\b" src-tauri/src`
Note: `notify::Config as NotifyConfig` in `config.rs:4` is a DIFFERENT type — leave it. Only rename `crate::models::Config`.

- [ ] **Step 3: Fix every `MonthlyFile { commitments: … }` literal to include dimensions**

`MonthlyFile { commitments: vec![] }` no longer compiles (missing `dimensions`). Run: `grep -rn "MonthlyFile {" src-tauri/src`
For each, add `dimensions: vec![],`. Locations: `files.rs:175`, `commands.rs:54,187,276,519` (and any test). Example for `files.rs:174-177`:

```rust
        return Ok(MonthlyFile {
            dimensions: vec![],
            commitments: vec![],
        });
```

- [ ] **Step 4: Build**

Run: `cd src-tauri && cargo build`
Expected: PASS (validation refactor is Task 2.2; `validate_config` still exists, now `&Template`).

### Task 2.2: `validate_config` → `validate_dimensions(&[Dimension])` + validate month dims

**Files:**
- Modify: `src-tauri/src/config.rs:15-82` (`validate_config`), `:84-117` (`validate_monthly`), tests `:249-359`
- Callers: `commands.rs:176,265` and `config.rs:173` (watcher)

- [ ] **Step 1: Change the validator signature**

In `src-tauri/src/config.rs`, change `validate_config` to take a slice. Replace line 15:

```rust
pub fn validate_dimensions(dimensions: &[Dimension]) -> Vec<ConfigErrorDetail> {
```

And line 19 (the loop header) from `for (i, dim) in config.dimensions.iter().enumerate()` to:

```rust
    for (i, dim) in dimensions.iter().enumerate() {
```

Add `Dimension` to the import on line 2: `use crate::models::{ConfigErrorDetail, Dimension, MonthlyFile, Template};`

- [ ] **Step 2: Make `validate_monthly` also validate the month's dimensions**

In `validate_monthly` (line 84), at the START of the function body (before the commitments loop), prepend:

```rust
    let mut errors = validate_dimensions(&monthly.dimensions);
```

and DELETE the existing `let mut errors = Vec::new();` line so `errors` is declared once. The rest (commitment checks) appends to it.

- [ ] **Step 3: Update callers**

- `commands.rs:1` import: `use crate::config::{validate_config, validate_monthly};` → `use crate::config::{validate_dimensions, validate_monthly};`
- `commands.rs:176`: `let mut all_errors = validate_config(&config);` → `let mut all_errors = validate_dimensions(&config.dimensions);` (here `config` is the `Template` read from `read_template`).
- `commands.rs:265` (in `set_root_path`): same replacement.
- `config.rs:173` (watcher): `let errors = validate_config(&config);` → `let errors = validate_dimensions(&config.dimensions);`

- [ ] **Step 4: Update `config.rs` tests**

The tests call `validate_config(&config)` where `config` is `Template { dimensions }`. Replace each `validate_config(&config)` → `validate_dimensions(&config.dimensions)` (tests at lines ~269, 283, 299, 315, 340, 356). Leave the `validate_monthly` tests, but note they now also run dimension validation on an empty `dimensions` vec (which passes — no change to expected counts).

- [ ] **Step 5: Run config tests**

Run: `cd src-tauri && cargo test --lib config::`
Expected: PASS.

- [ ] **Step 6: Full suite + commit**

Run: `cd src-tauri && cargo test`
Expected: PASS.

```bash
git add src-tauri/
git commit -m "refactor: MonthlyFile.dimensions + validate_dimensions; rename Config to Template"
```

---

## Phase 3 — Resolve effective dims + instantiate on first write (core behavior)

### Task 3.1: `year_month_from_date`, `resolve_month_dimensions`, `ensure_month_instantiated`

**Files:**
- Modify: `src-tauri/src/files.rs` (add functions near `read_template`)
- Test: `src-tauri/src/files.rs` `#[cfg(test)] mod tests` (pure-logic parts) + integration for IO

- [ ] **Step 1: Write failing unit test for `year_month_from_date`**

Add to `files.rs` tests module:

```rust
    #[test]
    fn test_year_month_from_date() {
        assert_eq!(year_month_from_date("2026-07-15").unwrap(), (2026, 7));
    }

    #[test]
    fn test_year_month_from_date_invalid() {
        assert!(year_month_from_date("nope").is_err());
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd src-tauri && cargo test --lib files::tests::test_year_month_from_date`
Expected: FAIL — `cannot find function year_month_from_date`.

- [ ] **Step 3: Implement the three functions**

Add to `files.rs` (after `read_template`, before `cleanup_tmp_files`). Add `use crate::models::Dimension;` — extend the top import on line 1 to `use crate::models::{Dimension, DayFile, Entry, MonthlyFile, Template};`.

```rust
/// Parse (year, month) from an ISO date string "YYYY-MM-DD".
pub fn year_month_from_date(date: &str) -> Result<(i32, u32), String> {
    use chrono::Datelike;
    let d = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}", date, e))?;
    Ok((d.year(), d.month()))
}

/// Effective dimensions for a month: the month's own `dimensions` block if
/// non-empty, otherwise the template's. Tolerant of missing files (returns
/// empty vec) so replay and uninstantiated months never error.
pub fn resolve_month_dimensions(root: &Path, year: i32, month: u32) -> Vec<Dimension> {
    if let Ok(monthly) = read_monthly_file(root, year, month) {
        if !monthly.dimensions.is_empty() {
            return monthly.dimensions;
        }
    }
    match read_template(root) {
        Ok(t) => t.dimensions,
        Err(_) => vec![],
    }
}

/// Snapshot the template into a month's `_monthly.md` if it has no dimensions
/// block yet. Preserves any existing commitments (merge, not overwrite).
/// No-op if already instantiated or the template has no dimensions.
pub fn ensure_month_instantiated(root: &Path, year: i32, month: u32) -> Result<(), String> {
    let mut monthly = read_monthly_file(root, year, month)?;
    if !monthly.dimensions.is_empty() {
        return Ok(());
    }
    let template_dims = match read_template(root) {
        Ok(t) => t.dimensions,
        Err(_) => vec![],
    };
    if template_dims.is_empty() {
        return Ok(());
    }
    monthly.dimensions = template_dims;
    write_monthly_file(root, year, month, &monthly)
}
```

- [ ] **Step 4: Run the unit tests**

Run: `cd src-tauri && cargo test --lib files::tests::test_year_month_from_date`
Expected: PASS (both variants).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/files.rs
git commit -m "feat: resolve_month_dimensions + ensure_month_instantiated helpers"
```

### Task 3.2: `validate_required_dimensions` takes `&[Dimension]`; wire resolve into files-layer entry writes

**Files:**
- Modify: `src-tauri/src/commands.rs:126-136` (signature + tests at `:1057-1121`), `src-tauri/src/files.rs:90-105` (`append_new_entry`), `:108-138` (`update_entry_in_file`)

- [ ] **Step 1: Change `validate_required_dimensions` signature**

In `src-tauri/src/commands.rs`, replace lines 126-136:

```rust
pub fn validate_required_dimensions(
    dimensions: &[Dimension],
    entry_dimensions: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    for dim in dimensions {
        if dim.required && !entry_dimensions.contains_key(&dim.key) {
            return Err(format!("Missing required dimension: {}", dim.name));
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Update the `make_config` test helper + its callers**

In `commands.rs` tests (line ~1057), `make_config` returns `Template`. Callers do `validate_required_dimensions(&config, &dims)`. Change every call to pass the slice: `validate_required_dimensions(&config.dimensions, &dims)` (tests at ~1090, 1099, 1112, 1119).

- [ ] **Step 3: Rewrite `files::append_new_entry` to use resolved month dims**

In `src-tauri/src/files.rs`, replace lines 95-97 (inside `append_new_entry`):

```rust
    let duration = crate::commands::parse_duration(&new_entry.duration)?;
    let (year, month) = year_month_from_date(date)?;
    ensure_month_instantiated(root, year, month)?;
    let dims = resolve_month_dimensions(root, year, month);
    crate::commands::validate_required_dimensions(&dims, &new_entry.dimensions)?;
```

- [ ] **Step 4: Rewrite `files::update_entry_in_file` dim validation**

In `src-tauri/src/files.rs`, replace lines 130-134 (the `if let Some(ref dims) = update.dimensions { … }` block):

```rust
        if let Some(ref dims) = update.dimensions {
            let (year, month) = year_month_from_date(date)?;
            let effective = resolve_month_dimensions(root, year, month);
            crate::commands::validate_required_dimensions(&effective, dims)?;
            entry.dimensions = dims.clone();
        }
```

(`date` is the function's `&str` param — already in scope.)

- [ ] **Step 5: Build**

Run: `cd src-tauri && cargo build`
Expected: errors at the command-layer call sites (`append_entry`/`update_entry` still pass `&config`). Fixed in 3.3.

### Task 3.3: Wire instantiation + resolve into the 5 write commands

**Files:**
- Modify: `src-tauri/src/commands.rs` — `append_entry:342-346`, `update_entry:385-393`, `delete_entry:436-437`, `set_day_note:467-468`, `set_commitments:606-635`

- [ ] **Step 1: `append_entry` — instantiate, resolve, validate**

In `append_entry`, replace lines 342-346:

```rust
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let duration = parse_duration(&entry.duration)?;
    let (year, month) = files::year_month_from_date(&date)?;
    files::ensure_month_instantiated(root, year, month)?;
    let dims = files::resolve_month_dimensions(root, year, month);
    validate_required_dimensions(&dims, &entry.dimensions)?;
```

- [ ] **Step 2: `update_entry` — instantiate + resolve**

In `update_entry`, replace lines 385-393:

```rust
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let (year, month) = files::year_month_from_date(&date)?;
    files::ensure_month_instantiated(root, year, month)?;
    if let Some(ref dur_str) = update.duration {
        parse_duration(dur_str)?;
    }
    if let Some(ref dims) = update.dimensions {
        let effective = files::resolve_month_dimensions(root, year, month);
        validate_required_dimensions(&effective, dims)?;
    }
```

- [ ] **Step 3: `delete_entry` — instantiate**

In `delete_entry`, after line 437 (`validate_date_format(&date)?;`) insert:

```rust
    let (year, month) = files::year_month_from_date(&date)?;
    files::ensure_month_instantiated(root, year, month)?;
```

- [ ] **Step 4: `set_day_note` — instantiate**

In `set_day_note`, after line 468 (`validate_date_format(&date)?;`) insert:

```rust
    let (year, month) = files::year_month_from_date(&date)?;
    files::ensure_month_instantiated(root, year, month)?;
```

- [ ] **Step 5: `set_commitments` — instantiate + preserve dimensions on write**

In `set_commitments`, replace lines 606-635. Key change: instantiate first, then read the (now-snapshotted) monthly and replace only `commitments`, so the `dimensions` block is preserved:

```rust
    let root = std::path::Path::new(&root_path);

    // 1. Validate
    validate_commitments(&commitments)?;

    // 2. Snapshot template dims if this month is fresh (preserves any dims block)
    files::ensure_month_instantiated(root, year, month)?;

    // 3. Read old state for diff
    let old = read_monthly_file_safe(root, year, month)?;

    // 4. Detect changes
    let changes = detect_goal_changes(&old.commitments, &commitments);

    // 5. Check deleted goals for existing entries
    for goal_name in &changes.deleted {
        let count = count_entries_with_goal(root, year, month, goal_name)?;
        if count > 0 {
            return Err(format!(
                "Cannot delete goal '{}': used by {} entries this month",
                goal_name, count
            ));
        }
    }

    // 6. Apply renames to all day files
    for (old_name, new_name) in &changes.renames {
        rename_goal_in_entries(root, year, month, old_name, new_name)?;
    }

    // 7. Write _monthly.md, preserving the dimensions block
    let mut monthly = read_monthly_file_safe(root, year, month)?;
    monthly.commitments = commitments;
    files::write_monthly_file(root, year, month, &monthly)?;

    let ok = true;
    error_log::log_command_exit("set_commitments", ok, "");
    Ok(monthly.commitments)
```

- [ ] **Step 6: Add `Dimension` to commands imports if needed**

`commands.rs:5` is `use crate::models::*;` — `Dimension` is already in scope. No change.

- [ ] **Step 7: Build + full suite**

Run: `cd src-tauri && cargo build && cargo test`
Expected: PASS. If `set_commitments` integration tests assert the written `_monthly.md` byte content, they may now also contain a `dimensions:` block — update those assertions to match (see Phase 6 if any surface here).

- [ ] **Step 8: Commit**

```bash
git add src-tauri/
git commit -m "feat: instantiate month dimensions from template on first write"
```

---

## Phase 4 — `get_month_dimensions` command + `InitResult` contract

### Task 4.1: `MonthDimensions` model + reshape `InitResult::Ready`

**Files:**
- Modify: `src-tauri/src/models.rs` (add `MonthDimensions`; edit `InitResult::Ready`)

- [ ] **Step 1: Add `MonthDimensions` struct**

In `src-tauri/src/models.rs`, after `AvailableMonth` (line 130), add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthDimensions {
    pub dimensions: Vec<Dimension>,
    pub from_template: bool,
}
```

- [ ] **Step 2: Reshape `InitResult::Ready`**

Replace the `Ready` variant (lines 104-110):

```rust
    Ready {
        root_path: String,
        dimensions: Vec<Dimension>,
        from_template: bool,
        today: DayFile,
        commitments: Vec<Commitment>,
        scan_warnings: Vec<ScanWarning>,
    },
```

- [ ] **Step 3: Update `models.rs` tests that construct `Ready`**

Tests at lines 176-237, 302-341 build `InitResult::Ready { … config, … }`. Replace each `config,` (or `config: Template { dimensions: vec![] }`) with `dimensions: vec![], from_template: false,` and remove the now-unused `let config = …;` bindings. Example (the `init_result_ready_with_scan_warnings` test):

```rust
        let today = DayFile { note: None, entries: vec![] };
        let warning = ScanWarning {
            kind: "SkippedFile".to_string(),
            path: "2026/06/bad.md".to_string(),
            message: "parse error".to_string(),
        };
        let result = InitResult::Ready {
            root_path: "/tmp/logbook-test".to_string(),
            dimensions: vec![],
            from_template: false,
            today,
            commitments: vec![],
            scan_warnings: vec![warning],
        };
```

Apply the same to `init_result_ready_empty_scan_warnings` and `init_result_ready_json_roundtrip`.

- [ ] **Step 4: Build**

Run: `cd src-tauri && cargo build`
Expected: errors in `commands.rs` (`init`/`set_root_path` still set `config`). Fixed in 4.2.

### Task 4.2: `get_month_dimensions` command + update `init`/`set_root_path`

**Files:**
- Modify: `src-tauri/src/commands.rs` — `init:230-236`, `set_root_path:315-321`; add `get_month_dimensions`

- [ ] **Step 1: Add the `get_month_dimensions` command**

In `src-tauri/src/commands.rs`, add after `get_commitments` (after line 503):

```rust
#[tauri::command]
pub fn get_month_dimensions(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<MonthDimensions, String> {
    error_log::log_command_enter("get_month_dimensions", &format!("{}-{:02}", year, month));
    let root = std::path::Path::new(&root_path);
    // A month is "instantiated" iff its _monthly.md has a non-empty dimensions block.
    let from_template = match files::read_monthly_file(root, year, month) {
        Ok(m) => m.dimensions.is_empty(),
        Err(_) => true,
    };
    let dimensions = files::resolve_month_dimensions(root, year, month);
    error_log::log_command_exit(
        "get_month_dimensions",
        true,
        &format!("{} dims, from_template={}", dimensions.len(), from_template),
    );
    Ok(MonthDimensions { dimensions, from_template })
}
```

- [ ] **Step 2: Update `init` to return month-scoped dims**

In `init`, the monthly file is read at line 179 into `monthly`. Replace the `Ready` construction (lines 230-236):

```rust
    let from_template = monthly.dimensions.is_empty();
    let dimensions = if from_template {
        match files::read_template(root) {
            Ok(t) => t.dimensions,
            Err(_) => vec![],
        }
    } else {
        monthly.dimensions.clone()
    };

    InitResult::Ready {
        root_path: root_path.to_string_lossy().into_owned(),
        dimensions,
        from_template,
        today,
        commitments: monthly.commitments,
        scan_warnings,
    }
```

(`config` from `read_template` at line 161 is still used for `validate_dimensions` at line 176 — keep it.)

- [ ] **Step 3: Update `set_root_path` the same way**

In `set_root_path`, replace the `Ready` construction (lines 315-321) with the same pattern, using its `monthly` (line 268) and `path.clone()` for `root_path`:

```rust
    let from_template = monthly.dimensions.is_empty();
    let dimensions = if from_template {
        match files::read_template(root_path) {
            Ok(t) => t.dimensions,
            Err(_) => vec![],
        }
    } else {
        monthly.dimensions.clone()
    };

    Ok(InitResult::Ready {
        root_path: path.clone(),
        dimensions,
        from_template,
        today,
        commitments: monthly.commitments,
        scan_warnings,
    })
```

Note: in `set_root_path` the path variable is `root_path` (a `&Path`, line 246). Use it for `read_template`.

- [ ] **Step 4: Register the command in `lib.rs`**

In `src-tauri/src/lib.rs`, inside `tauri::generate_handler![ … ]` (line 53+), add a line after `commands::get_commitments,` (line 61):

```rust
            commands::get_month_dimensions,
```

- [ ] **Step 5: Build + full suite**

Run: `cd src-tauri && cargo build && cargo test`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat: get_month_dimensions command; init returns month-scoped dims"
```

---

## Phase 5 — Frontend: month-scoped dimensions

### Task 5.1: Types + store

**Files:**
- Modify: `src/types.ts:9-11,80-92`, `src/stores/useStore.ts:2,13,33`

- [ ] **Step 1: Update `types.ts`**

In `src/types.ts`, replace the `Config` interface (lines 9-11) with a `MonthDimensions` type (keep `Dimension`):

```typescript
export interface MonthDimensions {
  dimensions: Dimension[];
  from_template: boolean;
}
```

Replace the `Ready` arm of `InitResult` (lines 83-92):

```typescript
  | {
      status: "Ready";
      data: {
        root_path: string;
        dimensions: Dimension[];
        from_template: boolean;
        today: DayFile;
        commitments: Commitment[];
        scan_warnings: ScanWarning[];
      };
    };
```

- [ ] **Step 2: Update the store shape**

In `src/stores/useStore.ts`: change the import on line 2 to drop `Config` and add `Dimension`:

```typescript
import type { Dimension, DayFile, Commitment, CommitmentProgress, ConfigErrorDetail, AppStatus, Entry } from "../types";
```

Replace `config: Config | null;` (line 13) with:

```typescript
  dimensions: Dimension[];
  fromTemplate: boolean;
```

In `createStore()` (line 33 area), replace `config: null,` with:

```typescript
    dimensions: [],
    fromTemplate: false,
```

- [ ] **Step 3: Typecheck (expect consumer errors)**

Run: `pnpm vue-tsc --noEmit`
Expected: errors in `App.vue`, `SetupScreen.vue`, `MonthView.vue`, `EntryRow.vue` referencing `store.config` / `result.data.config`. Fixed next.

### Task 5.2: Wire init + setup result into the store

**Files:**
- Modify: `src/App.vue:93-103`, `src/components/SetupScreen.vue:25-29` and the `confirm()` text (~line 38)

- [ ] **Step 1: `App.vue` Ready case**

In `src/App.vue`, replace the `case "Ready":` body (lines 93-103):

```typescript
      case "Ready":
        store.rootPath = result.data.root_path;
        store.dimensions = result.data.dimensions;
        store.fromTemplate = result.data.from_template;
        store.today = result.data.today;
        store.commitments = result.data.commitments;
        store.status = "ready";
        if (result.data.scan_warnings.length > 0) {
          scanWarnings.value = result.data.scan_warnings;
          showScanWarning.value = true;
        }
        break;
```

- [ ] **Step 2: `SetupScreen.vue` Ready case + confirm text**

In `src/components/SetupScreen.vue`, replace `store.config = result.data.config;` (line 27) with:

```typescript
      store.dimensions = result.data.dimensions;
      store.fromTemplate = result.data.from_template;
```

Update the `confirm(...)` message (~line 38) from `"No config.yaml found. Create one with default settings?"` to `"No template.yaml found. Create one with default settings?"`.

- [ ] **Step 3: Typecheck**

Run: `pnpm vue-tsc --noEmit`
Expected: remaining errors only in `MonthView.vue` and `EntryRow.vue`.

### Task 5.3: `MonthView.vue` — fetch dims on month load + replace usages + preview indicator

**Files:**
- Modify: `src/components/MonthView.vue:42-73` (`loadMonth`), `:112`, `:371`; import `MonthDimensions`

- [ ] **Step 1: Fetch `get_month_dimensions` inside `loadMonth`**

In `src/components/MonthView.vue`, inside `loadMonth(year, month, …)` — alongside the existing `get_commitment_progress` invoke (around line 71) — add a fetch that refreshes the store's dimensions for the month being loaded:

```typescript
  try {
    const md = (await invoke("get_month_dimensions", { rootPath: store.rootPath, year, month })) as MonthDimensions;
    store.dimensions = md.dimensions;
    store.fromTemplate = md.from_template;
  } catch (e) { logError("MonthView.loadMonthDimensions", e); }
```

Add `MonthDimensions` to the type import at the top of the `<script setup>` block (it imports from `../types`).

- [ ] **Step 2: Replace `store.config?.dimensions` usages**

- Line 112: `const validKeys = new Set((store.config?.dimensions || []).map(d => d.key));` → `const validKeys = new Set(store.dimensions.map(d => d.key));`
- Line 371: `:dimensions="store.config?.dimensions || []"` → `:dimensions="store.dimensions"`

- [ ] **Step 3: Add the "uses default template" preview indicator**

Find the CommitmentsPanel / entry-composer header region in the template. Add a subtle indicator shown only when `store.fromTemplate` is true. Use design tokens (no raw px, `text-micro`/`text-secondary`, muted color). Example, placed above the entry composer:

```html
        <p v-if="store.fromTemplate" class="text-micro text-gray-400 mb-sm">
          本月沿用默认模板（尚未自定义维度）
        </p>
```

(Adjust placement to sit near the dimension-driven input without disrupting existing layout; keep spacing tokens.)

- [ ] **Step 4: `EntryRow.vue`**

In `src/components/composite/EntryRow.vue:20`, replace `const dimensions = computed(() => store.config?.dimensions || []);` with:

```typescript
const dimensions = computed(() => store.dimensions);
```

- [ ] **Step 5: Typecheck (test files still red — fixed in 5.4)**

Run: `pnpm vue-tsc --noEmit`
Expected: production `src/` typechecks clean; errors remain in `src/__tests__/**` (they reference `makeConfig` / `store.config`). Those are migrated in Task 5.4 — do NOT commit yet.

### Task 5.4: Migrate frontend test fixtures + specs off `Config`

**Files:**
- Modify: `src/__tests__/mocks/fixtures.ts:1-9,48-57`; test files: `useStore.test.ts`, `components/App.test.ts`, `components/SetupScreen.test.ts`, `components/MonthView.test.ts`, `components/EntryList.test.ts`, `components/composite/EntryRow.test.ts`

- [ ] **Step 1: Replace `makeConfig` with `makeDimensions` in `fixtures.ts`**

In `src/__tests__/mocks/fixtures.ts`, remove `Config` from the import (lines 1-9) and replace `makeConfig` (lines 48-57) with a helper returning the dimension array directly:

```typescript
export function makeDimensions(): Dimension[] {
  return [
    makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
    makeDimension({ name: "Business Line", key: "business-line", source: "static", values: ["Platform", "Growth"], required: true }),
    makeDimension({ name: "Category", key: "category", source: "static", values: ["Coding", "Meeting"], required: false }),
  ];
}
```

- [ ] **Step 2: Find every importer / usage**

Run: `grep -rn "makeConfig\|store\.config\b\|config: makeConfig\|config:" src/__tests__`
This lists the 6 test files plus the `store.ts`/`fixtures.ts` doc comments.

- [ ] **Step 3: Migrate the three usage shapes**

Apply these uniform transforms across the test files (replace `makeConfig` import with `makeDimensions`):

a) **Store stub object** — e.g. `EntryList.test.ts:10`:
`reactive({ config: makeConfig(), commitments: [makeCommitment()] })`
→ `reactive({ dimensions: makeDimensions(), fromTemplate: false, commitments: [makeCommitment()] })`

b) **`InitResult` Ready payload** — e.g. `App.test.ts:155` / `SetupScreen.test.ts`:
`data: { root_path: "/test", config: makeConfig(), today: makeDayFile(), commitments: [], scan_warnings: [] }`
→ `data: { root_path: "/test", dimensions: makeDimensions(), from_template: false, today: makeDayFile(), commitments: [], scan_warnings: [] }`
(and standalone `config: makeConfig(),` lines like `App.test.ts:341,383,404` → `dimensions: makeDimensions(), from_template: false,`)

c) **Assertions** — `useStore.test.ts:9`:
`expect(store.config).toBeNull();` → `expect(store.dimensions).toEqual([]);`
`App.test.ts:126` / `SetupScreen.test.ts:100`:
`expect(store.config).toEqual(config);` → `expect(store.dimensions).toEqual(makeDimensions());`
(remove any now-unused `const config = makeConfig();` binding these referenced; if a test captured `const config = makeConfig()` and asserts equality, replace with `makeDimensions()` on both sides.)

Update the doc-comment usages in `mocks/store.ts:9` and `fixtures.ts:16` (`makeConfig` → `makeDimensions`) for accuracy.

- [ ] **Step 4: Typecheck + full unit suite**

Run: `pnpm vue-tsc --noEmit && pnpm test`
Expected: typecheck clean; all vitest tests PASS. If a component test asserts the absence/presence of the "本月沿用默认模板" indicator, it must set `fromTemplate` accordingly in its store stub.

- [ ] **Step 5: Commit**

```bash
git add src/
git commit -m "feat(ui): month-scoped dimensions + template preview indicator"
```

---

## Phase 6 — Integration tests + docs

### Task 6.1: Integration tests for the spec scenarios

**Files:**
- Create: `src-tauri/tests/monthly_dimensions_integration.rs`

- [ ] **Step 1: Write the integration tests**

Create `src-tauri/tests/monthly_dimensions_integration.rs`. These exercise the public command/files surface against a temp dir. Mirror the helper style used in existing integration tests (write `template.yaml`, call commands with the temp root).

```rust
use std::fs;
use std::path::PathBuf;
use tauri_app_lib::files;
use tauri_app_lib::models::CreateEntryInput;
use std::collections::HashMap;

fn fresh_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_template(root: &PathBuf, body: &str) {
    fs::write(root.join("template.yaml"), body).unwrap();
}

const TPL_BIZ_GOAL: &str =
    "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values: [产品, 市场]\n  - name: Goal\n    key: goal\n    source: monthly\n";

// 1. Pure read of a fresh month returns template dims, from_template=true, no file written.
#[test]
fn fresh_month_reads_template_without_writing() {
    let root = fresh_root("logbook_md_fresh_read");
    write_template(&root, TPL_BIZ_GOAL);

    let dims = files::resolve_month_dimensions(&root, 2026, 7);
    assert_eq!(dims.len(), 2);
    assert_eq!(dims[0].key, "biz");

    // resolve must NOT have created _monthly.md
    let monthly = files::monthly_path(&root, 2026, 7);
    assert!(!monthly.exists(), "resolve must not write _monthly.md");

    let _ = fs::remove_dir_all(&root);
}

// 2. First append instantiates a snapshot; later template edits don't change it.
#[test]
fn first_append_snapshots_template() {
    let root = fresh_root("logbook_md_snapshot");
    write_template(&root, TPL_BIZ_GOAL);

    let input = CreateEntryInput {
        item: "task".into(),
        duration: "30".into(),
        dimensions: HashMap::new(),
    };
    files::append_new_entry(&root, "2026-07-15", &input).unwrap();

    // _monthly.md now carries the dimensions block
    let monthly = files::read_monthly_file(&root, 2026, 7).unwrap();
    assert_eq!(monthly.dimensions.len(), 2);

    // Change the template; the month keeps its snapshot.
    write_template(&root, "dimensions:\n  - name: Other\n    key: other\n    source: static\n    values: [x]\n");
    let dims = files::resolve_month_dimensions(&root, 2026, 7);
    assert_eq!(dims.len(), 2, "snapshot must not follow template changes");
    assert_eq!(dims[0].key, "biz");

    let _ = fs::remove_dir_all(&root);
}

// 3. A hand-written month block overrides the template.
#[test]
fn month_block_overrides_template() {
    let root = fresh_root("logbook_md_override");
    write_template(&root, TPL_BIZ_GOAL);
    let month_dir = root.join("2026").join("08");
    fs::create_dir_all(&month_dir).unwrap();
    fs::write(
        month_dir.join("_monthly.md"),
        "---\ndimensions:\n  - name: Client\n    key: client\n    source: static\n    values: [甲]\n---\n",
    )
    .unwrap();

    let dims = files::resolve_month_dimensions(&root, 2026, 8);
    assert_eq!(dims.len(), 1);
    assert_eq!(dims[0].key, "client");

    let _ = fs::remove_dir_all(&root);
}

// 4. ensure_month_instantiated preserves existing commitments (merge, not overwrite).
#[test]
fn instantiate_preserves_commitments() {
    let root = fresh_root("logbook_md_preserve");
    write_template(&root, TPL_BIZ_GOAL);
    let month_dir = root.join("2026").join("09");
    fs::create_dir_all(&month_dir).unwrap();
    fs::write(
        month_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it\n---\n",
    )
    .unwrap();

    files::ensure_month_instantiated(&root, 2026, 9).unwrap();

    let monthly = files::read_monthly_file(&root, 2026, 9).unwrap();
    assert_eq!(monthly.dimensions.len(), 2, "dims snapshotted");
    assert_eq!(monthly.commitments.len(), 1, "commitments preserved");
    assert_eq!(monthly.commitments[0].role, "Dev");

    let _ = fs::remove_dir_all(&root);
}

// 5. Missing template → resolve is lenient (empty), ensure is a no-op.
#[test]
fn missing_template_is_lenient() {
    let root = fresh_root("logbook_md_notpl");
    // no template.yaml written
    let dims = files::resolve_month_dimensions(&root, 2026, 7);
    assert!(dims.is_empty());
    files::ensure_month_instantiated(&root, 2026, 7).unwrap(); // no panic, no-op
    assert!(!files::monthly_path(&root, 2026, 7).exists());
    let _ = fs::remove_dir_all(&root);
}
```

- [ ] **Step 2: Run the new tests**

Run: `cd src-tauri && cargo test --test monthly_dimensions_integration`
Expected: PASS (5 tests). If `tauri_app_lib::files` items aren't public enough, confirm `pub fn` on `resolve_month_dimensions`/`ensure_month_instantiated`/`monthly_path`/`read_monthly_file`/`append_new_entry` (all already `pub`).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/monthly_dimensions_integration.rs
git commit -m "test: monthly dimension template integration scenarios"
```

### Task 6.2: Update docs + fixtures

**Files:**
- Modify: `SPEC.md`, `src-tauri/CLAUDE.md`, `src-tauri/tests/fixtures/2026/06/_monthly.md`

- [ ] **Step 1: `SPEC.md` data structures + commands**

Update `SPEC.md`:
- Rename `config.yaml` → `template.yaml` in the file-operations section; describe it as the default dimension template.
- `struct Config` → `struct Template` (still `{ dimensions: Vec<Dimension> }`); note `Dimension.source` "monthly" still drives Goal.
- Add `dimensions: Vec<Dimension>` to `MonthlyFile`.
- Add `MonthDimensions { dimensions, from_template }` and the `get_month_dimensions(root_path, year, month)` command to the command list (now 15 commands).
- `InitResult::Ready` now carries `dimensions: Vec<Dimension>, from_template: bool` instead of `config`.
- Note: month dimensions are snapshotted from the template on first write (`ensure_month_instantiated`); pure reads don't write.

- [ ] **Step 2: `src-tauri/CLAUDE.md` 关键约定**

Update the bullet "Goal 维度 `source: monthly`…" region: state that the dimension set lives per-month in `_monthly.md`, snapshotted from `template.yaml` on first write; `config.yaml` no longer exists. Add a line for `ensure_month_instantiated` / `resolve_month_dimensions`.

- [ ] **Step 3: Refresh the fixture `_monthly.md` (optional but recommended)**

Add a `dimensions:` block to `src-tauri/tests/fixtures/2026/06/_monthly.md` so the fixture reflects an instantiated month:

```yaml
---
dimensions:
  - name: Goal
    key: goal
    source: monthly
  - name: Biz
    key: biz
    source: static
    values:
      - Product
      - Marketing
      - Engineering
commitments:
  - role: Dev
    allocation: 40
    goals:
      - Ship it
      - Review
  - role: PM
    allocation: 10
    goals:
      - Planning
---
```

If any fixture-based test asserts the old `_monthly.md` shape, update that assertion.

- [ ] **Step 4: Full verify gate**

Run: `pnpm vue-tsc --noEmit && cd src-tauri && cargo check && cargo test`
Expected: PASS across the board.

- [ ] **Step 5: Commit**

```bash
git add SPEC.md src-tauri/CLAUDE.md src-tauri/tests/fixtures/
git commit -m "docs: monthly dimension templates (SPEC + backend conventions)"
```

---

## Manual verification (after Phase 6)

Run the app: `pnpm tauri dev` (repo root). Confirm against a scratch data dir:

1. **Fresh setup** writes `template.yaml` (not `config.yaml`); first launch with no root → setup → starter `template.yaml` created.
2. **Current month** shows template dimensions; if never written, the "本月沿用默认模板" indicator is visible.
3. **Add an entry** in a fresh month → `_monthly.md` appears with a `dimensions:` block; indicator disappears.
4. **Edit `template.yaml`** (add a Client dimension) → already-instantiated months are unchanged; a brand-new month picks it up.
5. **Hand-edit a month's `_monthly.md` `dimensions:`** → that month's entry input reflects the change after the watcher reload; other months unaffected.
6. **Navigate across months** → the dimension chips/input update per month (no stale dimensions from the previously-viewed month).

---

## Notes on risk & sequencing

- **Phases 1–4 are backend-only and each ends green** — safe checkpoints. Phase 5 is the only one that changes the IPC payload consumers; do it in one pass since `init` and the store change together.
- **Biggest latent bug:** any frontend path still assuming `dimensions` is global. After Phase 5, grep `grep -rn "store.config" src` must return nothing.
- **Replay tolerance:** `resolve_month_dimensions` never errors on missing files, so op-log replay (which copies only `template.yaml`, not `_monthly.md`) validates leniently — matching prior behavior where missing required dims simply weren't enforced.
- **No migration:** old `config.yaml` files are not read. Per the spec, this is intentional (pre-production, test data only).
