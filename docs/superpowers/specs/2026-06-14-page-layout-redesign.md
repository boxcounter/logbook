# Page Layout Redesign — Design Spec

> 2026-06-14 · Status: designed

## Summary

Redesign the main window layout from a day/week/month granularity model to a fixed month-centric model. The window remains a two-column layout: left column shows month overview (month navigator + commitments), right column shows day-level detail within the selected month (date strip, day note, quick entry, entry list).

## Motivation

- Current three-granularity model (day/week/month) adds complexity for week/month views that see little use
- Month is the natural organizing unit — commitments are monthly, files are organized by month
- Commitments visibility is secondary in the current layout (right sidebar); moving it to the left column gives it a dedicated, stable position
- De-emphasizing week simplifies the mental model and reduces UI surface area

## Component Architecture

```
App.vue
├── SetupScreen.vue                (unchanged)
├── ConfigErrorBanner.vue          (unchanged)
└── MonthView.vue                  (NEW, replaces TodayView.vue)
    ├── MonthSidebar.vue           (NEW, left 1/3 container, sticky)
    │   ├── MonthNavigator.vue     (NEW)
    │   └── CommitmentsPanel.vue   (reused, unchanged)
    └── DayDetail.vue              (NEW, right 2/3 container)
        ├── DayStrip.vue           (NEW)
        ├── DayNote                (inline, extracted from DateNavigator)
        ├── QuickEntry.vue         (reused)
        ├── EntryList.vue          (reused)
        └── file path link         (inline)
```

### Components to Remove

| Component | Reason |
|-----------|--------|
| TodayView.vue | Replaced by MonthView.vue |
| DateNavigator.vue | Split into MonthNavigator + DayStrip + DayNote |
| SummaryBar.vue | Replaced by inline summary row in EntryList |

## Layout

```
┌─────────────────┬──────────────────────────────────────────┐
│  Left (1/3)     │  Right (2/3)                             │
│                 │                                          │
│ ┌─────────────┐ │ ┌──────────────────────────────────────┐ │
│ │ ← Jun 2026 →│ │ │ DayStrip: 1  2  3 ... 30  (horiz    │ │
│ │   [▼ pick]  │ │ │ scroll, dots for days w/ entries,    │ │
│ └─────────────┘ │ │ 7-day separators, future dates grey)  │ │
│                 │ └──────────────────────────────────────┘ │
│ ┌─────────────┐ │ ┌──────────────────────────────────────┐ │
│ │ Commitments │ │ │ DayNote (contenteditable)            │ │
│ │             │ │ └──────────────────────────────────────┘ │
│ │ Developer   │ │ ┌──────────────────────────────────────┐ │
│ │ 12.5h/20h   │ │ │ QuickEntry (only when selected date  │ │
│ │ ██████░░░░  │ │ │ is today)                            │ │
│ │             │ │ └──────────────────────────────────────┘ │
│ │ Designer    │ │ ┌──────────────────────────────────────┐ │
│ │ 3.2h/10h    │ │ │ EntryList                            │ │
│ │ ███░░░░░░░  │ │ │  ├ Entry 1                          │ │
│ │             │ │ │  ├ Entry 2                          │ │
│ │ PM          │ │ │  └ Entry 3                          │ │
│ │ 8.0h/15h    │ │ │  ─────────────────────              │ │
│ │ █████░░░░░  │ │ │  3 entries / 7.0h  (inline summary) │ │
│ └─────────────┘ │ └──────────────────────────────────────┘ │
│                 │ …/2026/06/2026-06-14.md                 │
└─────────────────┴──────────────────────────────────────────┘
```

## Component Details

### MonthNavigator

- Displays `← <Month> <Year> →` with left/right arrow buttons for sequential navigation
- Clicking the month-year text opens a quick-jump popover with two `<select>` dropdowns:
  - Year select: only years that have data on disk
  - Month select: only months that have data on disk
- Changing either select navigates to that month immediately
- Clicking outside the popover dismisses it

### DayStrip

- Horizontal row of 31 date cells (or fewer for short months), scrollable
- Visual rules:
  - Each cell: date number, clickable
  - Selected date: blue background, white text
  - Today: subtle indicator (bold or underline) distinct from selection
  - Future dates (day > today within current month): grey text, not clickable
  - Days with entries: small blue dot below the number
  - Days without entries: no dot
  - Every 7th day: right border is a thicker separator line (visual week grouping)
- Default scroll position: scroll to make today (current month) or last day (past month) visible
- Clicking a date updates `store.currentDate` and loads that day's entries from already-loaded `monthEntries`

### DayNote

- Extracted from current DateNavigator contenteditable div
- Behavior unchanged: blur triggers `set_day_note` invoke
- Positioned between DayStrip and QuickEntry
- Placeholder text: "Add a note…"

### EntryList (modified)

- Reused with one change: at the bottom of the list, an inline summary row displays: `<N> entries / <X>h`
- No longer receives SummaryBar as a sibling — the summary is part of EntryList's own template

### QuickEntry, CommitmentsPanel, EntryItem, EntryGroup, DimensionPanel, EntryInput

- Unchanged. Reused as-is.

## Store Changes

```typescript
// REMOVED
granularity: Granularity    // no longer needed
periodEntries: Record<string, Entry[]>  // replaced by monthEntries

// ADDED
monthEntries: Record<string, Entry[]>   // entries for all days in current month
```

- `currentDate` remains — always stores the currently selected date (YYYY-MM-DD)
- `currentYear` and `currentMonth` are derived from `currentDate` via computed

## Data Flow

### Month Load (triggered by MonthNavigator)

1. Set `currentDate` based on target month: today if navigating to current month; last day of month if navigating to a past month
2. `invoke("get_entries")` for each day in the month → populate `store.monthEntries`
3. `invoke("get_commitment_progress")` for the month → populate `store.commitmentProgress`
4. Set `store.today` from `monthEntries[currentDate]`

### Day Selection (triggered by DayStrip click)

1. Update `store.currentDate`
2. Set `store.today` from `store.monthEntries[currentDate]` (no network call)

### Entry Mutations (append/update/delete)

1. Optimistic update as currently implemented
2. After confirmation: update `monthEntries` for the affected date + refresh `commitmentProgress`

### App Init / Window Focus

- Current behavior preserved — on init, determine current date, load entries + commitments for that month
- On window focus, re-check if date changed (cross-midnight) and re-init if needed

## Data Availability for Quick-Jump Selects

The backend needs to provide which months have data for the quick-jump dropdowns to filter. Add a command:

```rust
get_available_months(root_path: String) -> Vec<AvailableMonth>
// AvailableMonth { year: i32, month: u32 }
```

This scans the `{root_path}/` directory for `YYYY/MM/` subdirectories that contain at least one `.md` file. Sorted descending (newest first).

If this command proves expensive, it can be run once at init and cached in the store; file watchers already cover config and monthly files, so month availability changes only when a directory is created or deleted.

## What Stays Unchanged

- All Rust backend commands except the new `get_available_months`
- Tauri event system (config-changed, commitments-changed)
- File watching
- SetupScreen, ConfigErrorBanner behavior
- Undo toast
- EntryInput, DimensionPanel, mention support
- CSS/Tailwind approach

## What Is Removed

- `Granularity` type and all granularity-related logic
- `datesInPeriod` week/month branches (keep only the month branch for loading)
- `SummaryBar.vue` component
- `DateNavigator.vue` component
- `TodayView.vue` component
- `weekLabel` utility (no longer needed)

## Testing Strategy

### New Component Tests
- `MonthNavigator.test.ts`: arrow navigation emits correct month/year, quick-jump dropdowns filter available months
- `DayStrip.test.ts`: renders correct number of days, dot indicators, future-date greying, selection emits, 7-day separators
- `MonthView.test.ts`: integration test — month load populates entries, day click updates view

### Updated Component Tests
- `EntryList.test.ts`: verify inline summary row renders correct count and duration
- `App.test.ts`: verify MonthView is rendered when screen === "ready"

### Removed Tests
- `DateNavigator.test.ts`: replaced by MonthNavigator + DayStrip tests
- `SummaryBar.test.ts`: removed
- `TodayView.test.ts`: replaced by MonthView test

### Utils
- `dates.test.ts`: remove week/month tests for `datesInPeriod`, remove `weekLabel` tests
