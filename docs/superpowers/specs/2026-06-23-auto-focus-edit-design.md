# Auto-focus on Edit

**Date:** 2026-06-23
**Status:** designed

## Problem

Double-clicking an entry row enters edit mode, but neither input gets focus. The user must click or Tab into the desired field. This violates Interaction Principles §2 ("any dialog/editor must move focus into itself when opened").

Additionally, the user wants the cursor placed at the end of the existing text when entering edit mode, so they can immediately type deltas like `+35m` in the duration field.

## Goal

When edit mode activates, auto-focus the input matching the click target, with cursor at end of text:

| Trigger | Focus target |
|---------|-------------|
| Double-click on item text | Item input |
| Double-click on duration | Duration input |
| Double-click on dimension chip / whitespace | Item input (default) |
| "…" button click | Item input (default) |

## Design

### §1 EntryRow.vue — Detect click target

- Add `data-edit-target="item"` to the item text element and `data-edit-target="duration"` to the duration display span.
- Replace the inline `@dblclick="editing = true"` with a function that uses `e.target.closest('[data-edit-target]')` to determine the target area. Falls back to `'item'`.
- Store the result in a new `focusTarget` ref (`'item' | 'duration'`), passed as a prop to `EntryRowEdit`.
- The "…" button handler explicitly sets `focusTarget = 'item'`.

Dimension chips are inside the item text container, so `closest()` on a chip click walks up to `data-edit-target="item"` — chips focus item, which is the agreed default.

### §2 EntryRowEdit.vue — Focus on mount

- Add optional prop: `focusTarget?: 'item' | 'duration'` (defaults to `'item'`).
- Add template refs (`itemInputEl`, `durInputEl`) on both `<input>` elements.
- In `onMounted`, after `nextTick()` (to guarantee DOM is rendered from the parent's `v-if`):
  - Call `.focus()` on the input matching `focusTarget`.
  - Call `setSelectionRange(input.value.length, input.value.length)` to place the cursor at the end.

Pattern matches existing `EntryComposer.vue` focus-on-mount behavior.

### §3 Edge cases

| Scenario | Behavior |
|----------|----------|
| Rapid double-clicks on different rows | Each `EntryRow` holds independent `editing` + `focusTarget` refs — no cross-talk |
| Empty item or duration value | `setSelectionRange(0, 0)` is a valid no-op |
| Browser tab not visible | Optional chaining (`target?.focus()`) handles null ref silently |

## Interaction Principles alignment

- **§2 iron law** ("move focus into itself when opened"): now satisfied via `onMounted` + `nextTick` → `focus()`.
- **§4** ("place cursor where the user is most likely to operate next"): cursor at end, ready for appending deltas.

## Changes

| File | Lines (est.) | What |
|------|-------------|------|
| `src/components/composite/EntryRow.vue` | +12 | `data-edit-target` attributes, `onDblClick` function, `focusTarget` ref, pass prop |
| `src/components/composite/EntryRowEdit.vue` | +15 | `focusTarget` prop, template refs, `nextTick` + focus logic in `onMounted` |
| Tests | +25 | Click-target detection, focus call assertions |
