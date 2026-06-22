# Empty State & Sidebar Width

**Date**: 2026-06-22
**Status**: design

## Motivation

Two small UX issues:

1. **Non-today empty state is misleading.** `EntryList` shows *"No entries yet. Log your first work item below."* unconditionally when a day has zero entries. But `EntryComposer` (the new-entry input) only renders for today. On past/future days the user sees a prompt to type "below" with nothing there.
2. **Left sidebar is too narrow.** The calendar and commitments panel sit in a fixed 220px sidebar. Calendar day cells cramp at that width; commitment cards have little room. The right content area takes all remaining space with no upper bound, producing excessively wide entry rows on large screens.

## Design

### 1. Empty state: conditional message by day

Give `EntryList` an `isToday` boolean prop. Keep the existing message for today; show a plain *"No entries for this day."* for any other day.

**EntryList.vue** — add prop and conditional:

```ts
defineProps<{
  entries: Entry[];
  justAddedId?: string | null;
  isToday?: boolean;
}>();
```

```html
<div v-if="entries.length === 0" class="p-2xl text-center text-[var(--color-text-secondary)] text-secondary">
  {{ isToday ? "No entries yet. Log your first work item below." : "No entries for this day." }}
</div>
```

**MonthView.vue** — pass the prop:

```html
<EntryList
  :entries="dayEntries"
  :just-added-id="justAddedId"
  :is-today="isSelectedToday"
  @update="..."
  @delete="..."
  @update-dimensions="..."
/>
```

`isToday` defaults to `false` (optional prop), so existing callers that don't pass it get the non-today message — safe default.

### 2. Sidebar width: 220px → 280px

One-line change in `MonthView.vue` line 332:

```diff
- <aside class="w-[220px] flex-shrink-0 flex flex-col gap-0 ...">
+ <aside class="w-[280px] flex-shrink-0 flex flex-col gap-0 ...">
```

No max-width constraint on the right content area at this time. The main area continues to use `flex-1 min-w-0`.

## Files touched

| File | Change |
|---|---|
| `src/components/EntryList.vue` | Add `isToday` prop; conditional empty message |
| `src/components/MonthView.vue` | Pass `:is-today="isSelectedToday"` to EntryList; sidebar `w-[220px]` → `w-[280px]` |

## Non-goals

- Adding max-width to right content area (deferred)
- Allowing entry creation on past days
- Responsive breakpoints
