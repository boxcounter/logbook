# UX Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship 4 UX fixes — first-class day navigation with remapped shortcuts, day-note relocation, always-visible entry separators, and focus behavior that stays on the selected date instead of snapping to today.

**Architecture:** Frontend-only (Vue 3 + TypeScript). No Rust/backend changes — day navigation and notes reuse the existing `get_entries` / `set_day_note` commands. Day navigation is added as a `shiftDay` helper in `MonthView`, surfaced via DayHeader arrow buttons and remapped `⌘[`/`⌘]` (day) + `⌘⇧[`/`⌘⇧]` (month) global shortcuts. Focus behavior tracks a `lastKnownToday` ref to distinguish a genuine midnight crossing from an ordinary refocus.

**Tech Stack:** Vue 3 `<script setup>`, Tailwind v4 utility classes with CSS custom-property tokens, Vitest + @vue/test-utils. Test command: `npm test` (vitest run); single file: `npx vitest run <path>`.

**Spec:** `docs/superpowers/specs/2026-06-20-ux-improvements-design.md`

---

## File Structure

**Created:** none.

**Modified:**
- `src/utils/dates.ts` — add `addDays(dateStr, n)` helper.
- `src/components/DayHeader.vue` — add `‹ ›` day-nav buttons, `canGoNext` prop, `prev-day`/`next-day` emits.
- `src/components/MonthView.vue` — `shiftDay`, wire DayHeader emits, remap keyboard shortcuts, pass `can-go-next`, relocate the day-note block above EntryList.
- `src/components/TwoLineInput.vue` — drop the two month-nav hint spans.
- `src/components/HeatmapCalendar.vue` — update month-arrow tooltips to the new shortcuts.
- `src/components/EntryList.vue` — drop `gap-[2px]`.
- `src/components/composite/EntryRow.vue` — hairline top divider between rows (not first), drop hover border + rounded, simplify highlight keyframe.
- `src/App.vue` — `lastKnownToday`-based focus reset.

**Test files modified:**
- `src/__tests__/dates.test.ts`
- `src/__tests__/components/DayHeader.test.ts`
- `src/__tests__/components/MonthView.test.ts`
- `src/__tests__/components/TwoLineInput.test.ts`
- `src/__tests__/components/HeatmapCalendar.test.ts`
- `src/__tests__/components/composite/EntryRow.test.ts`
- `src/__tests__/components/App.test.ts`

---

## Task 1: `addDays` date helper

**Files:**
- Modify: `src/utils/dates.ts`
- Test: `src/__tests__/dates.test.ts`

- [ ] **Step 1: Write the failing test**

Append to `src/__tests__/dates.test.ts`:

```typescript
import { formatDate, datesInMonth, parseDate, addDays } from "../utils/dates";

describe("addDays", () => {
  it("adds a positive offset within a month", () => {
    expect(addDays("2026-06-12", 3)).toBe("2026-06-15");
  });
  it("subtracts across a month boundary", () => {
    expect(addDays("2026-06-01", -1)).toBe("2026-05-31");
  });
  it("adds across a month boundary", () => {
    expect(addDays("2026-06-30", 1)).toBe("2026-07-01");
  });
  it("handles year boundaries", () => {
    expect(addDays("2026-12-31", 1)).toBe("2027-01-01");
  });
  it("handles leap-year February", () => {
    expect(addDays("2028-02-28", 1)).toBe("2028-02-29");
  });
});
```

Note: the existing file imports `{ formatDate, datesInMonth, parseDate }` on line 2 — change that import line to add `addDays` rather than adding a duplicate import.

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/dates.test.ts`
Expected: FAIL — `addDays is not a function` / no exported member `addDays`.

- [ ] **Step 3: Implement `addDays`**

Append to `src/utils/dates.ts`:

```typescript
/** Return the date n days from dateStr (n may be negative), as YYYY-MM-DD. */
export function addDays(dateStr: string, n: number): string {
  const d = parseDate(dateStr);
  d.setDate(d.getDate() + n);
  return formatDate(d);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/dates.test.ts`
Expected: PASS (all `addDays` cases + existing cases).

- [ ] **Step 5: Commit**

```bash
git add src/utils/dates.ts src/__tests__/dates.test.ts
git commit -m "feat(dates): add addDays helper for day navigation"
```

---

## Task 2: DayHeader day-navigation arrows

**Files:**
- Modify: `src/components/DayHeader.vue`
- Test: `src/__tests__/components/DayHeader.test.ts`

- [ ] **Step 1: Write the failing test**

Append these cases inside the `describe("DayHeader", ...)` block in `src/__tests__/components/DayHeader.test.ts`:

```typescript
  it("emits prev-day when the left arrow is clicked", async () => {
    const wrapper = mount(DayHeader, {
      props: { title: "X", isToday: false, entryCount: 0, totalMinutes: 0, canGoNext: true },
    });
    await wrapper.find("[data-test='prev-day']").trigger("click");
    expect(wrapper.emitted("prev-day")).toBeTruthy();
  });

  it("emits next-day when the right arrow is clicked and canGoNext is true", async () => {
    const wrapper = mount(DayHeader, {
      props: { title: "X", isToday: false, entryCount: 0, totalMinutes: 0, canGoNext: true },
    });
    await wrapper.find("[data-test='next-day']").trigger("click");
    expect(wrapper.emitted("next-day")).toBeTruthy();
  });

  it("does not emit next-day when canGoNext is false", async () => {
    const wrapper = mount(DayHeader, {
      props: { title: "X", isToday: true, entryCount: 0, totalMinutes: 0, canGoNext: false },
    });
    await wrapper.find("[data-test='next-day']").trigger("click");
    expect(wrapper.emitted("next-day")).toBeFalsy();
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/DayHeader.test.ts`
Expected: FAIL — `[data-test='prev-day']` not found.

- [ ] **Step 3: Implement the arrows**

Replace the entire contents of `src/components/DayHeader.vue` with:

```vue
<!-- src/components/DayHeader.vue -->
<script setup lang="ts">
import { computed } from "vue";
import { formatDuration } from "../utils/format";

const props = defineProps<{
  title: string;
  isToday: boolean;
  entryCount: number;
  totalMinutes: number;
  canGoNext: boolean;
}>();

const emit = defineEmits<{
  "prev-day": [];
  "next-day": [];
}>();

const countLabel = computed(() => (props.entryCount === 1 ? "entry" : "entries"));
const total = computed(() => formatDuration(props.totalMinutes));

function onNext() {
  if (props.canGoNext) emit("next-day");
}
</script>

<template>
  <div class="flex justify-between items-baseline mb-[20px] pb-[14px] border-b border-[var(--color-divider)]">
    <div class="flex items-center gap-[8px]">
      <button
        data-test="prev-day"
        class="inline-flex items-center justify-center w-[22px] h-[22px] rounded-[var(--radius-form-lg)]
               border border-[var(--color-border-form)] text-[var(--color-text-secondary)]
               hover:text-[var(--color-text-primary)] hover:bg-[var(--color-surface-muted)] cursor-pointer transition-colors"
        title="Previous day (⌘[)"
        @click="emit('prev-day')"
      >‹</button>
      <span class="text-[length:var(--app-text-xl)] font-bold text-[var(--color-text-primary)] tracking-[-0.3px]">{{ title }}</span>
      <button
        data-test="next-day"
        class="inline-flex items-center justify-center w-[22px] h-[22px] rounded-[var(--radius-form-lg)]
               border border-[var(--color-border-form)] transition-colors"
        :class="canGoNext
          ? 'text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-surface-muted)] cursor-pointer'
          : 'text-[var(--color-text-disabled)] opacity-40 cursor-default'"
        title="Next day (⌘])"
        @click="onNext"
      >›</button>
      <span
        v-if="isToday"
        data-test="today-badge"
        class="ml-[6px] align-middle text-[length:var(--app-text-micro)] font-semibold
               text-[var(--color-brand-link)] bg-[var(--color-brand-soft-bg)] px-[8px] py-[2px] rounded-[var(--radius-md)]"
      >Today</span>
    </div>
    <span class="text-[length:var(--app-text-xs)] text-[var(--color-text-secondary)]">
      <span class="mono">{{ entryCount }}</span> {{ countLabel }} · <span class="mono">{{ total }}</span>
    </span>
  </div>
</template>
```

Note: the existing tests at `DayHeader.test.ts:8-9,17,19,24` pass props without `canGoNext`. TypeScript types are not enforced at mount in these tests (runtime mount), so they keep passing; `canGoNext` is simply `undefined` → falsy there, which only affects the next-day button state, not the assertions those tests make. Leave them as-is.

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/DayHeader.test.ts`
Expected: PASS (new + existing).

- [ ] **Step 5: Commit**

```bash
git add src/components/DayHeader.vue src/__tests__/components/DayHeader.test.ts
git commit -m "feat(day-header): add prev/next day arrows with future guard"
```

---

## Task 3: MonthView day navigation + keyboard remap

**Files:**
- Modify: `src/components/MonthView.vue`
- Test: `src/__tests__/components/MonthView.test.ts`

- [ ] **Step 1: Write the failing tests**

Add to the top imports of `src/__tests__/components/MonthView.test.ts` (the existing import on line 7 pulls fixtures; add `addDays`):

```typescript
import { addDays } from "../../utils/dates";
```

Add these cases inside `describe("MonthView", ...)`:

```typescript
  it("prev-day from DayHeader moves currentDate back one day", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    wrapper.findComponent({ name: "DayHeader" }).vm.$emit("prev-day");
    await wrapper.vm.$nextTick();
    expect(store.currentDate).toBe(addDays(todayDateStr(), -1));
  });

  it("next-day is a no-op when the selected day is today", async () => {
    const store = makeStore(); // currentDate === today
    const wrapper = mountView(store);
    wrapper.findComponent({ name: "DayHeader" }).vm.$emit("next-day");
    await wrapper.vm.$nextTick();
    expect(store.currentDate).toBe(todayDateStr());
  });

  it("passes can-go-next=false to DayHeader when today is selected", () => {
    const store = makeStore();
    const wrapper = mountView(store);
    expect(wrapper.findComponent({ name: "DayHeader" }).props("canGoNext")).toBe(false);
  });

  it("⌘[ moves back one day", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "[", metaKey: true }));
    await wrapper.vm.$nextTick();
    expect(store.currentDate).toBe(addDays(todayDateStr(), -1));
  });

  it("⌘⇧[ moves back one month", async () => {
    const store = makeStore();
    const wrapper = mountView(store);
    const expectedMonth = addDays(todayDateStr(), 0).slice(0, 7); // current YYYY-MM
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "[", metaKey: true, shiftKey: true }));
    await wrapper.vm.$nextTick();
    // currentDate now in a different (earlier) month
    expect(store.currentDate.slice(0, 7)).not.toBe(expectedMonth);
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npx vitest run src/__tests__/components/MonthView.test.ts`
Expected: FAIL — DayHeader has no `prev-day` handler / `canGoNext` prop is undefined / keyboard `[` currently shifts month not day.

- [ ] **Step 3: Implement in MonthView**

3a. Add `addDays` to the dates import (currently `src/components/MonthView.vue:13`):

```typescript
import { datesInMonth, yearMonthFromDate, parseDate, addDays } from "../utils/dates";
```

3b. Add a `shiftDay` function next to `shiftMonth` (after the `shiftMonth` block, around `:250`):

```typescript
function shiftDay(delta: number) {
  if (delta > 0 && isSelectedToday.value) return; // never navigate into the future
  const next = addDays(store.currentDate, delta);
  if (next in store.monthEntries) {
    handleSelectDay(next);
  } else {
    const { year, month } = yearMonthFromDate(next);
    loadMonth(year, month, parseInt(next.slice(8, 10), 10));
  }
}
```

3c. Replace `onGlobalKeydown` (currently `:251-255`) with shift-aware routing:

```typescript
function onGlobalKeydown(e: KeyboardEvent) {
  if (!(e.metaKey || e.ctrlKey)) return;
  if (e.key === "[") {
    e.preventDefault();
    e.shiftKey ? shiftMonth(-1) : shiftDay(-1);
  } else if (e.key === "]") {
    e.preventDefault();
    e.shiftKey ? shiftMonth(1) : shiftDay(1);
  }
}
```

3d. Wire DayHeader (currently `:300-305`) — add `:can-go-next` and the two handlers:

```vue
      <DayHeader
        :title="dayTitle"
        :is-today="isSelectedToday"
        :entry-count="dayEntries.length"
        :total-minutes="dayTotalMinutes"
        :can-go-next="!isSelectedToday"
        @prev-day="shiftDay(-1)"
        @next-day="shiftDay(1)"
      />
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run src/__tests__/components/MonthView.test.ts`
Expected: PASS (new + existing — including the existing `⌘`-free flows and the note-esc test).

- [ ] **Step 5: Commit**

```bash
git add src/components/MonthView.vue src/__tests__/components/MonthView.test.ts
git commit -m "feat(month-view): day navigation via arrows + ⌘[/] (day), ⌘⇧[/] (month)"
```

---

## Task 4: Trim month-nav hints from the input bar

**Files:**
- Modify: `src/components/TwoLineInput.vue`
- Test: `src/__tests__/components/TwoLineInput.test.ts`

- [ ] **Step 1: Write the failing test**

Add inside `describe("TwoLineInput", ...)` in `src/__tests__/components/TwoLineInput.test.ts`:

```typescript
  it("shows only @ and # hints, not month-navigation hints", () => {
    const wrapper = mountInput();
    expect(wrapper.text()).toContain("dim");
    expect(wrapper.text()).toContain("time");
    expect(wrapper.text()).not.toContain("prev month");
    expect(wrapper.text()).not.toContain("next month");
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/TwoLineInput.test.ts`
Expected: FAIL — text still contains "prev month" / "next month".

- [ ] **Step 3: Remove the two month hint spans**

In `src/components/TwoLineInput.vue`, delete the two `<span>` lines for `⌘[` and `⌘]` (currently `:199-200`). The hints block (`:196-201`) becomes:

```vue
    <!-- Hints -->
    <div class="flex gap-[14px] mt-[4px] text-[length:var(--app-text-micro)] text-[var(--color-text-disabled)] group-focus-within:text-[var(--color-text-muted)] hover:text-[var(--color-text-muted)] transition-colors">
      <span><kbd class="mono px-[5px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)] text-[length:var(--app-text-2xs)]">@</kbd> dim</span>
      <span><kbd class="mono px-[5px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)] text-[length:var(--app-text-2xs)]">#</kbd> time</span>
    </div>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/TwoLineInput.test.ts`
Expected: PASS (new + existing).

- [ ] **Step 5: Commit**

```bash
git add src/components/TwoLineInput.vue src/__tests__/components/TwoLineInput.test.ts
git commit -m "refactor(input): drop month-nav hints from the entry composer"
```

---

## Task 5: Update HeatmapCalendar month-arrow tooltips

**Files:**
- Modify: `src/components/HeatmapCalendar.vue`
- Test: `src/__tests__/components/HeatmapCalendar.test.ts`

- [ ] **Step 1: Write the failing test**

Add inside `describe("HeatmapCalendar", ...)` in `src/__tests__/components/HeatmapCalendar.test.ts`:

```typescript
  it("month arrows advertise the ⌘⇧[ / ⌘⇧] shortcuts", () => {
    const wrapper = mountCalendar();
    expect(wrapper.find("[data-test='prev-month']").attributes("title")).toContain("⌘⇧[");
    expect(wrapper.find("[data-test='next-month']").attributes("title")).toContain("⌘⇧]");
  });
```

If `mountCalendar` is not already a helper in this file, mount inline instead, mirroring the existing tests in the file (use the same props the file's other tests already pass to `mount(HeatmapCalendar, ...)`).

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/HeatmapCalendar.test.ts`
Expected: FAIL — title attribute still reads `(⌘[)` / `(⌘])`.

- [ ] **Step 3: Update the tooltips**

In `src/components/HeatmapCalendar.vue`, change the two `title` attributes:
- `:107` `title="Previous month (⌘[)"` → `title="Previous month (⌘⇧[)"`
- `:112` `title="Next month (⌘])"` → `title="Next month (⌘⇧])"`

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/HeatmapCalendar.test.ts`
Expected: PASS (new + existing).

- [ ] **Step 5: Commit**

```bash
git add src/components/HeatmapCalendar.vue src/__tests__/components/HeatmapCalendar.test.ts
git commit -m "docs(calendar): update month-arrow tooltips to ⌘⇧[/] shortcuts"
```

---

## Task 6: Relocate the day note above the entry list

**Files:**
- Modify: `src/components/MonthView.vue`
- Test: `src/__tests__/components/MonthView.test.ts`

- [ ] **Step 1: Write the failing test**

Add inside `describe("MonthView", ...)` in `src/__tests__/components/MonthView.test.ts`:

```typescript
  it("renders the day note above the entry list", () => {
    const wrapper = mountView();
    const html = wrapper.html();
    const noteIdx = html.indexOf('contenteditable');
    const listIdx = html.indexOf('No entries'); // empty-state marker, or fall back below
    // When there are entries, locate EntryList by its scroll container class instead:
    const listAnchor = listIdx !== -1 ? listIdx : html.indexOf('overflow-y-auto');
    expect(noteIdx).toBeGreaterThan(-1);
    expect(listAnchor).toBeGreaterThan(-1);
    expect(noteIdx).toBeLessThan(listAnchor);
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/MonthView.test.ts -t "above the entry list"`
Expected: FAIL — note currently renders after EntryList (`noteIdx` > `listAnchor`).

- [ ] **Step 3: Move the note block**

In `src/components/MonthView.vue`, cut the entire note wrapper block (currently `:315-327`):

```vue
      <div class="mt-[16px] py-[8px]">
        <div
          ref="noteRef"
          class="text-[length:var(--app-text-xs)] italic text-[var(--color-text-secondary)] leading-[1.5] cursor-text px-[10px] py-[6px] rounded-[var(--radius-form-lg)] outline-none hover:bg-[var(--color-page-bg)]"
          contenteditable="true"
          data-placeholder="Add a note…"
          @blur="saveNote"
          @paste="onNotePaste"
          @input="onNoteInput"
          @focus="onNoteFocus"
          @keydown.esc="onNoteEsc"
        ></div>
      </div>
```

Paste it **between** `<DayHeader ... />` and `<EntryList ... />`, and change the outer margin from `mt-[16px]` to `mt-[4px] mb-[8px]` so it sits snugly under the header:

```vue
      <DayHeader
        ...
      />

      <div class="mt-[4px] mb-[8px] py-[4px]">
        <div
          ref="noteRef"
          class="text-[length:var(--app-text-xs)] italic text-[var(--color-text-secondary)] leading-[1.5] cursor-text px-[10px] py-[6px] rounded-[var(--radius-form-lg)] outline-none hover:bg-[var(--color-page-bg)]"
          contenteditable="true"
          data-placeholder="Add a note…"
          @blur="saveNote"
          @paste="onNotePaste"
          @input="onNoteInput"
          @focus="onNoteFocus"
          @keydown.esc="onNoteEsc"
        ></div>
      </div>

      <EntryList
        ...
      />
```

Leave all script logic (`noteRef`, `saveNote`, `onNoteFocus`, `onNoteEsc`, `onNotePaste`, `onNoteInput`, the `watch`) untouched.

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run src/__tests__/components/MonthView.test.ts`
Expected: PASS (new ordering test + existing note-esc test, which is position-independent).

- [ ] **Step 5: Commit**

```bash
git add src/components/MonthView.vue src/__tests__/components/MonthView.test.ts
git commit -m "refactor(month-view): move day note above the entry list"
```

---

## Task 7: Hairline dividers between entry rows

**Files:**
- Modify: `src/components/EntryList.vue`, `src/components/composite/EntryRow.vue`
- Test: `src/__tests__/components/composite/EntryRow.test.ts`

- [ ] **Step 1: Write the failing tests**

Add inside `describe("EntryRow", ...)` in `src/__tests__/components/composite/EntryRow.test.ts` (this file already mounts EntryRow with `entry`/`index` props — match its existing mount helper; the snippet below shows the assertions):

```typescript
  it("draws a top hairline divider for non-first rows", () => {
    const wrapper = mountRow({ index: 1 });
    expect(wrapper.find("[data-test='entry-row']").classes()).toContain("border-t");
  });

  it("does not draw a top divider on the first row", () => {
    const wrapper = mountRow({ index: 0 });
    expect(wrapper.find("[data-test='entry-row']").classes()).not.toContain("border-t");
  });
```

If the file has no `mountRow` helper, write one mirroring its existing mounts:

```typescript
function mountRow(opts: { index: number }) {
  const store = reactive({ config: makeConfig(), commitments: [makeCommitment()] });
  return mount(EntryRow, {
    props: { entry: makeEntry({ item: "A", duration: 60 }), index: opts.index },
    global: { provide: { [STORE_KEY as symbol]: store } },
  });
}
```

(Imports needed if adding the helper: `reactive` from `vue`, `STORE_KEY` from `../../../stores/useStore`, `makeEntry`/`makeConfig`/`makeCommitment` from `../../mocks/fixtures` — check the file's existing imports first and only add what's missing.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `npx vitest run src/__tests__/components/composite/EntryRow.test.ts`
Expected: FAIL — row has no `border-t` class (currently `border border-transparent`).

- [ ] **Step 3a: Update EntryList container**

In `src/components/EntryList.vue:16`, remove `gap-[2px]`:

```vue
  <div class="flex-1 flex flex-col overflow-y-auto pr-[4px]">
```

- [ ] **Step 3b: Update EntryRow row classes**

In `src/components/composite/EntryRow.vue`, replace the read-only row `<div>` opening tag (currently `:53-60`) with:

```vue
  <div
    v-else
    data-test="entry-row"
    class="group flex justify-between items-start gap-[8px] px-[14px] py-[9px]
           hover:bg-[var(--color-surface-muted)] transition-colors"
    :class="[{ 'just-added': justAdded }, index > 0 ? 'border-t border-[var(--color-divider)]' : '']"
    @dblclick="editing = true"
  >
```

- [ ] **Step 3c: Simplify the highlight keyframe**

In `src/components/composite/EntryRow.vue` `<style scoped>` (currently `:88-93`), drop the `border-color` from the animation so it doesn't fight the divider:

```css
/* Newly-added entry: blue background that fades over 1.5s (spec §5.2 step 7). */
@keyframes fadeHighlight {
  0% { background-color: var(--anim-highlight-bg); }
  100% { background-color: transparent; }
}
.just-added { animation: fadeHighlight var(--anim-highlight-duration) ease-out forwards; }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run src/__tests__/components/composite/EntryRow.test.ts src/__tests__/components/EntryList.test.ts`
Expected: PASS (new dividers + existing EntryRow/EntryList behavior).

- [ ] **Step 5: Commit**

```bash
git add src/components/EntryList.vue src/components/composite/EntryRow.vue src/__tests__/components/composite/EntryRow.test.ts
git commit -m "feat(entry-list): always-visible hairline dividers between rows"
```

---

## Task 8: Focus reset only on a true midnight crossing

**Files:**
- Modify: `src/App.vue`
- Test: `src/__tests__/components/App.test.ts`

- [ ] **Step 1: Write the failing tests**

The existing App test (`App.test.ts:24-66`) already captures `focusChangedCallback` and uses `vi.useFakeTimers()`. Add a `todayStr` helper and these cases inside `describe("App", ...)`:

```typescript
  function ymd(d: Date): string {
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  it("refocus on the same day does NOT reset the selected date", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 10, 0, 0));
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", config: makeConfig(), today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    store.currentDate = "2026-06-12"; // user navigated to a past day
    vi.clearAllMocks();

    focusChangedCallback?.({ payload: true });
    await vi.runAllTimersAsync();

    expect(store.currentDate).toBe("2026-06-12");
    expect(mockInvoke).not.toHaveBeenCalledWith("init");
  });

  it("midnight crossing while viewing today FOLLOWS to the new today", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 23, 59, 0));
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", config: makeConfig(), today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    expect(store.currentDate).toBe("2026-06-20");
    store.screen = "ready";
    vi.setSystemTime(new Date(2026, 5, 21, 0, 1, 0)); // crossed midnight
    focusChangedCallback?.({ payload: true });
    await vi.runAllTimersAsync();

    expect(store.currentDate).toBe("2026-06-21");
    expect(mockInvoke).toHaveBeenCalledWith("init");
  });

  it("midnight crossing while viewing another day STAYS put", async () => {
    vi.setSystemTime(new Date(2026, 5, 20, 23, 59, 0));
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", config: makeConfig(), today: makeDayFile(), commitments: [], scan_warnings: [] },
    });
    const { store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    store.currentDate = "2026-06-12"; // viewing a different day
    store.screen = "ready";
    vi.clearAllMocks();
    vi.setSystemTime(new Date(2026, 5, 21, 0, 1, 0));
    focusChangedCallback?.({ payload: true });
    await vi.runAllTimersAsync();

    expect(store.currentDate).toBe("2026-06-12");
    expect(mockInvoke).not.toHaveBeenCalledWith("init");
  });
```

Note: the `Ready` branch sets `store.screen = "ready"`. After mount with the Ready fixture, `screen` is already `"ready"`; the explicit assignment in the tests is belt-and-suspenders.

- [ ] **Step 2: Run tests to verify they fail**

Run: `npx vitest run src/__tests__/components/App.test.ts -t "midnight"`
Expected: FAIL — current code resets to today on every refocus where `currentDate !== today` (the "same day" and "stays put" cases fail; the follow case may pass incidentally — that's fine, it stays green after).

- [ ] **Step 3: Implement `lastKnownToday` logic**

In `src/App.vue`:

3a. Add a `lastKnownToday` ref and a `todayStr` helper near the other refs (after `:23`):

```typescript
function todayStr(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
}
let lastKnownToday = todayStr();
```

3b. Replace the focus handler body (currently `:45-56`) with:

```typescript
    unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) return;
      focusRequestId.value++;
      const newToday = todayStr();
      if (newToday === lastKnownToday) return; // same calendar day: leave the view alone
      // Midnight crossed since we were last focused.
      if (store.currentDate === lastKnownToday && store.screen === "ready") {
        store.currentDate = newToday; // we were following "today" → follow to the new today
        initApp();
      }
      lastKnownToday = newToday;
    });
```

Note: `lastKnownToday` is captured at module setup time (mount). The `todayStr` helper here duplicates the one already inlined in the old code — by extracting it, both the init value and the focus check use the same logic.

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run src/__tests__/components/App.test.ts`
Expected: PASS (3 new midnight cases + all existing App tests).

- [ ] **Step 5: Commit**

```bash
git add src/App.vue src/__tests__/components/App.test.ts
git commit -m "fix(focus): keep selected date on refocus; follow to new today only across midnight"
```

---

## Task 9: Full verification

**Files:** none (verification only).

- [ ] **Step 1: Run the full test suite**

Run: `npm test`
Expected: PASS — all suites green.

- [ ] **Step 2: Typecheck + build**

Run: `npm run build`
Expected: `vue-tsc --noEmit` passes (no type errors from the new `canGoNext` prop / emits / `addDays`) and Vite build succeeds.

- [ ] **Step 3: Manual smoke (optional but recommended)**

Run: `npm run tauri dev`
Verify by hand:
- DayHeader `‹ ›` switch days; `›` is greyed on today.
- `⌘[`/`⌘]` change day; `⌘⇧[`/`⌘⇧]` change month.
- Input composer shows only `@ dim` / `# time`.
- Day note sits directly under the date header.
- Entry rows show hairline separators at rest.
- Select a past day, switch to another app and back → stays on that day. (Midnight-follow can't be smoke-tested without waiting; it's covered by unit tests.)

- [ ] **Step 4: Commit (only if Step 3 surfaced fixes)**

```bash
git add -A
git commit -m "test: verify UX improvements end-to-end"
```

---

## Self-Review

**Spec coverage:**
- ① Day nav controls → Task 2 (DayHeader arrows); shortcuts/`shiftDay`/guard → Task 3; input hint trim → Task 4; sidebar tooltip → Task 5. ✓
- ② Note relocation → Task 6. ✓
- ③ Hairline dividers → Task 7. ✓
- ④ Focus reset matrix → Task 8. ✓
- Future-day guard (control + shortcut) → Task 2 (`canGoNext`) + Task 3 (`shiftDay` delta>0 guard + `⌘]` routing through `shiftDay`). ✓
- `addDays` util → Task 1. ✓

**Placeholder scan:** No TBD/TODO; every code step shows full code. Test helpers that depend on file-local conventions (`mountRow`, `mountCalendar`) include explicit fallback code. ✓

**Type/name consistency:** `addDays(dateStr, n)` defined in Task 1, used in Tasks 3 (component) and 3/7 (tests). DayHeader prop `canGoNext` (camelCase in props, `:can-go-next` in template) and emits `prev-day`/`next-day` consistent across Tasks 2 & 3. `shiftDay`/`shiftMonth` names consistent. `data-test` hooks: `prev-day`, `next-day`, `entry-row`, `prev-month`, `next-month` all match existing or newly-added markup. ✓
