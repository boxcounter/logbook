# Dimension Editor — GUI + CLI

**Date:** 2026-06-27
**Status:** design (pending implementation plan)

## Problem

Dimensions and their values are currently managed by hand-editing YAML files (`template.yaml`, `_monthly.md`). Users must know the file location, format, and validation rules. This blocks adoption.

## Scope

GUI and CLI for creating, editing, reordering, and deleting dimensions and their static values. Editing always targets the month currently viewed in MonthView; a separate action promotes the in-memory editor state to the template.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Entry point | ⚙ icon in composer input row, right-aligned | Stable, always visible, no sidebar competition, contextually near dimension use |
| Edit target | Current month (via `_monthly.md`); "Save as template" action available | Edit-first-then-promote workflow; reduces cognitive load vs. month/template toggle |
| Key | Set at creation, locked thereafter | Avoids data migration when renaming |
| Source (`static` / `monthly`) | Set at creation, locked thereafter | Prevents multi-monthly conflicts |
| Deletion | Remove from config; entries keep their dimension data (orphaned) | No silent data loss; old entries remain readable |
| Reordering | Drag to reorder in dimension list sidebar | Controls popover display order |
| CLI | `dimensions get/set` | Matches `commitments set` pattern |
| Seed data | Goal dimension (`source: monthly`) shipped in default template | Makes dimensions discoverable for new users without docs/onboarding |

## GUI

### Entry point

In the composer input row (`EntryComposer.vue`), right of the Enter badge:

```
[+] [What did you work on?         ] [Enter] [⚙]
```

The ⚙ icon is `text-[var(--color-text-muted)]`, matching placeholder color. On hover: brand color.

No tooltip needed — the icon itself is the affordance, and Goal's presence in the popover makes the concept discoverable.

### Modal layout

Opens on ⚙ click. Teleported to `<body>`, centered overlay (`fixed inset-0 z-50 bg-black/30`).

```
┌─────────────────────────────────────────────────────────┐
│ Edit Dimensions                                    [×]  │
│ Editing June 2026              [Save as template]       │
├──────────────┬──────────────────────────────────────────┤
│              │                                          │
│  ▎ Goal      │  Name: [Biz________]                     │
│  ▎ Biz    ←  │  Key:  biz    Source: static [▼]         │
│  ▎ Importance│  ☑ Required                              │
│              │                                          │
│              │  VALUES                                  │
│              │  ⠿ [Product________] [×]                 │
│              │  ⠿ [Marketing______] [×]                 │
│              │  ⠿ [Engineering____] [×]                 │
│              │  [New value_______] [+]                   │
│              │                                          │
│              │                              [Delete dim]│
│  ─────────── │                                          │
│  + Add dim   │                                          │
│              │                                          │
├──────────────┴──────────────────────────────────────────┤
│                                    [Cancel]  [Save]     │
└─────────────────────────────────────────────────────────┘
```

**Header row:**
- Title: "Edit Dimensions"
- Subtitle: "Editing `<month year>`" with a `[Save as template]` text button (brand-link color). Clicking it immediately writes current dimensions to `template.yaml`. No confirmation dialog — the action is low-risk (template is just a default; instantiated months aren't affected).

**Left panel (210px):**
- Dimension list, each row: color bar (3px × 16px), name, source badge (`static`/`monthly`). Selected row has `bg-[#eff6ff]`.
- Bottom: `+ Add dimension` dashed-border button.
- Drag to reorder: drag grip on each row (dots-123456 pattern, matches CommitmentsModal).
- Order persisted on save.

**Right panel (flex-1):**
- Name input (editable, inline)
- Key display (read-only, monospace)
- Source display (read-only badge)
- Required checkbox
- **Values section** (only for `source: static`):
  - Each value row: drag grip, text input, × remove button
  - Bottom: `[New value input] [+]` to add
  - Drag to reorder within values
- **Goal (monthly) special case:** Values section replaced with informational text: "Values are derived from commitment goals. Edit commitments to change available values." Source and required are still displayed (required can be toggled).
- **Delete dimension** button: bottom-left, danger-styled. On click:
  - If no entries use this dimension: delete immediately.
  - If entries exist: show a banner "This dimension is used by N entries this month. Deleting will remove it from the configuration but existing entries will keep their values." with "Delete anyway" and "Cancel".

**Footer:**
- Cancel (closes without saving, discard confirmation if dirty)
- Save (writes to `_monthly.md`)

### Add dimension flow

Clicking `+ Add dimension` inserts a focused form at the bottom of the left list:

```
┌─────────────────┐
│ Name: [______]  │
│ Key:  [______]  │  ← user types; validated on blur
│ Source: static  │  ← default, dropdown (static / monthly)
│                 │
│ [Cancel] [Create]│
└─────────────────┘
```

- Key input: validated on blur — alphanumeric + hyphens/underscores only, no duplicates.
- If source = monthly and another monthly already exists: inline error "Only one monthly-source dimension is allowed."
- On create: dimension appears in list, selected, right panel shows its detail editor.

### Save

- Validates all dimensions (same rules as `validate_dimensions`).
- On success: writes to current month's `_monthly.md`. If month is uninstantiated, this triggers instantiation (dimensions + existing commitments preserved).
- File watcher picks up the change; frontend store updates via the existing `config-changed` / `commitments-changed` event flow.
- If validation fails: highlight the offending dimension in the left list; show error message inline in the right panel.

### "Save as template"

- Appears in the modal header, right-aligned.
- Enabled whenever dimensions are valid.
- Writes the in-memory editor state (not the persisted file) to `template.yaml` immediately. Does NOT require a prior Save — you can edit, save-as-template, then Cancel without affecting the month.
- Toast confirmation: "Dimensions saved to template" with the existing toast component.
- Does NOT retroactively affect already-instantiated months.

### Discard confirmation

If the user closes the modal (×, Cancel, Escape, backdrop click) with unsaved changes:
- Show an inline prompt in the footer area: "Discard changes?" + "Keep editing" (brand solid) + "Discard" (danger text).
- Matches the existing pattern in `EntryRowEdit.vue` and `CommitmentsModal`.

### Keyboard

- `Escape`: close modal (with discard confirmation if dirty).
- `Ctrl+S` / `Cmd+S`: save.

## CLI

### `logbook-cli dimensions get`

```
logbook-cli dimensions get [--year Y] [--month M] [--template]
```

- Without flags: returns effective dimensions for current month (resolved: monthly snapshot if instantiated, else template).
- `--template`: returns template dimensions directly.
- Output: YAML (default) or `--json` flag for JSON.
- Exit 0 on success, 1 on error (missing file, parse error).

### `logbook-cli dimensions set`

```
logbook-cli dimensions set [--year Y] [--month M] [--template]
```

- Reads YAML or JSON from stdin.
- `--template`: writes to `template.yaml`. Without: writes to `_monthly.md`.
- Validates before writing (same rules as GUI).
- On success: writes atomically (tmp + rename), outputs nothing, exit 0.
- On validation failure: prints errors to stderr, exit 1.
- Overwrites entire dimensions array (not partial update — matches `commitments set`).

## Data Flow

```
GUI save ──→ _monthly.md (current month)
GUI "Save as template" ──→ template.yaml
CLI set ──→ _monthly.md or template.yaml
CLI get ←── resolve_month_dimensions() ←── _monthly.md or template.yaml

File watcher (existing):
  template.yaml change ──→ re-validate, emit config-changed
  _monthly.md change ──→ re-validate, emit commitments-changed
  → frontend store updates dimensions reactively
```

### Month instantiation

If the month was uninstantiated (no `_monthly.md` dimensions) when the user saves:
- `ensure_month_instantiated` is called (existing function).
- The saved dimensions become the month's snapshot.
- Subsequent template changes won't affect this month.

### Orphaned dimension values

When a dimension is deleted from config:
- Entry `dimensions` map retains the key-value pair.
- `EntryRow.vue` already renders chips for any key present in the entry's dimensions map, regardless of whether the key exists in current config.
- The orphaned chip appears without a color from the dimension palette — use a neutral gray chip style.
- `DimensionPopover` no longer lists the deleted dimension as selectable.

## Validation Rules (reuse from `config.rs::validate_dimensions`)

- `name`: non-empty.
- `key`: non-empty, `[a-zA-Z0-9_-]+`, unique among all dimensions.
- `source`: `"static"` or `"monthly"`.
- If `source: "static"`: `values` must be present and non-empty.
- At most one dimension with `source: "monthly"`.

Additional GUI-level validation:
- No two dimensions with the same key.
- No two values with the same name within a dimension.

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Delete monthly-source dimension | Same as any deletion — remove from config, entries keep data. `monthly_dim_key` fallback to `"goal"` still works for commitment progress. |
| Rename a value that's used in entries | Old entries keep old value string. No migration. User can find-replace via CLI or manual file edit. |
| Template missing when saving "as template" | Create `template.yaml` with the dimensions (treat "Save as template" as idempotent create-or-update). |
| Month is uninstantiated, user saves dimensions | Instantiate month + save dimensions. |
| User opens editor, another process changes `_monthly.md` | File watcher triggers `commitments-changed`. Modal should show a "File changed externally" banner and disable save until reload. Stretch: if complexity is high, skip for v1 — the file watcher is sub-second, the race window is tiny in single-user desktop use. |
| Duplicate key on creation | Inline validation error on blur: "Key 'biz' already exists." |
| Empty values list for static dimension | Validation error on save: "Dimension 'Biz' has no values." |

## Out of Scope

- Value renaming with entry data migration.
- Onboarding flow for new users (deferred until more users exist).
- Editing dimensions for past months that are already instantiated (only current month is targeted; editing past months can be done via CLI).
- Import/export of dimension configurations beyond what CLI `get/set` provides.
