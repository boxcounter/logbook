# Integration Tests, Error Log & Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete HANDOFF.md next steps: integration tests for critical paths, error.log fallback, and Phase 2 Week/Month granularity.

**Architecture:** Three independent workstreams. A (integration tests) and B (error.log) have no dependencies and can run in parallel. C (Phase 2) depends on A being done first to ensure the test harness is solid before modifying frontend grouping logic.

**Tech Stack:** Rust (Tauri 2.x, yaml_serde 0.10, notify, regex), Vue 3 + Composition API + TypeScript, Chart.js

---

## Pre-flight

- [ ] **Step 1: Confirm baseline**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook/src-tauri && cargo test
```

Expected: 32 tests pass, 0 fail.

- [ ] **Step 2: Confirm fixture is valid**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook/src-tauri && cargo test --test fixture_diagnostic
```

Expected: 1 test pass (test_read_and_validate_fixture).

---

## Part A: Integration Tests

### Task A1: Convert diagnostic test to proper integration test

**Files:**
- Replace: `src-tauri/tests/fixture_diagnostic.rs` → `src-tauri/tests/fixture_integration.rs`
- The diagostic test already proves fixture is valid. Replace println debugging with assertions.

- [ ] **Step 1: Rename and rewrite the test with proper assertions**

Delete `tests/fixture_diagnostic.rs`. Create `tests/fixture_integration.rs`:

```rust
/// Integration tests using the real fixture at ~/Downloads/logbook-test.
use std::path::Path;

fn fixture_root() -> std::path::PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    Path::new(&home).join("Downloads/logbook-test")
}

#[test]
fn test_read_and_validate_config() {
    let root = fixture_root();
    let config = tauri_app_lib::files::read_config(&root)
        .expect("read_config should succeed");
    let errors = tauri_app_lib::config::validate_config(&config);
    assert!(errors.is_empty(),
        "Config validation failed:\n{}",
        errors.iter().map(|e| format!("  [{}] {}", e.kind, e.message)).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn test_read_and_validate_monthly() {
    let root = fixture_root();
    let monthly = tauri_app_lib::files::read_monthly_file(&root, 2026, 6)
        .expect("read_monthly_file should succeed");
    let errors = tauri_app_lib::config::validate_monthly(&monthly);
    assert!(errors.is_empty(),
        "Monthly validation failed:\n{}",
        errors.iter().map(|e| format!("  [{}] {}", e.kind, e.message)).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn test_config_dimensions_count() {
    let root = fixture_root();
    let config = tauri_app_lib::files::read_config(&root).unwrap();
    assert_eq!(config.dimensions.len(), 4);
    let keys: Vec<&str> = config.dimensions.iter().map(|d| d.key.as_str()).collect();
    assert!(keys.contains(&"importance-urgency"));
    assert!(keys.contains(&"business-line"));
    assert!(keys.contains(&"category"));
    assert!(keys.contains(&"goal"));
}

#[test]
fn test_monthly_commitments_count() {
    let root = fixture_root();
    let monthly = tauri_app_lib::files::read_monthly_file(&root, 2026, 6).unwrap();
    assert_eq!(monthly.commitments.len(), 2);
    assert_eq!(monthly.commitments[0].role, "Developer");
    assert_eq!(monthly.commitments[1].role, "Director");
}
```

- [ ] **Step 2: Run integration tests**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook/src-tauri && cargo test --test fixture_integration
```

Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && git add src-tauri/tests/ && git rm src-tauri/tests/fixture_diagnostic.rs 2>/dev/null; git commit -m "test: add fixture integration tests"
```

### Task A2: Entry CRUD roundtrip integration test

**Files:**
- Create: `src-tauri/tests/entry_crud_integration.rs`

Uses `std::env::temp_dir()` (no reliance on fixture), tests append → read → update → delete full cycle.

- [ ] **Step 1: Write the test**

```rust
/// Integration test: append → read → update → delete entry roundtrip.
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tauri_app_lib::models::{NewEntry, UpdateEntry, DayFile};

fn test_root() -> std::path::PathBuf {
    std::env::temp_dir().join("logbook_integration_test")
}

fn setup() {
    let root = test_root();
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    // Write minimal config.yaml
    fs::write(
        root.join("config.yaml"),
        "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
    )
    .unwrap();
    // Write minimal _monthly.md for June 2026
    let monthly_dir = root.join("2026/06");
    fs::create_dir_all(&monthly_dir).unwrap();
    fs::write(
        monthly_dir.join("_monthly.md"),
        "---\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it\n---\n",
    )
    .unwrap();
}

fn teardown() {
    let _ = fs::remove_dir_all(test_root());
}

#[test]
fn test_append_read_update_delete_roundtrip() {
    setup();
    let root = test_root();
    let date = "2026-06-12";

    // Append entry
    let mut dims = HashMap::new();
    dims.insert("goal".to_string(), "Ship it".to_string());

    let new_entry = NewEntry {
        item: "Integration test entry".to_string(),
        duration: "45".to_string(),
        dimensions: dims,
    };

    let entry = tauri_app_lib::files::append_to_day_file(&root, date, &new_entry);
    assert!(entry.is_ok(), "append failed: {:?}", entry.err());
    let entry = entry.unwrap();
    assert_eq!(entry.item, "Integration test entry");
    assert_eq!(entry.duration, 45);

    // Read back
    let day_file = tauri_app_lib::files::read_day_file(&root, date);
    assert!(day_file.is_ok(), "read failed: {:?}", day_file.err());
    let day_file = day_file.unwrap();
    assert_eq!(day_file.entries.len(), 1);
    assert_eq!(day_file.entries[0].id, entry.id);

    // Update entry
    let update = UpdateEntry {
        item: Some("Updated entry".to_string()),
        duration: Some("90".to_string()),
        dimensions: None,
    };
    let updated = tauri_app_lib::files::update_entry_in_file(&root, date, &entry.id, &update);
    assert!(updated.is_ok(), "update failed: {:?}", updated.err());
    let updated = updated.unwrap();
    assert_eq!(updated.entries[0].item, "Updated entry");
    assert_eq!(updated.entries[0].duration, 90);

    // Delete entry
    let deleted = tauri_app_lib::files::delete_entry_from_file(&root, date, &entry.id);
    assert!(deleted.is_ok(), "delete failed: {:?}", deleted.err());
    let deleted = deleted.unwrap();
    assert!(deleted.entries.is_empty());

    teardown();
}

#[test]
fn test_set_and_clear_day_note() {
    setup();
    let root = test_root();
    let date = "2026-06-12";

    let df = tauri_app_lib::files::set_day_note_in_file(&root, date, "测试笔记")
        .expect("set_day_note failed");
    assert_eq!(df.note, Some("测试笔记".to_string()));

    let df = tauri_app_lib::files::read_day_file(&root, date).unwrap();
    assert_eq!(df.note, Some("测试笔记".to_string()));

    let df = tauri_app_lib::files::set_day_note_in_file(&root, date, "")
        .expect("clear note failed");
    assert_eq!(df.note, None);

    teardown();
}

#[test]
fn test_read_nonexistent_date_returns_empty() {
    setup();
    let root = test_root();
    let df = tauri_app_lib::files::read_day_file(&root, "2026-06-15").unwrap();
    assert!(df.entries.is_empty());
    assert!(df.note.is_none());
    teardown();
}
```

- [ ] **Step 2: Make `models` module public (if not already)**

Check that `src-tauri/src/lib.rs` has `pub mod models;`. If not, change to `pub mod models;`.

- [ ] **Step 3: Make `append_to_day_file` accept `NewEntry` or add a bridge function**

The current `append_to_day_file` takes `&Entry`, but integration tests create `NewEntry`. Add a public helper to `files.rs`:

```rust
/// Append entry from NewEntry (for integration tests and internal use).
pub fn append_new_entry(root: &Path, date: &str, new_entry: &crate::models::NewEntry) -> Result<Entry, String> {
    let duration = crate::commands::parse_duration(&new_entry.duration)?;
    let entry = Entry {
        id: uuid::Uuid::new_v4().to_string(),
        item: new_entry.item.clone(),
        duration,
        dimensions: new_entry.dimensions.clone(),
    };
    append_to_day_file(root, date, &entry)
}
```

- [ ] **Step 4: Run tests**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook/src-tauri && cargo test --test entry_crud_integration
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && git add src-tauri/tests/ src-tauri/src/files.rs && git commit -m "test: add entry CRUD roundtrip integration tests"
```

### Task A3: parse_duration integration test with real-world inputs

**Files:**
- Modify: `src-tauri/tests/entry_crud_integration.rs` (add test module)

Or create a separate test file. Since parse_duration is in `commands.rs` and already has unit tests, these integration tests cover edge cases the unit tests might miss.

- [ ] **Step 1: Add parse_duration edge case tests**

Append to `src-tauri/tests/entry_crud_integration.rs`:

```rust
#[test]
fn test_parse_duration_via_append() {
    // Test that NewEntry with various duration formats roundtrips correctly
    setup();
    let root = test_root();
    let date = "2026-06-12";

    let cases = vec![
        ("1.5h", 90),
        ("30m", 30),
        ("90", 90),
        ("2h", 120),
        ("1h 30m", 90),
    ];

    for (input, expected) in cases {
        let new_entry = NewEntry {
            item: format!("Test {}", input),
            duration: input.to_string(),
            dimensions: HashMap::new(),
        };
        let entry = tauri_app_lib::files::append_new_entry(&root, date, &new_entry)
            .expect(&format!("append failed for '{}'", input));
        assert_eq!(entry.duration, expected, "duration mismatch for '{}'", input);
    }

    let day_file = tauri_app_lib::files::read_day_file(&root, date).unwrap();
    assert_eq!(day_file.entries.len(), cases.len());

    teardown();
}
```

- [ ] **Step 2: Run tests**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook/src-tauri && cargo test --test entry_crud_integration
```

Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && git add src-tauri/tests/ && git commit -m "test: add parse_duration integration tests via append flow"
```

---

## Part B: Error Log Fallback

### Task B1: Rust error log infrastructure

**Files:**
- Create: `src-tauri/src/error_log.rs`
- Modify: `src-tauri/src/lib.rs` (register module + call init)
- Modify: `src-tauri/src/commands.rs` (log errors in command handlers)

- [ ] **Step 1: Write the error_log module**

Create `src-tauri/src/error_log.rs`:

```rust
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Initialize the error log path. Call once during app setup.
pub fn init(app_data_dir: &std::path::Path) {
    let log_path = app_data_dir.join("error.log");
    if let Ok(mut path) = LOG_PATH.lock() {
        *path = Some(log_path.clone());
    }
    // Write a startup marker
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = append_log(&format!("--- Logbook started at {} ---", timestamp));
}

/// Append a line to the error log. Non-blocking, best-effort.
fn append_log(line: &str) -> Result<(), String> {
    let path = LOG_PATH.lock().map_err(|e| format!("Lock error: {}", e))?;
    let path = path.as_ref().ok_or("Log not initialized")?;
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Failed to open error.log: {}", e))?;
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    writeln!(file, "[{}] {}", timestamp, line)
        .map_err(|e| format!("Failed to write error.log: {}", e))?;
    Ok(())
}

/// Log a Rust error with context.
pub fn log_error(context: &str, error: &str) {
    let _ = append_log(&format!("ERROR [{}] {}", context, error));
}

/// Log a frontend error (called via Tauri command or event).
pub fn log_frontend_error(message: &str) {
    let _ = append_log(&format!("FRONTEND {}", message));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_append_log_creates_file() {
        let tmp = std::env::temp_dir().join("logbook_error_log_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        init(&tmp);
        append_log("test message").unwrap();
        let log_path = tmp.join("error.log");
        assert!(log_path.exists());
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("test message"));
        let _ = fs::remove_dir_all(&tmp);
    }
}
```

- [ ] **Step 2: Add chrono dependency (if not already present)**

Check `Cargo.toml` for `chrono`. If missing:

```bash
cd /Users/boxcounter/code/Boxcounter/logbook/src-tauri && cargo add chrono
```

- [ ] **Step 3: Register module in lib.rs**

Add to `src-tauri/src/lib.rs`:

```rust
mod error_log;
```

And call `error_log::init` in the `setup` hook:

```rust
.setup(|app| {
    let app_handle = app.handle().clone();
    let app_data_dir = app.path().app_local_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    error_log::init(&app_data_dir);  // <-- add this line
    if let Some(root_path) = files::read_root_path(&app_data_dir) {
        if root_path.exists() {
            watch_files(app_handle, root_path);
        }
    }
    Ok(())
})
```

- [ ] **Step 4: Wire error_log into command error paths**

In `src-tauri/src/commands.rs`, add error_log calls in catch-all error handlers. For `init`:

```rust
Err(e) => {
    crate::error_log::log_error("init: read_config", &e);
    return InitResult::ConfigError(vec![ConfigErrorDetail {
        kind: "ConfigReadError".to_string(), message: e,
    }]);
}
```

For `set_root_path`'s read_config failure:

```rust
let config = files::read_config(root_path).map_err(|e| {
    crate::error_log::log_error("set_root_path: read_config", &e);
    format!("Failed to read config: {}", e)
})?;
```

For `append_entry`, `update_entry`, `delete_entry`, `set_day_note` catch blocks, add:

```rust
} catch (e) {
    crate::error_log::log_error("command_name", &format!("{:?}", e));
}
```

- [ ] **Step 5: Add Tauri command to accept frontend errors**

Add to `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub fn log_error(message: String) {
    crate::error_log::log_frontend_error(&message);
}
```

Register it in `lib.rs`'s `invoke_handler`:

```rust
.invoke_handler(tauri::generate_handler![
    commands::init,
    commands::set_root_path,
    commands::get_entries,
    commands::append_entry,
    commands::update_entry,
    commands::delete_entry,
    commands::set_day_note,
    commands::get_commitments,
    commands::log_error,  // <-- add this
])
```

- [ ] **Step 6: Run tests**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook/src-tauri && cargo test
```

Expected: 33 tests pass (32 existing + 1 new error_log test).

- [ ] **Step 7: Commit**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && git add src-tauri/src/error_log.rs src-tauri/src/lib.rs src-tauri/src/commands.rs && git commit -m "feat: add error.log fallback infrastructure"
```

### Task B2: Frontend error capture → Rust log

**Files:**
- Modify: `src/App.vue`
- Modify: `src/components/TodayView.vue`
- Modify: `src/components/SetupScreen.vue`
- Modify: `src/components/QuickEntry.vue`

- [ ] **Step 1: Add a global error logging helper**

Create `src/utils/errorLog.ts`:

```typescript
import { invoke } from "@tauri-apps/api/core";

export function logError(context: string, error: unknown): void {
  const message = error instanceof Error ? `${error.name}: ${error.message}` : String(error);
  const entry = `[${context}] ${message}`;
  console.error(entry);
  // Fire-and-forget to Rust; don't await to avoid blocking
  invoke("log_error", { message: entry }).catch(() => {
    // If even this fails, we're in trouble — but don't crash
  });
}
```

- [ ] **Step 2: Wire into App.vue init error path**

In `src/App.vue`, import and use in the catch block:

```typescript
import { logError } from "./utils/errorLog";

// In initApp():
} catch (e) {
  logError("App.initApp", e);
  store.configErrors = [{ kind: "InitError", message: `Failed: ${e}` }];
  store.screen = "error";
}
```

- [ ] **Step 3: Wire into TodayView command calls**

In `src/components/TodayView.vue`, in each catch block:

```typescript
import { logError } from "../utils/errorLog";

// In handleUpdateEntry:
} catch (e) {
  logError("TodayView.handleUpdateEntry", e);
}

// In handleDeleteEntry:
} catch (e) {
  logError("TodayView.handleDeleteEntry", e);
  entries.splice(idx, 0, removed);
}
```

- [ ] **Step 4: Wire into SetupScreen**

In `src/components/SetupScreen.vue`:

```typescript
import { logError } from "../utils/errorLog";

// In selectFolder:
} catch (e) {
  logError("SetupScreen.selectFolder", e);
  store.configErrors = [{ kind: "SetupError", message: `Failed: ${e}` }];
  store.screen = "error";
}
```

- [ ] **Step 5: Wire into QuickEntry**

In `src/components/QuickEntry.vue`:

```typescript
import { logError } from "../utils/errorLog";

// In handleSubmit:
} catch (e) {
  logError("QuickEntry.handleSubmit", e);
}
```

- [ ] **Step 6: Verify compilation**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && pnpm vue-tsc --noEmit
```

- [ ] **Step 7: Commit**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && git add src/utils/errorLog.ts src/App.vue src/components/TodayView.vue src/components/SetupScreen.vue src/components/QuickEntry.vue && git commit -m "feat: wire frontend errors to error.log"
```

---

## Part C: Phase 2 — Week/Month Granularity

### Task C1: Types and store — add Granularity support

**Files:**
- Modify: `src/types.ts`
- Modify: `src/stores/useStore.ts`

- [ ] **Step 1: Add Granularity type**

In `src/types.ts`, add:

```typescript
export type Granularity = "day" | "week" | "month";
```

- [ ] **Step 2: Add granularity and periodEntries to store**

In `src/stores/useStore.ts`, add to `AppStore`:

```typescript
export interface AppStore {
  screen: Screen;
  rootPath: string;
  config: Config | null;
  configErrors: ConfigErrorDetail[];
  today: DayFile | null;
  commitments: Commitment[];
  lastDimensions: Record<string, string>;
  currentDate: string;
  granularity: Granularity;
  periodEntries: Record<string, Entry[]>;  // date → entries for week/month views
}
```

Add defaults in `createStore()`:

```typescript
granularity: "day",
periodEntries: {},
```

- [ ] **Step 3: Verify compilation**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && pnpm vue-tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && git add src/types.ts src/stores/useStore.ts && git commit -m "feat: add Granularity type and periodEntries to store"
```

### Task C2: DateNavigator — add granularity picker

**Files:**
- Modify: `src/components/DateNavigator.vue`

- [ ] **Step 1: Add granularity dropdown and navigation logic**

In `DateNavigator.vue` `<script setup>`, add:

```typescript
import type { Granularity } from "../types";

function shift(delta: number) {
  const d = dateObj();
  if (store.granularity === "day") {
    d.setDate(d.getDate() + delta);
  } else if (store.granularity === "week") {
    d.setDate(d.getDate() + delta * 7);
  } else {
    d.setMonth(d.getMonth() + delta);
  }
  store.currentDate = [
    d.getFullYear(),
    String(d.getMonth() + 1).padStart(2, "0"),
    String(d.getDate()).padStart(2, "0"),
  ].join("-");
  loadDay();
}

const displayDate = computed(() => {
  const d = dateObj();
  const fmt = d.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" });
  if (store.granularity === "day") {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const target = new Date(d);
    target.setHours(0, 0, 0, 0);
    const diff = Math.round((target.getTime() - today.getTime()) / 86400000);
    if (diff === 0) return `Today — ${fmt}`;
    if (diff === -1) return `Yesterday — ${fmt}`;
    if (diff === 1) return `Tomorrow — ${fmt}`;
    return fmt;
  }
  if (store.granularity === "week") {
    return weekLabel(d);
  }
  // month
  return d.toLocaleDateString("en-US", { month: "long", year: "numeric" });
});

function weekLabel(d: Date): string {
  const day = d.getDay();
  const monday = new Date(d);
  monday.setDate(d.getDate() - (day === 0 ? 6 : day - 1));
  const sunday = new Date(monday);
  sunday.setDate(monday.getDate() + 6);
  const fmt = (dt: Date) => dt.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  return `${fmt(monday)} – ${fmt(sunday)}`;
}
```

In `<template>`, add granularity picker next to the date display:

```html
<div class="flex items-center gap-2">
  <select
    :value="store.granularity"
    class="text-xs border border-gray-300 rounded px-1.5 py-0.5 focus:outline-none focus:ring-1 focus:ring-blue-500"
    @change="store.granularity = ($event.target as HTMLSelectElement).value as Granularity; loadDay()"
  >
    <option value="day">Day</option>
    <option value="week">Week</option>
    <option value="month">Month</option>
  </select>
</div>
```

Place it between the ← and → buttons, above or next to the date display.

- [ ] **Step 2: Update the shiftDate calls**

Replace the `<button>` onclick handlers to use the new `shift()` function instead of `shiftDate()`:

```html
<button ... @click="shift(-1)">←</button>
<button ... @click="shift(1)">→</button>
```

- [ ] **Step 3: Remove old shiftDate function** (replaced by `shift`)

- [ ] **Step 4: Verify compilation**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && pnpm vue-tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && git add src/components/DateNavigator.vue && git commit -m "feat: add Day/Week/Month granularity picker to DateNavigator"
```

### Task C3: TodayView — load period entries for Week/Month views

**Files:**
- Modify: `src/components/TodayView.vue`

When granularity is Week or Month, fetch entries for all dates in the range and populate `store.periodEntries`.

- [ ] **Step 1: Add period loading logic**

In `TodayView.vue` `<script setup>`, add:

```typescript
import type { Granularity, Entry } from "../types";

function datesInPeriod(dateStr: string, granularity: Granularity): string[] {
  const d = new Date(dateStr + "T00:00:00");
  const dates: string[] = [];
  if (granularity === "day") {
    dates.push(dateStr);
  } else if (granularity === "week") {
    const day = d.getDay();
    const monday = new Date(d);
    monday.setDate(d.getDate() - (day === 0 ? 6 : day - 1));
    for (let i = 0; i < 7; i++) {
      const dt = new Date(monday);
      dt.setDate(monday.getDate() + i);
      dates.push(formatDate(dt));
    }
  } else {
    const year = d.getFullYear();
    const month = d.getMonth();
    const lastDay = new Date(year, month + 1, 0).getDate();
    for (let day = 1; day <= lastDay; day++) {
      dates.push(`${year}-${String(month + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`);
    }
  }
  return dates;
}

function formatDate(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

async function loadPeriod() {
  const dates = datesInPeriod(store.currentDate, store.granularity);
  const map: Record<string, Entry[]> = {};
  for (const date of dates) {
    try {
      const df = (await invoke("get_entries", { rootPath: store.rootPath, date })) as DayFile;
      map[date] = df.entries;
    } catch (e) {
      logError("TodayView.loadPeriod", e);
      map[date] = [];
    }
  }
  store.periodEntries = map;
  // For day granularity, also set store.today for backward compat
  if (store.granularity === "day") {
    store.today = { note: null, entries: map[store.currentDate] || [] };
  }
}
```

- [ ] **Step 2: Call loadPeriod on mount and granularity change**

The existing `loadDay()` calls throughout TodayView and DateNavigator should delegate to `loadPeriod()`. Modify `loadDay()` in DateNavigator to call `loadPeriod()` from TodayView.

Since `loadPeriod` is in TodayView, and DateNavigator is a child, use an emit or provide/inject:

Better approach: keep `loadDay` in DateNavigator for day mode, but on granularity change, emit an event up to TodayView to reload.

Simplest: In DateNavigator, when granularity or date changes, always emit a `navigate` event. TodayView listens and calls `loadPeriod`.

In DateNavigator:
```typescript
const emit = defineEmits<{ navigate: [] }>();
// In loadDay(), after setting store.currentDate:
emit("navigate");
```

In TodayView:
```html
<DateNavigator @navigate="loadPeriod" />
```

And call `loadPeriod()` in `onMounted`.

- [ ] **Step 3: Expose loadPeriod for QuickEntry refresh**

QuickEntry currently calls a local `refreshDay()`. Replace with an emit to TodayView:

In QuickEntry, emit `"appended"`. TodayView listens and calls `loadPeriod()`.

Or simpler: just have QuickEntry emit and TodayView handle it:

```html
<QuickEntry v-if="isToday()" @appended="loadPeriod" />
```

In QuickEntry, change `emit` to include `"appended"`:
```typescript
const emit = defineEmits<{
  submit: [item: string, durationMinutes: number];
  appended: [];
}>();
// After successful append:
emit("appended");
```

- [ ] **Step 4: Verify compilation**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && pnpm vue-tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && git add src/components/TodayView.vue src/components/DateNavigator.vue src/components/QuickEntry.vue && git commit -m "feat: load period entries for Week/Month views"
```

### Task C4: EntryList — group entries with collapsible sections

**Files:**
- Modify: `src/components/EntryList.vue`
- Create: `src/components/EntryGroup.vue`

- [ ] **Step 1: Create EntryGroup.vue for collapsible day/week sections**

Create `src/components/EntryGroup.vue`:

```vue
<script setup lang="ts">
import { ref } from "vue";
import type { Entry } from "../types";
import { formatDuration } from "../utils/format";
import EntryItem from "./EntryItem.vue";

const props = defineProps<{
  label: string;
  entries: Entry[];
  defaultOpen?: boolean;
}>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
}>();

const open = ref(props.defaultOpen ?? true);
const totalMinutes = props.entries.reduce((s, e) => s + e.duration, 0);
</script>

<template>
  <div class="border-b border-gray-100 last:border-b-0">
    <button
      class="w-full flex items-center justify-between px-4 py-2 hover:bg-gray-50 text-left"
      @click="open = !open"
    >
      <span class="text-sm font-medium text-gray-600">{{ label }}</span>
      <span class="text-xs text-gray-400">{{ entries.length }} entries · {{ formatDuration(totalMinutes) }}</span>
    </button>
    <div v-if="open" class="px-4 pb-2">
      <EntryItem
        v-for="(entry, index) in entries"
        :key="entry.id"
        :entry="entry"
        :index="index"
        @update="(entryId, item, dur) => emit('update', entryId, item, dur)"
        @delete="(entryId) => emit('delete', entryId)"
      />
    </div>
  </div>
</template>
```

- [ ] **Step 2: Update EntryList to accept granularity and group entries**

In `EntryList.vue`, add:

```typescript
import type { Granularity } from "../types";
import { computed } from "vue";
import EntryGroup from "./EntryGroup.vue";

const props = defineProps<{
  entries: Entry[];
  granularity: Granularity;
  periodEntries?: Record<string, Entry[]>;
  currentDate?: string;
}>();

interface Group {
  label: string;
  entries: Entry[];
}

const groups = computed<Group[]>(() => {
  if (props.granularity === "day") {
    if (props.entries.length === 0) return [];
    return [{ label: "", entries: props.entries }];
  }
  if (!props.periodEntries) return [];

  const result: Group[] = [];
  if (props.granularity === "week") {
    // Sort dates, group by day
    const sorted = Object.keys(props.periodEntries).sort();
    for (const date of sorted) {
      const entries = props.periodEntries[date];
      if (entries.length === 0) continue;
      const d = new Date(date + "T00:00:00");
      const label = d.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" });
      result.push({ label, entries });
    }
  } else {
    // Month: group by week
    const weeks: Record<string, Entry[]> = {};
    for (const [date, entries] of Object.entries(props.periodEntries)) {
      if (entries.length === 0) continue;
      const d = new Date(date + "T00:00:00");
      const weekStart = new Date(d);
      const day = d.getDay();
      weekStart.setDate(d.getDate() - (day === 0 ? 6 : day - 1));
      const weekEnd = new Date(weekStart);
      weekEnd.setDate(weekStart.getDate() + 6);
      const fmt = (dt: Date) => dt.toLocaleDateString("en-US", { month: "short", day: "numeric" });
      const key = `${fmt(weekStart)} – ${fmt(weekEnd)}`;
      if (!weeks[key]) weeks[key] = [];
      weeks[key].push(...entries);
    }
    for (const [label, entries] of Object.entries(weeks)) {
      result.push({ label, entries });
    }
  }
  return result;
});
```

Update template:

```html
<template>
  <div class="bg-white rounded-lg shadow-sm">
    <div v-if="groups.length === 0" class="p-8 text-center text-gray-400 text-sm">
      No entries for this period.
    </div>
    <template v-else>
      <!-- Day mode: flat list (current behavior) -->
      <div v-if="granularity === 'day'" class="px-4">
        <EntryItem
          v-for="(entry, index) in entries"
          :key="entry.id"
          :entry="entry"
          :index="index"
          @update="(entryId, item, dur) => emit('update', entryId, item, dur)"
          @delete="(entryId) => emit('delete', entryId)"
        />
      </div>
      <!-- Week/Month: grouped with collapsible sections -->
      <EntryGroup
        v-else
        v-for="group in groups"
        :key="group.label"
        :label="group.label"
        :entries="group.entries"
        :defaultOpen="true"
        @update="(entryId, item, dur) => emit('update', entryId, item, dur)"
        @delete="(entryId) => emit('delete', entryId)"
      />
    </template>
  </div>
</template>
```

- [ ] **Step 3: Update TodayView to pass new props to EntryList**

In `TodayView.vue` template:

```html
<EntryList
  :entries="store.today?.entries || []"
  :granularity="store.granularity"
  :periodEntries="store.periodEntries"
  :currentDate="store.currentDate"
  @update="(entryId, item, dur) => handleUpdateEntry(entryId, item, dur)"
  @delete="(entryId) => handleDeleteEntry(entryId)"
/>
```

- [ ] **Step 4: Verify compilation**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && pnpm vue-tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && git add src/components/EntryGroup.vue src/components/EntryList.vue src/components/TodayView.vue && git commit -m "feat: add grouped entry display with collapsible sections"
```

### Task C5: SummaryBar — multi-level totals

**Files:**
- Modify: `src/components/SummaryBar.vue`

- [ ] **Step 1: Add granularity-aware totals**

In `SummaryBar.vue`, update props and template:

```typescript
import type { Granularity, Entry } from "../types";

const props = defineProps<{
  entries: Entry[];
  granularity: Granularity;
  periodEntries?: Record<string, Entry[]>;
}>();

const totalMinutes = computed(() => {
  if (props.granularity === "day") {
    return props.entries.reduce((s, e) => s + e.duration, 0);
  }
  if (!props.periodEntries) return 0;
  return Object.values(props.periodEntries)
    .flat()
    .reduce((s, e) => s + e.duration, 0);
});

const entryCount = computed(() => {
  if (props.granularity === "day") {
    return props.entries.length;
  }
  if (!props.periodEntries) return 0;
  return Object.values(props.periodEntries).reduce((s, arr) => s + arr.length, 0);
});

// For Week/Month, also show Day/Week subtotals
const dayTotals = computed(() => {
  if (props.granularity !== "week" || !props.periodEntries) return null;
  let sum = 0;
  for (const date of Object.keys(props.periodEntries).sort()) {
    const daySum = props.periodEntries[date].reduce((s, e) => s + e.duration, 0);
    sum += daySum;
  }
  return sum;
});
```

Update template to show tiered totals:

```html
<template>
  <div v-if="entryCount > 0" class="text-xs text-gray-500 px-1 space-y-0.5">
    <!-- Day mode: single total -->
    <template v-if="granularity === 'day'">
      <div class="flex justify-between">
        <span>{{ entryCount }} {{ entryCount === 1 ? "entry" : "entries" }}</span>
        <span class="font-medium text-gray-700">{{ formatDuration(totalMinutes) }}</span>
      </div>
    </template>
    <!-- Week mode: day subtotals + week total -->
    <template v-else-if="granularity === 'week' && periodEntries">
      <div v-for="(entries, date) in periodEntries" :key="date" class="flex justify-between ml-2">
        <span>{{ dateLabel(date) }}</span>
        <span>{{ formatDuration(entries.reduce((s, e) => s + e.duration, 0)) }}</span>
      </div>
      <div class="flex justify-between font-medium text-gray-700 pt-1 border-t border-gray-200">
        <span>Week total</span>
        <span>{{ formatDuration(totalMinutes) }}</span>
      </div>
    </template>
    <!-- Month mode: week subtotals + month total -->
    <template v-else-if="granularity === 'month' && periodEntries">
      <div v-for="(weekSum, weekLabel) in weekTotals" :key="weekLabel" class="flex justify-between ml-2">
        <span>{{ weekLabel }}</span>
        <span>{{ formatDuration(weekSum) }}</span>
      </div>
      <div class="flex justify-between font-medium text-gray-700 pt-1 border-t border-gray-200">
        <span>Month total</span>
        <span>{{ formatDuration(totalMinutes) }}</span>
      </div>
    </template>
  </div>
</template>
```

Add helper functions:

```typescript
function dateLabel(dateStr: string): string {
  const d = new Date(dateStr + "T00:00:00");
  return d.toLocaleDateString("en-US", { weekday: "short", day: "numeric" });
}

const weekTotals = computed<Record<string, number>>(() => {
  if (props.granularity !== "month" || !props.periodEntries) return {};
  const weeks: Record<string, number> = {};
  for (const [date, entries] of Object.entries(props.periodEntries)) {
    const d = new Date(date + "T00:00:00");
    const day = d.getDay();
    const weekStart = new Date(d);
    weekStart.setDate(d.getDate() - (day === 0 ? 6 : day - 1));
    const weekEnd = new Date(weekStart);
    weekEnd.setDate(weekStart.getDate() + 6);
    const fmt = (dt: Date) => dt.toLocaleDateString("en-US", { month: "short", day: "numeric" });
    const key = `${fmt(weekStart)} – ${fmt(weekEnd)}`;
    weeks[key] = (weeks[key] || 0) + entries.reduce((s, e) => s + e.duration, 0);
  }
  return weeks;
});
```

- [ ] **Step 2: Update TodayView to pass new props to SummaryBar**

```html
<SummaryBar
  :entries="store.today?.entries || []"
  :granularity="store.granularity"
  :periodEntries="store.periodEntries"
/>
```

- [ ] **Step 3: Verify compilation**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && pnpm vue-tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && git add src/components/SummaryBar.vue src/components/TodayView.vue && git commit -m "feat: add multi-level totals to SummaryBar for Week/Month views"
```

---

## Post-Implementation Verification

- [ ] Run full Rust test suite:

```bash
cd /Users/boxcounter/code/Boxcounter/logbook/src-tauri && cargo test
```

Expected: All tests pass (32 unit + integration tests from Part A + 1 error_log test).

- [ ] Run TypeScript type check:

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && pnpm vue-tsc --noEmit
```

Expected: No errors.

- [ ] Run full stop hook check:

```bash
cd /Users/boxcounter/code/Boxcounter/logbook && pnpm vue-tsc --noEmit && cd src-tauri && cargo check && cargo test
```

Expected: Everything green.
