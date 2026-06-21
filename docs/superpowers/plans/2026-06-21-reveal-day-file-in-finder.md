# Reveal Day File in File Manager — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clicking the path indicator in `MonthView` opens the day file's directory in the OS file manager and selects the file (falling back to the month dir, then the data root, when the file doesn't exist) — replacing the current "open the `.md` in the default editor" behavior.

**Architecture:** Rename the Tauri command `open_in_editor` → `reveal_day_file`. A pure function `resolve_reveal_target(root, date)` decides *what* to reveal (testable, filesystem `exists()` checks only); the command performs the *side effect* via the already-installed `tauri-plugin-opener` (`reveal_item_in_dir` / `open_path`). Frontend just renames its handler + invoke string. No IPC payload/contract change — only the command name changes, which must be synced across 5 sites.

**Tech Stack:** Rust (Tauri 2.x command), `tauri-plugin-opener` 2.5.4 (already a dependency; `opener:default` capability already granted), Vue 3 + TypeScript, Vitest, `cargo test`.

**Spec:** `docs/superpowers/specs/2026-06-21-reveal-day-file-in-finder-design.md`

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `src-tauri/src/commands.rs` | Modify | Remove `open_in_editor`; add `RevealTarget` struct, pure `resolve_reveal_target`, `reveal_day_file` command + unit tests |
| `src-tauri/src/lib.rs` | Modify (line 65) | Register `reveal_day_file` instead of `open_in_editor` in `invoke_handler` |
| `src/components/MonthView.vue` | Modify (lines ~242–246, ~381) | Rename handler `openInEditor` → `revealDayFile`; invoke `"reveal_day_file"`; update `@click` |
| `src/__tests__/mocks/tauri.ts` | Modify (line ~43) | Rename mock `case "open_in_editor"` → `case "reveal_day_file"` |
| `SPEC.md` | Modify (line 33) | Update command signature + comment |

**Design notes locked in here:**
- `resolve_reveal_target` reuses `files::day_path(root, date)` (which builds `root/YYYY/MM/YYYY-MM-DD.md` and validates the date). The month dir is derived as `day_path.parent()` — no manual string slicing.
- `RevealTarget` is module-private and has **no** serde derive — it never crosses IPC, so `models.rs` and the JSON contract are untouched.
- Fallback ladder is exactly: file → month dir → data root. The year dir is intentionally skipped (matches spec table).

---

## Task 1: Pure decision function `resolve_reveal_target` (TDD)

**Files:**
- Modify: `src-tauri/src/commands.rs` (add struct + fn near `open_in_editor`, ~line 879; add tests inside the existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing tests**

Add these three tests inside the existing `#[cfg(test)] mod tests { ... }` block in `src-tauri/src/commands.rs` (e.g. right after `test_read_monthly_file_safe_corrupt`). They use `std::env::temp_dir()` and clean up, matching the neighboring fs tests.

```rust
#[test]
fn test_resolve_reveal_target_file_exists() {
    use std::fs;
    let tmp = std::env::temp_dir().join("logbook_test_reveal_file");
    let _ = fs::remove_dir_all(&tmp);
    let date = "2026-06-21";
    let file = files::day_path(&tmp, date).unwrap();
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "---\nentries: []\n---\n").unwrap();

    let t = resolve_reveal_target(&tmp, date).unwrap();
    assert_eq!(t.path, file);
    assert!(t.select, "existing file must be selected");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_resolve_reveal_target_falls_back_to_month_dir() {
    use std::fs;
    let tmp = std::env::temp_dir().join("logbook_test_reveal_month");
    let _ = fs::remove_dir_all(&tmp);
    let date = "2026-06-21";
    let month_dir = tmp.join("2026").join("06");
    fs::create_dir_all(&month_dir).unwrap(); // dir exists, day file does not

    let t = resolve_reveal_target(&tmp, date).unwrap();
    assert_eq!(t.path, month_dir);
    assert!(!t.select, "directory target must not be selected");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_resolve_reveal_target_falls_back_to_root() {
    use std::fs;
    let tmp = std::env::temp_dir().join("logbook_test_reveal_root");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap(); // only root exists
    let date = "2026-06-21";

    let t = resolve_reveal_target(&tmp, date).unwrap();
    assert_eq!(t.path, tmp);
    assert!(!t.select);

    let _ = fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test resolve_reveal_target`
Expected: FAIL to compile — `cannot find function resolve_reveal_target` / `cannot find type RevealTarget`.

- [ ] **Step 3: Write the struct + minimal implementation**

Add to `src-tauri/src/commands.rs`, immediately above where `open_in_editor` currently sits (around line 879, after `get_available_months`):

```rust
/// What the file manager should reveal/open for a given day.
struct RevealTarget {
    path: std::path::PathBuf,
    /// true  → reveal `path` and select it (it is the day file)
    /// false → open `path` as a directory (no file to select)
    select: bool,
}

/// Decide what to reveal for `date`:
/// - day file `root/YYYY/MM/YYYY-MM-DD.md` exists → select that file
/// - else the month dir `root/YYYY/MM/` exists    → open that dir
/// - else                                         → open the data root
fn resolve_reveal_target(root: &std::path::Path, date: &str) -> Result<RevealTarget, String> {
    let file = files::day_path(root, date)?;
    if file.exists() {
        return Ok(RevealTarget { path: file, select: true });
    }
    if let Some(month_dir) = file.parent() {
        if month_dir.exists() {
            return Ok(RevealTarget {
                path: month_dir.to_path_buf(),
                select: false,
            });
        }
    }
    Ok(RevealTarget {
        path: root.to_path_buf(),
        select: false,
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test resolve_reveal_target`
Expected: PASS — 3 passed. (A `dead_code` warning for `reveal`/field usage is acceptable until Task 2 wires it; the fields are read by the tests so no warning is expected.)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(backend): add resolve_reveal_target decision fn for reveal-in-finder"
```

---

## Task 2: `reveal_day_file` command + registration (replaces `open_in_editor`)

**Files:**
- Modify: `src-tauri/src/commands.rs` (add `OpenerExt` import; delete `open_in_editor` fn ~lines 879–918; add `reveal_day_file`)
- Modify: `src-tauri/src/lib.rs:65` (registration)

- [ ] **Step 1: Add the `OpenerExt` import**

In `src-tauri/src/commands.rs`, add to the import block at the top (after `use tauri::{AppHandle, Manager};`):

```rust
use tauri_plugin_opener::OpenerExt;
```

- [ ] **Step 2: Delete the old `open_in_editor` command**

Remove the entire `#[tauri::command] pub fn open_in_editor(root_path: String, date: String) -> Result<(), String> { ... }` function (it spans roughly lines 879–918, the block that builds `file_path` and shells out to `open` / `xdg-open` / `cmd start`).

- [ ] **Step 3: Add the `reveal_day_file` command**

Insert in its place (keep it near the other commands; it uses the `resolve_reveal_target` added in Task 1):

```rust
#[tauri::command]
pub fn reveal_day_file(app: AppHandle, root_path: String, date: String) -> Result<(), String> {
    error_log::log_command_enter("reveal_day_file", &format!("date={}", date));
    validate_date_format(&date)?;
    let root = std::path::Path::new(&root_path);
    let target = resolve_reveal_target(root, &date)?;

    let result = if target.select {
        app.opener()
            .reveal_item_in_dir(&target.path)
            .map_err(|e| format!("Failed to reveal {}: {}", target.path.display(), e))
    } else {
        app.opener()
            .open_path(target.path.to_string_lossy().into_owned(), None::<String>)
            .map_err(|e| format!("Failed to open {}: {}", target.path.display(), e))
    };

    error_log::log_command_exit("reveal_day_file", result.is_ok(), "");
    result
}
```

> `None::<String>` is required: `open_path`'s second param is `Option<impl Into<String>>`; a bare `None` cannot infer its type and fails to compile.

- [ ] **Step 4: Update the command registration**

In `src-tauri/src/lib.rs`, change line 65 inside `tauri::generate_handler![ ... ]`:

```rust
            commands::reveal_day_file,
```

(was `commands::open_in_editor,`)

- [ ] **Step 5: Verify backend compiles and all tests pass**

Run: `cd src-tauri && cargo check && cargo test`
Expected: `cargo check` clean (no `open_in_editor` references remain); `cargo test` all pass, including the 3 `resolve_reveal_target` tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(backend): replace open_in_editor with reveal_day_file command"
```

---

## Task 3: Frontend handler rename + test mock

**Files:**
- Modify: `src/components/MonthView.vue` (handler fn ~lines 242–246; `@click` ~line 381)
- Modify: `src/__tests__/mocks/tauri.ts` (mock `case`, ~line 43)

- [ ] **Step 1: Rename the handler function in `MonthView.vue`**

Replace the existing block (currently lines ~242–246):

```ts
async function openInEditor() {
  if (!store.rootPath) return;
  try { await invoke("open_in_editor", { rootPath: store.rootPath, date: store.currentDate }); }
  catch (e) { logError("MonthView.openInEditor", e); }
}
```

with:

```ts
async function revealDayFile() {
  if (!store.rootPath) return;
  try { await invoke("reveal_day_file", { rootPath: store.rootPath, date: store.currentDate }); }
  catch (e) { logError("MonthView.revealDayFile", e); }
}
```

- [ ] **Step 2: Update the `@click` binding in the template**

In the path-indicator button (around line 381), change:

```html
          @click="openInEditor"
```

to:

```html
          @click="revealDayFile"
```

Leave `displayPath`, `dayFilePath`, and the button `:title` (full-path tooltip) unchanged — only the click behavior changes.

- [ ] **Step 3: Update the Tauri mock**

In `src/__tests__/mocks/tauri.ts`, change the case (around line 43):

```ts
    case "reveal_day_file":
      return;
```

(was `case "open_in_editor":`)

- [ ] **Step 4: Run the frontend test suite + typecheck**

Run: `pnpm test && pnpm vue-tsc --noEmit`
Expected: all Vitest tests pass; `vue-tsc` reports no errors. (No existing test asserts this button's click, so the rename is non-breaking; the mock rename keeps the "no unmocked command" guard satisfied.)

- [ ] **Step 5: Commit**

```bash
git add src/components/MonthView.vue src/__tests__/mocks/tauri.ts
git commit -m "feat(ui): reveal day file in file manager on path-indicator click"
```

---

## Task 4: Documentation sync (`SPEC.md`)

**Files:**
- Modify: `SPEC.md:33`

- [ ] **Step 1: Update the command line in `SPEC.md`**

Change line 33 from:

```
open_in_editor(root_path: String, date: String) → Result<(), String>  // 用系统编辑器打开文件
```

to:

```
reveal_day_file(root_path: String, date: String) → Result<(), String>  // 在文件管理器中打开目录并选中日文件
```

- [ ] **Step 2: Verify no stale live references remain**

Run: `grep -rn "open_in_editor\|openInEditor" src-tauri/src src SPEC.md`
Expected: **no matches.** (Matches under `docs/superpowers/plans|specs` are historical snapshots and are intentionally left untouched.)

- [ ] **Step 3: Commit**

```bash
git add SPEC.md
git commit -m "docs(spec): rename open_in_editor to reveal_day_file"
```

---

## Task 5: Full verification

- [ ] **Step 1: Run the full project verification chain**

Run: `pnpm verify && cd src-tauri && cargo test`
Expected: `vitest run` green, `vue-tsc --noEmit` clean, `vite build` succeeds, `cargo test` green. (`pnpm verify` = `vitest run && vue-tsc --noEmit && vite build`; this is also what the Stop hook / CI run.)

- [ ] **Step 2: Manual smoke test (optional but recommended)**

Run: `pnpm tauri dev` (from repo root). In the app, click the path indicator bottom-right:
- On a day **with** an entry → file manager opens the month folder with `YYYY-MM-DD.md` selected.
- On a day **without** an entry but with other entries that month → file manager opens the month folder.
- On a day in a month with no data at all → file manager opens the data root folder.

- [ ] **Step 3: Consistency check before handoff**

Per project CLAUDE.md, run the `/check-consistency` skill (doc ↔ doc + doc ↔ code) before writing any HANDOFF. The command rename touches SPEC.md, code, and tests — confirm they agree.

---

## Self-Review

**1. Spec coverage:**
- Reveal file when it exists → Task 1 (`select=true`) + Task 2 (`reveal_item_in_dir`). ✓
- Fallback month dir → root → Task 1 (branches) + tests. ✓
- Use opener plugin, no capability change → Task 2 (uses `app.opener()`; `opener:default` already granted, no `capabilities/default.json` edit). ✓
- Command rename synced across 5 sites → commands.rs (T2), lib.rs (T2), MonthView.vue (T3), mocks/tauri.ts (T3), SPEC.md (T4); verified by grep in T4 Step 2. ✓
- Display text/tooltip unchanged → T3 Step 2 note. ✓
- No IPC contract change → `RevealTarget` private, no serde; `models.rs` untouched. ✓
- Pure/side-effect split + testing → T1 (pure fn unit tests) and T2 (side effect, no CI assertion). ✓

**2. Placeholder scan:** No TBD/TODO; every code step has full code; every command has expected output. ✓

**3. Type consistency:** `resolve_reveal_target(root: &Path, date: &str) -> Result<RevealTarget, String>` and `RevealTarget { path: PathBuf, select: bool }` are used identically in T1 (def + tests) and T2 (command). Frontend `revealDayFile` / `"reveal_day_file"` consistent across MonthView.vue and mock. ✓
