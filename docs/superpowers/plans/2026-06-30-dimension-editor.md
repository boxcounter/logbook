# Dimension Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `_monthly.md` into `dimensions.yaml` + `commitments.yaml`, fix watcher events, then build GUI (DimensionEditorModal) + CLI (`dimensions get/set`) for managing dimensions.

**Architecture:** Phase 1–2 splits files and fixes events (pure backend + wiring, no GUI). Phase 3 builds the GUI editor as a teleported modal matching CommitmentsModal patterns. Phase 4 adds CLI. Each phase produces working, testable software.

**Tech Stack:** Rust (Tauri 2.x, yaml_serde, notify), Vue 3 + TypeScript, vitest, vue-draggable-plus

---

### Task 1: Add `deleted: bool` to Dimension model

**Files:**
- Modify: `src-tauri/src/models.rs:12-21`
- Modify: `src/types.ts` (Dimension interface)

- [ ] **Step 1: Add `deleted` field to Rust model**

```rust
// src-tauri/src/models.rs — Dimension struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    pub name: String,
    pub key: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)] // false when absent; backward-compatible with existing files
    pub deleted: bool,
}
```

- [ ] **Step 2: Add `deleted` to TypeScript type**

```typescript
// src/types.ts — Dimension interface
export interface Dimension {
  name: string;
  key: string;
  source: "static" | "monthly";
  values?: string[];
  required: boolean;
  deleted: boolean; // new field
}
```

- [ ] **Step 3: Run type checks**

```bash
cd src-tauri && cargo check
npx vue-tsc --noEmit
```

Expected: Both pass.

- [ ] **Step 4: Run existing tests to verify backward compatibility (serde(default) handles missing field)**

```bash
cd src-tauri && cargo test
npx vitest run
```

Expected: All pass (serde `default` makes missing `deleted` → `false`).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src/types.ts
git commit -m "feat: add deleted field to Dimension model"
```

---

### Task 2: Update file path helpers

**Files:**
- Modify: `src-tauri/src/files.rs:38-50` (path functions)
- Modify: `src-tauri/src/files.rs:214-224` (read_template)

- [ ] **Step 1: Add new path constants/functions**

Replace `template_path` and `monthly_path`, add `dimensions_template_path`, `dimensions_path`, `commitments_path`:

```rust
// src-tauri/src/files.rs — replace existing path functions

/// Dimensions template: {root}/dimensions.template.yaml
pub fn dimensions_template_path(root: &Path) -> PathBuf {
    root.join("dimensions.template.yaml")
}

/// Monthly dimensions: {root}/{year}/{month:02}/dimensions.yaml
pub fn dimensions_path(root: &Path, year: i32, month: u32) -> PathBuf {
    root.join(year.to_string())
        .join(format!("{:02}", month))
        .join("dimensions.yaml")
}

/// Monthly commitments: {root}/{year}/{month:02}/commitments.yaml
pub fn commitments_path(root: &Path, year: i32, month: u32) -> PathBuf {
    root.join(year.to_string())
        .join(format!("{:02}", month))
        .join("commitments.yaml")
}
```

Keep the old `monthly_path` and `template_path` functions as deprecated wrappers used only for migration (Task 5). Add `#[allow(dead_code)]` or leave them referenced by migration code that writes to the old path.

- [ ] **Step 2: Update `read_template` → `read_dimensions_template`**

```rust
/// Read dimensions.template.yaml.
pub fn read_dimensions_template(root: &Path) -> Result<Template, String> {
    let path = dimensions_template_path(root);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let content = content.trim_start_matches('\u{feff}');
    yaml_serde::from_str::<Template>(content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}
```

- [ ] **Step 3: Write and run a unit test for the new paths**

```rust
// src-tauri/src/files.rs — in #[cfg(test)] mod tests (or add inline)

#[test]
fn test_new_file_paths() {
    let root = Path::new("/data");
    assert_eq!(
        dimensions_template_path(root),
        PathBuf::from("/data/dimensions.template.yaml")
    );
    assert_eq!(
        dimensions_path(root, 2026, 6),
        PathBuf::from("/data/2026/06/dimensions.yaml")
    );
    assert_eq!(
        commitments_path(root, 2026, 6),
        PathBuf::from("/data/2026/06/commitments.yaml")
    );
}
```

```bash
cd src-tauri && cargo test test_new_file_paths
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/files.rs
git commit -m "feat: add dimensions/commitments file path helpers"
```

---

### Task 3: Create read/write helpers for dimensions.yaml and commitments.yaml

**Files:**
- Modify: `src-tauri/src/files.rs` (new functions)

- [ ] **Step 1: Add `read_dimensions_file`**

```rust
/// Read a month's dimensions.yaml. Returns empty Vec if file doesn't exist.
pub fn read_dimensions_file(root: &Path, year: i32, month: u32) -> Result<Vec<Dimension>, String> {
    let path = dimensions_path(root, year, month);
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let content = content.trim_start_matches('\u{feff}');
    // dimensions.yaml is a flat YAML array of Dimension objects
    yaml_serde::from_str::<Vec<Dimension>>(content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}
```

- [ ] **Step 2: Add `write_dimensions_file` (atomic write)**

```rust
/// Write dimensions to a month's dimensions.yaml (atomic: tmp then rename).
pub fn write_dimensions_file(
    root: &Path,
    year: i32,
    month: u32,
    dimensions: &[Dimension],
) -> Result<(), String> {
    let path = dimensions_path(root, year, month);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let yaml_body = yaml_serde::to_string(dimensions)
        .map_err(|e| format!("Failed to serialize dimensions: {}", e))?;
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, yaml_body)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}
```

- [ ] **Step 3: Add `read_commitments_file`**

```rust
/// Read a month's commitments.yaml. Returns empty Vec if file doesn't exist.
pub fn read_commitments_file(
    root: &Path,
    year: i32,
    month: u32,
) -> Result<Vec<Commitment>, String> {
    let path = commitments_path(root, year, month);
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let content = content.trim_start_matches('\u{feff}');
    yaml_serde::from_str::<Vec<Commitment>>(content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}
```

- [ ] **Step 4: Add `write_commitments_file` (atomic write)**

```rust
/// Write commitments to a month's commitments.yaml (atomic: tmp then rename).
pub fn write_commitments_file(
    root: &Path,
    year: i32,
    month: u32,
    commitments: &[Commitment],
) -> Result<(), String> {
    let path = commitments_path(root, year, month);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let yaml_body = yaml_serde::to_string(commitments)
        .map_err(|e| format!("Failed to serialize commitments: {}", e))?;
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, yaml_body)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/files.rs
git commit -m "feat: add read/write helpers for dimensions.yaml and commitments.yaml"
```

---

### Task 4: Update resolve_month_dimensions to use new files

**Files:**
- Modify: `src-tauri/src/files.rs:240-255` (`resolve_month_dimensions`)

- [ ] **Step 1: Rewrite `resolve_month_dimensions`**

```rust
/// Effective dimensions for a month: the month's dimensions.yaml if it exists,
/// otherwise the dimensions.template.yaml. Tolerates missing files (returns
/// empty vec).
pub fn resolve_month_dimensions(
    root: &Path,
    year: i32,
    month: u32,
) -> Result<Vec<Dimension>, String> {
    let dims = read_dimensions_file(root, year, month)?;
    if !dims.is_empty() {
        return Ok(dims);
    }
    if !dimensions_template_path(root).exists() {
        return Ok(vec![]);
    }
    Ok(read_dimensions_template(root)?.dimensions)
}
```

- [ ] **Step 2: Update all call sites that import or reference the old function name**

Search for `resolve_month_dimensions` and verify it still compiles (function signature is unchanged).

```bash
cd src-tauri && cargo check
```

Expected: All references resolve. The function signature `(root, year, month) -> Result<Vec<Dimension>, String>` is unchanged.

- [ ] **Step 3: Run existing tests**

```bash
cd src-tauri && cargo test
```

Expected: Tests that use `resolve_month_dimensions` pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/files.rs
git commit -m "refactor: resolve_month_dimensions uses dimensions.yaml"
```

---

### Task 5: Replace ensure_month_instantiated with create_dimensions_if_missing

**Files:**
- Modify: `src-tauri/src/files.rs:257-276`
- Modify: all call sites of `ensure_month_instantiated` (check `files.rs:102`, `commands.rs`)

- [ ] **Step 1: Find all call sites**

```bash
grep -rn "ensure_month_instantiated" src-tauri/src/
```

- [ ] **Step 2: Write the replacement function**

```rust
/// Create dimensions.yaml from template if the month has no dimensions yet.
/// No-op if dimensions.yaml already exists or the template has no dimensions.
pub fn create_dimensions_if_missing(
    root: &Path,
    year: i32,
    month: u32,
) -> Result<(), String> {
    if dimensions_path(root, year, month).exists() {
        return Ok(());
    }
    if !dimensions_template_path(root).exists() {
        return Ok(());
    }
    let template_dims = read_dimensions_template(root)?.dimensions;
    if template_dims.is_empty() {
        return Ok(());
    }
    write_dimensions_file(root, year, month, &template_dims)
}
```

- [ ] **Step 3: Replace at all call sites**

In `files.rs` line 102 (`append_new_entry`):
```rust
// before:
ensure_month_instantiated(root, year, month)?;
// after:
create_dimensions_if_missing(root, year, month)?;
```

Update `commands.rs` call sites similarly (search for `ensure_month_instantiated`). The function is also called in `append_entry`, `update_entry`, `set_commitments` — replace each.

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/
git commit -m "refactor: replace ensure_month_instantiated with create_dimensions_if_missing"
```

---

### Task 6: Update commands.rs set_commitments to use commitments.yaml

**Files:**
- Modify: `src-tauri/src/commands.rs` (`set_commitments` function)

- [ ] **Step 1: Update `set_commitments` to write to `commitments.yaml`**

Find the existing `set_commitments` command. Replace the write path from `write_monthly_file` to `write_commitments_file`:

```rust
// In set_commitments, replace:
write_monthly_file(root, year, month, &monthly)?;

// With:
write_commitments_file(root, year, month, &saved)?;
```

Also update `get_commitments` to read from `commitments.yaml`:
```rust
// Replace read_monthly_file + extract commitments
// With:
read_commitments_file(root, year, month)
```

Also update `get_commitment_progress` to read commitments from `commitments.yaml` (search for its `read_monthly_file` call and replace).

- [ ] **Step 2: Update `set_commitments` to call `create_dimensions_if_missing` instead of `ensure_month_instantiated`**

```rust
// In set_commitments, replace:
ensure_month_instantiated(root, year, month)?;
// With:
create_dimensions_if_missing(root, year, month)?;
```

- [ ] **Step 3: Run the commitment integration tests**

```bash
cd src-tauri && cargo test --test integration_tests -- commitments
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/files.rs
git commit -m "refactor: commitments use commitments.yaml instead of _monthly.md"
```

---

### Task 7: Update file watcher for new files and events

**Files:**
- Modify: `src-tauri/src/config.rs:240-315` (watcher event loop)

- [ ] **Step 1: Update watcher to watch new files**

The watcher watches the root directory. It already receives events for all files. Update the file name matching:

```rust
// In config.rs watcher loop (lines 249-250):
// Replace:
if file_name == "template.yaml" {
// With:
if file_name == "dimensions.template.yaml" {
```

For monthly files (lines 276):
```rust
// Replace:
} else if file_name == "_monthly.md" {
// With:
} else if file_name == "dimensions.yaml" {
    // ... emit dimensions-changed
} else if file_name == "commitments.yaml" {
    // ... emit commitments-changed
}
```

- [ ] **Step 2: Split the monthly handler into two**

Currently `_monthly.md` handler reads the file and emits `commitments-changed`. Split:

**dimensions.yaml handler:**
```rust
} else if file_name == "dimensions.yaml" {
    let (year, month) = match month_from_monthly_path(path) {
        Some(ym) => ym,
        None => {
            crate::error_log::log_error(
                "file_watcher",
                &format!("could not parse month from dimensions.yaml path: {}", path.display()),
            );
            continue;
        }
    };
    match files::read_dimensions_file(&watch_root, year, month) {
        Ok(dims) => {
            let errors = validate_dimensions(&dims);
            if let Err(e) = app_handle.emit("dimensions-changed", &errors) {
                crate::error_log::log_error(
                    "file_watcher",
                    &format!("emit dimensions-changed failed: {}", e),
                );
            }
        }
        Err(e) => {
            if let Err(e2) = app_handle.emit(
                "dimensions-changed",
                &vec![ConfigErrorDetail {
                    kind: "ParseError".to_string(),
                    message: e,
                }],
            ) {
                crate::error_log::log_error(
                    "file_watcher",
                    &format!("emit dimensions-changed failed: {}", e2),
                );
            }
        }
    }
```

**commitments.yaml handler:**
```rust
} else if file_name == "commitments.yaml" {
    let (year, month) = match month_from_monthly_path(path) {
        Some(ym) => ym,
        None => {
            crate::error_log::log_error(
                "file_watcher",
                &format!("could not parse month from commitments.yaml path: {}", path.display()),
            );
            continue;
        }
    };
    match files::read_commitments_file(&watch_root, year, month) {
        Ok(commitments) => {
            // Validate alongside any existing dimensions
            let dims = files::read_dimensions_file(&watch_root, year, month)
                .unwrap_or_default();
            let monthly = MonthlyFile { dimensions: dims, commitments };
            let errors = validate_monthly(&monthly);
            if let Err(e) = app_handle.emit("commitments-changed", &errors) {
                crate::error_log::log_error(
                    "file_watcher",
                    &format!("emit commitments-changed failed: {}", e),
                );
            }
        }
        Err(e) => {
            if let Err(e2) = app_handle.emit(
                "commitments-changed",
                &vec![ConfigErrorDetail {
                    kind: "ParseError".to_string(),
                    message: e,
                }],
            ) {
                crate::error_log::log_error(
                    "file_watcher",
                    &format!("emit commitments-changed failed: {}", e2),
                );
            }
        }
    }
```

- [ ] **Step 3: dimensions.template.yaml: validate only, no emit**

```rust
if file_name == "dimensions.template.yaml" {
    match files::read_dimensions_template(&watch_root) {
        Ok(config) => {
            let _ = validate_dimensions(&config.dimensions);
            // Template changes do not affect the current view — no emit.
            // Validation errors are surfaced on the next init/dimensions read.
        }
        Err(_) => {
            // Parse error; next read will surface it.
        }
    }
}
```

- [ ] **Step 4: Update the `#[cfg(test)]` watcher tests**

Update tests in `config.rs` (lines ~500+) to use new file names and event names.

```bash
cd src-tauri && cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "refactor: update file watcher for split files and new events"
```

---

### Task 8: Update Frontend event listeners (App.vue + store)

**Files:**
- Modify: `src/App.vue:64-89` (event listeners)
- Modify: `src/stores/useStore.ts` (add `dimensions-changed` handler)
- Modify: `src/components/MonthView.vue` (add `loadMonthDimensions` call)

- [ ] **Step 1: Replace `config-changed` listener in App.vue**

```typescript
// src/App.vue — in onMounted, replace the config-changed block (lines 64-76):
unlistenDimensions = await listen<ConfigErrorDetail[]>("dimensions-changed", async (event) => {
  if (store.status !== "ready") return;
  if (event.payload.length > 0) {
    store.configErrors = event.payload;
    store.configCategory = "in_place";
    store.status = "error";
    return;
  }
  // Reload dimensions for the currently viewed month only — not initApp()
  const { year, month } = yearMonthFromDate(store.currentDate);
  try {
    const result = await invoke("get_month_dimensions", {
      rootPath: store.rootPath,
      year,
      month,
    }) as MonthDimensionsResult;
    store.dimensions = result.dimensions;
    store.fromTemplate = result.from_template;
  } catch (e) {
    logError("App.dimensionsChanged", e);
  }
});
```

Update the variable declarations at the top:
```typescript
let unlistenConfig: (() => void) | null = null;  // delete this line
// add:
let unlistenDimensions: (() => void) | null = null;
```

Update `onUnmounted` to call `unlistenDimensions?.()`.

- [ ] **Step 2: Update `commitments-changed` handler to also reload dimensions**

```typescript
// src/App.vue — in onMounted, update the commitments-changed block (lines 80-89):
unlistenCommitments = await listen<ConfigErrorDetail[]>("commitments-changed", async () => {
  if (store.status !== "ready") return;
  const { year, month } = yearMonthFromDate(store.currentDate);
  try {
    // Reload both commitments AND dimensions (monthly file contains both)
    const dimsResult = await invoke("get_month_dimensions", {
      rootPath: store.rootPath, year, month,
    }) as MonthDimensionsResult;
    store.dimensions = dimsResult.dimensions;
    store.fromTemplate = dimsResult.from_template;
    store.commitments = (await invoke("get_commitments", { rootPath: store.rootPath, year, month })) as Commitment[];
    store.commitmentProgress = (await invoke("get_commitment_progress", { rootPath: store.rootPath, year, month })) as CommitmentProgress[];
  } catch (e) {
    logError("App.commitmentsChanged", e);
  }
});
```

- [ ] **Step 3: Update tests for new event names**

```bash
grep -rn "config-changed" src/__tests__/
```

Update all test references from `"config-changed"` to `"dimensions-changed"`.

```bash
npx vitest run
```

- [ ] **Step 4: Commit**

```bash
git add src/App.vue src/stores/useStore.ts src/__tests__/
git commit -m "refactor: replace config-changed with dimensions-changed, narrow handlers"
```

---

### Task 9: Migration — split existing _monthly.md files

**Files:**
- Modify: `src-tauri/src/commands.rs` (`load_root_state` / `init` function)
- Create/Modify: `src-tauri/src/files.rs` (migration function)

- [ ] **Step 1: Write the migration function**

```rust
// src-tauri/src/files.rs

/// Migrate old _monthly.md → dimensions.yaml + commitments.yaml.
/// Returns (had_dimensions, had_commitments) on success.
pub fn migrate_monthly_file(
    root: &Path,
    year: i32,
    month: u32,
) -> Result<(bool, bool), String> {
    let old_path = monthly_path(root, year, month);
    if !old_path.exists() {
        return Ok((false, false));
    }

    let monthly = read_monthly_file(root, year, month)?;
    let had_dims = !monthly.dimensions.is_empty();
    let had_commits = !monthly.commitments.is_empty();

    // Write dimensions.yaml if dimensions exist and file doesn't already exist
    if had_dims && !dimensions_path(root, year, month).exists() {
        write_dimensions_file(root, year, month, &monthly.dimensions)?;
    }

    // Write commitments.yaml if commitments exist and file doesn't already exist
    if had_commits && !commitments_path(root, year, month).exists() {
        write_commitments_file(root, year, month, &monthly.commitments)?;
    }

    // Rename old file to .bak (don't delete — user can recover)
    let bak_path = old_path.with_extension("md.bak");
    fs::rename(&old_path, &bak_path)
        .map_err(|e| format!("Failed to rename {} to {}: {}", old_path.display(), bak_path.display(), e))?;

    Ok((had_dims, had_commits))
}
```

- [ ] **Step 2: Call migration during `init` / `load_root_state`**

In `commands.rs`, find `load_root_state`. After root path is validated and before returning `InitResult::Ready`, call migration for the current month:

```rust
// In load_root_state, after validating template:
let now = chrono::Local::now();
let (year, month) = (now.year(), now.month());

// Migrate if old _monthly.md exists
if files::monthly_path(&root, year, month).exists() {
    let (had_dims, had_commits) = files::migrate_monthly_file(&root, year, month)
        .unwrap_or((false, false));
    if had_dims || had_commits {
        crate::error_log::log_info(
            "migration",
            &format!("Migrated {}/{:02}/_monthly.md → dimensions.yaml ({}) + commitments.yaml ({})",
                year, month, had_dims, had_commits),
        );
    }
}
```

- [ ] **Step 3: Write an integration test for migration**

Create `src-tauri/tests/migration_test.rs`:

```rust
#[test]
fn test_migrate_monthly_file() {
    // 1. Create a temp dir with _monthly.md containing dimensions + commitments
    // 2. Call migrate_monthly_file
    // 3. Assert dimensions.yaml and commitments.yaml exist with correct content
    // 4. Assert _monthly.md was renamed to _monthly.md.bak
    // 5. Clean up
}
```

```bash
cd src-tauri && cargo test test_migrate_monthly_file
```

- [ ] **Step 4: Run full test suite**

```bash
cd src-tauri && cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/files.rs src-tauri/src/commands.rs src-tauri/tests/
git commit -m "feat: migrate _monthly.md to dimensions.yaml + commitments.yaml"
```

---

### Task 10: Backend — save_dimensions command

**Files:**
- Modify: `src-tauri/src/commands.rs` (new `save_dimensions` command)
- Modify: `src-tauri/src/lib.rs` (register command)

- [ ] **Step 1: Add `save_dimensions` Tauri command**

```rust
// src-tauri/src/commands.rs

#[tauri::command]
pub fn save_dimensions(
    root_path: String,
    year: i32,
    month: u32,
    dimensions: Vec<Dimension>,
) -> Result<Vec<Dimension>, String> {
    let root = Path::new(&root_path);
    if !root.exists() {
        return Err("Root path does not exist".to_string());
    }

    // Validate before writing
    validate_dimensions(&dimensions)?;

    // Write to dimensions.yaml (creates file if month not instantiated)
    write_dimensions_file(root, year, month, &dimensions)?;

    // Return the saved dimensions (so frontend can update store directly)
    Ok(dimensions)
}
```

- [ ] **Step 2: Add `save_dimensions_template` command**

```rust
#[tauri::command]
pub fn save_dimensions_template(
    root_path: String,
    dimensions: Vec<Dimension>,
) -> Result<(), String> {
    let root = Path::new(&root_path);

    // Validate before writing
    validate_dimensions(&dimensions)?;

    // Write to dimensions.template.yaml
    let template = Template { dimensions };
    let path = dimensions_template_path(root);
    let yaml_body = yaml_serde::to_string(&template)
        .map_err(|e| format!("Failed to serialize template: {}", e))?;
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, yaml_body)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}
```

- [ ] **Step 3: Register commands in lib.rs**

```rust
// src-tauri/src/lib.rs
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::save_dimensions,
    commands::save_dimensions_template,
])
```

- [ ] **Step 4: Write integration tests**

```bash
cd src-tauri && cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add save_dimensions and save_dimensions_template commands"
```

---

### Task 11: DimensionEditorModal.vue — component skeleton

**Files:**
- Create: `src/components/composite/DimensionEditorModal.vue`
- Create: `src/__tests__/components/composite/DimensionEditorModal.test.ts`

- [ ] **Step 1: Write the failing test (modal opens/closes)**

```typescript
// src/__tests__/components/composite/DimensionEditorModal.test.ts
import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import DimensionEditorModal from "../../../components/composite/DimensionEditorModal.vue";
import type { Dimension } from "../../../types";

const MOCK_DIMENSIONS: Dimension[] = [
  { name: "Goal", key: "goal", source: "monthly", values: undefined, required: false, deleted: false },
  { name: "Biz", key: "biz", source: "static", values: ["Product", "Marketing", "Engineering"], required: true, deleted: false },
  { name: "Importance", key: "importance-urgency", source: "static", values: ["P0", "P1"], required: false, deleted: false },
];

describe("DimensionEditorModal", () => {
  it("renders when open is true", () => {
    const wrapper = mount(DimensionEditorModal, {
      props: { open: true, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6 },
    });
    expect(wrapper.find('[data-test="overlay"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("Edit Dimensions");
  });

  it("does not render when open is false", () => {
    const wrapper = mount(DimensionEditorModal, {
      props: { open: false, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6 },
    });
    expect(wrapper.find('[data-test="overlay"]').exists()).toBe(false);
  });

  it("emits close when Cancel is clicked", async () => {
    const wrapper = mount(DimensionEditorModal, {
      props: { open: true, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6 },
    });
    await wrapper.find('[data-test="cancel"]').trigger("click");
    expect(wrapper.emitted("close")).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run test — fail (no component)**

```bash
npx vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts
```

Expected: FAIL (component not found)

- [ ] **Step 3: Write minimal component skeleton**

Create `src/components/composite/DimensionEditorModal.vue` with the CommitmentsModal pattern:

```vue
<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import type { Dimension } from "../../types";

const props = defineProps<{
  open: boolean;
  dimensions: Dimension[];
  rootPath: string;
  year: number;
  month: number;
}>();

const emit = defineEmits<{ close: []; saved: [Dimension[]] }>();

const overlayRef = ref<HTMLElement>();
const showDiscard = ref(false);

watch(() => props.open, (o) => {
  if (!o) return;
  showDiscard.value = false;
  nextTick(() => overlayRef.value?.focus());
}, { immediate: true });

function requestClose() { emit("close"); }

// Keyboard: esc to close, cmd+enter to save (placeholder)
function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") { e.preventDefault(); requestClose(); }
}

const monthLabel = new Date(props.year, props.month - 1, 1)
  .toLocaleDateString("en-US", { month: "long", year: "numeric" });
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      ref="overlayRef"
      data-test="overlay" tabindex="-1"
      @keydown="onKeydown"
      @click.self="requestClose"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
    >
      <div
        role="dialog" aria-modal="true"
        class="relative w-[660px] max-w-[92vw] max-h-[88vh] flex flex-col bg-[var(--color-surface)]
               border border-[var(--color-border-form)] rounded-[var(--radius-lg)]
               shadow-[var(--shadow-popover)] overflow-hidden"
      >
        <!-- Header -->
        <div class="flex justify-between items-start px-2xl pt-xl pb-lg border-b border-[var(--color-divider)]">
          <div>
            <div class="text-title font-bold text-[var(--color-text-primary)] tracking-[-0.3px]">Edit Dimensions</div>
            <div class="text-secondary text-[var(--color-text-muted)] mt-2xs">Editing {{ monthLabel }}</div>
          </div>
          <span class="text-[var(--color-text-muted)] cursor-pointer text-[20px] leading-none" @click="requestClose">&times;</span>
        </div>

        <!-- Body placeholder -->
        <div class="flex-1 overflow-y-auto px-2xl py-xl">
          <p class="text-secondary text-[var(--color-text-muted)]">Dimension editor body — coming in next task.</p>
        </div>

        <!-- Footer -->
        <div class="flex justify-end gap-sm px-2xl py-lg border-t border-[var(--color-divider)]">
          <button
            data-test="cancel"
            class="text-secondary font-semibold text-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer"
            @click="requestClose"
          >Cancel</button>
          <button
            data-test="save"
            class="text-secondary font-semibold text-white bg-[var(--color-brand-solid)] hover:bg-[var(--color-brand-link)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer disabled:opacity-50"
          >Save</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
```

- [ ] **Step 4: Run test — pass**

```bash
npx vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "feat: DimensionEditorModal skeleton — opens/closes with header/footer"
```

---

### Task 12: DimensionEditorModal — dimension list (left panel)

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue` (add left panel)
- Modify: `src/__tests__/components/composite/DimensionEditorModal.test.ts` (add list tests)

- [ ] **Step 1: Write failing test**

```typescript
// In DimensionEditorModal.test.ts, add:
it("renders all dimensions in the left panel", () => {
  const wrapper = mount(DimensionEditorModal, {
    props: { open: true, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6 },
  });
  expect(wrapper.text()).toContain("Goal");
  expect(wrapper.text()).toContain("Biz");
  expect(wrapper.text()).toContain("Importance");
});

it("selects a dimension on click", async () => {
  const wrapper = mount(DimensionEditorModal, {
    props: { open: true, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6 },
  });
  const bizRow = wrapper.findAll('[data-test="dim-row"]')[1]; // second row = Biz
  await bizRow.trigger("click");
  // Should show Biz's name input in the right panel
  const nameInput = wrapper.find('input[placeholder="Dimension name"]');
  expect((nameInput.element as HTMLInputElement).value).toBe("Biz");
});
```

- [ ] **Step 2: Run test — fail**

```bash
npx vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts -t "renders all dimensions"
```

- [ ] **Step 3: Add left panel with dimension list to component**

Add a reactive `draft` ref (deep copy of props.dimensions on open), selected index, and left panel markup. Follow CommitmentsModal pattern: draft is initialized on open.

```vue
<script setup lang="ts">
// ... existing script ...
import { ref, computed, watch, nextTick } from "vue";

const draft = ref<Dimension[]>([]);
const selectedIndex = ref(0);

watch(() => props.open, (o) => {
  if (!o) return;
  // Deep clone dimensions into draft
  draft.value = JSON.parse(JSON.stringify(props.dimensions));
  selectedIndex.value = 0;
  showDiscard.value = false;
  nextTick(() => overlayRef.value?.focus());
}, { immediate: true });

const isDirty = computed(() =>
  JSON.stringify(draft.value) !== JSON.stringify(props.dimensions)
);

const selectedDimension = computed(() => draft.value[selectedIndex.value] ?? null);

function selectDim(index: number) { selectedIndex.value = index; }
```

Replace the body placeholder with two-column layout:

```html
<div class="flex-1 flex min-h-0">
  <!-- Left panel: dimension list -->
  <div class="w-[210px] flex-shrink-0 border-r border-[var(--color-divider)] bg-[var(--color-surface-muted)] p-md flex flex-col">
    <div class="flex-1 space-y-2xs">
      <div
        v-for="(dim, i) in draft"
        :key="dim.key"
        data-test="dim-row"
        :class="[
          'flex items-center gap-sm px-sm py-sm rounded-[var(--radius-form-lg)] cursor-pointer',
          i === selectedIndex ? 'bg-[var(--color-brand-soft-bg)]' : ''
        ]"
        @click="selectDim(i)"
      >
        <div
          class="w-[3px] h-[16px] rounded-[1px] flex-shrink-0"
          :style="{ background: `var(--dim-bar-${dim.key})` }"
        ></div>
        <span class="text-body text-[var(--color-text-primary)] flex-1">{{ dim.name }}</span>
        <span class="text-micro text-[var(--color-text-muted)]">{{ dim.source }}</span>
      </div>
    </div>
    <button class="text-secondary font-semibold text-[var(--color-brand-link)] text-left mt-sm cursor-pointer">
      + Add dimension
    </button>
  </div>

  <!-- Right panel placeholder -->
  <div class="flex-1 px-2xl py-xl">
    <p class="text-secondary text-[var(--color-text-muted)]">Right panel — next task.</p>
  </div>
</div>
```

- [ ] **Step 4: Run test — pass**

```bash
npx vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts -t "renders all dimensions"
```

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "feat: DimensionEditorModal left panel — dimension list with selection"
```

---

### Task 13: DimensionEditorModal — right panel (static dimension editor)

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue` (right panel)
- Modify: `src/__tests__/components/composite/DimensionEditorModal.test.ts` (add editor tests)

- [ ] **Step 1: Write failing test**

```typescript
it("shows values for selected static dimension", () => {
  const wrapper = mount(DimensionEditorModal, {
    props: { open: true, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6 },
  });
  // Biz is index 1, has values
  expect(wrapper.text()).toContain("Product");
  expect(wrapper.text()).toContain("Marketing");
  expect(wrapper.text()).toContain("Engineering");
});

it("updates dimension name on input", async () => {
  const wrapper = mount(DimensionEditorModal, {
    props: { open: true, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6 },
  });
  const nameInput = wrapper.find('input[placeholder="Dimension name"]');
  await nameInput.setValue("Business");
  expect((nameInput.element as HTMLInputElement).value).toBe("Business");
});
```

- [ ] **Step 2: Run — fail**

```bash
npx vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts -t "shows values"
```

- [ ] **Step 3: Add right panel for static dimension**

Replace the right panel placeholder with the full editor:

```html
<!-- Right panel -->
<div class="flex-1 px-2xl py-xl flex flex-col" v-if="selectedDimension">
  <!-- Name -->
  <div class="flex items-baseline gap-sm mb-lg">
    <input
      v-model="selectedDimension.name"
      placeholder="Dimension name"
      class="text-title font-bold tracking-[-0.3px] border-0 border-b border-[var(--color-border-form)] px-0 py-2xs w-[160px] outline-none bg-transparent text-[var(--color-text-primary)]"
    >
    <span class="text-secondary text-[var(--color-text-muted)]">
      key: <code class="text-secondary text-[var(--color-text-secondary)] bg-[var(--color-page-bg)] px-xs py-2xs rounded-[var(--radius-sm)] font-mono">{{ selectedDimension.key }}</code>
      <span class="text-micro text-[var(--color-text-disabled)] ml-2xs">(locked)</span>
    </span>
  </div>

  <!-- Source + Required -->
  <div class="flex items-center gap-lg mb-xl">
    <div class="flex items-center gap-xs">
      <span class="text-secondary text-[var(--color-text-muted)]">Source:</span>
      <span class="text-micro font-semibold uppercase tracking-[0.3px] px-sm py-2xs rounded-[var(--radius-sm)] bg-[#f3f4f6] text-[var(--color-text-secondary)]">
        {{ selectedDimension.source }}
      </span>
      <span class="text-micro text-[var(--color-text-disabled)]">(locked)</span>
    </div>
    <label class="text-secondary flex items-center gap-xs text-[var(--color-text-primary)] cursor-pointer">
      <input type="checkbox" v-model="selectedDimension.required" class="accent-[var(--color-brand-solid)]">
      Required
    </label>
  </div>

  <!-- Values (static only) -->
  <template v-if="selectedDimension.source === 'static'">
    <div class="text-micro font-bold uppercase tracking-[0.5px] text-[var(--color-text-muted)] mb-sm">Values</div>
    <div class="space-y-xs mb-sm">
      <div
        v-for="(val, vi) in selectedDimension.values"
        :key="vi"
        class="flex items-center gap-xs"
      >
        <span class="text-micro text-[var(--color-text-disabled)] w-[16px] text-center">⠿</span>
        <input
          v-model="selectedDimension.values![vi]"
          class="text-body px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)] flex-1 outline-none text-[var(--color-text-primary)]"
        >
        <span
          class="cursor-pointer text-[var(--color-text-disabled)] w-[20px] text-center"
          @click="selectedDimension.values!.splice(vi, 1)"
        >&times;</span>
      </div>
    </div>
    <div class="flex gap-xs items-center pl-[22px]">
      <input
        v-model="newValue"
        placeholder="New value"
        class="text-body px-sm py-xs border border-dashed border-[var(--color-border-form)] rounded-[var(--radius-form)] w-[140px] outline-none text-[var(--color-placeholder)]"
        @keydown.enter="addValue"
      >
      <button
        class="text-secondary px-md py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)] bg-white text-[var(--color-text-secondary)] cursor-pointer"
        @click="addValue"
      >+</button>
    </div>
  </template>

  <!-- Monthly info -->
  <div v-else class="text-secondary text-[var(--color-text-secondary)] bg-[var(--color-page-bg)] rounded-[var(--radius-form-lg)] p-md">
    Values are derived from commitment goals.<br>Edit commitments to change available values.
  </div>

  <!-- Delete -->
  <div class="mt-auto pt-md border-t border-[var(--color-divider)]">
    <button
      class="text-secondary px-md py-sm border border-[#fecaca] rounded-[var(--radius-form)] bg-white text-[var(--color-danger)] cursor-pointer"
      @click="toggleDelete"
    >{{ selectedDimension.deleted ? 'Undo delete' : 'Delete dimension' }}</button>
  </div>
</div>
```

Add the missing methods:
```typescript
const newValue = ref("");

function addValue() {
  const v = newValue.value.trim();
  if (!v || !selectedDimension.value || selectedDimension.value.source !== "static") return;
  if (!selectedDimension.value.values) selectedDimension.value.values = [];
  if (selectedDimension.value.values.includes(v)) return; // no duplicates
  selectedDimension.value.values.push(v);
  newValue.value = "";
}

function toggleDelete() {
  if (!selectedDimension.value) return;
  selectedDimension.value.deleted = !selectedDimension.value.deleted;
}
```

- [ ] **Step 4: Run tests — pass**

```bash
npx vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "feat: DimensionEditorModal right panel — static dimension editor with values"
```

---

### Task 14: DimensionEditorModal — save logic, discard confirmation, save-as-template

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue` (save, discard)
- Modify: `src/__tests__/components/composite/DimensionEditorModal.test.ts` (save tests)

- [ ] **Step 1: Write failing save test**

```typescript
it("emits saved with updated dimensions on Save", async () => {
  const wrapper = mount(DimensionEditorModal, {
    props: { open: true, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6 },
  });
  // Change Biz name
  const nameInput = wrapper.find('input[placeholder="Dimension name"]');
  await nameInput.setValue("Business Line");
  // Click Save
  await wrapper.find('[data-test="save"]').trigger("click");
  expect(wrapper.emitted("saved")).toBeTruthy();
  const saved = (wrapper.emitted("saved")![0] as Dimension[][])[0];
  expect(saved.find((d: Dimension) => d.key === "biz")!.name).toBe("Business Line");
});
```

- [ ] **Step 2: Add save logic and discard confirmation**

```typescript
// In script setup:
import { invoke } from "@tauri-apps/api/core";
import { logError } from "../../utils/errorLog";

const saving = ref(false);
const error = ref("");

const isDirty = computed(() =>
  JSON.stringify(draft.value) !== JSON.stringify(props.dimensions)
);

function requestClose() {
  if (isDirty.value) { showDiscard.value = true; return; }
  emit("close");
}

async function save() {
  saving.value = true;
  error.value = "";
  try {
    const saved = await invoke("save_dimensions", {
      rootPath: props.rootPath,
      year: props.year,
      month: props.month,
      dimensions: draft.value,
    }) as Dimension[];
    emit("saved", saved);
    emit("close");
  } catch (e) {
    logError("DimensionEditorModal.save", e);
    error.value = typeof e === "string" ? e : String(e);
  } finally {
    saving.value = false;
  }
}

async function saveAsTemplate() {
  try {
    await invoke("save_dimensions_template", {
      rootPath: props.rootPath,
      dimensions: draft.value,
    });
  } catch (e) {
    logError("DimensionEditorModal.saveAsTemplate", e);
  }
}

function onKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); save(); return; }
  if (e.key === "Escape") { e.preventDefault(); requestClose(); }
}
```

Update the header to include "Save as template":
```html
<div class="text-secondary text-[var(--color-text-muted)] mt-2xs">
  Editing {{ monthLabel }}
  <span class="text-[var(--color-brand-link)] cursor-pointer font-medium ml-lg" @click="saveAsTemplate">Save as template</span>
</div>
```

Update footer with error display and saving state:
```html
<div v-if="error" class="px-2xl pb-sm text-secondary text-[var(--color-danger)]">{{ error }}</div>
<div class="flex justify-end gap-sm px-2xl py-lg border-t border-[var(--color-divider)]">
  <button data-test="cancel" ...>Cancel</button>
  <button data-test="save" :disabled="saving" @click="save" ...>{{ saving ? 'Saving…' : 'Save' }}</button>
</div>
```

Add discard overlay (after footer, inside the dialog):
```html
<div v-if="showDiscard" data-test="discard-confirm" class="absolute inset-0 flex items-center justify-center bg-black/10">
  <div class="bg-[var(--color-surface)] border border-[var(--color-border-form)] rounded-[var(--radius-card)] shadow-[var(--shadow-toast)] p-lg max-w-[300px]">
    <div class="text-body font-semibold text-[var(--color-text-primary)] mb-xs">Discard changes?</div>
    <div class="text-secondary text-[var(--color-text-secondary)] mb-md">Your edits to dimensions won't be saved.</div>
    <div class="flex justify-end gap-sm">
      <button class="text-secondary font-semibold text-[var(--color-text-muted)] px-md py-sm cursor-pointer" @click="showDiscard = false">Keep editing</button>
      <button data-test="discard-yes" class="text-secondary font-semibold text-[var(--color-danger)] rounded-[var(--radius-form)] px-md py-sm cursor-pointer" @click="() => { showDiscard = false; emit('close'); }">Discard</button>
    </div>
  </div>
</div>
```

- [ ] **Step 3: Run tests — pass**

```bash
npx vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts
```

- [ ] **Step 4: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "feat: DimensionEditorModal — save, discard confirmation, save-as-template"
```

---

### Task 15: Wire ⚙ into EntryComposer and DimensionEditorModal into MonthView

**Files:**
- Modify: `src/components/EntryComposer.vue` (add ⚙ icon)
- Modify: `src/components/MonthView.vue` (integrate DimensionEditorModal)

- [ ] **Step 1: Add ⚙ to EntryComposer**

In `EntryComposer.vue`, find the composer input row template. Add the ⚙ icon after the Enter badge:

```html
<!-- In the input row, after the Enter badge: -->
<span
  class="text-[length:14px] text-[var(--color-text-muted)] hover:text-[var(--color-brand-solid)] cursor-pointer flex-shrink-0 py-2xs px-2xs"
  @click.stop="$emit('editDimensions')"
  title="Edit dimensions"
>⚙</span>
```

Add the emit declaration:
```typescript
const emit = defineEmits<{
  submit: [value: CreateEntryInput];
  editDimensions: [];
}>();
```

- [ ] **Step 2: Add DimensionEditorModal to MonthView**

In `MonthView.vue`, add a reactive state for the modal:

```typescript
const showDimEditor = ref(false);

function openDimEditor() { showDimEditor.value = true; }

function onDimensionsSaved(dims: Dimension[]) {
  store.dimensions = dims;
  store.fromTemplate = false;
  showDimEditor.value = false;
}
```

Add the modal in the template:
```html
<DimensionEditorModal
  :open="showDimEditor"
  :dimensions="store.dimensions"
  :root-path="store.rootPath"
  :year="selectedYear"
  :month="selectedMonth"
  @close="showDimEditor = false"
  @saved="onDimensionsSaved"
/>
```

Wire the composer's `@editDimensions` to `openDimEditor`:
```html
<EntryComposer ... @edit-dimensions="openDimEditor" />
```

- [ ] **Step 3: Run frontend type check + tests**

```bash
npx vue-tsc --noEmit
npx vitest run
```

- [ ] **Step 4: Commit**

```bash
git add src/components/EntryComposer.vue src/components/MonthView.vue
git commit -m "feat: wire ⚙ into EntryComposer, DimensionEditorModal into MonthView"
```

---

### Task 16: Add dimension + Show deleted

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue` (add form, show deleted)
- Modify: `src/__tests__/components/composite/DimensionEditorModal.test.ts` (add tests)

- [ ] **Step 1: Write failing test for add dimension**

```typescript
it("shows add dimension form on button click", async () => {
  const wrapper = mount(DimensionEditorModal, {
    props: { open: true, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6 },
  });
  await wrapper.find('[data-test="add-dimension"]').trigger("click");
  expect(wrapper.find('[data-test="add-dim-form"]').exists()).toBe(true);
});

it("adds a new dimension on create", async () => {
  const wrapper = mount(DimensionEditorModal, {
    props: { open: true, dimensions: MOCK_DIMENSIONS, rootPath: "/test", year: 2026, month: 6 },
  });
  await wrapper.find('[data-test="add-dimension"]').trigger("click");
  await wrapper.find('[data-test="new-dim-name"]').setValue("Category");
  await wrapper.find('[data-test="new-dim-key"]').setValue("category");
  await wrapper.find('[data-test="new-dim-create"]').trigger("click");
  // Should now have 4 dimensions
  const rows = wrapper.findAll('[data-test="dim-row"]');
  expect(rows).toHaveLength(4);
});
```

- [ ] **Step 2: Add add-dimension form**

Add state and methods:
```typescript
const showAddForm = ref(false);
const newDimName = ref("");
const newDimKey = ref("");
const newDimSource = ref<"static" | "monthly">("static");
const keyError = ref("");

function validateKey(key: string): string | null {
  if (!key) return "Key is required";
  if (!/^[a-zA-Z0-9_-]+$/.test(key)) return "Only letters, numbers, hyphens, and underscores allowed";
  if (draft.value.some(d => d.key === key)) return `Key '${key}' already exists.`;
  return null;
}

function startAddDim() {
  showAddForm.value = true;
  newDimName.value = "";
  newDimKey.value = "";
  newDimSource.value = "static";
  keyError.value = "";
}

function createDim() {
  const err = validateKey(newDimKey.value.trim());
  if (err) { keyError.value = err; return; }
  if (newDimSource.value === "monthly" && draft.value.some(d => !d.deleted && d.source === "monthly")) {
    keyError.value = "Only one monthly-source dimension is allowed.";
    return;
  }
  draft.value.push({
    name: newDimName.value.trim() || "Untitled",
    key: newDimKey.value.trim(),
    source: newDimSource.value,
    values: newDimSource.value === "static" ? [] : undefined,
    required: false,
    deleted: false,
  });
  selectedIndex.value = draft.value.length - 1;
  showAddForm.value = false;
  keyError.value = "";
}
```

Add the inline form in the left panel (after the dimension list, before the + button):
```html
<div v-if="showAddForm" data-test="add-dim-form" class="mt-sm p-sm border border-[var(--color-brand-solid)] rounded-[var(--radius-form-lg)] bg-[var(--color-brand-soft-bg)]">
  <input v-model="newDimName" data-test="new-dim-name" placeholder="Name" class="text-body w-full px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)] mb-sm outline-none">
  <input v-model="newDimKey" data-test="new-dim-key" placeholder="Key" class="text-secondary w-full px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)] mb-xs outline-none font-mono">
  <div v-if="keyError" class="text-micro text-[var(--color-danger)] mb-xs">{{ keyError }}</div>
  <select v-model="newDimSource" class="text-micro px-xs py-2xs border border-[var(--color-border-form)] rounded-[var(--radius-form)] mb-sm">
    <option value="static">static</option>
    <option value="monthly">monthly</option>
  </select>
  <div class="flex gap-xs">
    <button class="text-micro font-semibold text-[var(--color-text-muted)] px-sm py-xs cursor-pointer" @click="showAddForm = false">Cancel</button>
    <button data-test="new-dim-create" class="text-micro font-semibold text-white bg-[var(--color-brand-solid)] rounded-[var(--radius-form)] px-sm py-xs cursor-pointer" @click="createDim">Create</button>
  </div>
</div>
```

- [ ] **Step 2.5: Update right panel — read-only for deleted dimensions**

When `selectedDimension.deleted` is true, the right panel fields should be disabled and the delete button replaced with "Restore":

```html
<!-- Name input: add :disabled -->
<input
  v-model="selectedDimension.name"
  :disabled="selectedDimension.deleted"
  ...
>

<!-- Values inputs: add :disabled -->
<input
  v-model="selectedDimension.values![vi]"
  :disabled="selectedDimension.deleted"
  ...
>

<!-- Delete/Restore button -->
<button
  v-if="!selectedDimension.deleted"
  class="text-secondary px-md py-sm border border-[#fecaca] rounded-[var(--radius-form)] bg-white text-[var(--color-danger)] cursor-pointer"
  @click="toggleDelete"
>Delete dimension</button>
<button
  v-else
  class="text-secondary font-semibold px-md py-sm rounded-[var(--radius-form)] text-[var(--color-brand-link)] cursor-pointer"
  @click="toggleDelete"
>Restore</button>
```

- [ ] **Step 3: Add Show deleted toggle**

```typescript
const showDeleted = ref(false);

const visibleDimensions = computed(() =>
  showDeleted.value ? draft.value : draft.value.filter(d => !d.deleted)
);
```

Add toggle in left panel (below the dimension list, above + Add dimension):
```html
<label
  v-if="draft.some(d => d.deleted)"
  class="flex items-center gap-xs text-micro text-[var(--color-text-muted)] cursor-pointer mt-sm"
>
  <input type="checkbox" v-model="showDeleted" class="accent-[var(--color-brand-solid)]">
  Show deleted
</label>
```

Update the v-for to use `visibleDimensions` instead of `draft`. Deleted rows get `opacity-40` and are not draggable.

- [ ] **Step 4: Run tests — pass**

```bash
npx vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "feat: add dimension form + show deleted toggle"
```

---

### Task 17: Drag to reorder (left panel + values)

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue` (vue-draggable-plus)
- Modify: `src/__tests__/components/composite/DimensionEditorModal.test.ts` (drag tests)

- [ ] **Step 1: Add drag handle classes to the left panel dimension rows**

```html
<!-- Each dimension row gets a drag grip before the color bar: -->
<span class="text-micro text-[var(--color-text-disabled)] w-[16px] text-center cursor-grab drag-grip-dim">⠿</span>
```

- [ ] **Step 2: Wrap dimension list with VueDraggable**

```vue
<script setup>
import { VueDraggable } from "vue-draggable-plus";
</script>

<VueDraggable v-model="draft" handle=".drag-grip-dim" :animation="150" class="flex-1 space-y-2xs">
  <!-- dimension rows -->
</VueDraggable>
```

- [ ] **Step 3: Wrap values list with VueDraggable for static values**

```html
<VueDraggable v-model="selectedDimension.values!" handle=".drag-grip-val" :animation="150" class="space-y-xs">
  <!-- value rows -->
</VueDraggable>
```

Update value row drag grip to use `drag-grip-val` class.

- [ ] **Step 4: Run typecheck + tests**

```bash
npx vue-tsc --noEmit
npx vitest run
```

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue
git commit -m "feat: drag to reorder dimensions and values"
```

---

### Task 18: CLI — dimensions get and set

**Files:**
- Create: `src-tauri/src/cli/dimensions.rs`
- Modify: `src-tauri/src/cli/mod.rs` (register subcommand)
- Modify: `src-tauri/src/cli/bin/logbook-cli.rs` (register command)

- [ ] **Step 1: Create `src-tauri/src/cli/dimensions.rs`**

```rust
use clap::Subcommand;
use crate::cli::output::{print_error, print_output};

#[derive(Subcommand)]
pub enum DimensionsCommands {
    /// Get dimensions for a month or the template
    Get {
        /// Year
        #[arg(long, required_unless_present = "template")]
        year: Option<i32>,
        /// Month (1-12)
        #[arg(long, required_unless_present = "template")]
        month: Option<u32>,
        /// Get template dimensions instead of monthly
        #[arg(long)]
        template: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Set dimensions for a month or the template (reads from stdin)
    Set {
        /// Year
        #[arg(long, required_unless_present = "template")]
        year: Option<i32>,
        /// Month (1-12)
        #[arg(long, required_unless_present = "template")]
        month: Option<u32>,
        /// Set template dimensions instead of monthly
        #[arg(long)]
        template: bool,
        /// Input is JSON instead of YAML
        #[arg(long)]
        json: bool,
    },
}

pub fn handle_dimensions(cmd: DimensionsCommands, root: &std::path::Path) -> Result<(), String> {
    match cmd {
        DimensionsCommands::Get { year, month, template, json } => {
            let dims: Vec<crate::models::Dimension> = if template {
                crate::files::read_dimensions_template(root)?.dimensions
            } else {
                let y = year.unwrap();
                let m = month.unwrap();
                let source_line = format!(
                    "# source: {}/{}/dimensions.yaml",
                    y,
                    format!("{:02}", m)
                );
                let dims = crate::files::resolve_month_dimensions(root, y, m)?;
                if !json {
                    println!("{}", source_line);
                }
                dims
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&dims).map_err(|e| e.to_string())?);
            } else {
                println!("{}", yaml_serde::to_string(&dims).map_err(|e| e.to_string())?);
            }
            Ok(())
        }
        DimensionsCommands::Set { year, month, template, json } => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).map_err(|e| e.to_string())?;
            let dims: Vec<crate::models::Dimension> = if json {
                serde_json::from_str(&input).map_err(|e| format!("Invalid JSON: {}", e))?
            } else {
                yaml_serde::from_str(&input).map_err(|e| format!("Invalid YAML: {}", e))?
            };
            crate::config::validate_dimensions(&dims)?;
            if template {
                let tmpl = crate::models::Template { dimensions: dims };
                let path = crate::files::dimensions_template_path(root);
                let yaml = yaml_serde::to_string(&tmpl).map_err(|e| e.to_string())?;
                let tmp = path.with_extension("tmp");
                std::fs::write(&tmp, &yaml).map_err(|e| e.to_string())?;
                std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
            } else {
                crate::files::write_dimensions_file(root, year.unwrap(), month.unwrap(), &dims)?;
            }
            Ok(())
        }
    }
}
```

- [ ] **Step 2: Register in `cli/mod.rs`**

```rust
mod dimensions;
use dimensions::{DimensionsCommands, handle_dimensions};

// In the CLI enum, add:
#[derive(Subcommand)]
enum Cli {
    // ... existing ...
    #[command(subcommand)]
    Dimensions(DimensionsCommands),
}

// In the match arm:
Cli::Dimensions(cmd) => handle_dimensions(cmd, &root),
```

- [ ] **Step 3: Build and test CLI**

```bash
cd src-tauri && cargo build --bin logbook-cli
./target/debug/logbook-cli dimensions get --template --json
```

Expected: Outputs current template dimensions as JSON.

```bash
echo '[{"name":"Test","key":"test","source":"static","values":["a","b"],"required":false,"deleted":false}]' | ./target/debug/logbook-cli dimensions set --template --json
./target/debug/logbook-cli dimensions get --template
```

Expected: Outputs the Test dimension.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/cli/dimensions.rs src-tauri/src/cli/mod.rs src-tauri/src/cli/bin/logbook-cli.rs
git commit -m "feat: CLI dimensions get/set commands"
```

---

### Task 19: End-to-end verification

**Files:**
- (No new files — verification run)

- [ ] **Step 1: Run full backend test suite**

```bash
cd src-tauri && cargo test && cargo clippy
```

Expected: All tests pass, no clippy warnings.

- [ ] **Step 2: Run full frontend test suite + type check + build**

```bash
npx vue-tsc --noEmit && npx vitest run && npm run build
```

Expected: All pass.

- [ ] **Step 3: Manual smoke test**

```bash
pnpm tauri dev
```

Test flow:
1. App starts, opens June 2026 — migration runs (if _monthly.md exists)
2. Click ⚙ in composer → DimensionEditorModal opens
3. Edit Biz name → Save → dimensions persist
4. Click "Save as template" → template updated
5. Add new dimension "Category" → appears in popover
6. Delete "Importance" → Toast appears → Show deleted → Restore
7. CLI: `logbook-cli dimensions get --year 2026 --month 6` returns correct data

- [ ] **Step 4: Commit any final fixes**

```bash
git add -A && git commit -m "chore: e2e verification fixes"
```
