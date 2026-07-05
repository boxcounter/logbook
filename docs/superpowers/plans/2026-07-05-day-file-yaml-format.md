# Day File .md → .yaml Format Change — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Switch day file format from `.md` with YAML frontmatter (`---\n{yaml}\n---`) to pure `.yaml` (no `---` wrappers) and bump data version to 2.

**Architecture:** Every code path that filters/parses day files by `.md` extension gets updated to `.yaml`. The `parse_frontmatter()` helper is removed in favor of direct `yaml_serde::from_str`. A new `logbook-cli migrate` subcommand handles existing data conversion.

**Tech Stack:** Rust (yaml_serde 0.10, clap, chrono), TypeScript (mock only — no runtime changes)

## Global Constraints

- YAML 序列化用 `yaml_serde`（0.10），不是 `serde_yml`
- 文件写入：先写 `.tmp` 再 rename（原子写入）
- 命名约定：落盘格式与代码标识符解耦
- `CURRENT_DATA_VERSION` 从 1 → 2
- `.tmp` 孤儿文件清理逻辑不变
- 前端除 `reveal_day_file` mock 外无代码变动

---

### Task 1: files.rs — core format changes

**Files:**
- Modify: `src-tauri/src/files.rs`

**Changes:**

- [ ] **Step 1: Update `day_path()` extension from `.md` to `.yaml`**

Change line 27 comment and line 41 format string:

```rust
// Line 27: comment
/// Day file path: {root}/{year}/{month:02}/{date}.yaml

// Line 41: format string
.join(format!("{:04}-{:02}-{:02}.yaml", d.year(), d.month(), d.day())))
```

- [ ] **Step 2: Update `read_day_file()` — use `yaml_serde::from_str` directly**

Replace lines 112-125 with:

```rust
/// Read a day file. Returns empty DayFile if file doesn't exist.
pub fn read_day_file(root: &Path, date: &str) -> Result<DayFile, String> {
    let path = day_path(root, date)?;
    if !path.exists() {
        return Ok(DayFile {
            note: None,
            entries: vec![],
        });
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let content = content.trim_start_matches('\u{feff}');
    yaml_serde::from_str::<DayFile>(content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}
```

- [ ] **Step 3: Update `write_day_file()` — remove `---` wrapping**

Replace lines 127-140 with:

```rust
/// Write a full day file (atomic: tmp then rename).
pub fn write_day_file(root: &Path, date: &str, day_file: &DayFile) -> Result<(), String> {
    let path = day_path(root, date)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let yaml_body =
        yaml_serde::to_string(day_file).map_err(|e| format!("Failed to serialize: {}", e))?;
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &yaml_body).map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}
```

Note: `path.with_extension("tmp")` unchanged — when `day_path` returns `foo.yaml`, it produces `foo.tmp` (same as before with `foo.md` → `foo.tmp`).

- [ ] **Step 4: Remove `parse_frontmatter()` function**

Delete lines 461-486 (the entire function).

- [ ] **Step 5: Update unit tests**

Delete tests `test_parse_frontmatter_basic`, `test_parse_frontmatter_with_note`, `test_parse_frontmatter_with_utf8_bom`.

Update `test_day_path` (line 554):

```rust
assert_eq!(p, PathBuf::from("/data/2026/06/2026-06-12.yaml"));
```

Update `test_day_path_normalizes_unpadded_date` (line 563):

```rust
assert_eq!(p, PathBuf::from("/data/2026/06/2026-06-05.yaml"));
```

Update `test_cleanup_tmp_files_removes_orphaned_tmp` (lines 773-780):

```rust
fs::write(day_dir.join("2026-07-04.yaml.tmp"), "data").unwrap();
// Also create a valid .yaml file that should survive
fs::write(day_dir.join("2026-07-04.yaml"), "note:\nentries: []\n").unwrap();

cleanup_tmp_files(&tmp);

assert!(!day_dir.join("2026-07-04.yaml.tmp").exists(), "orphaned .tmp should be removed");
assert!(day_dir.join("2026-07-04.yaml").exists(), "valid .yaml should survive");
```

- [ ] **Step 6: Run unit tests**

```bash
cd src-tauri && cargo test -p tauri_app_lib -- files
```

Expected: all tests PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/files.rs
git commit -m "feat: switch day file format .md -> .yaml in files.rs"
```

---

### Task 2: scan.rs + integrity.rs — extension checks

**Files:**
- Modify: `src-tauri/src/scan.rs`
- Modify: `src-tauri/src/integrity.rs`

**Changes:**

- [ ] **Step 1: Update `scan.rs` — change `.md` to `.yaml` and remove `_monthly.md`**

In `scan_data_dir` (lines 8-11), update comments:

```rust
/// - `.yaml` files (except `_monthly.yaml`): validate date stem and YAML.
```

Line 66: `file_name.ends_with(".md")` → `file_name.ends_with(".yaml")`

Lines 70-73: Remove `_monthly.md` skip block entirely.

Line 75: `&file_name[..file_name.len() - 3]` → `&file_name[..file_name.len() - 5]` (strip `.yaml` = 5 chars)

- [ ] **Step 2: Update `scan.rs` tests**

`test_valid_day_file_no_warnings` (lines 167-172): change `.md` → `.yaml` in path, and update content to pure YAML:

```rust
let day_file = root.join("2026/06/2026-06-15.yaml");
write_file(
    &day_file,
    "note: A valid note\nentries: []\n",
);
```

`test_invalid_filename_reported` (lines 190-198): `.md` → `.yaml`:

```rust
let bad_file = root.join("2026/06/not-a-date.yaml");
write_file(&bad_file, "note: ok\n");
// ...
assert!(warnings[0].path.contains("not-a-date.yaml"));
```

`test_corrupt_frontmatter_reported` (lines 210-219): `.md` → `.yaml`, and update corrupt content:

```rust
let day_file = root.join("2026/06/2026-06-15.yaml");
write_file(&day_file, "\tindented: not valid yaml\n");
// ...
assert!(warnings[0].path.contains("2026-06-15.yaml"));
```

`test_orphaned_tmp_reported` (line 231): `.md.tmp` → `.yaml.tmp`:

```rust
let tmp_file = root.join("2026/06/2026-06-15.yaml.tmp");
```

`test_non_md_files_skipped` (lines 248-262): rename test to `test_non_yaml_files_skipped`, update comment to `non-.yaml files skipped`:

```rust
#[test]
fn test_non_yaml_files_skipped() {
    let root = temp_root();
    let txt = root.join("2026/06/notes.txt");
    write_file(&txt, "some text file\n");

    let warnings = scan_data_dir(&root);
    assert!(
        warnings.is_empty(),
        "non-.yaml files should be skipped, got {:?}",
        warnings
    );

    fs::remove_dir_all(&root).expect("cleanup");
}
```

`test_multiple_issues_accumulated` (lines 269-291): update all `.md` → `.yaml`:

```rust
write_file(&root.join("2026/06/not-a-date.yaml"), "note: ok\n");
write_file(
    &root.join("2026/06/2026-06-15.yaml"),
    "\tbad yaml here\n",
);
write_file(&root.join("2026/06/2026-06-16.yaml.tmp"), "orphan\n");
```

- [ ] **Step 3: Update `integrity.rs` — change `.md` to `.yaml` and remove `_monthly.md`**

Line 248:

```rust
if !file_name.ends_with(".yaml") {
    continue;
}
```

Remove `_monthly.md` from the condition (it was `|| file_name == "_monthly.md"`).

Line 251:

```rust
let date = file_name.trim_end_matches(".yaml");
```

Update test fixture paths in `integrity.rs` tests (lines 286-321): all `.md` → `.yaml`:

```rust
path: "2026/07/05.yaml".into(),
// ...
path: "x.yaml".into(),
// ...
path: "a.yaml".into(),
// ...
path: "b.yaml".into(),
```

- [ ] **Step 4: Run unit tests**

```bash
cd src-tauri && cargo test -p tauri_app_lib -- scan integrity
```

Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/scan.rs src-tauri/src/integrity.rs
git commit -m "feat: update scan + integrity for .yaml day files"
```

---

### Task 3: commands.rs — bulk extension changes + `_monthly.md` removal

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Changes:**

- [ ] **Step 1: `get_month_entries` (line 524)**

```rust
if !file_name.ends_with(".yaml") {
    continue;
}
```
Also remove `file_name == "_monthly.md" ||` from the condition. Line 527:

```rust
let date = file_name.trim_end_matches(".yaml");
```

- [ ] **Step 2: `get_commitment_progress` (line 981)**

```rust
if !file_name.ends_with(".yaml") {
    continue;
}
```
Line 984:

```rust
match crate::files::read_day_file(root, file_name.trim_end_matches(".yaml")) {
```

- [ ] **Step 3: `set_commitments` repair sweep (lines 1226-1250)**

```rust
if !file_name.ends_with(".yaml") {
    continue;
}
```
Remove `file_name == "_monthly.md" ||` from condition. Line 1229:

```rust
if let Ok(mut day_file) = crate::files::read_day_file(root, file_name.trim_end_matches(".yaml")) {
```
Line 1250:

```rust
if let Err(e) = crate::files::write_day_file(root, file_name.trim_end_matches(".yaml"), &day_file) {
```

- [ ] **Step 4: `batch_count_entries_for_goals` (line 1314-1317)**

```rust
if !file_name.ends_with(".yaml") {
    continue;
}
```
Remove `file_name == "_monthly.md" ||`. Line 1317:

```rust
let date = file_name.trim_end_matches(".yaml");
```

- [ ] **Step 5: `batch_rename_goals_in_entries` (line 1366-1369)**

```rust
if !file_name.ends_with(".yaml") {
    continue;
}
```
Remove `file_name == "_monthly.md" ||`. Line 1369:

```rust
let date = file_name.trim_end_matches(".yaml");
```

- [ ] **Step 6: `cleanup_deleted_goals_in_entries` (line 1432-1435)**

```rust
if !file_name.ends_with(".yaml") {
    continue;
}
```
Remove `file_name == "_monthly.md" ||`. Line 1435:

```rust
let date = file_name.trim_end_matches(".yaml");
```

- [ ] **Step 7: `get_available_months` (lines 1646-1654)**

Update comment: `// Check if this month directory contains at least one .yaml file`

```rust
let has_md = match std::fs::read_dir(month_entry.path()) { // keep var name
    Ok(entries) => {
        let mut found = false;
        for e in entries {
            match e {
                Ok(entry) => {
                    let name_str = entry.file_name().to_string_lossy().into_owned();
                    if name_str.ends_with(".yaml") {
                        found = true;
                        break;
                    }
                }
```

- [ ] **Step 8: `resolve_reveal_target` comment (line 1700)**

```rust
/// - day file `root/YYYY/MM/YYYY-MM-DD.yaml` exists → select that file
```

- [ ] **Step 9: Update test fixtures in `commands.rs` test module**

`test_get_commitment_progress_comprehensive` (lines 2137-2146):

```rust
fs::write(
    monthly_dir.join("2026-06-01.yaml"),
    "entries:\n  - id: e1\n    item: Code\n    duration: 60\n    dimensions:\n      goal: Ship it\n  - id: e2\n    item: PR\n    duration: 30\n    dimensions:\n      goal: Review\n",
)
.unwrap();

fs::write(
    monthly_dir.join("2026-06-02.yaml"),
    "entries:\n  - id: e3\n    item: Plan\n    duration: 45\n    dimensions:\n      goal: Planning\n",
)
.unwrap();
```

`test_get_commitment_progress_ignores_unmatched_goals` (line 2192):

```rust
fs::write(
    monthly_dir.join("2026-06-01.yaml"),
    "entries:\n  - id: e1\n    item: Unknown task\n    duration: 60\n    dimensions:\n      goal: Not a goal\n",
)
.unwrap();
```

`test_get_commitment_progress_with_role_dimension` (lines 2274, 2291):

```rust
// Day 1 (lines 2256-2274): remove --- wrappers, rename .md → .yaml
let day1 = "entries:\n  - id: e1\n    item: Code feature\n    duration: 120\n    dimensions:\n      role: Dev\n      goal: Ship X\n  - id: e2\n    item: Standup\n    duration: 30\n    dimensions:\n      role: Dev\n  - id: e3\n    item: Email\n    duration: 15\n    dimensions: {}\n";
std::fs::write(month_dir.join("2026-07-01.yaml"), day1).unwrap();

// Day 2 (lines 2277-2291): remove --- wrappers, rename .md → .yaml
let day2 = "entries:\n  - id: e4\n    item: Roadmap planning\n    duration: 60\n    dimensions:\n      goal: Roadmap\n  - id: e5\n    item: Mismatch case\n    duration: 45\n    dimensions:\n      role: Dev\n      goal: Roadmap\n";
std::fs::write(month_dir.join("2026-07-02.yaml"), day2).unwrap();
```

- [ ] **Step 10: Run unit tests**

```bash
cd src-tauri && cargo test -p tauri_app_lib -- commands
```

Expected: all tests PASS.

- [ ] **Step 11: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: update commands.rs for .yaml day files, remove _monthly.md guards"
```

---

### Task 4: operation_log.rs — renames + extension

**Files:**
- Modify: `src-tauri/src/operation_log.rs`

**Changes:**

- [ ] **Step 1: Rename `collect_md_files` → `collect_yaml_files`**

Line 455: update comment. Line 457: rename function.

- [ ] **Step 2: Rename `collect_md_files_recursive` → `collect_yaml_files_recursive`**

Line 468: rename function.

- [ ] **Step 3: Update extension check in `collect_yaml_files_recursive`**

Line 491:

```rust
} else if path.extension().map_or(false, |ext| ext == "yaml") {
```

- [ ] **Step 4: Update callers in `verify_op_log`**

Lines 334 and 341:

```rust
let original_files = collect_yaml_files(root)?;
// ...
let replay_files = collect_yaml_files(&replay_root)?;
```

- [ ] **Step 5: Run unit tests**

```bash
cd src-tauri && cargo test -p tauri_app_lib -- operation_log
```

Expected: all tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/operation_log.rs
git commit -m "feat: rename collect_md_files -> collect_yaml_files in operation_log"
```

---

### Task 5: models.rs — version bump

**Files:**
- Modify: `src-tauri/src/models.rs`

**Changes:**

- [ ] **Step 1: Bump `CURRENT_DATA_VERSION`**

Line 93:

```rust
pub const CURRENT_DATA_VERSION: u32 = 2;
```

- [ ] **Step 2: Run unit tests**

```bash
cd src-tauri && cargo test -p tauri_app_lib -- models
```

Expected: all tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "feat: bump CURRENT_DATA_VERSION to 2 for .yaml format"
```

---

### Task 6: Integration test updates

**Files:**
- Modify: `src-tauri/tests/entry_crud_integration.rs`
- Modify: `src-tauri/tests/scan_integration.rs`
- Modify: `src-tauri/tests/op_log_verify_integration.rs`
- Modify: `src-tauri/tests/cli_integration.rs`
- Modify: `src-tauri/tests/commitment_editor_integration.rs`
- Modify: `src-tauri/tests/integrity_guard_integration.rs`

**Changes:**

- [ ] **Step 1: `entry_crud_integration.rs`**

Lines 21-28: Remove `_monthly.md` write (dead format). Write `commitments.yaml` instead:

```rust
// Write commitments.yaml for June 2026
let monthly_dir = root.join("2026/06");
fs::create_dir_all(&monthly_dir).unwrap();
fs::write(
    monthly_dir.join("commitments.yaml"),
    "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it\n",
)
.unwrap();
```

Also write a `dimensions.yaml` in the month dir to instantiate the month:

```rust
fs::write(
    monthly_dir.join("dimensions.yaml"),
    "[]\n",
)
.unwrap();
```

- [ ] **Step 2: `scan_integration.rs`**

Line 4: update comment `YYYY-MM-DD.md` → `YYYY-MM-DD.yaml`

Line 41: `readme.md` → `readme.txt` (a non-.yaml file to test skip behavior):

```rust
write_file(&root.join("readme.txt"), "just some notes\n");
```

Lines 44-47: rename `.md` → `.yaml`, update bad content:

```rust
write_file(
    &root.join("2026/06/2026-06-15.yaml"),
    "\tbad yaml content\n",
);
```

Lines 84-87: similar update for the clean test case:

```rust
write_file(
    &root.join("2026/06/2026-06-15.yaml"),
    "note: Clean day\nentries: []\n",
);
```

- [ ] **Step 3: `op_log_verify_integration.rs`**

Line 97: rename `.md` → `.yaml`, remove `---` wrappers from content:

```rust
let day_content = "entries:\n- id: manual-id\n  item: manual entry\n  duration: 30\n  dimensions: {}\n";
fs::write(day_dir.join("2026-06-15.yaml"), day_content).unwrap();
```

Also update any other `.md` references in the file — search for `.md` and replace with `.yaml`, removing `---` from any day file content strings.

- [ ] **Step 4: `cli_integration.rs`**

Line 480: `.md` → `.yaml`:

```rust
let day_path = tmp.join("2026").join("06").join("2026-06-15.yaml");
```

Also update the file content written to this path to use pure YAML (no `---` wrappers).

- [ ] **Step 5: `commitment_editor_integration.rs`**

Line 237: `notes.md` → `notes.txt` (non-yaml file):

```rust
fs::write(root.join("2026/06/notes.txt"), "# scratch notes\n").unwrap();
```

Line 273: `.md` → `.yaml`, update bad content:

```rust
fs::write(root.join("2026/06/2026-06-03.yaml"), "\tbad: yaml\n").unwrap();
```

Search for any other `.md` references and update similarly.

- [ ] **Step 6: `cli_integration.rs`**

Line 41: change `format!("{}.md", date)` → `format!("{}.yaml", date)`. This writes day files in a helper function used by multiple tests.

Line 480: `.md` → `.yaml`:

```rust
let day_path = tmp.join("2026").join("06").join("2026-06-15.yaml");
```

Also update any `---` wrapped content strings in the file to pure YAML (remove `---` markers).

- [ ] **Step 7: `integrity_guard_integration.rs`**

All `format!("{}.md", date)` → `format!("{}.yaml", date)` (lines 44, 97, 126, 181, 210, 239, 276).

Path string literals (lines 140, 155): `"test.md"` → `"test.yaml"`.

Line 261 comment: `"valid .md file"` → `"valid .yaml file"`.

For any test data content strings with `---` markers, update to pure YAML. For "not valid yaml" fixtures, update to still-invalid YAML (e.g., `"\tbad: yaml\n"`).

- [ ] **Step 8: Run all integration tests**

```bash
cd src-tauri && cargo test --test '*' -- --nocapture 2>&1 | tail -50
```

Expected: all integration tests PASS.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/tests/
git commit -m "test: update integration tests for .yaml day file format"
```

---

### Task 7: Migration CLI tool

**Files:**
- Create: `src-tauri/src/cli/migrate.rs`
- Modify: `src-tauri/src/cli/mod.rs`

**Interfaces:**
- Consumes: `crate::cli::root_path::resolve_root_path`, `crate::files` (version_path, read_version_file, write_version_file)
- Produces: `pub fn run(root: &Path)` — called from `mod.rs`

- [ ] **Step 1: Create `src-tauri/src/cli/migrate.rs`**

```rust
use std::fs;
use std::path::Path;

pub fn run(root: &Path) -> Result<(), String> {
    eprintln!("Scanning for .md day files in {}...", root.display());

    let mut converted = 0u32;
    let mut skipped = 0u32;
    let mut errors: Vec<String> = Vec::new();

    // Walk {root}/{YYYY}/{MM}/*.md
    match fs::read_dir(root) {
        Ok(year_entries) => {
            for year_entry in year_entries {
                let year_entry = match year_entry {
                    Ok(e) => e,
                    Err(e) => {
                        errors.push(format!("Failed to read dir entry: {}", e));
                        continue;
                    }
                };
                let year_path = year_entry.path();
                if !year_path.is_dir() {
                    continue;
                }
                let year_name = year_entry.file_name().to_string_lossy().into_owned();
                if year_name.parse::<i32>().is_err() {
                    continue;
                }

                match fs::read_dir(&year_path) {
                    Ok(month_entries) => {
                        for month_entry in month_entries {
                            let month_entry = match month_entry {
                                Ok(e) => e,
                                Err(e) => {
                                    errors.push(format!("Failed to read month dir: {}", e));
                                    continue;
                                }
                            };
                            let month_path = month_entry.path();
                            if !month_path.is_dir() {
                                continue;
                            }

                            match fs::read_dir(&month_path) {
                                Ok(day_entries) => {
                                    for day_entry in day_entries {
                                        let day_entry = match day_entry {
                                            Ok(e) => e,
                                            Err(e) => {
                                                errors.push(format!("Failed to read day entry: {}", e));
                                                continue;
                                            }
                                        };
                                        let path = day_entry.path();
                                        let file_name = path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("");

                                        // Only match YYYY-MM-DD.md
                                        if !file_name.ends_with(".md") {
                                            continue;
                                        }
                                        let stem = &file_name[..file_name.len() - 3];
                                        if chrono::NaiveDate::parse_from_str(stem, "%Y-%m-%d").is_err() {
                                            continue;
                                        }

                                        // Check if .yaml already exists (idempotent)
                                        let yaml_path = month_path.join(format!("{}.yaml", stem));
                                        if yaml_path.exists() {
                                            skipped += 1;
                                            continue;
                                        }

                                        // Read .md, strip --- markers, write .yaml
                                        match convert_day_file(&path, &yaml_path) {
                                            Ok(()) => {
                                                converted += 1;
                                                // Delete old .md
                                                let _ = fs::remove_file(&path);
                                            }
                                            Err(e) => {
                                                errors.push(format!(
                                                    "Failed to convert {}: {}",
                                                    path.display(),
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    errors.push(format!(
                                        "Failed to read {}: {}",
                                        month_path.display(),
                                        e
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!(
                            "Failed to read {}: {}",
                            year_path.display(),
                            e
                        ));
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!("Failed to read root dir {}: {}", root.display(), e));
        }
    }

    // Bump version.txt to 2
    if converted > 0 || skipped > 0 {
        crate::files::write_version_file(root, 2)?;
    }

    eprintln!("Converted: {}, Skipped (already .yaml): {}", converted, skipped);
    if !errors.is_empty() {
        eprintln!("Errors:");
        for e in &errors {
            eprintln!("  - {}", e);
        }
    }

    Ok(())
}

fn convert_day_file(md_path: &Path, yaml_path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(md_path)
        .map_err(|e| format!("Failed to read: {}", e))?;
    let content = content.trim_start_matches('\u{feff}');
    let content = content.trim();

    // Strip --- markers
    let yaml = if content.starts_with("---") {
        let after = &content[3..];
        if let Some(end) = after.find("\n---") {
            &after[..end]
        } else if after.ends_with("---") {
            &after[..after.len() - 3]
        } else {
            after
        }
    } else {
        content
    };

    let yaml = yaml.trim();
    if yaml.is_empty() {
        return Ok(());
    }

    // Validate it parses as DayFile
    let _: crate::models::DayFile = yaml_serde::from_str(yaml)
        .map_err(|e| format!("YAML parse error: {}", e))?;

    // Write with trailing newline
    fs::write(yaml_path, format!("{}\n", yaml))
        .map_err(|e| format!("Failed to write: {}", e))?;

    Ok(())
}
```

- [ ] **Step 2: Add `migrate` subcommand to `src-tauri/src/cli/mod.rs`**

Add module declaration (line 3):

```rust
pub mod migrate;
```

Add `Migrate` variant to `Commands` enum (after `Dimensions`):

```rust
    /// Migrate day files from .md (frontmatter) to .yaml format
    Migrate,
```

Add handler in `run()` function (after `Commands::Dimensions` arm):

```rust
        Commands::Migrate => {
            if let Err(e) = migrate::run(&root) {
                output::print_error(&e);
                std::process::exit(1);
            }
        }
```

- [ ] **Step 3: Build and test**

```bash
cd src-tauri && cargo build --bin logbook-cli
```

- [ ] **Step 4: Quick smoke test**

```bash
# Setup test data directory
TEST_DIR=$(mktemp -d)
echo "$TEST_DIR" > "$TEST_DIR/root_path.txt"
mkdir -p "$TEST_DIR/2026/06"
echo '---\nnote: test\nentries: []\n---' > "$TEST_DIR/2026/06/2026-06-15.md"

# Run migration
./target/debug/logbook-cli --root-path "$TEST_DIR" migrate

# Verify
test -f "$TEST_DIR/2026/06/2026-06-15.yaml" && echo "PASS: .yaml created"
test ! -f "$TEST_DIR/2026/06/2026-06-15.md" && echo "PASS: .md removed"
cat "$TEST_DIR/version.txt"

# Cleanup
rm -rf "$TEST_DIR"
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cli/migrate.rs src-tauri/src/cli/mod.rs
git commit -m "feat: add logbook-cli migrate subcommand for .md -> .yaml conversion"
```

---

### Task 8: Verify — full test suite and type check

- [ ] **Step 1: Run all Rust tests**

```bash
cd src-tauri && cargo test 2>&1
```

Expected: all tests PASS (unit + integration).

- [ ] **Step 2: Rust type check**

```bash
cd src-tauri && cargo check
```

Expected: no errors.

- [ ] **Step 3: Frontend type check**

```bash
pnpm vue-tsc --noEmit
```

Expected: no errors (no frontend changes needed; `reveal_day_file` mock in `src/__tests__/mocks/tauri.ts` line 43 just returns `undefined`).

- [ ] **Step 4: Full stop hook**

```bash
pnpm test
```

Expected: vitest + cargo test all PASS.

- [ ] **Step 5: Commit** (if needed — cleanup)

```bash
git add -A && git commit -m "chore: verify all tests pass after .yaml format switch"
```
