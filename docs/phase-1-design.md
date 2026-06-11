# Phase 1 Design — Skeleton

> Date: 2026-06-12 (updated after UX review)
> Parent: [SPEC.md](../SPEC.md)

## Scope

Rust backend with init/setup commands + Today view (day granularity) with Commitments panel, entry CRUD, and day notes.
Working Tauri + Vue scaffold exists, this is the first real implementation phase.

Scope expanded from original spec — delete, edit duration, and day notes brought forward from Phase 2.

## UX Decisions

| # | Decision | Choice |
|---|----------|--------|
| 1 | Layout | Two-column: left 2/3 (input + entry list), right 1/3 (Commitments panel) |
| 2 | Commitments Goals | Always visible, vertical list (one per line), name left + time right, zero-spent dimmed |
| 3 | Commitments times | Human-readable: `2h 15m / 5.7h` (spent / allocation). No balance indicator. |
| 4 | DimensionPanel | Chips showing inherited values + select dropdowns (Phase 1: select; keyboard shortcuts reserved for Phase 4) |
| 5 | Input | Single field, natural language with duration parsing. Live preview of parsed duration below input. |
| 6 | Entry row | Compact: number · item · dimension text labels · duration · × (hover only) |
| 7 | Duration display | Display: human-readable (`1h 30m`). Edit: raw minutes (`90`), supports `+45`/`-30` delta syntax, Enter to confirm, Esc to cancel. |
| 8 | SummaryBar | Bottom of left column: `X entries · Xh Xm` |
| 9 | Delete | Click × → entry disappears → toast "Entry deleted · Undo" (5s auto-dismiss). Undo restores entry. |
| 10 | Day note | Editable inline text below date. Placeholder: "Add a note… (e.g. 五一假期, 春节补班)". Stored in day file frontmatter. |

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| First-run setup | Native folder picker (plugin-dialog) | Minimal friction, stored to local data dir |
| Rust code organization | Multi-module from start | commands / config / files / models split |
| Dimension inheritance | Frontend only (`lastDimensions` ref) | UI behavior, not storage concern |
| Init result handling | Single enum return | One IPC roundtrip, simple frontend switch |
| Config watch lifecycle | Tauri `setup` hook spawns thread | Automatic, no frontend awareness needed |
| Duration parsing | Regex scan all embedded + sum | Supports compound entries like "meeting (15m), chat (45m)" |
| Root path persistence | `root_path.txt` in `app_local_data_dir()` | Simple, standard Tauri pattern |
| serde_yaml → serde_yml | `serde_yaml` 0.9 deprecated，换 `serde_yml` 1.x（drop-in replacement） |
| Frontmatter 解析 | 定位行首 `---`，避免 body 内 `---` 误匹配 |
| Commitments vs Priorities | Commitments replaces Priorities entirely | Overlap resolved — Commitments covers the "remind me" use case |
| Goal dimension | `source: monthly` in config | Values auto-extracted from `_monthly.md` |
| All dimension charts | Unified donut charts | I/U removed from matrix; simplicity > specialization |
| Duration edit | Click-to-edit with `+X`/`-X` delta syntax | Avoids mental math; raw minutes in edit mode, human-readable in display |
| Delete UX | Toast undo (no confirm dialog) | Rare operation; toast avoids interrupting flow |
| Day note | Inline contenteditable below date | Lightweight, stored as `note` field in day file frontmatter |

## Rust Module Structure

```
src-tauri/src/
├── main.rs          // fn main() { logbook_lib::run() }  — unchanged
├── lib.rs           // Builder, setup hook (config + _monthly.md watch), command registration
├── commands.rs      // #[tauri::command] fn init, get_entries, append_entry, update_entry, delete_entry, set_day_note, get_commitments
├── config.rs        // fn validate_config, fn validate_monthly, fn watch_files
├── files.rs         // fn read_day_file, fn write_day_file, fn update_entry_in_file, fn delete_entry_from_file, path constructors
└── models.rs        // All structs, enums, InitResult, ConfigError
```

## Dependencies to Add

```toml
serde_yml = "1"
notify = "6"
chrono = "0.4"
regex = "1"
tauri-plugin-dialog = "2"
uuid = { version = "1", features = ["v4"] }
```

## Init Flow

```
App.vue mounted
  → invoke('init')    // no args; Rust reads root_path.txt from app_local_data
  → Rust returns InitResult:

  InitResult::NeedsSetup
    → SetupScreen.vue (folder picker)
    → user selects folder via @tauri-apps/plugin-dialog
    → invoke('set_root_path', { path })
    → Rust saves path → re-validates config + _monthly.md → emits ready
    → frontend switches to TodayView

  InitResult::ConfigError(errors)
    → ConfigErrorBanner.vue
    → user fixes config.yaml or _monthly.md in editor
    → file watch detects change → emit 'config-changed' event
    → frontend re-renders

  InitResult::Ready { config, today, commitments }
    → TodayView.vue renders (two-column layout)
```

## Command Signatures

```rust
#[tauri::command]
fn init(app: AppHandle) -> InitResult

#[tauri::command]
fn set_root_path(app: AppHandle, path: String) -> Result<InitResult, String>

#[tauri::command]
fn get_entries(root_path: String, date: String) -> DayFile

#[tauri::command]
fn append_entry(root_path: String, date: String, entry: NewEntry) -> Entry

#[tauri::command]
fn update_entry(root_path: String, date: String, entry_id: String, update: UpdateEntry) -> DayFile

#[tauri::command]
fn delete_entry(root_path: String, date: String, entry_id: String) -> DayFile

#[tauri::command]
fn set_day_note(root_path: String, date: String, note: String) -> DayFile

#[tauri::command]
fn get_commitments(root_path: String, year: i32, month: u32) -> Vec<Commitment>
```

`validate_config` and `validate_monthly` are internal — called by `init` and file watch. Frontend gets results via `InitResult::ConfigError` or events.

## Frontend Component Tree

```
App.vue
├── SetupScreen.vue                     // First run folder picker
├── ConfigErrorBanner.vue               // Config validation errors
└── TodayView.vue                       // Two-column layout
    ├── [Left 2/3]
    │   ├── DateNavigator.vue           // ← date → + inline day note
    │   ├── QuickEntry.vue
    │   │   ├── EntryInput.vue          // Single field + live duration preview
    │   │   └── DimensionPanel.vue      // Chips + select dropdowns (toggle expand)
    │   ├── EntryList.vue
    │   │   └── EntryItem.vue           // # · item · dims · duration (hover ×)
    │   └── SummaryBar.vue              // X entries · Xh Xm
    └── [Right 1/3]
        └── CommitmentsPanel.vue        // Role bars + goals (always visible, vertical list)
```

## Component Details

### Duration — Display vs Edit

- **Display**: `formatDuration(minutes)` → `1h 30m`, `45m`, `2h`
- **Edit**: Click duration → input shows raw minutes (`90`). User types absolute (`135`) or delta (`+45`, `-30`). Enter confirms, Esc cancels.
- **Delta parsing**: `+45` → current + 45, `-30` → current - 30. Clamp to ≥ 0.

### CommitmentsPanel.vue

- Each Role: name + spent/allocation + progress bar + goal list
- Spent/allocation in human-readable: `2h 15m / 5.7h`
- Goals: vertical list, name left + time right, zero-spent goals dimmed
- No balance indicator (progress bar is sufficient)

### EntryInput.vue — Duration Parsing

1. Regex scan entire input for all duration patterns
2. Sum all found → live preview as `Duration: 1h 30m (90m)`
3. If no embedded durations found → fallback: trailing duration
4. If still nothing → submit blocked with error hint
5. Remove matched duration fragments from item text, clean up orphaned parentheses/brackets

Examples:
- `"Sprint planning 1.5h"` → item: "Sprint planning", duration: 90
- `"准备会议（15m），和 Alex 面聊（45m）"` → item: "准备会议，和 Alex 面聊", duration: 60
- `"团队周会 1h（含 Q&A 15m）"` → item: "团队周会（含 Q&A）", duration: 75

### DimensionPanel.vue

- Default: chips row showing inherited values (Goal, Category, Biz Line)
- Click chip or "▸ Dimensions" → expands select dropdowns
- Selections inherited from `lastDimensions` ref, updated after each append

### EntryItem.vue

- Display mode: number · item text · dimension labels (compact, inline) · human-readable duration
- × button: hidden by default, appears on row hover (opacity transition)
- **Double-click item text** → input field to edit content (Enter confirm, Esc cancel)
- **Double-click duration** → input field to edit time (see Duration section above)
- Use case: user adds text to an existing entry throughout the day (e.g. appending minor tasks to a catch-all entry)

### Delete Flow

1. User clicks × on an entry row
2. Entry row removed from DOM immediately
3. Toast appears at bottom: "Entry deleted · Undo" (dark background, 5s auto-dismiss)
4. If user clicks Undo → entry restored to original position
5. If toast dismisses → `invoke('delete_entry', { rootPath, date, index })` called on dismiss

### Day Note

- Inline `contenteditable` div below date in DateNavigator
- Shows placeholder "Add a note…" when empty
- On blur: `invoke('set_day_note', { rootPath, date, note })` saves to day file frontmatter
- Stored as `note` field in day file YAML frontmatter

## File Formats

### Day File

```yaml
---
note: "春节补班"
entries:
  - item: Sprint planning
    duration: 90
    dimensions:
      goal: Ship onboarding v2
      business-line: Slax Reader
      category: Meeting
---
```

### Monthly File (unchanged)

```yaml
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
      - Complete performance review cycle
      - 1:1 sessions
---
```

### Config (unchanged)

```yaml
dimensions:
  - name: Goal
    key: goal
    source: monthly
  - name: Business Line
    key: business-line
    source: static
    values: [Slax Reader, ZSXQ, Internal]
  - name: Category
    key: category
    source: static
    values: [Meeting, Deep Work, Communication, Management]
```

## Not in Phase 1

- Week/Month granularity (Phase 2)
- Stats/charts (Phase 3)
- Keyboard shortcuts (Phase 4)
- Animations (Phase 4)
- Input error tolerance improvements (Phase 4)
