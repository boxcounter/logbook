# Logbook Phase 1 (Skeleton) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Working Tauri app with 8 Rust commands, two-column Today view, full entry CRUD (create/read/update/delete with undo), day notes, and Commitments panel with always-visible goal breakdown.

**Architecture:** Rust backend: 6 modules (models, files, config, commands, lib, main), 8 Tauri commands. Vue 3 frontend: reactive store via provide/inject, 12 components in two-column layout. File watcher pushes config/monthly changes to frontend via Tauri events.

**Tech Stack:** Tauri 2.x, Rust (serde, serde_yml, notify 6, chrono 0.4, regex 1, tauri-plugin-dialog 2), Vue 3 + Composition API + TypeScript, Tailwind CSS v4, Vite 6

**Spec:** [SPEC.md](../SPEC.md) | **Design:** [phase-1-design.md](../phase-1-design.md)

---

## File Map

```
src-tauri/
├── Cargo.toml              → MODIFY: add deps
├── capabilities/default.json → MODIFY: add dialog:default
└── src/
    ├── main.rs             → (unchanged)
    ├── lib.rs              → MODIFY: plugins, setup hook, command registration
    ├── models.rs           → CREATE: all structs/enums
    ├── files.rs            → CREATE: path helpers, file I/O, root_path persistence
    ├── config.rs           → CREATE: validation, file watching
    └── commands.rs         → CREATE: 8 Tauri commands + parse_duration

src/
├── main.ts                 → MODIFY: add store provide + CSS import
├── App.vue                 → MODIFY: replace demo with init flow + view routing
├── types.ts                → CREATE: TypeScript types
├── stores/
│   └── useStore.ts         → CREATE: reactive store
├── components/
│   ├── SetupScreen.vue         → CREATE
│   ├── ConfigErrorBanner.vue   → CREATE
│   ├── TodayView.vue           → CREATE
│   ├── DateNavigator.vue       → CREATE
│   ├── CommitmentsPanel.vue    → CREATE
│   ├── QuickEntry.vue          → CREATE
│   ├── EntryInput.vue          → CREATE
│   ├── DimensionPanel.vue      → CREATE
│   ├── EntryList.vue           → CREATE
│   ├── EntryItem.vue           → CREATE
│   └── SummaryBar.vue          → CREATE
├── utils/
│   └── format.ts           → CREATE: formatDuration + parseDuration helpers
└── assets/
    └── main.css            → CREATE: Tailwind directives + base styles

package.json                → MODIFY: add deps via pnpm
vite.config.ts              → MODIFY (or CREATE): add tailwindcss plugin
```

---

### Task 1: Install dependencies and configure tooling

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `package.json` (via pnpm install)
- Create: `src/assets/main.css`
- Modify: `src/main.ts`
- Modify/Create: `vite.config.ts`

- [ ] **Step 1: Add Rust dependencies**

Edit `src-tauri/Cargo.toml`, add to `[dependencies]`:

```toml
serde_yml = "1"
notify = { version = "6", features = ["macos_fsevent"] }
chrono = "0.4"
regex = "1"
tauri-plugin-dialog = "2"
uuid = { version = "1", features = ["v4"] }
```

Run:
```bash
cd src-tauri && cargo check
```
Expected: cargo downloads new crates, compiles cleanly.

- [ ] **Step 2: Add dialog permission**

Edit `src-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:default"
  ]
}
```

- [ ] **Step 3: Install frontend dependencies**

```bash
pnpm add tailwindcss @tailwindcss/vite @tauri-apps/plugin-dialog
```

- [ ] **Step 4: Create vite.config.ts with Tailwind plugin**

Create `vite.config.ts` (overwrite if exists):

```typescript
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
```

- [ ] **Step 5: Create main.css and update main.ts**

Create `src/assets/main.css`:

```css
@import "tailwindcss";

body {
  @apply bg-gray-50 text-gray-900;
}

@media (prefers-color-scheme: dark) {
  body {
    @apply bg-gray-900 text-gray-100;
  }
}
```

Edit `src/main.ts` — add CSS import:

```typescript
import { createApp } from "vue";
import App from "./App.vue";
import "./assets/main.css";

createApp(App).mount("#app");
```

- [ ] **Step 6: Verify full stack compiles**

```bash
pnpm vue-tsc --noEmit
cd src-tauri && cargo check
```
Expected: both pass.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: add dependencies and Tailwind CSS setup"
```

---

### Task 2: Rust data models

**Files:**
- Create: `src-tauri/src/models.rs`

- [ ] **Step 1: Write models.rs**

Create `src-tauri/src/models.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Config ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub dimensions: Vec<Dimension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    pub name: String,
    pub key: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

fn default_source() -> String {
    "static".to_string()
}

// --- Monthly file ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyFile {
    #[serde(default)]
    pub commitments: Vec<Commitment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commitment {
    pub role: String,
    pub allocation: u32, // hours per week
    #[serde(default)]
    pub goals: Vec<String>,
}

// --- Entries ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default)]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String, // UUID v4, generated at creation
    pub item: String,
    pub duration: u32, // minutes
    #[serde(default)]
    pub dimensions: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEntry {
    pub item: String,
    pub duration: String, // pre-parsed total by frontend, e.g. "60"
    #[serde(default)]
    pub dimensions: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<HashMap<String, String>>,
}

// --- Init result ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum InitResult {
    NeedsSetup,
    ConfigError(Vec<ConfigErrorDetail>),
    Ready {
        config: Config,
        today: DayFile,
        commitments: Vec<Commitment>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigErrorDetail {
    pub kind: String,
    pub message: String,
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd src-tauri && cargo check
```
Expected: compiles (models not yet imported elsewhere).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/models.rs
git commit -m "feat: add Rust data models"
```

---

### Task 3: Rust file operations

**Files:**
- Create: `src-tauri/src/files.rs`

- [ ] **Step 1: Write files.rs**

Create `src-tauri/src/files.rs`:

```rust
use crate::models::{DayFile, Entry, MonthlyFile, Config};
use std::fs;
use std::path::{Path, PathBuf};

/// Day file path: {root}/{year}/{month:02}/{date}.md
/// date format: "2026-06-12". Date is canonical from filename, not stored in frontmatter.
pub fn day_path(root: &Path, date: &str) -> PathBuf {
    let parts: Vec<&str> = date.split('-').collect();
    let year = parts.get(0).unwrap_or(&"0000");
    let month = parts.get(1).unwrap_or(&"00");
    root.join(year).join(month).join(format!("{}.md", date))
}

/// Monthly file path: {root}/{year}/{month:02}/_monthly.md
pub fn monthly_path(root: &Path, year: i32, month: u32) -> PathBuf {
    root.join(year.to_string())
        .join(format!("{:02}", month))
        .join("_monthly.md")
}

/// Config path: {root}/config.yaml
pub fn config_path(root: &Path) -> PathBuf {
    root.join("config.yaml")
}

/// Read a day file. Returns empty DayFile if file doesn't exist.
pub fn read_day_file(root: &Path, date: &str) -> Result<DayFile, String> {
    let path = day_path(root, date);
    if !path.exists() {
        return Ok(DayFile { note: None, entries: vec![] });
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    parse_frontmatter::<DayFile>(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Write a full day file (atomic: temp then rename).
pub fn write_day_file(root: &Path, date: &str, day_file: &DayFile) -> Result<(), String> {
    let path = day_path(root, date);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let yaml_body = serde_yml::to_string(day_file)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    let content = format!("---\n{}---\n", yaml_body);
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &content)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}

/// Append an entry to a day file. Creates directories if needed.
pub fn append_to_day_file(root: &Path, date: &str, entry: &Entry) -> Result<Entry, String> {
    let mut day_file = read_day_file(root, date)?;
    let entry = entry.clone();
    day_file.entries.push(entry.clone());
    write_day_file(root, date, &day_file)?;
    Ok(entry)
}

/// Update an entry by ID. Applies only the fields present in `update`.
pub fn update_entry_in_file(root: &Path, date: &str, entry_id: &str, update: &crate::models::UpdateEntry) -> Result<DayFile, String> {
    let mut day_file = read_day_file(root, date)?;
    let pos = day_file.entries.iter().position(|e| e.id == entry_id)
        .ok_or_else(|| format!("Entry {} not found", entry_id))?;
    let entry = &mut day_file.entries[pos];
    if let Some(ref item) = update.item { entry.item = item.clone(); }
    if let Some(ref dur_str) = update.duration {
        entry.duration = crate::commands::parse_duration(dur_str)
            .map_err(|e| format!("Invalid duration: {}", e))?;
    }
    if let Some(ref dims) = update.dimensions { entry.dimensions = dims.clone(); }
    write_day_file(root, date, &day_file)?;
    Ok(day_file)
}

/// Delete an entry by ID from a day file.
pub fn delete_entry_from_file(root: &Path, date: &str, entry_id: &str) -> Result<DayFile, String> {
    let mut day_file = read_day_file(root, date)?;
    let pos = day_file.entries.iter().position(|e| e.id == entry_id)
        .ok_or_else(|| format!("Entry {} not found", entry_id))?;
    day_file.entries.remove(pos);
    write_day_file(root, date, &day_file)?;
    Ok(day_file)
}

/// Set the day note.
pub fn set_day_note_in_file(root: &Path, date: &str, note: &str) -> Result<DayFile, String> {
    let mut day_file = read_day_file(root, date)?;
    day_file.note = if note.is_empty() { None } else { Some(note.to_string()) };
    write_day_file(root, date, &day_file)?;
    Ok(day_file)
}

/// Read monthly file. Returns empty MonthlyFile if not found.
pub fn read_monthly_file(root: &Path, year: i32, month: u32) -> Result<MonthlyFile, String> {
    let path = monthly_path(root, year, month);
    if !path.exists() {
        return Ok(MonthlyFile { commitments: vec![] });
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    parse_frontmatter::<MonthlyFile>(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Read config.yaml. Returns error if file missing.
pub fn read_config(root: &Path) -> Result<Config, String> {
    let path = config_path(root);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_yml::from_str::<Config>(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Root path persistence (atomic write)
pub fn save_root_path(app_data_dir: &Path, root_path: &Path) -> Result<(), String> {
    let path = app_data_dir.join("root_path.txt");
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, root_path.to_string_lossy().as_ref())
        .map_err(|e| format!("Failed to save root path: {}", e))?;
    fs::rename(&tmp, &path)
        .map_err(|e| format!("Failed to rename root path file: {}", e))
}

/// Read persisted root path
pub fn read_root_path(app_data_dir: &Path) -> Option<PathBuf> {
    let path = app_data_dir.join("root_path.txt");
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .map(|s| PathBuf::from(s.trim()))
    } else {
        None
    }
}

/// Parse YAML frontmatter: extract between --- delimiters.
/// Second `---` must appear at line start (or end of file) to avoid
/// matching horizontal rules in Markdown body.
fn parse_frontmatter<T: serde::de::DeserializeOwned>(content: &str) -> Result<T, String> {
    let content = content.trim();
    if !content.starts_with("---") {
        return Err("No frontmatter found".to_string());
    }
    let after_first = &content[3..];
    // Find `\n---` (second delimiter at line start) or end of string
    let end = after_first.find("\n---").unwrap_or_else(|| {
        if after_first.starts_with("---") { 0 } else { after_first.len() }
    });
    let yaml_str = if end == 0 { "" } else { &after_first[..end] }.trim();
    if yaml_str.is_empty() {
        return serde_yml::from_str("{}").map_err(|e| format!("YAML parse error: {}", e));
    }
    serde_yml::from_str(yaml_str).map_err(|e| format!("YAML parse error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Entry;
    use std::collections::HashMap;

    #[test]
    fn test_parse_frontmatter_basic() {
        let input = "---\nentries: []\n---\n";
        let df: DayFile = parse_frontmatter(input).unwrap();
        assert!(df.entries.is_empty());
        assert!(df.note.is_none());
    }

    #[test]
    fn test_parse_frontmatter_with_note() {
        let input = "---\nnote: \"test note\"\nentries: []\n---\n";
        let df: DayFile = parse_frontmatter(input).unwrap();
        assert_eq!(df.note, Some("test note".to_string()));
    }

    #[test]
    fn test_day_path() {
        let root = Path::new("/data");
        let p = day_path(root, "2026-06-12");
        assert_eq!(p, PathBuf::from("/data/2026/06/2026-06-12.md"));
    }

    #[test]
    fn test_monthly_path() {
        let root = Path::new("/data");
        let p = monthly_path(root, 2026, 6);
        assert_eq!(p, PathBuf::from("/data/2026/06/_monthly.md"));
    }

    #[test]
    fn test_config_path() {
        let root = Path::new("/data");
        let p = config_path(root);
        assert_eq!(p, PathBuf::from("/data/config.yaml"));
    }

    #[test]
    fn test_read_empty_day_file() {
        let tmp = std::env::temp_dir().join("logbook_test_empty2");
        let _ = fs::remove_dir_all(&tmp);
        let df = read_day_file(&tmp, "2026-06-12").unwrap();
        assert!(df.entries.is_empty());
        assert!(df.note.is_none());
    }

    #[test]
    fn test_append_and_read_roundtrip() {
        let tmp = std::env::temp_dir().join("logbook_test_rt2");
        let _ = fs::remove_dir_all(&tmp);

        let entry = Entry {
            id: uuid::Uuid::new_v4().to_string(),
            item: "Test".to_string(),
            duration: 30,
            dimensions: HashMap::new(),
        };
        append_to_day_file(&tmp, "2026-06-12", &entry).unwrap();

        let df = read_day_file(&tmp, "2026-06-12").unwrap();
        assert_eq!(df.entries.len(), 1);
        assert_eq!(df.entries[0].item, "Test");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_update_entry() {
        let tmp = std::env::temp_dir().join("logbook_test_update");
        let _ = fs::remove_dir_all(&tmp);

        let e1 = Entry { id: "id-a".into(), item: "A".into(), duration: 10, dimensions: HashMap::new() };
        let e2 = Entry { id: "id-b".into(), item: "B".into(), duration: 20, dimensions: HashMap::new() };
        append_to_day_file(&tmp, "2026-06-12", &e1).unwrap();
        append_to_day_file(&tmp, "2026-06-12", &e2).unwrap();

        let update = crate::models::UpdateEntry { item: Some("B-modified".into()), duration: None, dimensions: None };
        let df = update_entry_in_file(&tmp, "2026-06-12", "id-b", &update).unwrap();
        assert_eq!(df.entries[1].item, "B-modified");
        assert_eq!(df.entries[1].duration, 20); // unchanged

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_delete_entry() {
        let tmp = std::env::temp_dir().join("logbook_test_delete");
        let _ = fs::remove_dir_all(&tmp);

        let e1 = Entry { id: "id-a".into(), item: "A".into(), duration: 10, dimensions: HashMap::new() };
        let e2 = Entry { id: "id-b".into(), item: "B".into(), duration: 20, dimensions: HashMap::new() };
        append_to_day_file(&tmp, "2026-06-12", &e1).unwrap();
        append_to_day_file(&tmp, "2026-06-12", &e2).unwrap();

        let df = delete_entry_from_file(&tmp, "2026-06-12", "id-a").unwrap();
        assert_eq!(df.entries.len(), 1);
        assert_eq!(df.entries[0].item, "B");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_set_day_note() {
        let tmp = std::env::temp_dir().join("logbook_test_note");
        let _ = fs::remove_dir_all(&tmp);

        let df = set_day_note_in_file(&tmp, "2026-06-12", "春节补班").unwrap();
        assert_eq!(df.note, Some("春节补班".to_string()));

        let df2 = read_day_file(&tmp, "2026-06-12").unwrap();
        assert_eq!(df2.note, Some("春节补班".to_string()));

        // Clear note
        let df3 = set_day_note_in_file(&tmp, "2026-06-12", "").unwrap();
        assert_eq!(df3.note, None);

        let _ = fs::remove_dir_all(&tmp);
    }
}
```

- [ ] **Step 2: Add `mod files; mod models;` to lib.rs temporarily**

Edit `src-tauri/src/lib.rs`:

```rust
mod commands;
mod config;
mod files;
mod models;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Also create placeholder `src-tauri/src/commands.rs` and `src-tauri/src/config.rs`:

```rust
// commands.rs — placeholder, filled in Task 5
```

```rust
// config.rs — placeholder, filled in Task 4
```

- [ ] **Step 3: Verify compilation and run tests**

```bash
cd src-tauri && cargo check && cargo test
```
Expected: compiles, 10 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/files.rs src-tauri/src/lib.rs src-tauri/src/commands.rs src-tauri/src/config.rs
git commit -m "feat: add file operations (read/write/update/delete day files, config, monthly, root_path)"
```

---

### Task 4: Rust config validation and file watching

**Files:**
- Modify: `src-tauri/src/config.rs` (replace placeholder)

- [ ] **Step 1: Write config.rs**

Replace `src-tauri/src/config.rs`:

```rust
use crate::models::{Config, ConfigErrorDetail, MonthlyFile};
use crate::files;
use notify::{Event, EventKind, RecursiveMode, Watcher, Config as NotifyConfig};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

fn is_valid_key(key: &str) -> bool {
    key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

pub fn validate_config(config: &Config) -> Vec<ConfigErrorDetail> {
    let mut errors = Vec::new();
    let mut monthly_count = 0;

    for (i, dim) in config.dimensions.iter().enumerate() {
        if dim.name.is_empty() {
            errors.push(ConfigErrorDetail {
                kind: "MissingName".to_string(),
                message: format!("Dimension at index {}: name is required", i),
            });
        }
        if dim.key.is_empty() {
            errors.push(ConfigErrorDetail {
                kind: "MissingKey".to_string(),
                message: format!("Dimension at index {}: key is required", i),
            });
        } else if !is_valid_key(&dim.key) {
            errors.push(ConfigErrorDetail {
                kind: "KeyInvalidChars".to_string(),
                message: format!(
                    "Dimension '{}': key '{}' contains invalid characters (use a-z, 0-9, -, _)",
                    dim.name, dim.key
                ),
            });
        }
        match dim.source.as_str() {
            "static" => {
                match &dim.values {
                    None => errors.push(ConfigErrorDetail {
                        kind: "MissingValues".to_string(),
                        message: format!("Dimension '{}' (key: {}): source is 'static' but values is not set", dim.name, dim.key),
                    }),
                    Some(vals) if vals.is_empty() => errors.push(ConfigErrorDetail {
                        kind: "ValuesEmpty".to_string(),
                        message: format!("Dimension '{}' (key: {}): values list is empty", dim.name, dim.key),
                    }),
                    _ => {}
                }
            }
            "monthly" => {
                monthly_count += 1;
                if monthly_count > 1 {
                    errors.push(ConfigErrorDetail {
                        kind: "MultipleMonthly".to_string(),
                        message: format!("Dimension '{}': only one dimension may have source: monthly", dim.name),
                    });
                }
            }
            other => {
                errors.push(ConfigErrorDetail {
                    kind: "InvalidSource".to_string(),
                    message: format!("Dimension '{}': invalid source '{}' (expected 'static' or 'monthly')", dim.name, other),
                });
            }
        }
    }
    errors
}

pub fn validate_monthly(monthly: &MonthlyFile) -> Vec<ConfigErrorDetail> {
    let mut errors = Vec::new();
    let mut seen_goals = std::collections::HashSet::new();

    for (i, c) in monthly.commitments.iter().enumerate() {
        if c.role.is_empty() {
            errors.push(ConfigErrorDetail {
                kind: "MissingRole".to_string(),
                message: format!("Commitment at index {}: role is required", i),
            });
        }
        if c.allocation == 0 {
            errors.push(ConfigErrorDetail {
                kind: "ZeroAllocation".to_string(),
                message: format!("Commitment '{}': allocation is 0 (should be hours per week)", c.role),
            });
        }
        for goal in &c.goals {
            if !seen_goals.insert(goal.clone()) {
                errors.push(ConfigErrorDetail {
                    kind: "DuplicateGoal".to_string(),
                    message: format!("Goal '{}' appears in multiple commitments (each goal must be unique)", goal),
                });
            }
        }
    }
    errors
}

pub fn watch_files(app_handle: AppHandle, root_path: PathBuf) {
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })
        .expect("Failed to create file watcher");

        watcher
            .configure(NotifyConfig::default())
            .expect("Failed to configure watcher");

        // Watch root directory recursively to catch config.yaml
        // and all _monthly.md files, including across month boundaries.
        watcher.watch(&root_path, RecursiveMode::Recursive).ok();

        for event in rx {
            let is_modify = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_));
            if !is_modify { continue; }

            for path in &event.paths {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "config.yaml" {
                    match files::read_config(&root_path) {
                        Ok(config) => {
                            let errors = validate_config(&config);
                            let _ = app_handle.emit("config-changed", &errors);
                        }
                        Err(e) => {
                            let _ = app_handle.emit("config-changed", &vec![ConfigErrorDetail {
                                kind: "ParseError".to_string(), message: e,
                            }]);
                        }
                    }
                } else if file_name == "_monthly.md" {
                    // Re-read current month each time (handles month boundary)
                    let now = chrono::Local::now();
                    match files::read_monthly_file(&root_path, now.year(), now.month()) {
                        Ok(monthly) => {
                            let errors = validate_monthly(&monthly);
                            let _ = app_handle.emit("commitments-changed", &errors);
                        }
                        Err(e) => {
                            let _ = app_handle.emit("commitments-changed", &vec![ConfigErrorDetail {
                                kind: "ParseError".to_string(), message: e,
                            }]);
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Config, Dimension, MonthlyFile, Commitment};

    #[test]
    fn test_validate_config_valid() {
        let config = Config {
            dimensions: vec![
                Dimension { name: "Biz".into(), key: "biz".into(), source: "static".into(), values: Some(vec!["X".into()]) },
                Dimension { name: "Goal".into(), key: "goal".into(), source: "monthly".into(), values: None },
            ],
        };
        assert!(validate_config(&config).is_empty());
    }

    #[test]
    fn test_validate_config_missing_values() {
        let config = Config {
            dimensions: vec![Dimension {
                name: "Cat".into(), key: "cat".into(), source: "static".into(), values: None,
            }],
        };
        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "MissingValues");
    }

    #[test]
    fn test_validate_config_empty_values() {
        let config = Config {
            dimensions: vec![Dimension {
                name: "Cat".into(), key: "cat".into(), source: "static".into(), values: Some(vec![]),
            }],
        };
        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "ValuesEmpty");
    }

    #[test]
    fn test_validate_config_invalid_key() {
        let config = Config {
            dimensions: vec![Dimension {
                name: "Bad".into(), key: "bad key!".into(), source: "static".into(), values: Some(vec!["x".into()]),
            }],
        };
        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "KeyInvalidChars");
    }

    #[test]
    fn test_validate_config_multiple_monthly() {
        let config = Config {
            dimensions: vec![
                Dimension { name: "G1".into(), key: "g1".into(), source: "monthly".into(), values: None },
                Dimension { name: "G2".into(), key: "g2".into(), source: "monthly".into(), values: None },
            ],
        };
        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "MultipleMonthly");
    }

    #[test]
    fn test_validate_config_invalid_source() {
        let config = Config {
            dimensions: vec![Dimension {
                name: "Bad".into(), key: "bad".into(), source: "dynamic".into(), values: None,
            }],
        };
        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "InvalidSource");
    }

    #[test]
    fn test_validate_monthly_valid() {
        let monthly = MonthlyFile {
            commitments: vec![Commitment { role: "Dev".into(), allocation: 40, goals: vec!["Ship X".into()] }],
        };
        assert!(validate_monthly(&monthly).is_empty());
    }

    #[test]
    fn test_validate_monthly_empty_role() {
        let monthly = MonthlyFile {
            commitments: vec![Commitment { role: "".into(), allocation: 10, goals: vec![] }],
        };
        let errors = validate_monthly(&monthly);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "MissingRole");
    }

    #[test]
    fn test_validate_monthly_zero_allocation() {
        let monthly = MonthlyFile {
            commitments: vec![Commitment { role: "Dev".into(), allocation: 0, goals: vec![] }],
        };
        let errors = validate_monthly(&monthly);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "ZeroAllocation");
    }

    #[test]
    fn test_validate_monthly_duplicate_goal() {
        let monthly = MonthlyFile {
            commitments: vec![
                Commitment { role: "Dev".into(), allocation: 20, goals: vec!["Shared".into()] },
                Commitment { role: "PM".into(), allocation: 10, goals: vec!["Shared".into()] },
            ],
        };
        let errors = validate_monthly(&monthly);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "DuplicateGoal");
    }
}
```

- [ ] **Step 2: Verify compilation and run tests**

```bash
cd src-tauri && cargo check && cargo test
```
Expected: compiles, all 20 tests pass (10 files + 10 config).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "feat: add config validation and file watching"
```

---

### Task 5: Rust Tauri commands

**Files:**
- Modify: `src-tauri/src/commands.rs` (replace placeholder)

- [ ] **Step 1: Write commands.rs**

Replace `src-tauri/src/commands.rs`:

```rust
use crate::config::{validate_config, validate_monthly};
use crate::files::{self, save_root_path, read_root_path};
use crate::models::*;
use regex::Regex;
use tauri::AppHandle;

/// Parse a duration string to minutes.
/// Handles: "90", "1.5h", "30m", "1h 30m", "准备会议（15m），面聊（45m）"
pub fn parse_duration(input: &str) -> Result<u32, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Duration is empty".to_string());
    }

    // Try plain number first
    if let Ok(n) = input.parse::<u32>() {
        if n > 0 { return Ok(n); }
        return Err("Duration must be positive".to_string());
    }

    // Try float (e.g. "1.5")
    if let Ok(n) = input.parse::<f64>() {
        if n > 0.0 { return Ok(n.round() as u32); }
        return Err("Duration must be positive".to_string());
    }

    // Scan for all duration patterns
    let re = Regex::new(r"(\d+(?:\.\d+)?)\s*(h|m|H|M)?").unwrap();
    let mut total: f64 = 0.0;
    let mut matched = false;

    for cap in re.captures_iter(input) {
        let value: f64 = cap[1].parse().unwrap_or(0.0);
        let unit = cap.get(2).map(|m| m.as_str().to_lowercase()).unwrap_or_else(|| "m".to_string());
        match unit.as_str() {
            "h" => { total += value * 60.0; matched = true; }
            "m" | "" => { total += value; matched = true; }
            _ => {}
        }
    }

    if !matched {
        return Err(format!("Could not parse duration from '{}'. Examples: 1.5h, 30m, 45", input));
    }

    let total = total.round() as u32;
    if total == 0 { return Err("Parsed duration is zero".to_string()); }
    Ok(total)
}

#[tauri::command]
pub fn init(app: AppHandle) -> InitResult {
    let app_data_dir = app.path().app_local_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let root_path = match read_root_path(&app_data_dir) {
        Some(p) => p,
        None => return InitResult::NeedsSetup,
    };

    let root = std::path::Path::new(&root_path);

    let config = match files::read_config(root) {
        Ok(c) => c,
        Err(e) => return InitResult::ConfigError(vec![ConfigErrorDetail {
            kind: "ConfigReadError".to_string(), message: e,
        }]),
    };

    let mut all_errors = validate_config(&config);

    let now = chrono::Local::now();
    let monthly = files::read_monthly_file(root, now.year(), now.month()).unwrap_or_else(|_| MonthlyFile { commitments: vec![] });
    all_errors.extend(validate_monthly(&monthly));

    if !all_errors.is_empty() {
        return InitResult::ConfigError(all_errors);
    }

    let today_date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());
    let today = files::read_day_file(root, &today_date).unwrap_or_else(|_| DayFile { note: None, entries: vec![] });

    InitResult::Ready { config, today, commitments: monthly.commitments }
}

#[tauri::command]
pub fn set_root_path(app: AppHandle, path: String) -> Result<InitResult, String> {
    let app_data_dir = app.path().app_local_data_dir().map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let root_path = std::path::Path::new(&path);
    if !root_path.exists() { return Err(format!("Path does not exist: {}", path)); }
    if !root_path.is_dir() { return Err(format!("Path is not a directory: {}", path)); }

    save_root_path(&app_data_dir, root_path)?;

    let config = files::read_config(root_path).map_err(|e| format!("Failed to read config: {}", e))?;
    let mut all_errors = validate_config(&config);

    let now = chrono::Local::now();
    let monthly = files::read_monthly_file(root_path, now.year(), now.month()).unwrap_or_else(|_| MonthlyFile { commitments: vec![] });
    all_errors.extend(validate_monthly(&monthly));

    if !all_errors.is_empty() {
        return Ok(InitResult::ConfigError(all_errors));
    }

    let today_date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());
    let today = files::read_day_file(root_path, &today_date).unwrap_or_else(|_| DayFile { note: None, entries: vec![] });

    Ok(InitResult::Ready { config, today, commitments: monthly.commitments })
}

#[tauri::command]
pub fn get_entries(root_path: String, date: String) -> Result<DayFile, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    files::read_day_file(root, &date)
}

#[tauri::command]
pub fn append_entry(root_path: String, date: String, entry: NewEntry) -> Result<Entry, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let duration = parse_duration(&entry.duration)?;
    let entry = Entry {
        id: uuid::Uuid::new_v4().to_string(),
        item: entry.item,
        duration,
        dimensions: entry.dimensions,
    };
    files::append_to_day_file(root, &date, &entry)
}

#[tauri::command]
pub fn update_entry(root_path: String, date: String, entry_id: String, update: UpdateEntry) -> Result<DayFile, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    // Parse duration if it's being updated
    if let Some(ref dur_str) = update.duration {
        parse_duration(dur_str)?; // validate only; files.rs parses again
    }
    files::update_entry_in_file(root, &date, &entry_id, &update)
}

#[tauri::command]
pub fn delete_entry(root_path: String, date: String, entry_id: String) -> Result<DayFile, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    files::delete_entry_from_file(root, &date, &entry_id)
}

#[tauri::command]
pub fn set_day_note(root_path: String, date: String, note: String) -> Result<DayFile, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    files::set_day_note_in_file(root, &date, &note)
}

#[tauri::command]
pub fn get_commitments(root_path: String, year: i32, month: u32) -> Result<Vec<Commitment>, String> {
    let root = std::path::Path::new(&root_path);
    let monthly = files::read_monthly_file(root, year, month)?;
    Ok(monthly.commitments)
}

fn validate_date_format(date: &str) -> Result<(), String> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}. Expected YYYY-MM-DD", date, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_plain_number() { assert_eq!(parse_duration("90").unwrap(), 90); }

    #[test]
    fn test_parse_duration_float() { assert_eq!(parse_duration("1.5").unwrap(), 2); }

    #[test]
    fn test_parse_duration_hours() { assert_eq!(parse_duration("1.5h").unwrap(), 90); }

    #[test]
    fn test_parse_duration_minutes() { assert_eq!(parse_duration("30m").unwrap(), 30); }

    #[test]
    fn test_parse_duration_compound() { assert_eq!(parse_duration("1h 30m").unwrap(), 90); }

    #[test]
    fn test_parse_duration_embedded_chinese() {
        assert_eq!(parse_duration("准备会议（15m），面聊（45m）").unwrap(), 60);
    }

    #[test]
    fn test_parse_duration_zero() { assert!(parse_duration("0").is_err()); }

    #[test]
    fn test_parse_duration_empty() { assert!(parse_duration("").is_err()); }

    #[test]
    fn test_parse_duration_invalid() { assert!(parse_duration("no duration").is_err()); }

    #[test]
    fn test_validate_date_format_valid() { assert!(validate_date_format("2026-06-12").is_ok()); }

    #[test]
    fn test_validate_date_format_invalid() { assert!(validate_date_format("bad").is_err()); }

    #[test]
    fn test_validate_date_format_month_99() { assert!(validate_date_format("2026-99-12").is_err()); }
}
```

- [ ] **Step 2: Verify compilation and run tests**

```bash
cd src-tauri && cargo check && cargo test
```
Expected: compiles, all 31 tests pass (10 files + 10 config + 11 commands).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add 8 Tauri commands (init, set_root_path, get_entries, append_entry, update_entry, delete_entry, set_day_note, get_commitments)"
```

---

### Task 6: Wire up Rust backend

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Update lib.rs**

Replace `src-tauri/src/lib.rs`:

```rust
mod commands;
mod config;
mod files;
mod models;

use config::watch_files;
use std::path::PathBuf;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let app_data_dir = app.path().app_local_data_dir().unwrap_or_else(|_| PathBuf::from("."));
            if let Some(root_path) = files::read_root_path(&app_data_dir) {
                if root_path.exists() {
                    watch_files(app_handle, root_path);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::init,
            commands::set_root_path,
            commands::get_entries,
            commands::append_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::set_day_note,
            commands::get_commitments,
        ])  // Note: update_entry now takes entry_id + UpdateEntry; delete_entry takes entry_id
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Verify full backend compiles and tests pass**

```bash
cd src-tauri && cargo check && cargo test
```
Expected: all 31 tests pass, all 8 commands registered.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: wire up Rust backend (8 commands, setup hook, file watcher)"
```

---

### Task 7: Frontend TypeScript types and store

**Files:**
- Create: `src/types.ts`
- Create: `src/stores/useStore.ts`

- [ ] **Step 1: Write types.ts**

Create `src/types.ts`:

```typescript
export interface Dimension {
  name: string;
  key: string;
  source: "static" | "monthly";
  values?: string[];
}

export interface Config {
  dimensions: Dimension[];
}

export interface Commitment {
  role: string;
  allocation: number; // hours per week
  goals: string[];
}

export interface Entry {
  id: string; // UUID v4
  item: string;
  duration: number; // minutes
  dimensions: Record<string, string>;
}

export interface DayFile {
  note: string | null;
  entries: Entry[];
}

export interface NewEntry {
  item: string;
  duration: string;
  dimensions: Record<string, string>;
}

export interface UpdateEntry {
  item?: string;
  duration?: string;
  dimensions?: Record<string, string>;
}

export type InitResult =
  | { status: "NeedsSetup" }
  | { status: "ConfigError"; data: ConfigErrorDetail[] }
  | {
      status: "Ready";
      data: { config: Config; today: DayFile; commitments: Commitment[] };
    };

export interface ConfigErrorDetail {
  kind: string;
  message: string;
}

export type Screen = "loading" | "setup" | "error" | "ready";
```

- [ ] **Step 2: Write useStore.ts**

Create `src/stores/useStore.ts`:

```typescript
import { reactive, inject, provide, type InjectionKey } from "vue";
import type { Config, DayFile, Commitment, ConfigErrorDetail, Screen } from "../types";

export interface AppStore {
  screen: Screen;
  rootPath: string;
  config: Config | null;
  configErrors: ConfigErrorDetail[];
  today: DayFile | null;
  commitments: Commitment[];
  lastDimensions: Record<string, string>;
  currentDate: string;
}

const STORE_KEY: InjectionKey<AppStore> = Symbol("AppStore");

export function createStore(): AppStore {
  const now = new Date();
  const dateStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;

  return reactive<AppStore>({
    screen: "loading",
    rootPath: "",
    config: null,
    configErrors: [],
    today: null,
    commitments: [],
    lastDimensions: {},
    currentDate: dateStr,
  });
}

export function provideStore(store: AppStore): void {
  provide(STORE_KEY, store);
}

export function useStore(): AppStore {
  const store = inject(STORE_KEY);
  if (!store) throw new Error("AppStore not provided. Call provideStore() in root component.");
  return store;
}
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
pnpm vue-tsc --noEmit
```
Expected: no type errors.

- [ ] **Step 4: Commit**

```bash
git add src/types.ts src/stores/useStore.ts
git commit -m "feat: add TypeScript types and reactive store"
```

---

### Task 8: SetupScreen and ConfigErrorBanner

**Files:**
- Create: `src/components/SetupScreen.vue`
- Create: `src/components/ConfigErrorBanner.vue`

- [ ] **Step 1: Write SetupScreen.vue**

Create `src/components/SetupScreen.vue`:

```vue
<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useStore } from "../stores/useStore";
import type { InitResult } from "../types";

const store = useStore();

async function selectFolder() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select Logbook data folder",
  });
  if (!selected) return;

  const path = typeof selected === "string" ? selected : selected.path;
  try {
    const result = (await invoke("set_root_path", { path })) as InitResult;
    if (result.status === "Ready") {
      store.rootPath = path;
      store.config = result.data.config;
      store.today = result.data.today;
      store.commitments = result.data.commitments;
      store.screen = "ready";
    } else if (result.status === "ConfigError") {
      store.rootPath = path;
      store.configErrors = result.data;
      store.screen = "error";
    }
  } catch (e) {
    store.configErrors = [{ kind: "SetupError", message: `Failed: ${e}` }];
    store.screen = "error";
  }
}
</script>

<template>
  <div class="flex flex-col items-center justify-center min-h-screen p-8">
    <h1 class="text-2xl font-bold mb-4">Welcome to Logbook</h1>
    <p class="text-gray-600 mb-6 text-center max-w-md">
      Logbook stores work records as Markdown files with YAML frontmatter.
      Choose a folder to store your data.
    </p>
    <button
      class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
      @click="selectFolder"
    >
      Choose Data Folder
    </button>
  </div>
</template>
```

- [ ] **Step 2: Write ConfigErrorBanner.vue**

Create `src/components/ConfigErrorBanner.vue`:

```vue
<script setup lang="ts">
import { useStore } from "../stores/useStore";
const store = useStore();
</script>

<template>
  <div class="bg-red-50 border border-red-200 rounded-lg p-4 mx-4 mt-4">
    <h2 class="text-red-800 font-semibold mb-2">
      Configuration Errors ({{ store.configErrors.length }})
    </h2>
    <p class="text-red-600 text-sm mb-3">
      Fix these errors in your config.yaml or _monthly.md file.
      Changes are detected automatically.
    </p>
    <ul class="list-disc list-inside space-y-1">
      <li v-for="(err, i) in store.configErrors" :key="i" class="text-red-700 text-sm">
        <span class="font-mono text-xs bg-red-100 px-1 rounded">{{ err.kind }}</span>
        {{ err.message }}
      </li>
    </ul>
  </div>
</template>
```

- [ ] **Step 3: Verify TypeScript**

```bash
pnpm vue-tsc --noEmit
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/SetupScreen.vue src/components/ConfigErrorBanner.vue
git commit -m "feat: add SetupScreen and ConfigErrorBanner"
```

---

### Task 9: Utility functions

**Files:**
- Create: `src/utils/format.ts`

- [ ] **Step 1: Write format.ts**

Create `src/utils/format.ts`:

```typescript
/** Format minutes to human-readable: 90 → "1h 30m", 45 → "45m", 120 → "2h" */
export function formatDuration(minutes: number): string {
  if (minutes >= 60) {
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    return m > 0 ? `${h}h ${m}m` : `${h}h`;
  }
  return `${minutes}m`;
}

const DURATION_RE = /(\d+(?:\.\d+)?)\s*(h|m)?/gi;

/** Parse duration from text. Accumulates as float, rounds once at end (matches Rust). */
export function parseDurationFromText(text: string): number | null {
  let total = 0;
  let matched = false;
  const re = new RegExp(DURATION_RE.source, "gi");
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    const value = parseFloat(m[1]);
    const unit = (m[2] || "m").toLowerCase();
    total += unit === "h" ? value * 60 : value;
    matched = true;
  }
  return matched ? Math.round(total) : null;
}

/** Remove duration patterns from text and clean up orphaned brackets/parentheses. */
export function stripDurations(text: string): string {
  let cleaned = text.replace(DURATION_RE, "");
  cleaned = cleaned
    .replace(/[([（]\s*[)\]）]/g, "")
    .replace(/\s*[,;，；]\s*$/, "")
    .replace(/\s+/g, " ")
    .trim();
  return cleaned || text.trim();
}

/** Parse delta input: "+45" → adds to current, "-30" → subtracts, "150" → absolute. */
export function resolveDelta(input: string, currentMinutes: number): number {
  const trimmed = input.trim();
  if (/^[+-]/.test(trimmed)) {
    const delta = parseInt(trimmed.substring(1), 10) || 0;
    const result = trimmed.startsWith("-") ? currentMinutes - delta : currentMinutes + delta;
    return Math.max(0, result);
  }
  const absolute = parseInt(trimmed, 10);
  return isNaN(absolute) ? currentMinutes : Math.max(0, absolute);
}
```

- [ ] **Step 2: Commit**

```bash
git add src/utils/format.ts
git commit -m "feat: add utility functions (formatDuration, parseDurationFromText, stripDurations, resolveDelta)"
```

---

### Task 10: CommitmentsPanel

**Files:**
- Create: `src/components/CommitmentsPanel.vue`

- [ ] **Step 1: Write CommitmentsPanel.vue**

Create `src/components/CommitmentsPanel.vue`:

```vue
<script setup lang="ts">
import { computed } from "vue";
import type { Commitment, Entry } from "../types";
import { formatDuration } from "../utils/format";

const props = defineProps<{
  commitments: Commitment[];
  entries: Entry[];
}>();

interface GoalStat {
  name: string;
  spent: number;
}

interface CommitmentStat {
  role: string;
  allocationMinutes: number;
  spentMinutes: number;
  goals: GoalStat[];
}

const stats = computed<CommitmentStat[]>(() => {
  return props.commitments.map((c) => {
    const dailyAllocation = Math.round((c.allocation * 60) / 7);
    const goals: GoalStat[] = c.goals.map((name) => ({
      name,
      spent: props.entries
        .filter((e) => e.dimensions["goal"] === name)
        .reduce((sum, e) => sum + e.duration, 0),
    }));
    const spentMinutes = goals.reduce((sum, g) => sum + g.spent, 0);
    return { role: c.role, allocationMinutes: dailyAllocation, spentMinutes, goals };
  });
});

function pct(spent: number, alloc: number): string {
  if (alloc === 0) return "0%";
  return Math.min(100, Math.round((spent / alloc) * 100)) + "%";
}

function barColor(spent: number, alloc: number): string {
  if (alloc === 0) return "bg-gray-300";
  const ratio = spent / alloc;
  if (ratio > 1) return "bg-red-500";
  if (ratio > 0.8) return "bg-yellow-500";
  return "bg-green-500";
}
</script>

<template>
  <div v-if="stats.length > 0" class="bg-white rounded-lg shadow-sm p-4">
    <h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-3">Commitments</h3>
    <div v-for="s in stats" :key="s.role" class="mb-4 last:mb-0">
      <div class="flex justify-between items-center text-sm mb-1">
        <span class="font-semibold text-gray-700">{{ s.role }}</span>
        <span class="text-gray-500 text-xs">
          {{ formatDuration(s.spentMinutes) }} / {{ (s.allocationMinutes / 60).toFixed(1) }}h
        </span>
      </div>
      <div class="h-1.5 bg-gray-100 rounded-full overflow-hidden mb-2">
        <div
          :class="barColor(s.spentMinutes, s.allocationMinutes)"
          class="h-full rounded-full transition-all"
          :style="{ width: pct(s.spentMinutes, s.allocationMinutes) }"
        />
      </div>
      <div class="ml-2 flex flex-col gap-0.5 text-xs">
        <div
          v-for="g in s.goals"
          :key="g.name"
          class="flex justify-between"
          :class="g.spent > 0 ? 'text-gray-600' : 'text-gray-300'"
        >
          <span>{{ g.name }}</span>
          <span v-if="g.spent > 0" class="font-medium text-gray-700">{{ formatDuration(g.spent) }}</span>
          <span v-else>0m</span>
        </div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Verify TypeScript**

```bash
pnpm vue-tsc --noEmit
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/CommitmentsPanel.vue
git commit -m "feat: add CommitmentsPanel with always-visible goals"
```

---

### Task 11: EntryInput and DimensionPanel

**Files:**
- Create: `src/components/EntryInput.vue`
- Create: `src/components/DimensionPanel.vue`

- [ ] **Step 1: Write EntryInput.vue**

Create `src/components/EntryInput.vue`:

```vue
<script setup lang="ts">
import { ref, computed } from "vue";
import { parseDurationFromText, stripDurations, formatDuration } from "../utils/format";

const input = ref("");
const error = ref("");

const emit = defineEmits<{
  submit: [item: string, durationMinutes: number];
}>();

const parsedPreview = computed(() => {
  if (!input.value.trim()) return null;
  const d = parseDurationFromText(input.value.trim());
  if (!d) return null;
  return `${formatDuration(d)} (${d}m)`;
});

function handleSubmit() {
  error.value = "";
  const trimmed = input.value.trim();
  if (!trimmed) return;
  const d = parseDurationFromText(trimmed);
  if (!d) {
    error.value = "Could not parse duration. Examples: 1.5h, 30m, 45";
    return;
  }
  const item = stripDurations(trimmed);
  emit("submit", item, d);
  input.value = "";
}
</script>

<template>
  <div>
    <div class="flex gap-2">
      <input
        v-model="input"
        type="text"
        class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-sm"
        placeholder="What did you work on? (e.g. Sprint planning 1.5h)"
        @keydown.enter="handleSubmit"
      />
      <button
        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 text-sm font-medium"
        :disabled="!input.trim()"
        @click="handleSubmit"
      >
        Log
      </button>
    </div>
    <div class="flex justify-between mt-1 min-h-[1.25rem]">
      <span v-if="parsedPreview" class="text-xs text-gray-500">Duration: {{ parsedPreview }}</span>
      <span v-if="error" class="text-xs text-red-500">{{ error }}</span>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Write DimensionPanel.vue**

Create `src/components/DimensionPanel.vue`:

```vue
<script setup lang="ts">
import { computed } from "vue";
import type { Dimension, Commitment } from "../types";

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  values: Record<string, string>;
}>();

const emit = defineEmits<{
  "update:values": [values: Record<string, string>];
}>();

const effectiveDimensions = computed(() => props.dimensions.filter((d) => d.source !== "monthly"));
const monthlyDimension = computed(() => props.dimensions.find((d) => d.source === "monthly"));

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of props.commitments) {
    for (const g of c.goals) goals.add(g);
  }
  return [...goals];
});

function setValue(key: string, value: string) {
  emit("update:values", { ...props.values, [key]: value });
}

// Chips for display
const activeChips = computed(() => {
  const chips: { dim: Dimension; value: string }[] = [];
  for (const dim of props.dimensions) {
    const val = props.values[dim.key];
    if (val) chips.push({ dim, value: val });
  }
  return chips;
});
</script>

<template>
  <div>
    <!-- Chips row -->
    <div class="flex flex-wrap gap-1.5 mb-2">
      <span
        v-for="chip in activeChips"
        :key="chip.dim.key"
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs cursor-pointer border"
        :class="{
          'bg-blue-50 border-blue-200 text-blue-800': chip.dim.source === 'monthly',
          'bg-green-50 border-green-200 text-green-800': chip.dim.key === 'category',
          'bg-amber-50 border-amber-200 text-amber-800': chip.dim.key === 'business-line' || chip.dim.source === 'static',
        }"
      >
        {{ chip.value }}
      </span>
    </div>
    <!-- Selects (toggleable) -->
    <div class="flex flex-col gap-2">
      <div v-for="dim in effectiveDimensions" :key="dim.key" class="flex items-center gap-2">
        <label class="text-xs text-gray-500 w-16 shrink-0">{{ dim.name }}</label>
        <select
          :value="values[dim.key] || ''"
          class="flex-1 px-2 py-1 text-xs border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
          @change="setValue(dim.key, ($event.target as HTMLSelectElement).value)"
        >
          <option value="">--</option>
          <option v-for="v in dim.values" :key="v" :value="v">{{ v }}</option>
        </select>
      </div>
      <div v-if="monthlyDimension" class="flex items-center gap-2">
        <label class="text-xs text-gray-500 w-16 shrink-0">{{ monthlyDimension.name }}</label>
        <select
          :value="values[monthlyDimension.key] || ''"
          class="flex-1 px-2 py-1 text-xs border border-gray-300 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
          @change="setValue(monthlyDimension.key, ($event.target as HTMLSelectElement).value)"
        >
          <option value="">--</option>
          <option v-for="g in goalOptions" :key="g" :value="g">{{ g }}</option>
        </select>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 3: Verify TypeScript**

```bash
pnpm vue-tsc --noEmit
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/EntryInput.vue src/components/DimensionPanel.vue
git commit -m "feat: add EntryInput (duration parsing) and DimensionPanel (chips + selects)"
```

---

### Task 12: QuickEntry

**Files:**
- Create: `src/components/QuickEntry.vue`

- [ ] **Step 1: Write QuickEntry.vue**

Create `src/components/QuickEntry.vue`:

```vue
<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import EntryInput from "./EntryInput.vue";
import DimensionPanel from "./DimensionPanel.vue";
import type { Dimension, DayFile } from "../types";

const store = useStore();
const showDimensions = ref(false);
const dimValues = ref<Record<string, string>>({});

watch(
  () => store.lastDimensions,
  (ld) => { if (Object.keys(ld).length > 0) dimValues.value = { ...ld }; },
  { immediate: true }
);

function sanitizeValues(vals: Record<string, string>, dims: Dimension[]): Record<string, string> {
  const validKeys = new Set(dims.map((d) => d.key));
  const cleaned: Record<string, string> = {};
  for (const [k, v] of Object.entries(vals)) {
    if (validKeys.has(k)) cleaned[k] = v;
  }
  return cleaned;
}

async function handleSubmit(item: string, durationMinutes: number) {
  const dimensions = sanitizeValues(dimValues.value, store.config?.dimensions || []);
  const newEntry = { item, duration: String(durationMinutes), dimensions };

  try {
    await invoke("append_entry", { rootPath: store.rootPath, date: store.currentDate, entry: newEntry });
    store.lastDimensions = { ...dimensions };
    await refreshDay();
    dimValues.value = {};
  } catch (e) {
    console.error("append_entry failed:", e);
  }
}

async function refreshDay() {
  const dayFile = (await invoke("get_entries", { rootPath: store.rootPath, date: store.currentDate })) as DayFile;
  store.today = dayFile;
  // Commitments only change via _monthly.md edits (file watcher handles refresh).
  // CommitmentsPanel recomputes stats from in-memory commitments + updated entries.
}
</script>

<template>
  <div class="bg-white rounded-lg shadow-sm p-4 space-y-3">
    <EntryInput @submit="handleSubmit" />
    <button class="text-xs text-blue-600 hover:text-blue-800" @click="showDimensions = !showDimensions">
      {{ showDimensions ? "▾ Hide" : "▸ Show" }} Dimensions
    </button>
    <DimensionPanel
      v-if="showDimensions"
      :dimensions="store.config?.dimensions || []"
      :commitments="store.commitments"
      :values="dimValues"
      @update:values="(v) => (dimValues = v)"
    />
  </div>
</template>
```

- [ ] **Step 2: Verify TypeScript**

```bash
pnpm vue-tsc --noEmit
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/QuickEntry.vue
git commit -m "feat: add QuickEntry with dimension inheritance"
```

---

### Task 13: EntryItem and EntryList

**Files:**
- Create: `src/components/EntryItem.vue`
- Create: `src/components/EntryList.vue`

- [ ] **Step 1: Write EntryItem.vue**

Create `src/components/EntryItem.vue`:

```vue
<script setup lang="ts">
import { ref } from "vue";
import type { Entry } from "../types";
import { formatDuration, resolveDelta } from "../utils/format";

const props = defineProps<{
  entry: Entry;
}>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
}>();

// Item text editing
const editingItem = ref(false);
const itemInput = ref("");

function startEditItem() {
  itemInput.value = props.entry.item;
  editingItem.value = true;
}

function commitItem() {
  editingItem.value = false;
  const newItem = itemInput.value.trim() || "(untitled)";
  if (newItem !== props.entry.item) {
    emit("update", props.entry.id, newItem, props.entry.duration);
  }
}

function cancelItem() {
  editingItem.value = false;
}

function handleItemKey(e: KeyboardEvent) {
  if (e.key === "Enter") { e.preventDefault(); commitItem(); }
  if (e.key === "Escape") { e.preventDefault(); cancelItem(); }
}

// Duration editing
const editingDuration = ref(false);
const durInput = ref("");

function startEditDuration() {
  durInput.value = String(props.entry.duration);
  editingDuration.value = true;
}

function commitDuration() {
  editingDuration.value = false;
  const newDur = resolveDelta(durInput.value, props.entry.duration);
  if (newDur !== props.entry.duration) {
    emit("update", props.entry.id, props.entry.item, newDur);
  }
}

function cancelDuration() {
  editingDuration.value = false;
}

function handleDurKey(e: KeyboardEvent) {
  if (e.key === "Enter") { e.preventDefault(); commitDuration(); }
  if (e.key === "Escape") { e.preventDefault(); cancelDuration(); }
}

function dimLabel(dims: Record<string, string>): string {
  return Object.entries(dims)
    .filter(([, v]) => v)
    .map(([, v]) => v)
    .join(" · ");
}
</script>

<template>
  <div class="flex items-start gap-3 py-3 border-b border-gray-100 last:border-b-0 group">
    <span class="text-xs text-gray-400 w-5 text-right pt-0.5 tabular-nums shrink-0">{{ index + 1 }}</span>
    <div class="flex-1 min-w-0">
      <!-- Item text: double-click to edit -->
      <div v-if="editingItem" class="text-sm">
        <input
          ref="itemInputRef"
          v-model="itemInput"
          class="w-full px-1 py-0.5 border-2 border-blue-500 rounded text-sm outline-none"
          @keydown="handleItemKey"
          @blur="commitItem"
        />
      </div>
      <div
        v-else
        class="text-sm text-gray-800 cursor-default rounded px-0.5 -mx-0.5 hover:bg-gray-50"
        @dblclick="startEditItem"
      >
        {{ entry.item }}
      </div>
      <div v-if="dimLabel(entry.dimensions)" class="text-xs text-gray-400 mt-0.5">
        {{ dimLabel(entry.dimensions) }}
      </div>
    </div>
    <!-- Duration: double-click to edit -->
    <span v-if="editingDuration" class="text-sm shrink-0">
      <input
        ref="durInputRef"
        v-model="durInput"
        class="w-14 text-right px-1 py-0.5 border-2 border-blue-500 rounded text-sm outline-none tabular-nums"
        @keydown="handleDurKey"
        @blur="commitDuration"
      />
    </span>
    <span
      v-else
      class="text-sm text-gray-600 tabular-nums shrink-0 cursor-default rounded px-1 hover:bg-gray-50"
      @dblclick="startEditDuration"
    >
      {{ formatDuration(entry.duration) }}
    </span>
    <!-- Delete: hover only -->
    <button
      class="text-gray-400 hover:text-red-500 text-base leading-none shrink-0 opacity-0 group-hover:opacity-100 transition-opacity p-0.5"
      title="Delete"
      @click="emit('delete', props.entry.id)"
    >
      ×
    </button>
  </div>
</template>
```

- [ ] **Step 2: Write EntryList.vue**

Create `src/components/EntryList.vue`:

```vue
<script setup lang="ts">
import type { Entry } from "../types";
import EntryItem from "./EntryItem.vue";

defineProps<{ entries: Entry[] }>();
const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
}>();
</script>

<template>
  <div class="bg-white rounded-lg shadow-sm">
    <div v-if="entries.length === 0" class="p-8 text-center text-gray-400 text-sm">
      No entries yet. Log your first work item above.
    </div>
    <div v-else class="px-4">
      <EntryItem
        v-for="entry in entries"
        :key="entry.id"
        :entry="entry"
        @update="(entryId, item, dur) => emit('update', entryId, item, dur)"
        @delete="(entryId) => emit('delete', entryId)"
      />
    </div>
  </div>
</template>
```

- [ ] **Step 3: Verify TypeScript**

```bash
pnpm vue-tsc --noEmit
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/EntryItem.vue src/components/EntryList.vue
git commit -m "feat: add EntryItem (double-click edit, hover ×) and EntryList"
```

---

### Task 14: DateNavigator and SummaryBar

**Files:**
- Create: `src/components/DateNavigator.vue`
- Create: `src/components/SummaryBar.vue`

- [ ] **Step 1: Write DateNavigator.vue**

Create `src/components/DateNavigator.vue`:

```vue
<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import type { DayFile } from "../types";

const store = useStore();
const noteRef = ref<HTMLDivElement>();

// Sync note from store → DOM (not via template interpolation to avoid VDOM conflict)
watch(
  () => store.today?.note,
  (n) => {
    if (noteRef.value && noteRef.value.textContent !== (n || "")) {
      noteRef.value.textContent = n || "";
    }
  },
  { immediate: true }
);

function dateObj(): Date {
  return new Date(store.currentDate + "T00:00:00");
}

const displayDate = computed(() => {
  const d = dateObj();
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const target = new Date(d);
  target.setHours(0, 0, 0, 0);
  const diff = Math.round((target.getTime() - today.getTime()) / 86400000);

  const s = d.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" });
  if (diff === 0) return `Today — ${s}`;
  if (diff === -1) return `Yesterday — ${s}`;
  if (diff === 1) return `Tomorrow — ${s}`;
  return s;
});

function shiftDate(days: number) {
  const d = dateObj();
  d.setDate(d.getDate() + days);
  store.currentDate = [
    d.getFullYear(),
    String(d.getMonth() + 1).padStart(2, "0"),
    String(d.getDate()).padStart(2, "0"),
  ].join("-");
  loadDay();
}

async function loadDay() {
  try {
    const df = (await invoke("get_entries", { rootPath: store.rootPath, date: store.currentDate })) as DayFile;
    store.today = df;
  } catch (e) {
    console.error("get_entries failed:", e);
  }
}

async function saveNote() {
  const text = noteRef.value?.textContent || "";
  try {
    await invoke("set_day_note", { rootPath: store.rootPath, date: store.currentDate, note: text });
  } catch (e) {
    console.error("set_day_note failed:", e);
  }
}
</script>

<template>
  <div class="flex items-center justify-between">
    <button class="px-2 py-1 text-gray-500 hover:bg-gray-100 rounded transition-colors text-sm" @click="shiftDate(-1)">←</button>
    <div class="text-center">
      <div class="text-sm font-semibold text-gray-700">{{ displayDate }}</div>
      <div
        ref="noteRef"
        class="text-xs text-gray-500 font-normal mt-0.5 outline-none rounded px-1.5 -mx-1.5 hover:bg-gray-100 focus:bg-white focus:ring-2 focus:ring-blue-500 cursor-text min-w-[60px]"
        contenteditable="true"
        data-placeholder="Add a note…"
        @blur="saveNote"
      ></div>
    </div>
    <button class="px-2 py-1 text-gray-500 hover:bg-gray-100 rounded transition-colors text-sm" @click="shiftDate(1)">→</button>
  </div>
</template>

<style scoped>
[contenteditable]:empty::before {
  content: attr(data-placeholder);
  color: #cbd5e1;
}
</style>
```

- [ ] **Step 2: Write SummaryBar.vue**

Create `src/components/SummaryBar.vue`:

```vue
<script setup lang="ts">
import { computed } from "vue";
import type { Entry } from "../types";
import { formatDuration } from "../utils/format";

const props = defineProps<{ entries: Entry[] }>();

const totalMinutes = computed(() => props.entries.reduce((s, e) => s + e.duration, 0));
const entryCount = computed(() => props.entries.length);
</script>

<template>
  <div v-if="entryCount > 0" class="flex justify-between text-xs text-gray-500 px-1">
    <span>{{ entryCount }} {{ entryCount === 1 ? "entry" : "entries" }}</span>
    <span class="font-medium text-gray-700">{{ formatDuration(totalMinutes) }}</span>
  </div>
</template>
```

- [ ] **Step 3: Verify TypeScript**

```bash
pnpm vue-tsc --noEmit
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/DateNavigator.vue src/components/SummaryBar.vue
git commit -m "feat: add DateNavigator (with day note) and SummaryBar"
```

---

### Task 15: TodayView

**Files:**
- Create: `src/components/TodayView.vue`

- [ ] **Step 1: Write TodayView.vue (two-column layout)**

Create `src/components/TodayView.vue`:

```vue
<script setup lang="ts">
import { inject } from "vue";
import { useStore } from "../stores/useStore";
import { invoke } from "@tauri-apps/api/core";
import DateNavigator from "./DateNavigator.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import QuickEntry from "./QuickEntry.vue";
import EntryList from "./EntryList.vue";
import SummaryBar from "./SummaryBar.vue";
import type { DayFile, Entry } from "../types";

const store = useStore();

// Inject undo toast trigger from App.vue
const triggerUndoToast = inject<(undoFn: () => void) => void>("triggerUndoToast", () => {});

async function handleUpdateEntry(entryId: string, item: string, durationMinutes: number) {
  const entry = store.today?.entries.find(e => e.id === entryId);
  if (!entry) return;

  // Only send changed fields
  const update: Record<string, unknown> = {};
  if (item !== entry.item) update.item = item;
  if (durationMinutes !== entry.duration) update.duration = String(durationMinutes);

  if (Object.keys(update).length === 0) return;

  try {
    const df = (await invoke("update_entry", {
      rootPath: store.rootPath,
      date: store.currentDate,
      entryId,
      update,
    })) as DayFile;
    store.today = df;
  } catch (e) {
    console.error("update_entry failed:", e);
  }
}

let pendingDeleteTimer: ReturnType<typeof setTimeout> | null = null;

async function handleDeleteEntry(entryId: string) {
  const entries = store.today?.entries;
  if (!entries) return;

  // Optimistic: remove from UI immediately
  const idx = entries.findIndex(e => e.id === entryId);
  if (idx === -1) return;
  const [removed] = entries.splice(idx, 1);

  // Schedule persistence
  let cancelled = false;
  pendingDeleteTimer = setTimeout(async () => {
    if (cancelled) return;
    try {
      await invoke("delete_entry", { rootPath: store.rootPath, date: store.currentDate, entryId });
    } catch (e) {
      console.error("delete_entry failed:", e);
      entries.splice(idx, 0, removed); // restore on failure
    }
  }, 5000);

  // Show undo toast
  triggerUndoToast(() => {
    cancelled = true;
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
    entries.splice(idx, 0, removed);
  });
}
</script>

<template>
  <div class="flex gap-4 p-4 max-w-4xl mx-auto items-start">
    <!-- Left 2/3 -->
    <div class="flex-[2] min-w-0 flex flex-col gap-3">
      <DateNavigator />
      <QuickEntry />
      <EntryList
        :entries="store.today?.entries || []"
        @update="(entryId, item, dur) => handleUpdateEntry(entryId, item, dur)"
        @delete="(entryId) => handleDeleteEntry(entryId)"
      />
      <SummaryBar :entries="store.today?.entries || []" />
    </div>
    <!-- Right 1/3 -->
    <div class="flex-1 min-w-[180px] flex flex-col gap-3 sticky top-4">
      <CommitmentsPanel
        :commitments="store.commitments"
        :entries="store.today?.entries || []"
      />
    </div>
  </div>
</template>
```

- [ ] **Step 2: Verify TypeScript**

```bash
pnpm vue-tsc --noEmit
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/TodayView.vue
git commit -m "feat: add TodayView (two-column layout, entry CRUD, commitments)"
```

---

### Task 16: App.vue integration with undo toast and main.ts update

**Files:**
- Modify: `src/App.vue`
- Modify: `src/main.ts`

- [ ] **Step 1: Update main.ts**

Replace `src/main.ts`:

```typescript
import { createApp } from "vue";
import App from "./App.vue";
import "./assets/main.css";
import { createStore, provideStore } from "./stores/useStore";

const app = createApp(App);
const store = createStore();
provideStore(store);
app.mount("#app");
```

- [ ] **Step 2: Update App.vue**

Replace `src/App.vue`:

```vue
<script setup lang="ts">
import { onMounted, ref, provide } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "./stores/useStore";
import SetupScreen from "./components/SetupScreen.vue";
import ConfigErrorBanner from "./components/ConfigErrorBanner.vue";
import TodayView from "./components/TodayView.vue";
import type { InitResult, ConfigErrorDetail } from "./types";

const store = useStore();
const showUndoToast = ref(false);
const undoAction = ref<(() => void) | null>(null);
let undoTimer: ReturnType<typeof setTimeout> | null = null;

onMounted(async () => {
  await listen<ConfigErrorDetail[]>("config-changed", (event) => {
    if (event.payload.length === 0) {
      initApp();
    } else {
      store.configErrors = event.payload;
      store.screen = "error";
    }
  });

  await listen<ConfigErrorDetail[]>("commitments-changed", () => {
    initApp();
  });

  initApp();
});

async function initApp() {
  try {
    const result = (await invoke("init")) as InitResult;
    switch (result.status) {
      case "NeedsSetup":
        store.screen = "setup";
        break;
      case "ConfigError":
        store.configErrors = result.data;
        store.screen = "error";
        break;
      case "Ready":
        store.config = result.data.config;
        store.today = result.data.today;
        store.commitments = result.data.commitments;
        store.screen = "ready";
        break;
    }
  } catch (e) {
    store.configErrors = [{ kind: "InitError", message: `Failed: ${e}` }];
    store.screen = "error";
  }
}

// Undo toast for delete
function triggerUndoToast(undoFn: () => void) {
  if (undoTimer) clearTimeout(undoTimer);
  undoAction.value = undoFn;
  showUndoToast.value = true;
  undoTimer = setTimeout(() => {
    showUndoToast.value = false;
    undoAction.value = null;
  }, 5000);
}

function dismissUndo() {
  if (undoTimer) clearTimeout(undoTimer);
  showUndoToast.value = false;
  undoAction.value = null;
}

function handleUndo() {
  if (undoAction.value) undoAction.value();
  dismissUndo();
}

// Provide undo trigger to descendants
provide("triggerUndoToast", triggerUndoToast);
</script>

<template>
  <div class="min-h-screen">
    <div v-if="store.screen === 'loading'" class="flex items-center justify-center min-h-screen text-gray-500">
      Loading…
    </div>
    <SetupScreen v-else-if="store.screen === 'setup'" />
    <template v-else-if="store.screen === 'error'">
      <ConfigErrorBanner />
      <button
        class="mx-4 mt-4 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm"
        @click="initApp"
      >
        Retry
      </button>
    </template>
    <TodayView v-else-if="store.screen === 'ready'" />

    <!-- Undo Toast -->
    <Teleport to="body">
      <transition name="toast">
        <div
          v-if="showUndoToast"
          class="fixed bottom-6 left-1/2 -translate-x-1/2 flex items-center gap-4 bg-gray-900 text-white px-5 py-3 rounded-lg shadow-lg z-50 text-sm"
        >
          <span>Entry deleted</span>
          <button class="text-blue-400 font-medium hover:text-blue-300" @click="handleUndo">Undo</button>
          <button class="text-gray-500 hover:text-gray-300 text-base leading-none" @click="dismissUndo">×</button>
        </div>
      </transition>
    </Teleport>
  </div>
</template>

<style>
.toast-enter-active { transition: all 0.2s ease-out; }
.toast-leave-active { transition: all 0.2s ease-in; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translate(-50%, 1rem); }
</style>
```

Note: the undo toast is wired in App.vue. The TodayView's `handleDeleteEntry` should use the undo pattern — remove entry from UI immediately, show toast, and only call `delete_entry` when the toast dismisses (or restore if Undo is clicked). For Phase 1 simplicity, the toast wiring is in App.vue; TodayView should inject `triggerUndoToast` and implement the optimistic delete flow.

- [ ] **Step 3: Verify full app compiles**

```bash
pnpm vue-tsc --noEmit
cd src-tauri && cargo check
```
Expected: both pass.

- [ ] **Step 4: Commit**

```bash
git add src/App.vue src/main.ts
git commit -m "feat: integrate App.vue with init flow, view routing, and undo toast"
```

---

### Task 17: End-to-end smoke test

**Files:** None (manual verification)

- [ ] **Step 1: Create test fixture**

```bash
mkdir -p /tmp/logbook-test/2026/06
```

Create `/tmp/logbook-test/config.yaml`:

```yaml
dimensions:
  - name: "Importance-Urgency"
    key: "importance-urgency"
    source: static
    values: ["Important+Urgent", "Important+Not Urgent", "Not Important+Urgent", "Not Important+Not Urgent"]
  - name: "Business Line"
    key: "business-line"
    source: static
    values: ["Slax Reader", "ZSXQ", "Internal"]
  - name: "Category"
    key: "category"
    source: static
    values: ["Meeting", "Deep Work", "Communication", "Management"]
  - name: "Goal"
    key: "goal"
    source: monthly
```

Create `/tmp/logbook-test/2026/06/_monthly.md`:

```markdown
---
commitments:
  - role: Developer
    allocation: 40
    goals:
      - Ship onboarding v2
      - Review auth PR
  - role: Director
    allocation: 20
    goals:
      - Performance reviews
      - 1:1 sessions
---
```

- [ ] **Step 2: Run the app**

```bash
pnpm tauri dev
```

Test checklist:
1. App opens → SetupScreen → click "Choose Data Folder" → select `/tmp/logbook-test`
2. TodayView appears (two-column: left input+list, right Commitments)
3. CommitmentsPanel shows 2 roles with 0 spent, goals vertical list
4. Type `Sprint planning 1.5h` → press Enter → entry appears in list (duration stripped from item)
5. Double-click duration → input shows `90` → type `+30` → Enter → shows `2h`
6. Double-click item text → edit → Enter → text updates
7. Hover entry row → × button appears → click × → entry disappears → toast appears
8. Click Undo → entry restored. Or wait 5s → toast disappears.
9. Click "Show Dimensions" → select Goal: "Ship onboarding v2" → log another entry
10. CommitmentsPanel shows time allocated to "Ship onboarding v2"
11. Click ← / → to navigate dates
12. Click date note area → type "测试备注" → click away → refresh or navigate away and back → note persists

Press Ctrl+C to stop.

- [ ] **Step 3: Verify all Rust tests**

```bash
cd src-tauri && cargo test
```
Expected: all 31 tests pass.

---

## Plan Completion Checklist

- [ ] `pnpm vue-tsc --noEmit` passes
- [ ] `cd src-tauri && cargo test` passes (31 tests)
- [ ] `pnpm tauri dev` launches
- [ ] First-run setup flow works
- [ ] Config error flow works
- [ ] Entry CRUD: create, read, update (text + duration), delete (with undo)
- [ ] Day note save/load
- [ ] Commitments panel renders with goal breakdown
- [ ] Double-click editing for both item text and duration
- [ ] Duration +/- delta syntax works
- [ ] Hover × delete button
- [ ] Two-column layout renders correctly
