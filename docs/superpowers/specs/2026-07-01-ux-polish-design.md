# UX Polish — Sidebar Goal Width, Window Title Version, File Path Right-Click

**Date**: 2026-07-01
**Status**: spec

## 1. Widen Goal Display in Commitments Sidebar

### Current State

- Sidebar: 280px (`w-[280px]`)
- Goal row: `<span>` with `overflow-hidden text-ellipsis whitespace-nowrap max-w-[130px]` + `title` attribute for full text on hover
- Duration number: no shrink protection, no left margin

### Problem

`max-w-[130px]` cuts off most goal names (e.g. "Implement real-time collaboration..." → "Implement real-t..."). The `title` tooltip exists but is not discoverable.

### Change

**Sidebar width** (`MonthView.vue` line 356):
- `w-[280px]` → `w-[320px]`

**Goal row layout** (`CommitmentsPanel.vue` lines 68-76):
- Goal name `<span>`: replace `max-w-[130px]` with `flex-1 min-w-0` (keeps `overflow-hidden text-ellipsis whitespace-nowrap` for natural truncation)
- Duration `<span>`: add `flex-shrink-0 ml-sm` so duration is anchored right and never squished

**Side effects**: HeatmapCalendar (above commitments in the same sidebar) gets 40px wider — the heatmap grid should scale naturally since it uses the full sidebar width.

## 2. Version Number in OS Window Title

### Current State

- `getVersion()` from `@tauri-apps/api/app` fetches version in `onMounted`
- Rendered as `<span v-if="appVersion" class="text-micro text-[var(--color-text-disabled)]">v{{ appVersion }}</span>` in bottom-right corner of `<main>`, alongside file path

### Problem

Version info is buried in small disabled-color text at the bottom-right; not visible at a glance.

### Change

- In `onMounted`, after `getVersion()` resolves, call `getCurrentWindow().setTitle('Logbook v' + v)`. Import `getCurrentWindow` from `@tauri-apps/api/window` (already used by App.vue).
- Remove the `<span v-if="appVersion">` element from the template (lines 438-439)
- Remove the `appVersion` ref declaration and the `getVersion` import since neither is used elsewhere
- Update test mock: `getCurrentWindow` mock (in `src/__tests__/mocks/tauri.ts`) must include a `setTitle` method (no-op `vi.fn()`)

## 3. Right-Click to Copy File Path

### Current State

- Bottom-right `<button>` shows `.../2026/07/2026-07-01.md` (compact display path)
- Full path via `title` attribute: `store.rootPath + '/' + dayFilePath`
- Click opens file in OS file manager (Finder)
- No right-click handling

### Problem

No way to get the full file path into clipboard without manually reconstructing it.

### Change

**Event handler** (`MonthView.vue`):
- Add `@contextmenu.prevent` on the `<button>` to suppress browser native context menu
- On right-click: `navigator.clipboard.writeText(store.rootPath + '/' + dayFilePath)` → briefly swap button text to "Copied!" for ~1.5s → restore via `setTimeout`

**Implementation sketch**:
```typescript
const copiedFeedback = ref(false);
let copyTimer: ReturnType<typeof setTimeout> | null = null;
async function copyFilePath(e: MouseEvent) {
  e.preventDefault();
  if (!store.rootPath) return;
  await navigator.clipboard.writeText(store.rootPath + '/' + dayFilePath.value);
  copiedFeedback.value = true;
  if (copyTimer) clearTimeout(copyTimer);
  copyTimer = setTimeout(() => { copiedFeedback.value = false; }, 1500);
}
```

Template:
```html
<button
  class="..."
  :title="store.rootPath + '/' + dayFilePath"
  @click="revealDayFile"
  @contextmenu.prevent="copyFilePath"
>{{ copiedFeedback ? 'Copied!' : displayPath }}</button>
```

**Note**: `onUnmounted` should clear `copyTimer` to avoid stale timeout after component teardown.

## Non-Goals

- No change to CommitmentsModal editor — editing UX is out of scope
- No change to HeatmapCalendar layout — it naturally adapts to sidebar width
- No change to left-click behavior of file path button (still opens in Finder)
