# Data Version Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `version.txt` file to the data root so the app can detect incompatible data formats and direct users to a migration tool.

**Architecture:** A new `check_data_version` function (commands.rs) reads `{root}/version.txt` and compares it against `CURRENT_DATA_VERSION` (models.rs). On mismatch or missing file, `init` returns a new `InitResult` variant that the frontend renders as a dedicated migration-required screen. `set_root_path` writes the initial version file. The main app never bumps the version — a separate migration tool handles that.

**Tech Stack:** Rust (Tauri 2.x), TypeScript, Vue 3

---

### Task 1: Add CURRENT_DATA_VERSION and InitResult variants

**Files:**
- Modify: `src-tauri/src/models.rs:121-147`

- [ ] **Step 1: Add CURRENT_DATA_VERSION constant**

```rust
/// Current data format version. The main app never bumps this — only a
/// format-changing PR does. A separate migration tool bumps version.txt on disk.
pub const CURRENT_DATA_VERSION: u32 = 1;
```

Insert it right above the `RecoveryCategory` enum (before line 121).

- [ ] **Step 2: Add new InitResult variants**

Replace the `InitResult` enum (lines 129-147) to include the two new variants:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum InitResult {
    NeedsSetup,
    DataVersionNotFound {
        root_path: String,
    },
    DataVersionMismatch {
        root_path: String,
        expected: u32,
        found: u32,
    },
    ConfigError {
        category: RecoveryCategory,
        root_path: String,
        errors: Vec<ConfigErrorDetail>,
        scan_warnings: Vec<ScanWarning>,
    },
    Ready {
        root_path: String,
        dimensions: Vec<Dimension>,
        usingDefaultDimensions: bool,
        today: DayFile,
        commitments: Vec<Commitment>,
        scan_warnings: Vec<ScanWarning>,
    },
}
```

- [ ] **Step 3: Run tests to verify compilation**

Run: `cd src-tauri && cargo test 2>&1`
Expected: All existing tests still pass. New variants match previous `match` arms (NeedsSetup, ConfigError, Ready still work).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "feat: add CURRENT_DATA_VERSION and InitResult data version variants"
```

---

### Task 2: Add version file I/O functions

**Files:**
- Modify: `src-tauri/src/files.rs:44-47`

- [ ] **Step 1: Write failing tests for read_version_file and write_version_file**

Insert after the existing `test_new_file_paths` test (after line 543) in `files.rs`:

```rust
#[test]
fn test_version_path() {
    let root = Path::new("/data");
    let p = version_path(root);
    assert_eq!(p, PathBuf::from("/data/version.txt"));
}

#[test]
fn test_write_and_read_version_roundtrip() {
    let tmp = std::env::temp_dir().join("logbook_test_version_rt");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    write_version_file(&tmp, 1).unwrap();
    let v = read_version_file(&tmp);
    assert_eq!(v, Ok(Some(1)));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_read_version_file_not_found() {
    let tmp = std::env::temp_dir().join("logbook_test_version_nf");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let v = read_version_file(&tmp);
    assert_eq!(v, Ok(None));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_read_version_file_invalid_content() {
    let tmp = std::env::temp_dir().join("logbook_test_version_invalid");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    // Empty file
    fs::write(tmp.join("version.txt"), "").unwrap();
    assert!(read_version_file(&tmp).is_err());

    // Non-integer content
    fs::write(tmp.join("version.txt"), "abc").unwrap();
    assert!(read_version_file(&tmp).is_err());

    // Whitespace-only
    fs::write(tmp.join("version.txt"), "  \n  ").unwrap();
    assert!(read_version_file(&tmp).is_err());

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test test_version_path test_write_and_read_version_roundtrip test_read_version_file_not_found test_read_version_file_invalid_content 2>&1`
Expected: All fail — `version_path`, `write_version_file`, `read_version_file` not found.

- [ ] **Step 3: Implement version_path, write_version_file, read_version_file**

Insert after `dimensions_template_path` (after line 51) in `files.rs`:

```rust
/// Data version file: {root}/version.txt
pub fn version_path(root: &Path) -> PathBuf {
    root.join("version.txt")
}

/// Write version.txt (atomic: tmp then rename).
pub fn write_version_file(root: &Path, version: u32) -> Result<(), String> {
    let path = version_path(root);
    let tmp = path.with_extension("tmp");
    let content = version.to_string();
    fs::write(&tmp, &content)
        .map_err(|e| format!("Failed to write version file: {}", e))?;
    fs::rename(&tmp, &path)
        .map_err(|e| format!("Failed to rename version file: {}", e))
}

/// Read version.txt. Returns Ok(None) if file doesn't exist.
/// Returns Err if file exists but content is not a valid unsigned integer.
pub fn read_version_file(root: &Path) -> Result<Option<u32>, String> {
    let path = version_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "version.txt is empty in {}",
            path.display()
        ));
    }
    trimmed
        .parse::<u32>()
        .map(Some)
        .map_err(|_| {
            format!(
                "version.txt contains invalid version '{}' in {}",
                trimmed,
                path.display()
            )
        })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test test_version_path test_write_and_read_version_roundtrip test_read_version_file_not_found test_read_version_file_invalid_content 2>&1`
Expected: All four tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/files.rs
git commit -m "feat: add version.txt read/write functions with atomic I/O"
```

---

### Task 3: Add check_data_version function

**Files:**
- Modify: `src-tauri/src/commands.rs:335-380`

- [ ] **Step 1: Write failing unit tests for check_data_version**

Add at the end of the `#[cfg(test)] mod tests` block in `commands.rs` (before the closing `}`):

```rust
#[test]
fn test_check_data_version_ok_when_absent() {
    use std::fs;
    let tmp = std::env::temp_dir().join("logbook_check_v_ok");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    fs::write(tmp.join("version.txt"), "1").unwrap();
    let result = check_data_version(&tmp, 1);
    assert!(result.is_ok(), "expected ok, got {:?}", result);
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_check_data_version_not_found() {
    use std::fs;
    let tmp = std::env::temp_dir().join("logbook_check_v_nf");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let result = check_data_version(&tmp, 1);
    assert!(result.is_err());
    match result.unwrap_err() {
        InitResult::DataVersionNotFound { root_path } => {
            assert_eq!(root_path, tmp.to_string_lossy());
        }
        other => panic!("expected DataVersionNotFound, got {:?}", other),
    }
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_check_data_version_mismatch() {
    use std::fs;
    let tmp = std::env::temp_dir().join("logbook_check_v_mm");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    fs::write(tmp.join("version.txt"), "5").unwrap();
    let result = check_data_version(&tmp, 1);
    assert!(result.is_err());
    match result.unwrap_err() {
        InitResult::DataVersionMismatch { root_path, expected, found } => {
            assert_eq!(root_path, tmp.to_string_lossy());
            assert_eq!(expected, 1);
            assert_eq!(found, 5);
        }
        other => panic!("expected DataVersionMismatch, got {:?}", other),
    }
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_check_data_version_invalid_content() {
    use std::fs;
    let tmp = std::env::temp_dir().join("logbook_check_v_inv");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    fs::write(tmp.join("version.txt"), "not-a-number").unwrap();
    let result = check_data_version(&tmp, 1);
    // Invalid content is treated like version not found
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        InitResult::DataVersionNotFound { .. }
    ));
    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test test_check_data_version_ok_when_absent test_check_data_version_not_found test_check_data_version_mismatch test_check_data_version_invalid_content 2>&1`
Expected: All fail — `check_data_version` not defined.

- [ ] **Step 3: Implement check_data_version**

Insert after the `parse_duration` function (after line 88) in `commands.rs`, before the `validate_required_dimensions` function:

```rust
/// Check that the data root has a version.txt matching `expected_version`.
/// Pure function — does not modify files.
pub fn check_data_version(
    root: &std::path::Path,
    expected_version: u32,
) -> Result<(), InitResult> {
    let version = match files::read_version_file(root) {
        Ok(Some(v)) => v,
        Ok(None) => {
            return Err(InitResult::DataVersionNotFound {
                root_path: root.to_string_lossy().into_owned(),
            });
        }
        Err(_e) => {
            // Invalid content → treat as version not found
            return Err(InitResult::DataVersionNotFound {
                root_path: root.to_string_lossy().into_owned(),
            });
        }
    };

    if version != expected_version {
        return Err(InitResult::DataVersionMismatch {
            root_path: root.to_string_lossy().into_owned(),
            expected: expected_version,
            found: version,
        });
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test test_check_data_version_ok_when_absent test_check_data_version_not_found test_check_data_version_mismatch test_check_data_version_invalid_content 2>&1`
Expected: All four tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add check_data_version function"
```

---

### Task 4: Modify init() to call check_data_version

**Files:**
- Modify: `src-tauri/src/commands.rs:288-334`

- [ ] **Step 1: Update the init command to check version before load_root_state**

Replace the body of the `init` function (lines 288-334) with:

```rust
#[tauri::command]
pub fn init(app: AppHandle) -> InitResult {
    error_log::log_command_enter("init", "");
    let app_data_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let root_path = match read_root_path(&app_data_dir) {
        Some(p) => p,
        None => {
            error_log::log_command_exit("init", true, "NeedsSetup");
            return InitResult::NeedsSetup;
        }
    };

    match check_data_version(&root_path, CURRENT_DATA_VERSION) {
        Err(e) => {
            match &e {
                InitResult::DataVersionNotFound { .. } => {
                    error_log::log_command_exit("init", true, "DataVersionNotFound");
                }
                InitResult::DataVersionMismatch { expected, found, .. } => {
                    error_log::log_command_exit(
                        "init",
                        false,
                        &format!("DataVersionMismatch: expected {}, found {}", expected, found),
                    );
                }
                _ => unreachable!(),
            }
            return e;
        }
        Ok(()) => {}
    }

    let result = load_root_state(&root_path);
    if root_path.exists() {
        crate::config::ensure_watcher(&app, root_path.clone());
    }
    match &result {
        InitResult::ConfigError { errors, scan_warnings, category, .. } => {
            for e in errors {
                error_log::log_error("init", &format!("{}: {}", e.kind, e.message));
            }
            for w in scan_warnings {
                error_log::log_error("init: scan", &format!("{}: {}", w.path, w.message));
            }
            error_log::log_command_exit(
                "init",
                false,
                &format!("{:?}: {} errors", category, errors.len()),
            );
        }
        InitResult::Ready { today, .. } => {
            error_log::log_command_exit(
                "init",
                true,
                &format!("Ready, {} entries today", today.entries.len()),
            );
        }
        InitResult::NeedsSetup => {
            error_log::log_command_exit("init", true, "NeedsSetup");
        }
        InitResult::DataVersionNotFound { .. } | InitResult::DataVersionMismatch { .. } => {
            unreachable!("version check should have returned early")
        }
    }
    result
}
```

- [ ] **Step 2: Run all tests to verify no regressions**

Run: `cd src-tauri && cargo test 2>&1`
Expected: All existing tests pass. (Note: integration tests that call `load_root_state` without a `version.txt` will fail at this stage — that's addressed in Task 6.)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add data version check to init"
```

---

### Task 5: Modify set_root_path() to write version file

**Files:**
- Modify: `src-tauri/src/commands.rs:336-381`

- [ ] **Step 1: Add version file write to set_root_path**

Replace the `set_root_path` function body (lines 336-381) with:

```rust
#[tauri::command]
pub fn set_root_path(app: AppHandle, path: String) -> Result<InitResult, String> {
    error_log::log_command_enter("set_root_path", &format!("path={}", path));
    let app_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let root_path = std::path::Path::new(&path);
    if !root_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if !root_path.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    save_root_path(&app_data_dir, root_path)?;

    files::write_version_file(root_path, CURRENT_DATA_VERSION)?;

    let result = load_root_state(root_path);
    crate::config::ensure_watcher(&app, root_path.to_path_buf());
    match &result {
        InitResult::ConfigError { errors, scan_warnings, category, .. } => {
            for e in errors {
                error_log::log_error("set_root_path", &format!("{}: {}", e.kind, e.message));
            }
            for w in scan_warnings {
                error_log::log_error("set_root_path: scan", &format!("{}: {}", w.path, w.message));
            }
            error_log::log_command_exit(
                "set_root_path",
                true,
                &format!("{:?}: {} errors", category, errors.len()),
            );
        }
        InitResult::Ready { today, .. } => {
            error_log::log_command_exit(
                "set_root_path",
                true,
                &format!("Ready, {} entries today", today.entries.len()),
            );
        }
        InitResult::NeedsSetup => {
            error_log::log_command_exit("set_root_path", true, "NeedsSetup");
        }
        InitResult::DataVersionNotFound { .. } | InitResult::DataVersionMismatch { .. } => {
            unreachable!("version just written should be current")
        }
    }
    Ok(result)
}
```

- [ ] **Step 2: Run all tests**

Run: `cd src-tauri && cargo test 2>&1`
Expected: All existing non-integration tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: write version.txt on set_root_path"
```

---

### Task 6: Integration tests

**Files:**
- Create: `src-tauri/tests/data_version_integration.rs`

- [ ] **Step 1: Write integration tests**

```rust
/// Integration tests for data version checking in load_root_state and init flow.
use std::fs;
use std::path::PathBuf;
use tauri_app_lib::commands::{check_data_version, load_root_state};
use tauri_app_lib::files;
use tauri_app_lib::models::{InitResult, CURRENT_DATA_VERSION};

fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!("logbook_data_version_{}", uuid::Uuid::new_v4()))
}

#[test]
fn load_root_state_with_no_version_file_works() {
    // load_root_state does NOT check the version — that's init's job.
    // This test confirms load_root_state is unchanged and still works
    // with valid config despite no version.txt.
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("dimensions.template.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
    )
    .unwrap();
    let result = load_root_state(&root);
    assert!(matches!(result, InitResult::Ready { .. }), "got {:?}", result);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn check_data_version_returns_ok_when_current() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    files::write_version_file(&root, CURRENT_DATA_VERSION).unwrap();
    let result = check_data_version(&root, CURRENT_DATA_VERSION);
    assert!(result.is_ok(), "expected ok, got {:?}", result);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn check_data_version_returns_not_found_when_missing() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    let result = check_data_version(&root, CURRENT_DATA_VERSION);
    assert!(matches!(result, Err(InitResult::DataVersionNotFound { .. })));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn check_data_version_returns_mismatch_when_wrong() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    files::write_version_file(&root, 99).unwrap();
    let result = check_data_version(&root, CURRENT_DATA_VERSION);
    assert!(matches!(result, Err(InitResult::DataVersionMismatch { .. })));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn write_version_file_and_read_back() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    files::write_version_file(&root, 3).unwrap();
    let version = files::read_version_file(&root).unwrap();
    assert_eq!(version, Some(3));
    fs::remove_dir_all(&root).unwrap();
}
```

- [ ] **Step 2: Run integration tests**

Run: `cd src-tauri && cargo test --test data_version_integration 2>&1`
Expected: All five tests pass.

- [ ] **Step 3: Run existing integration tests to verify no regressions**

Run: `cd src-tauri && cargo test --test recovery_category_integration 2>&1`
Expected: All tests pass (load_root_state unchanged by this feature).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/data_version_integration.rs
git commit -m "test: add data version integration tests"
```

---

### Task 7: Frontend — update types and applyInitResult

**Files:**
- Modify: `src/types.ts:96-118`
- Modify: `src/utils/applyInitResult.ts:1-30`

- [ ] **Step 1: Update InitResult type in types.ts**

Replace the `InitResult` type (lines 96-109) and add a new `AppStatus` value:

```typescript
export type InitResult =
  | { status: "NeedsSetup" }
  | { status: "DataVersionNotFound"; data: { root_path: string } }
  | { status: "DataVersionMismatch"; data: { root_path: string; expected: number; found: number } }
  | { status: "ConfigError"; data: { category: RecoveryCategory; root_path: string; errors: ConfigErrorDetail[]; scan_warnings: ScanWarning[] } }
  | {
      status: "Ready";
      data: {
        root_path: string;
        dimensions: Dimension[];
        usingDefaultDimensions: boolean;
        today: DayFile;
        commitments: Commitment[];
        scan_warnings: ScanWarning[];
      };
    };
```

Replace the `AppStatus` type (line 118):

```typescript
export type AppStatus = "loading" | "setup" | "migration_needed" | "error" | "ready";
```

- [ ] **Step 2: Update applyInitResult in applyInitResult.ts**

Replace the switch statement body (lines 10-29):

```typescript
export function applyInitResult(store: AppStore, result: InitResult): ScanWarning[] {
  switch (result.status) {
    case "NeedsSetup":
      store.status = "setup";
      return [];
    case "DataVersionNotFound":
    case "DataVersionMismatch":
      store.rootPath = result.data.root_path;
      store.status = "migration_needed";
      return [];
    case "ConfigError":
      store.configErrors = result.data.errors;
      store.configCategory = result.data.category;
      store.rootPath = result.data.root_path;
      store.status = "error";
      return result.data.scan_warnings;
    case "Ready":
      store.rootPath = result.data.root_path;
      store.dimensions = result.data.dimensions;
      store.usingDefaultDimensions = result.data.usingDefaultDimensions;
      store.today = result.data.today;
      store.commitments = result.data.commitments;
      store.configCategory = null;
      store.status = "ready";
      return result.data.scan_warnings;
  }
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `pnpm vue-tsc --noEmit 2>&1`
Expected: May still fail due to missing DataVersionScreen import in App.vue (addressed in Task 9). Check that types.ts and applyInitResult.ts have no errors specifically.

- [ ] **Step 4: Commit**

```bash
git add src/types.ts src/utils/applyInitResult.ts
git commit -m "feat: add DataVersionNotFound and DataVersionMismatch to frontend types"
```

---

### Task 8: Create DataVersionScreen component

**Files:**
- Create: `src/components/DataVersionScreen.vue`

- [ ] **Step 1: Create the component**

```vue
<script setup lang="ts">
defineProps<{
  message: string;
  rootPath: string;
}>();
</script>

<template>
  <div class="flex flex-col items-center justify-center min-h-screen gap-lg p-lg">
    <div class="max-w-md text-center text-secondary">
      <p class="mb-md">{{ message }}</p>
      <p class="text-micro text-gray">
        数据目录: {{ rootPath }}
      </p>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/DataVersionScreen.vue
git commit -m "feat: add DataVersionScreen component"
```

---

### Task 9: Wire DataVersionScreen into App.vue

**Files:**
- Modify: `src/App.vue:1-15, 230-244`

- [ ] **Step 1: Import DataVersionScreen**

Add the import after the existing `RecoveryScreen` import (line 10):

```typescript
import DataVersionScreen from "./components/DataVersionScreen.vue";
```

- [ ] **Step 2: Add template routing for migration_needed status**

Add before the `RecoveryScreen` line in the template (line 236):

```vue
<DataVersionScreen
  v-else-if="store.status === 'migration_needed'"
  :message="store.configErrors.length > 0 ? store.configErrors[0].message : 'Data format version mismatch. Please run the Logbook migration tool to update your data directory.'"
  :root-path="store.rootPath"
/>
```

Wait — the store doesn't currently carry the version error details. Let me refine: use `store.initResult` or add a dedicated field. Actually, let's keep it simple: the `DataVersionNotFound` and `DataVersionMismatch` both set `store.status = "migration_needed"` and `store.rootPath`. We need to surface the specific error. Let me add a `dataVersionMessage` field to the store.

- [ ] **Step 1b: Add dataVersionMessage to AppStore**

In `src/stores/useStore.ts`, add the field:

Add after line 23 in the `AppStore` interface:

```typescript
  dataVersionMessage: string | null;
```

Add in `createStore()` after line 45:

```typescript
    dataVersionMessage: null,
```

- [ ] **Step 1c: Set dataVersionMessage in applyInitResult**

Update the `DataVersionNotFound`/`DataVersionMismatch` case in `applyInitResult.ts`:

```typescript
    case "DataVersionNotFound":
      store.rootPath = result.data.root_path;
      store.dataVersionMessage = "Data version file not found. Please run the Logbook migration tool to initialize your data directory.";
      store.status = "migration_needed";
      return [];
    case "DataVersionMismatch":
      store.rootPath = result.data.root_path;
      store.dataVersionMessage = `Data format version mismatch. Expected version ${result.data.expected}, found version ${result.data.found}. Please run the Logbook migration tool to update your data directory.`;
      store.status = "migration_needed";
      return [];
```

- [ ] **Step 1d: Update DataVersionScreen to accept message prop directly**

Update the template to reference `store.dataVersionMessage`:

```vue
<DataVersionScreen
  v-else-if="store.status === 'migration_needed'"
  :message="store.dataVersionMessage ?? 'Data format version error.'"
  :root-path="store.rootPath"
/>
```

- [ ] **Step 2: Run TypeScript check**

Run: `pnpm vue-tsc --noEmit 2>&1`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add src/App.vue src/stores/useStore.ts src/utils/applyInitResult.ts src/components/DataVersionScreen.vue
git commit -m "feat: wire DataVersionScreen into App.vue"
```

---

### Task 10: Run full test suite and verify

- [ ] **Step 1: Run Rust tests**

Run: `cd src-tauri && cargo test 2>&1`
Expected: All tests pass, no regressions.

- [ ] **Step 2: Run frontend tests**

Run: `pnpm test 2>&1`
Expected: All tests pass.

- [ ] **Step 3: Verify full build**

Run: `pnpm vue-tsc --noEmit && cd src-tauri && cargo check`
Expected: No errors.

- [ ] **Step 4: Commit (if any final touch-ups needed)**

```bash
git add -A && git diff --cached --stat
```
