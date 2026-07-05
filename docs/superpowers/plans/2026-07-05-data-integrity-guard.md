# Data Integrity Guard — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add defense-in-depth data integrity protection — a global read-only guard that activates on format/semantic corruption in day files, preventing further writes from compounding damage.

**Architecture:** New `integrity` Rust module with a global `AtomicBool`. Startup scans current month + past 3 months for format/semantic errors. Each write command gates on the guard. Frontend shows a persistent banner when compromised, disables the entry composer. Watcher-driven recovery for config files; manual `⌘R` for day files.

**Tech Stack:** Rust (Tauri 2.x command), Vue 3 SFC + TypeScript, existing project conventions.

## Global Constraints

- Operation log must precede file mutation (existing pattern — do not reorder)
- Pre-write gate must check `IntegrityGuard::check()` before any file I/O
- Fail-fast on scan: first error sets guard, stops scanning
- Read commands (list, progress) must not be blocked
- Day files are not watched by the file watcher (existing constraint — do not add)
- Write commands return `Err(String)` on guard denial — no new error type needed

---

### Task 1: Add integrity types to Rust models

**Files:**
- Modify: `src-tauri/src/models.rs` (add types, extend InitResult)

**Interfaces:**
- Produces: `IntegrityIssue`, `IntegrityStatus`, `InitResult::Ready` gains `integrity_issues` field

- [ ] **Step 1: Add IntegrityIssue and IntegrityStatus to models.rs**

After the existing `ScanWarning` struct, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityIssue {
    pub path: String,
    pub message: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityStatus {
    pub compromised: bool,
    pub issues: Vec<IntegrityIssue>,
}
```

- [ ] **Step 2: Extend InitResult::Ready with integrity_issues**

In the `Ready` variant of `InitResult`, add:

```rust
Ready {
    root_path: String,
    dimensions: Vec<Dimension>,
    usingDefaultDimensions: bool,
    today: DayFile,
    commitments: Vec<Commitment>,
    scan_warnings: Vec<ScanWarning>,
    #[serde(default)]
    integrity_issues: Vec<IntegrityIssue>,
}
```

The `#[serde(default)]` ensures backward compatibility — old frontend code receiving the new field won't break.

- [ ] **Step 3: Update existing tests that construct InitResult::Ready**

Search `src-tauri/tests/` and `src-tauri/src/models.rs` for `InitResult::Ready` construction sites. Add `integrity_issues: vec![]` to each. Command to find them:

```bash
cd src-tauri && rg "InitResult::Ready" --type rust
```

- [ ] **Step 4: Run tests to confirm no breakage**

```bash
cd src-tauri && cargo test
```

Expected: All existing tests pass with the new field defaulting to empty.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs
git add -u src-tauri/tests/ src-tauri/src/
git commit -m "feat: add IntegrityIssue and IntegrityStatus types; extend InitResult::Ready"
```

---

### Task 2: Create IntegrityGuard module (core)

**Files:**
- Create: `src-tauri/src/integrity.rs`
- Modify: `src-tauri/src/lib.rs` (register module)

**Interfaces:**
- Produces: `IntegrityGuard::check()`, `set_compromised()`, `reset()`, `status()`

- [ ] **Step 1: Write the module file**

```rust
use crate::models::{IntegrityIssue, IntegrityStatus};
use std::sync::{LazyLock, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

static INTEGRITY_OK: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(true));
static INTEGRITY_ISSUES: LazyLock<Mutex<Vec<IntegrityIssue>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub fn check() -> Result<(), String> {
    if INTEGRITY_OK.load(Ordering::Acquire) {
        Ok(())
    } else {
        let issues = INTEGRITY_ISSUES
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let msg = if issues.is_empty() {
            "Write denied: data integrity compromised".to_string()
        } else {
            format!(
                "Write denied: data integrity compromised ({} issue{})",
                issues.len(),
                if issues.len() == 1 { "" } else { "s" }
            )
        };
        Err(msg)
    }
}

pub fn set_compromised(issue: IntegrityIssue) {
    INTEGRITY_ISSUES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(issue);
    INTEGRITY_OK.store(false, Ordering::Release);
}

pub fn reset() {
    INTEGRITY_ISSUES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    INTEGRITY_OK.store(true, Ordering::Release);
}

pub fn status() -> IntegrityStatus {
    let ok = INTEGRITY_OK.load(Ordering::Acquire);
    let issues = INTEGRITY_ISSUES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    IntegrityStatus {
        compromised: !ok,
        issues,
    }
}
```

- [ ] **Step 2: Register module in lib.rs**

Add after `pub mod scan;`:

```rust
pub mod integrity;
```

- [ ] **Step 3: Write unit tests in integrity.rs**

Add at the bottom of `integrity.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_uncompromised() {
        reset(); // ensure clean state
        assert!(check().is_ok());
        let s = status();
        assert!(!s.compromised);
        assert!(s.issues.is_empty());
    }

    #[test]
    fn set_compromised_blocks_writes() {
        reset();
        set_compromised(IntegrityIssue {
            path: "2026/07/05.md".into(),
            message: "corrupt YAML".into(),
            kind: "YamlParseError".into(),
        });
        assert!(check().is_err());
        let s = status();
        assert!(s.compromised);
        assert_eq!(s.issues.len(), 1);
        assert_eq!(s.issues[0].kind, "YamlParseError");
    }

    #[test]
    fn reset_restores_writes() {
        set_compromised(IntegrityIssue {
            path: "x.md".into(),
            message: "bad".into(),
            kind: "Test".into(),
        });
        reset();
        assert!(check().is_ok());
        let s = status();
        assert!(!s.compromised);
        assert!(s.issues.is_empty());
    }

    #[test]
    fn multiple_issues_accumulate() {
        reset();
        set_compromised(IntegrityIssue {
            path: "a.md".into(),
            message: "e1".into(),
            kind: "K1".into(),
        });
        set_compromised(IntegrityIssue {
            path: "b.md".into(),
            message: "e2".into(),
            kind: "K2".into(),
        });
        assert!(check().is_err());
        let s = status();
        assert_eq!(s.issues.len(), 2);
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test integrity
```

Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/integrity.rs src-tauri/src/lib.rs
git commit -m "feat: add IntegrityGuard module with global read-only state"
```

---

### Task 3: Implement day file integrity check functions

**Files:**
- Modify: `src-tauri/src/integrity.rs` (add check functions)

**Interfaces:**
- Produces: `check_day_file_integrity(root, date)`, `check_scoped_integrity(root)` — used by Tasks 4 and 7

- [ ] **Step 1: Add the check functions**

After the existing code in `integrity.rs`, add:

```rust
use std::path::Path;

/// Validate one day file for format + semantic integrity.
/// Returns Ok(()) or a single IntegrityIssue describing the first problem found.
pub fn check_day_file_integrity(root: &Path, date: &str) -> Result<(), IntegrityIssue> {
    use crate::files;

    let rel_path = {
        let dp = files::day_path(root, date)?;
        dp.strip_prefix(root)
            .unwrap_or(&dp)
            .to_string_lossy()
            .to_string()
    };

    // Layer 1: Format — can we parse the file at all?
    let day_file = match files::read_day_file(root, date) {
        Ok(df) => df,
        Err(e) => {
            return Err(IntegrityIssue {
                path: rel_path,
                message: format!("YAML parse failed: {}", e),
                kind: "YamlParseError".into(),
            });
        }
    };

    // Layer 2: Semantic — each entry must have valid fields
    let (year, month) = crate::files::year_month_from_date(date)
        .map_err(|e| IntegrityIssue {
            path: rel_path.clone(),
            message: e,
            kind: "DateParseError".into(),
        })?;
    let dims = crate::files::read_dimensions_file(root, year, month)
        .unwrap_or_default();

    for entry in &day_file.entries {
        if entry.duration == 0 {
            return Err(IntegrityIssue {
                path: rel_path,
                message: format!("Entry {} has duration = 0", entry.id),
                kind: "InvalidDuration".into(),
            });
        }

        if !is_valid_uuid_v4(&entry.id) {
            return Err(IntegrityIssue {
                path: rel_path,
                message: format!("Entry {} has invalid UUID: {}", entry.id, entry.id),
                kind: "InvalidUuid".into(),
            });
        }

        // Validate dimension keys exist in month dimensions
        for key in entry.dimensions.keys() {
            if !dims.iter().any(|d| &d.key == key) {
                return Err(IntegrityIssue {
                    path: rel_path,
                    message: format!(
                        "Entry {} has unknown dimension key '{}' (not in monthly dimensions.yaml)",
                        entry.id, key
                    ),
                    kind: "UnknownDimensionKey".into(),
                });
            }
        }

        // Required dimensions must have non-empty values
        for dim in &dims {
            if dim.required && !dim.deleted {
                match entry.dimensions.get(&dim.key) {
                    None => {
                        return Err(IntegrityIssue {
                            path: rel_path,
                            message: format!(
                                "Entry {} missing required dimension '{}'",
                                entry.id, dim.name
                            ),
                            kind: "MissingRequiredDimension".into(),
                        });
                    }
                    Some(v) if v.trim().is_empty() => {
                        return Err(IntegrityIssue {
                            path: rel_path,
                            message: format!(
                                "Entry {} has empty value for required dimension '{}'",
                                entry.id, dim.name
                            ),
                            kind: "EmptyRequiredDimension".into(),
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    // Layer 1.5: JSONL format check for the day's operation log
    let (year, month) = crate::files::year_month_from_date(date)
        .map_err(|e| IntegrityIssue {
            path: rel_path.clone(),
            message: e,
            kind: "DateParseError".into(),
        })?;
    let op_log_path = root
        .join(".logbook")
        .join("operations")
        .join(format!("{:04}", year))
        .join(format!("{:02}", month))
        .join(format!("{}.jsonl", date));
    if op_log_path.exists() {
        match std::fs::read_to_string(&op_log_path) {
            Ok(content) => {
                for (line_num, line) in content.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if serde_json::from_str::<serde_json::Value>(line).is_err() {
                        return Err(IntegrityIssue {
                            path: format!(
                                ".logbook/operations/{}/{:02}/{}.jsonl",
                                year,
                                month,
                                date
                            ),
                            message: format!("Line {} is invalid JSON", line_num + 1),
                            kind: "JsonlParseError".into(),
                        });
                    }
                }
            }
            Err(_) => {
                // File exists but can't be read as UTF-8 — report as format error
                return Err(IntegrityIssue {
                    path: format!(
                        ".logbook/operations/{}/{:02}/{}.jsonl",
                        year,
                        month,
                        date
                    ),
                    message: "File is not valid UTF-8".into(),
                    kind: "Utf8Error".into(),
                });
            }
        }
    }

    Ok(())
}

/// UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
fn is_valid_uuid_v4(s: &str) -> bool {
    uuid::Uuid::parse_str(s).map_or(false, |u| u.get_version_num() == 4)
}

/// Scan: current month + past 3 months. Returns accumulated IntegrityIssues.
/// Fail-fast: returns on first error.
pub fn check_scoped_integrity(root: &Path) -> Vec<IntegrityIssue> {
    use chrono::{Datelike, Local};

    let now = Local::now();
    let today = now.date_naive();
    let mut issues = Vec::new();

    for offset in 0..=3 {
        let target = today - chrono::Duration::days(offset * 30);
        let year = target.year();
        let month = target.month();

        let month_dir = root.join(year.to_string()).join(format!("{:02}", month));
        if !month_dir.exists() {
            continue;
        }

        let entries = match std::fs::read_dir(&month_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if !file_name.ends_with(".md") || file_name == "_monthly.md" {
                continue;
            }
            let date = file_name.trim_end_matches(".md");
            if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
                continue;
            }

            match check_day_file_integrity(root, date) {
                Ok(()) => {}
                Err(issue) => {
                    issues.push(issue);
                    return issues; // fail-fast
                }
            }
        }
    }

    issues
}
```

- [ ] **Step 2: Run cargo check**

```bash
cd src-tauri && cargo check
```

Expected: no compilation errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/integrity.rs
git commit -m "feat: add day file integrity check and scoped scan functions"
```

---

### Task 4: Wire startup integrity scan into load_root_state

**Files:**
- Modify: `src-tauri/src/commands.rs` (`load_root_state` function)

**Interfaces:**
- Consumes: `integrity::check_scoped_integrity`, `integrity::set_compromised`
- Produces: `InitResult::Ready` now includes `integrity_issues`

- [ ] **Step 1: Add integrity scan at the end of load_root_state**

At the end of `load_root_state` (line ~307, before the final `InitResult::Ready` return), add the integrity scan INSIDE the `Ready` branch only (not when there are config errors):

```rust
    // After the existing "if !all_errors.is_empty()" check and before the
    // final InitResult::Ready return, REPLACE the last block with:

    if !all_errors.is_empty() {
        return InitResult::ConfigError {
            category: RecoveryCategory::InPlace,
            root_path: root.to_string_lossy().into_owned(),
            errors: all_errors,
            scan_warnings,
        };
    }

    // ── Integrity scan: current month + past 3 months ──
    let integrity_issues = crate::integrity::check_scoped_integrity(root);
    for issue in &integrity_issues {
        crate::integrity::set_compromised(issue.clone());
    }

    InitResult::Ready {
        root_path: root.to_string_lossy().into_owned(),
        dimensions,
        usingDefaultDimensions: using_default_dimensions,
        today,
        commitments,
        scan_warnings,
        integrity_issues,
    }
```

Also add `use crate::integrity;` at the top of commands.rs among the other imports.

- [ ] **Step 2: Run tests**

```bash
cd src-tauri && cargo test
```

Expected: all tests pass. Existing tests might need `integrity::reset()` in test fixtures — check for any that now fail due to stale state.

- [ ] **Step 3: Handle test isolation — reset integrity guard in test fixtures**

Integration tests that set up corrupt day files might trigger the guard. For those, add `crate::integrity::reset()` at the end of their cleanup. Search for tests that write bad YAML:

```bash
cd src-tauri && rg "not valid|corrupt|garbage|broken" --type rust tests/
```

If any test fixture uses corrupt day files, add a `crate::integrity::reset()` call after the test.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/tests/
git commit -m "feat: wire startup integrity scan into load_root_state"
```

---

### Task 5: Add recheck_integrity Tauri command

**Files:**
- Modify: `src-tauri/src/commands.rs` (add command)
- Modify: `src-tauri/src/lib.rs` (register command)

**Interfaces:**
- Produces: `recheck_integrity` Tauri command — re-scans recent months, returns `IntegrityStatus`

- [ ] **Step 1: Add the command in commands.rs**

```rust
#[tauri::command]
pub fn recheck_integrity(root_path: String) -> crate::models::IntegrityStatus {
    use crate::integrity;

    let root = std::path::Path::new(&root_path);
    let issues = integrity::check_scoped_integrity(root);
    if !issues.is_empty() {
        for issue in &issues {
            integrity::set_compromised(issue.clone());
        }
    } else {
        integrity::reset();
    }
    integrity::status()
}
```

- [ ] **Step 2: Register in lib.rs invoke_handler**

Add `commands::recheck_integrity` to the `invoke_handler` list.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add recheck_integrity Tauri command"
```

---

### Task 6: Add pre-write gate to all write commands

**Files:**
- Modify: `src-tauri/src/commands.rs` (append_entry, update_entry, delete_entry, set_day_note, set_commitments)

**Interfaces:**
- Consumes: `integrity::check()`, `integrity::check_day_file_integrity`, `integrity::set_compromised`

- [ ] **Step 1: Add gate to append_entry**

At the top of `append_entry`, after `let root = std::path::Path::new(&root_path);`:

```rust
    integrity::check()?;
```

After validation (dimension checks) but before the op log write, add a pre-write integrity check on the target day file:

```rust
    // Pre-write integrity check on the target day file
    if let Err(issue) = integrity::check_day_file_integrity(root, &date) {
        integrity::set_compromised(issue.clone());
        return Err(format!(
            "Write denied: target file integrity check failed: {} — {}",
            issue.path, issue.message
        ));
    }
```

This must go AFTER all input validation (so we don't flag a pre-existing corrupted file on unrelated input errors) but BEFORE the op log write (so we don't log an operation that then fails).

- [ ] **Step 2: Add gate to update_entry**

Same as Step 1: global gate at top, pre-write integrity check before op log write.

- [ ] **Step 3: Add gate to delete_entry**

Same as Step 1.

- [ ] **Step 4: Add gate to set_day_note**

Same as Step 1.

- [ ] **Step 5: Add gate to set_commitments**

Global gate at top only. No pre-write file check needed since `set_commitments` doesn't target a specific day file.

- [ ] **Step 6: Verify compilation**

```bash
cd src-tauri && cargo check
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add pre-write integrity gate to all write commands"
```

---

### Task 7: Add integrity check in file watcher for recovery

**Files:**
- Modify: `src-tauri/src/config.rs` (watcher thread — dimensions.yaml and commitments.yaml handlers)

**Interfaces:**
- Consumes: `integrity::check_scoped_integrity`, `integrity::reset`
- Produces: watcher emits new `integrity-changed` event on recovery

- [ ] **Step 1: Add import and integrity recovery logic to watcher**

In `src-tauri/src/config.rs`, the watcher already has `use crate::files;` and `use crate::models::...` at the top. No new import needed since we use fully-qualified paths (`crate::integrity::...`).

In the `dimensions.yaml` watcher block (around line 350), after `app_handle.emit("dimensions-changed", &errors)` succeeds with empty errors, add:

```rust
// When validation passes with no errors, recheck integrity
if errors.is_empty() {
    let issues = crate::integrity::check_scoped_integrity(&watch_root);
    if issues.is_empty() {
        crate::integrity::reset();
        let _ = app_handle.emit("integrity-changed", &crate::integrity::status());
    }
}
```

Same for the `commitments.yaml` watcher block (around line 411), and the `dimensions.template.yaml` block (around line 314).

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "feat: add integrity recovery via file watcher on config changes"
```

---

### Task 8: CLI write denial

**Files:**
- Modify: `src-tauri/src/cli/entries.rs` (`add` function)
- Modify: `src-tauri/src/cli/commitments.rs` (`set` function)
- Modify: `src-tauri/src/cli/dimensions.rs` (`set` function)

**Interfaces:**
- Consumes: `integrity::check()`

- [ ] **Step 1: Gate CLI entries add**

In `entries.rs` `add` function, before reading stdin or calling `append_entry`:

```rust
    if let Err(e) = crate::integrity::check() {
        output::print_error(&e);
        std::process::exit(1);
    }
```

- [ ] **Step 2: Gate CLI commitments set**

In `commitments.rs` `set` function, before reading stdin or calling `set_commitments`:

```rust
    if let Err(e) = crate::integrity::check() {
        output::print_error(&e);
        std::process::exit(1);
    }
```

- [ ] **Step 3: Gate CLI dimensions set**

Read `src-tauri/src/cli/dimensions.rs` to find the set function and add the same gate.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/cli/
git commit -m "feat: add integrity guard to CLI write commands"
```

---

### Task 9: Add frontend types and store fields

**Files:**
- Modify: `src/types.ts` (add IntegrityIssue, IntegrityStatus types, update InitResult)
- Modify: `src/stores/useStore.ts` (add integrityIssues, integrityCompromised)
- Modify: `src/utils/applyInitResult.ts` (handle integrity_issues)

**Interfaces:**
- Produces: TypeScript types for integrity, store state for banner

- [ ] **Step 1: Add types to types.ts**

After `ScanWarning`, add:

```typescript
export interface IntegrityIssue {
  path: string;
  message: string;
  kind: string;
}

export interface IntegrityStatus {
  compromised: boolean;
  issues: IntegrityIssue[];
}
```

Update the `InitResult` union type's `Ready` variant to include:

```typescript
integrity_issues: IntegrityIssue[];
```

- [ ] **Step 2: Add store fields in useStore.ts**

Add to `AppStore` interface:

```typescript
  integrityIssues: IntegrityIssue[];
```

Add to `createStore()` return:

```typescript
    integrityIssues: [],
```

- [ ] **Step 3: Handle integrity_issues in applyInitResult.ts**

In the `"Ready"` case, after setting `store.status = "ready"`, add:

```typescript
      store.integrityIssues = result.data.integrity_issues ?? [];
```

- [ ] **Step 4: Verify TypeScript compilation**

```bash
cd src-tauri && cargo test
```

Actually just:

```bash
pnpm vue-tsc --noEmit
```

Expected: no type errors.

- [ ] **Step 5: Commit**

```bash
git add src/types.ts src/stores/useStore.ts src/utils/applyInitResult.ts
git commit -m "feat: add frontend types and store fields for integrity guard"
```

---

### Task 10: Create IntegrityBanner component

**Files:**
- Create: `src/components/IntegrityBanner.vue`

**Interfaces:**
- Consumes: `store.integrityIssues`
- Produces: persistent banner component shown in MonthView

- [ ] **Step 1: Write the component**

```vue
<script setup lang="ts">
import { useStore } from "../stores/useStore";
const store = useStore();
</script>

<template>
  <div
    v-if="store.integrityIssues.length > 0"
    class="bg-[var(--color-danger)]/5 border border-[var(--color-danger)]/20 rounded-[var(--radius-form-lg)] p-lg mx-lg mt-lg text-left"
  >
    <h2 class="text-[var(--color-danger)] font-semibold mb-sm">
      Data Protection Mode Active — entry and editing suspended
    </h2>
    <p class="text-[var(--color-danger)] text-secondary mb-md">
      File issues detected. Fix the files below with a text editor, then press ⌘R to reload.
      Or restore from backup and restart the app.
    </p>
    <div class="flex flex-col gap-lg">
      <div v-for="(issue, i) in store.integrityIssues" :key="i">
        <div class="text-[var(--color-danger)] font-semibold text-secondary mb-xs">
          {{ issue.path }}
        </div>
        <div class="text-[var(--color-danger)] text-secondary whitespace-pre-wrap">
          {{ issue.message }}
        </div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/IntegrityBanner.vue
git commit -m "feat: add IntegrityBanner component"
```

---

### Task 11: Wire IntegrityBanner into MonthView and disable composer

**Files:**
- Modify: `src/components/MonthView.vue` (add banner, conditional composer rendering)

- [ ] **Step 1: Import and place IntegrityBanner**

In `<script setup>`, add:

```typescript
import IntegrityBanner from "./IntegrityBanner.vue";
```

Replace the `ConfigErrorBanner` block in the template (around line 165) with both banners stacked:

```html
      <ConfigErrorBanner
        v-if="store.configErrors.length > 0 && store.status === 'ready'"
      />
      <IntegrityBanner />
```

- [ ] **Step 2: Disable EntryComposer when integrity is compromised**

Replace the EntryComposer block (around line 206):

```html
      <div v-if="isSelectedToday" class="mt-md">
        <div v-if="store.integrityIssues.length > 0" class="text-secondary text-center py-md text-[var(--color-text-disabled)]">
          Entry disabled — data protection mode active
        </div>
        <EntryComposer
          v-else
          ref="inputRef"
          :dimensions="store.dimensions"
          :commitments="store.commitments"
          @submit="handleSubmit"
          @edit-dimensions="openDimEditor"
        />
      </div>
```

- [ ] **Step 3: Disable day note editing when compromised**

The day note div (around line 181) uses `contenteditable="true"`. Change to dynamic binding:

```html
          :contenteditable="store.integrityIssues.length === 0 ? 'true' : 'false'"
```

This makes the note read-only while integrity is compromised, without needing to duplicate the `useDayNote` control flow.

- [ ] **Step 4: Add ⌘R handler for recheck_integrity and integrity-changed listener**

MonthView already has an `onMounted` (line 121) that registers `onGlobalKeydown`. Extend `onGlobalKeydown` to handle `⌘R` when integrity is compromised:

```typescript
function onGlobalKeydown(e: KeyboardEvent) {
  if (!(e.metaKey || e.ctrlKey)) return;
  if (e.key === "[") {
    e.preventDefault();
    e.shiftKey ? shiftMonth(-1) : shiftDay(-1);
  } else if (e.key === "]") {
    e.preventDefault();
    e.shiftKey ? shiftMonth(1) : shiftDay(1);
  } else if (e.key === "t" || e.key === "T") {
    e.preventDefault();
    goToToday();
  } else if (e.key === "r" || e.key === "R") {
    // ⌘R: only intercept when integrity is compromised
    if (store.integrityIssues.length > 0) {
      e.preventDefault();
      recheckIntegrity();
    }
  }
}
```

Add imports at the top of the `<script setup>` block (alongside the existing `invoke` import if not already present):

```typescript
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { IntegrityStatus } from "../types";
```

Add the async function and listener registration. In the existing `onMounted` (around line 121), add after `window.addEventListener("keydown", onGlobalKeydown);`:

```typescript
  unlistenIntegrity = await listen<IntegrityStatus>("integrity-changed", (event) => {
    store.integrityIssues = event.payload.issues;
  });
```

Add at the top level of `<script setup>`:

```typescript
let unlistenIntegrity: (() => void) | null = null;

async function recheckIntegrity() {
  try {
    const result = await invoke<IntegrityStatus>("recheck_integrity", {
      rootPath: store.rootPath,
    });
    store.integrityIssues = result.issues;
  } catch (_e) {
    // recheck_integrity can only fail if the root path isn't valid
  }
}
```

In the existing `onUnmounted` (line 131), add after `window.removeEventListener(...)`:

```typescript
  unlistenIntegrity?.();
```

- [ ] **Step 5: Commit**

```bash
git add src/components/MonthView.vue
git commit -m "feat: wire IntegrityBanner into MonthView, disable composer when compromised, add ⌘R recovery"
```

---

### Task 12: Integration tests

**Files:**
- Create: `src-tauri/tests/integrity_guard_integration.rs`

**Interfaces:**
- Tests: startup scan, pre-write gate, recovery flow

- [ ] **Step 1: Write integration test file**

```rust
use tauri_app_lib::{integrity, models::*};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!("logbook_integrity_test_{}", uuid::Uuid::new_v4()))
    }

    fn setup_fixture(root: &PathBuf) {
        // Write a valid dimensions.template.yaml
        let dims = r#"dimensions:
  - name: Biz
    key: biz
    source: static
    values: [A, B]
    required: true
  - name: Goal
    key: goal
    source: commitments:goals
"#;
        fs::create_dir_all(root).unwrap();
        fs::write(root.join("dimensions.template.yaml"), dims).unwrap();

        // Write current month dir with a valid day file
        use chrono::Local;
        let now = Local::now();
        let month_dir = root
            .join(format!("{}", now.year()))
            .join(format!("{:02}", now.month()));
        fs::create_dir_all(&month_dir).unwrap();

        let valid_entry = format!(
            "---\nentries:\n  - id: {}\n    item: test\n    duration: 30\n    dimensions:\n      biz: A\n---\n",
            uuid::Uuid::new_v4()
        );
        let today = format!(
            "{}-{:02}-{:02}",
            now.year(),
            now.month(),
            now.day()
        );
        fs::write(month_dir.join(format!("{}.md", today)), valid_entry).unwrap();

        // Write dimensions.yaml for the month so dimension key check works
        let month_dims = format!(
            "dimensions:\n  - name: Biz\n    key: biz\n    source: static\n    values: [A, B]\n    required: true\n  - name: Goal\n    key: goal\n    source: commitments:goals\n"
        );
        fs::write(month_dir.join("dimensions.yaml"), month_dims).unwrap();
    }

    fn cleanup(root: &PathBuf) {
        integrity::reset();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn guard_starts_uncompromised() {
        integrity::reset();
        assert!(integrity::check().is_ok());
    }

    #[test]
    fn startup_scan_passes_on_valid_data() {
        let root = temp_root();
        setup_fixture(&root);

        let issues = integrity::check_scoped_integrity(&root);
        assert!(issues.is_empty(), "expected no issues, got {:?}", issues);

        cleanup(&root);
    }

    #[test]
    fn startup_scan_detects_corrupt_yaml() {
        let root = temp_root();
        setup_fixture(&root);

        // Write a corrupt day file in the current month
        use chrono::Local;
        let now = Local::now();
        let month_dir = root
            .join(format!("{}", now.year()))
            .join(format!("{:02}", now.month()));
        let bad_date = format!(
            "{}-{:02}-{:02}",
            now.year(),
            now.month(),
            if now.day() > 1 { now.day() - 1 } else { 1 }
        );
        fs::write(month_dir.join(format!("{}.md", bad_date)), "this is not valid yaml\n").unwrap();

        let issues = integrity::check_scoped_integrity(&root);
        assert_eq!(issues.len(), 1, "expected 1 issue, got {:?}", issues);
        assert_eq!(issues[0].kind, "YamlParseError");

        cleanup(&root);
    }

    #[test]
    fn startup_scan_detects_zero_duration() {
        let root = temp_root();
        setup_fixture(&root);

        // Write a day file with duration: 0
        use chrono::Local;
        let now = Local::now();
        let month_dir = root
            .join(format!("{}", now.year()))
            .join(format!("{:02}", now.month()));
        let bad_date = format!(
            "{}-{:02}-{:02}",
            now.year(),
            now.month(),
            if now.day() > 1 { now.day() - 1 } else { 1 }
        );
        let bad_entry = format!(
            "---\nentries:\n  - id: {}\n    item: bad\n    duration: 0\n---\n",
            uuid::Uuid::new_v4()
        );
        fs::write(month_dir.join(format!("{}.md", bad_date)), bad_entry).unwrap();

        let issues = integrity::check_scoped_integrity(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "InvalidDuration");

        cleanup(&root);
    }

    #[test]
    fn set_compromised_then_check_denies_write() {
        integrity::reset();

        integrity::set_compromised(IntegrityIssue {
            path: "test.md".into(),
            message: "test error".into(),
            kind: "Test".into(),
        });

        assert!(integrity::check().is_err());

        integrity::reset();
    }

    #[test]
    fn reset_after_compromised_allows_write() {
        integrity::reset();

        integrity::set_compromised(IntegrityIssue {
            path: "test.md".into(),
            message: "test".into(),
            kind: "Test".into(),
        });
        integrity::reset();

        assert!(integrity::check().is_ok());
    }
}
```

- [ ] **Step 2: Run integration tests**

```bash
cd src-tauri && cargo test integrity_guard
```

Expected: all tests pass.

- [ ] **Step 3: Fix any test failures**

Check if the test fixtures interfere with existing integration tests. The `integrity::reset()` calls in `cleanup` should handle cross-test contamination.

- [ ] **Step 4: Run full test suite**

```bash
cd src-tauri && cargo test
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tests/integrity_guard_integration.rs
git commit -m "feat: add integration tests for integrity guard"
```
