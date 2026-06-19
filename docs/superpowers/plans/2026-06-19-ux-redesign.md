# Logbook UX Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the Logbook frontend from the current dual-column form layout into the three-zone "professional tool" UX (sidebar heatmap + commitments, main entry list, persistent two-line input) defined by the calibrated spec and demo.

**Architecture:** A 220px sidebar (`HeatmapCalendar` + `CommitmentsPanel`) and a flex-1 main column (`DayHeader` → scrollable `EntryList` → `DayNote` → `TwoLineInput` → file path), composed by a rewritten `MonthView`. All visual values come from CSS custom properties in `src/assets/tokens.css`. The Rust backend (commands, data model, file format) is **unchanged** — every mutation still flows through the existing `invoke()` commands, orchestrated by `MonthView`.

**Tech Stack:** Vue 3 (Composition API, `<script setup>`) + TypeScript + Tailwind CSS v4 (utilities reference tokens via `var()` in arbitrary-value classes) + Vitest + @vue/test-utils (jsdom). Backend: Tauri 2.x + Rust (untouched).

**Design references (read before starting):**
- `docs/superpowers/specs/2026-06-19-ux-redesign-design.md` — component spec tables (canonical visual values, calibrated)
- `docs/superpowers/specs/2026-06-19-ux-redesign-tokens.css` — token blueprint; its contents **overwrite** `src/assets/tokens.css`
- `demos/UX-REDESIGN-DEMO.html` — visual source of truth

**Conventions in this codebase (follow exactly):**
- Tokens used via Tailwind arbitrary values: `class="text-[var(--app-text-base)] bg-[var(--color-surface)]"`.
- Mono/numeric text uses a `.mono` class (added in Phase 0), matching the demo.
- Tests live in `src/__tests__/components/Foo.test.ts` and import components as `../../components/Foo.vue` (composite: `../../components/composite/Foo.vue`). **Two `../`, not three.**
- jsdom does not compute Tailwind styles — tests assert on **structure, text, emitted events, and `data-test` hooks**, never on colors/pixels.
- Components that need config/commitments either receive them as **props** (input components) or read `useStore()` (row/container components). This plan follows the existing split.

---

## File Structure

### New files (10 components + 1 util + their tests)
- `src/utils/heatmap.ts` — pure `heatLevel(minutes)` classifier for calendar cells
- `src/components/QuickJumpPopover.vue` — year/month dual-select jump popover
- `src/components/HeatmapCalendar.vue` — sidebar month nav + heatmap grid + month total (replaces `MonthNavigator` + `DayStrip`); hosts `QuickJumpPopover`
- `src/components/DimensionPopover.vue` — `@`-triggered two-phase dimension/value picker (replaces `MentionMenu`)
- `src/components/TwoLineInput.vue` — two-line entry input (replaces `EntryInput` + `QuickEntry` + `DimensionPanel`); hosts `DimensionPopover`
- `src/components/DayHeader.vue` — day title + Today badge + entry-count/total summary
- `src/components/composite/EntryRowEdit.vue` — inline edit mode for an entry (item + duration + chips + Save/Cancel/Delete); hosts `DimensionPopover`
- Tests: `src/__tests__/heatmap.test.ts`, `src/__tests__/components/QuickJumpPopover.test.ts`, `HeatmapCalendar.test.ts`, `DimensionPopover.test.ts`, `TwoLineInput.test.ts`, `DayHeader.test.ts`, `src/__tests__/components/composite/EntryRowEdit.test.ts`

### Rewritten files (5)
- `src/assets/tokens.css` — overwritten with the calibrated token set
- `src/components/composite/EntryRow.vue` — new visual spec + hover `⋯` trigger + double-click → `EntryRowEdit`
- `src/components/EntryList.vue` — rows only (summary moves to `DayHeader`), new visual spec
- `src/components/CommitmentsPanel.vue` — per-role expand/collapse + brand-gradient progress bar; keeps `CommitmentsEditor` edit flow
- `src/components/MonthView.vue` — three-zone layout, owns `append_entry`, owns `⌘[`/`⌘]` month navigation

### Modified files (2)
- `src/assets/main.css` — `var(--app-font)` → `var(--app-font-body)`; add `.mono` utility
- `src/App.vue` — no logic change expected; verify it still mounts `MonthView` (touch only if a prop/event contract changed)

### Deleted files (10 components + their tests)
Components: `DayStrip.vue`, `MonthNavigator.vue`, `QuickEntry.vue`, `DimensionPanel.vue`, `EntryInput.vue`, `composite/MentionMenu.vue`, `base/AppInput.vue`, `base/AppChip.vue`, `base/AppSelect.vue`, `base/Popover.vue`
Tests: matching `*.test.ts` for each of the above that exists.

### Retained files (unchanged)
- `SetupScreen.vue`, `ConfigErrorBanner.vue`, `base/Toast.vue`, `base/AppButton.vue`, `base/ProgressBar.vue`, `composite/CommitmentsEditor.vue`
- All of `src/stores/`, `src/types.ts`, `src/utils/format.ts`, `src/utils/dates.ts`, `src-tauri/**`

### Deferred (explicitly out of scope — see Self-Review)
- `⌘K` command palette (listed in spec §5.1 but no component exists in spec §4 or the demo)
- `#` manual-duration trigger (spec §5.2 method B) — duration auto-parses from item text (method A); `#` is a no-op hint for now

---

## Phase 0: Token Foundation

### Task 0.1: Overwrite tokens.css and fix main.css

**Files:**
- Rewrite: `src/assets/tokens.css`
- Modify: `src/assets/main.css`

- [ ] **Step 1: Overwrite `src/assets/tokens.css`**

Copy the **entire** contents of `docs/superpowers/specs/2026-06-19-ux-redesign-tokens.css` into `src/assets/tokens.css`, replacing the file. (That file is the calibrated blueprint: `--app-text-*` / `--app-font-*` prefixes, `--color-*`, `--color-token-*`, `--color-chip-*`, `--heatmap-*`, `--dim-bar-*`, `--color-popover-*`, `--anim-highlight-*`, plus dark-mode and reduced-motion blocks.)

- [ ] **Step 2: Fix the body font reference in `src/assets/main.css`**

The new tokens rename `--app-font` → `--app-font-body`. Update the one reference and add the `.mono` utility used across the redesign.

```css
@import './tokens.css';
@import 'tailwindcss';

body {
  font-family: var(--app-font-body);
  background-color: var(--color-page-bg);
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)' opacity='0.03'/%3E%3C/svg%3E");
  color: var(--color-text-primary);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* Mono/numeric text (durations, counts, keyboard hints) — matches demo `.mono` */
.mono {
  font-family: var(--app-font-mono);
  font-variant-numeric: tabular-nums;
}

@media (prefers-color-scheme: dark) {
  body { background-image: none; }
}

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

- [ ] **Step 3: Verify the suite still compiles and passes**

Run: `pnpm test && npx vue-tsc --noEmit`
Expected: all existing tests PASS, type-check CLEAN. (The current components still reference now-removed tokens only if they are scheduled for deletion/rewrite later — at this point the app still imports them, so the build must stay green. If `vue-tsc` or a test fails because a *retained* component references a removed token, STOP: re-run the audit `grep -rE '\-\-(app-font[^-]|radius-pill|radius-popover|spacing-)' src/components/{SetupScreen,ConfigErrorBanner}.vue src/components/base` and add the missing token back to `tokens.css` as a clearly-commented legacy alias.)

- [ ] **Step 4: Commit**

```bash
git add src/assets/tokens.css src/assets/main.css
git commit -m "feat(tokens): overwrite tokens.css with calibrated UX-redesign set"
```

---

## Phase 1: Leaf Components (no internal component dependencies)

### Task 1.1: heatmap level utility

**Files:**
- Create: `src/utils/heatmap.ts`
- Test: `src/__tests__/heatmap.test.ts`

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/heatmap.test.ts
import { describe, it, expect } from "vitest";
import { heatLevel } from "../utils/heatmap";

describe("heatLevel", () => {
  it("returns 'empty' for zero or negative minutes", () => {
    expect(heatLevel(0)).toBe("empty");
    expect(heatLevel(-5)).toBe("empty");
  });
  it("returns 'light' for under 2h", () => {
    expect(heatLevel(1)).toBe("light");
    expect(heatLevel(119)).toBe("light");
  });
  it("returns 'mid' for 2h to under 5h", () => {
    expect(heatLevel(120)).toBe("mid");
    expect(heatLevel(299)).toBe("mid");
  });
  it("returns 'heavy' for 5h and above", () => {
    expect(heatLevel(300)).toBe("heavy");
    expect(heatLevel(600)).toBe("heavy");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/heatmap.test.ts`
Expected: FAIL — "Cannot find module '../utils/heatmap'".

- [ ] **Step 3: Implement**

```typescript
// src/utils/heatmap.ts
export type HeatLevel = "empty" | "light" | "mid" | "heavy";

/** Classify a day's total logged minutes into a heatmap intensity bucket.
 *  Thresholds: 0 → empty, <2h → light, <5h → mid, >=5h → heavy. */
export function heatLevel(totalMinutes: number): HeatLevel {
  if (totalMinutes <= 0) return "empty";
  if (totalMinutes < 120) return "light";
  if (totalMinutes < 300) return "mid";
  return "heavy";
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/heatmap.test.ts`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/utils/heatmap.ts src/__tests__/heatmap.test.ts
git commit -m "feat: add heatLevel classifier for calendar heatmap"
```

### Task 1.2: QuickJumpPopover

**Files:**
- Create: `src/components/QuickJumpPopover.vue`
- Test: `src/__tests__/components/QuickJumpPopover.test.ts`

Spec §4.2. Props: current `year`, `month`, and `availableMonths`. Emits `jump {year, month}`. Year select lists unique years; month select lists months available for the selected year. (Logic lifted from the old `MonthNavigator` popover, isolated.)

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/components/QuickJumpPopover.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import QuickJumpPopover from "../../components/QuickJumpPopover.vue";
import type { AvailableMonth } from "../../stores/useStore";

const months: AvailableMonth[] = [
  { year: 2026, month: 6 },
  { year: 2026, month: 3 },
  { year: 2025, month: 8 },
];

function mountPop() {
  return mount(QuickJumpPopover, { props: { year: 2026, month: 6, availableMonths: months } });
}

describe("QuickJumpPopover", () => {
  it("year select lists unique years, descending", () => {
    const wrapper = mountPop();
    const years = wrapper.findAll("select")[0].findAll("option").map(o => parseInt(o.element.value, 10));
    expect(years).toEqual([2026, 2025]);
  });

  it("month select shows only months for the selected year", () => {
    const wrapper = mountPop();
    const monthVals = wrapper.findAll("select")[1].findAll("option").map(o => parseInt(o.element.value, 10));
    expect(monthVals).toEqual(expect.arrayContaining([3, 6]));
    expect(monthVals).not.toContain(8);
  });

  it("changing the month select emits jump", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("select")[1].setValue(3);
    expect(wrapper.emitted("jump")?.[0]).toEqual([{ year: 2026, month: 3 }]);
  });

  it("changing the year then month emits jump with the new year", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("select")[0].setValue(2025);
    await wrapper.findAll("select")[1].setValue(8);
    expect(wrapper.emitted("jump")?.[0]).toEqual([{ year: 2025, month: 8 }]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/QuickJumpPopover.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

```vue
<!-- src/components/QuickJumpPopover.vue -->
<script setup lang="ts">
import { ref, computed } from "vue";
import type { AvailableMonth } from "../stores/useStore";

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const props = defineProps<{
  year: number;
  month: number;
  availableMonths: AvailableMonth[];
}>();

const emit = defineEmits<{ jump: [{ year: number; month: number }] }>();

const selectedYear = ref(props.year);

const years = computed(() => {
  const ys = [...new Set(props.availableMonths.map(m => m.year))];
  ys.sort((a, b) => b - a);
  return ys;
});

const monthsForYear = computed(() =>
  props.availableMonths
    .filter(m => m.year === selectedYear.value)
    .map(m => m.month)
    .sort((a, b) => a - b)
);

function onMonthChange(month: number) {
  emit("jump", { year: selectedYear.value, month });
}
</script>

<template>
  <div
    class="flex gap-[8px] items-center bg-[var(--color-surface)] border border-[var(--color-border-form)]
           rounded-[var(--radius-form-lg)] shadow-[var(--shadow-quickjump)] px-[12px] py-[10px]"
  >
    <select
      v-model.number="selectedYear"
      class="text-[var(--app-text-xs)] text-[var(--color-text-primary)] bg-[var(--color-surface)]
             border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] px-[8px] py-[4px] outline-none"
    >
      <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
    </select>
    <select
      class="text-[var(--app-text-xs)] text-[var(--color-text-primary)] bg-[var(--color-surface)]
             border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] px-[8px] py-[4px] outline-none"
      @change="onMonthChange(parseInt(($event.target as HTMLSelectElement).value, 10))"
    >
      <option
        v-for="m in monthsForYear" :key="m" :value="m"
        :selected="m === month && selectedYear === year"
      >{{ MONTH_NAMES[m - 1] }}</option>
    </select>
    <span class="text-[var(--app-text-2xs)] text-[var(--color-text-secondary)] whitespace-nowrap">Go</span>
  </div>
</template>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/QuickJumpPopover.test.ts`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/components/QuickJumpPopover.vue src/__tests__/components/QuickJumpPopover.test.ts
git commit -m "feat: add QuickJumpPopover year/month jump"
```

### Task 1.3: DimensionPopover

**Files:**
- Create: `src/components/DimensionPopover.vue`
- Test: `src/__tests__/components/DimensionPopover.test.ts`

Spec §4.9. Props: `dimensions`, `commitments`, `dimValues`. Emits `select [dimKey, value]` and `close`. Two internal phases: `dim` (DIM-badge header, colored left bar per dim, required/optional meta) → `val` (warm-grey header with ← back, value list). After selecting a value, if required dims remain it returns to `dim` phase; otherwise emits `close`. Esc handling and re-trigger are owned by the parent (`TwoLineInput`/`EntryRowEdit`). Logic mirrors the old `MentionMenu`; visuals follow the calibrated spec.

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/components/DimensionPopover.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import DimensionPopover from "../../components/DimensionPopover.vue";
import { makeDimension, makeCommitment } from "../mocks/fixtures";

const dimensions = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering", "PM"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
  makeDimension({ name: "Business Line", key: "business-line", source: "static", values: ["Slax"], required: false }),
];
const commitments = [makeCommitment({ goals: ["Bug fixes", "Code review"] })];

function mountPop(dimValues: Record<string, string> = {}) {
  return mount(DimensionPopover, { props: { dimensions, commitments, dimValues } });
}

describe("DimensionPopover", () => {
  it("lists all dimensions with required/optional meta in dim phase", () => {
    const wrapper = mountPop();
    expect(wrapper.text()).toContain("Category");
    expect(wrapper.text()).toContain("Goal");
    expect(wrapper.text()).toContain("required");
    expect(wrapper.text()).toContain("optional");
  });

  it("shows static dimension values after selecting a dimension", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // Category
    expect(wrapper.text()).toContain("Engineering");
    expect(wrapper.text()).toContain("PM");
  });

  it("shows monthly goal options for a monthly-source dimension", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[1].trigger("click"); // Goal
    expect(wrapper.text()).toContain("Bug fixes");
    expect(wrapper.text()).toContain("Code review");
  });

  it("emits select with [dimKey, value] when a value is chosen", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click");
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click");
    expect(wrapper.emitted("select")?.[0]).toEqual(["category", "Engineering"]);
  });

  it("emits close once all required dimensions are filled after a selection", async () => {
    // category already filled; selecting goal value fills the last required dim
    const wrapper = mountPop({ category: "Engineering" });
    await wrapper.findAll("[data-test='dim-item']")[1].trigger("click"); // Goal
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click"); // Bug fixes
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("back button returns from val phase to dim phase", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click");
    await wrapper.find("[data-test='back-btn']").trigger("click");
    expect(wrapper.findAll("[data-test='dim-item']").length).toBe(3);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/DimensionPopover.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

```vue
<!-- src/components/DimensionPopover.vue -->
<script setup lang="ts">
import { ref, computed } from "vue";
import type { Dimension, Commitment } from "../types";

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  dimValues: Record<string, string>;
}>();

const emit = defineEmits<{
  select: [dimKey: string, value: string];
  close: [];
}>();

const phase = ref<"dim" | "val">("dim");
const activeDimKey = ref<string | null>(null);

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of props.commitments) for (const g of c.goals) goals.add(g);
  return [...goals];
});

const activeDim = computed(() => props.dimensions.find(d => d.key === activeDimKey.value) || null);

const activeValues = computed(() => {
  const d = activeDim.value;
  if (!d) return [];
  return d.source === "monthly" ? goalOptions.value : (d.values || []);
});

// Map a dimension key to its left-bar token class.
function barClass(key: string): string {
  const map: Record<string, string> = {
    category: "bg-[var(--dim-bar-cat)]",
    "business-line": "bg-[var(--dim-bar-biz)]",
    "importance-urgency": "bg-[var(--dim-bar-imp)]",
    goal: "bg-[var(--dim-bar-goal)]",
  };
  return map[key] || "bg-[var(--dim-bar-cat)]";
}

function selectDim(key: string) {
  activeDimKey.value = key;
  phase.value = "val";
}

function selectVal(value: string) {
  if (!activeDimKey.value) return;
  emit("select", activeDimKey.value, value);
  const allFilled = props.dimensions
    .filter(d => d.required)
    .every(d => props.dimValues[d.key]);
  if (allFilled) {
    emit("close");
  } else {
    phase.value = "dim";
    activeDimKey.value = null;
  }
}

function goBack() {
  phase.value = "dim";
  activeDimKey.value = null;
}
</script>

<template>
  <div
    class="w-[240px] bg-[var(--color-surface)] border border-[var(--color-border-form)]
           rounded-[var(--radius-card)] shadow-[var(--shadow-popover)] overflow-hidden"
  >
    <!-- Dim phase -->
    <template v-if="phase === 'dim'">
      <div
        class="px-[14px] py-[8px] text-[var(--app-text-micro)] font-bold uppercase tracking-wider
               text-[var(--color-popover-dim-header-text)] bg-[var(--color-popover-dim-header-bg)]
               border-b border-[var(--color-divider)] flex items-center gap-[8px]"
      >
        <span class="bg-[var(--color-brand-solid)] text-white px-[6px] py-[1px] rounded-[var(--radius-sm)] text-[var(--app-text-2xs)]">DIM</span>
        Pick a dimension
      </div>
      <div
        v-for="d in dimensions" :key="d.key"
        data-test="dim-item"
        class="px-[14px] py-[9px] text-[var(--app-text-sm)] text-[var(--color-text-primary)]
               flex items-center gap-[10px] cursor-pointer border-b border-[var(--color-surface-muted)]
               last:border-b-0 hover:bg-[var(--color-divider)]"
        @click="selectDim(d.key)"
      >
        <span class="w-[3px] h-[18px] rounded-[var(--radius-sm)] flex-shrink-0" :class="barClass(d.key)"></span>
        {{ d.name }}
        <span
          class="ml-auto text-[var(--app-text-micro)]"
          :class="d.required ? 'text-[var(--color-warning)] font-medium' : 'text-[var(--color-text-disabled)]'"
        >{{ d.required ? "required" : "optional" }}</span>
      </div>
      <div
        class="px-[14px] py-[6px] text-[var(--app-text-2xs)] text-[var(--color-text-disabled)]
               border-t border-[var(--color-divider)] flex gap-[12px]"
      >
        <span><kbd class="mono px-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">↵</kbd> select</span>
        <span><kbd class="mono px-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">esc</kbd> close</span>
      </div>
    </template>

    <!-- Val phase -->
    <template v-else>
      <div
        class="px-[14px] py-[8px] text-[var(--app-text-micro)] font-bold uppercase tracking-wider
               text-[var(--color-popover-val-header-text)] bg-[var(--color-popover-val-header-bg)]
               border-b border-[var(--color-divider)] flex items-center gap-[8px]"
      >
        <button data-test="back-btn" class="font-bold cursor-pointer leading-none" @click="goBack">←</button>
        {{ activeDim?.name }}
      </div>
      <div
        v-for="v in activeValues" :key="v"
        data-test="val-item"
        class="px-[14px] py-[9px] text-[var(--app-text-sm)] text-[var(--color-text-primary)]
               cursor-pointer border-b border-[var(--color-surface-muted)] last:border-b-0
               hover:bg-[var(--color-divider)]"
        @click="selectVal(v)"
      >{{ v }}</div>
      <div
        class="px-[14px] py-[6px] text-[var(--app-text-2xs)] text-[var(--color-text-disabled)]
               border-t border-[var(--color-divider)] flex gap-[12px]"
      >
        <span><kbd class="mono px-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">↵</kbd> select</span>
        <span><kbd class="mono px-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">esc</kbd> back to dims</span>
      </div>
    </template>
  </div>
</template>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/DimensionPopover.test.ts`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add src/components/DimensionPopover.vue src/__tests__/components/DimensionPopover.test.ts
git commit -m "feat: add DimensionPopover two-phase picker"
```

### Task 1.4: DayHeader

**Files:**
- Create: `src/components/DayHeader.vue`
- Test: `src/__tests__/components/DayHeader.test.ts`

Spec §4.4. Props: `title` (e.g. "Thursday, June 19"), `isToday`, `entryCount`, `totalMinutes`. Pure presentational. Summary uses `.mono` for the count and total.

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/components/DayHeader.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import DayHeader from "../../components/DayHeader.vue";

describe("DayHeader", () => {
  it("renders title and formatted summary", () => {
    const wrapper = mount(DayHeader, {
      props: { title: "Thursday, June 19", isToday: true, entryCount: 10, totalMinutes: 345 },
    });
    expect(wrapper.text()).toContain("Thursday, June 19");
    expect(wrapper.text()).toContain("10");
    expect(wrapper.text()).toContain("5h 45m");
  });

  it("shows Today badge only when isToday is true", () => {
    const today = mount(DayHeader, { props: { title: "X", isToday: true, entryCount: 0, totalMinutes: 0 } });
    expect(today.find("[data-test='today-badge']").exists()).toBe(true);
    const past = mount(DayHeader, { props: { title: "X", isToday: false, entryCount: 0, totalMinutes: 0 } });
    expect(past.find("[data-test='today-badge']").exists()).toBe(false);
  });

  it("uses singular 'entry' for a count of 1", () => {
    const wrapper = mount(DayHeader, { props: { title: "X", isToday: false, entryCount: 1, totalMinutes: 60 } });
    expect(wrapper.text()).toContain("1 entry");
    expect(wrapper.text()).not.toContain("1 entries");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/DayHeader.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

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
}>();

const countLabel = computed(() => (props.entryCount === 1 ? "entry" : "entries"));
const total = computed(() => formatDuration(props.totalMinutes));
</script>

<template>
  <div class="flex justify-between items-baseline mb-[20px] pb-[14px] border-b border-[var(--color-divider)]">
    <div>
      <span class="text-[var(--app-text-xl)] font-bold text-[var(--color-text-primary)] tracking-[-0.3px]">{{ title }}</span>
      <span
        v-if="isToday"
        data-test="today-badge"
        class="ml-[6px] align-middle text-[var(--app-text-micro)] font-semibold
               text-[var(--color-brand-link)] bg-[var(--color-brand-soft-bg)] px-[8px] py-[2px] rounded-[var(--radius-md)]"
      >Today</span>
    </div>
    <span class="text-[var(--app-text-xs)] text-[var(--color-text-secondary)]">
      <span class="mono">{{ entryCount }}</span> {{ countLabel }} · <span class="mono">{{ total }}</span>
    </span>
  </div>
</template>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/DayHeader.test.ts`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/components/DayHeader.vue src/__tests__/components/DayHeader.test.ts
git commit -m "feat: add DayHeader with title, Today badge, summary"
```

---

## Phase 2: Composite Components (depend on Phase 1 leaves)

### Task 2.1: TwoLineInput

**Files:**
- Create: `src/components/TwoLineInput.vue`
- Test: `src/__tests__/components/TwoLineInput.test.ts`

Spec §4.8, §5.2. Props: `dimensions`, `commitments`, `initialValues`. Emits `submit [item, durationMinutes, dimensions]`. Line 1: `+` prefix, item `<input>`, `⏎` badge. Line 2: a token chip per filled dimension (removable), a duration token if the item text parses to a duration, and a dashed "missing" indicator per unfilled required dimension. `@` opens `DimensionPopover`; selecting fills `dimValues`. `Enter` submits when item text + a parsed duration are present (required dims are a *soft* hint — never block). Exposes `clearInput()`. Auto-focuses on `focusRequestId` bumps when nothing else is focused (carried over from the old `EntryInput`).

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/components/TwoLineInput.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import TwoLineInput from "../../components/TwoLineInput.vue";
import { makeDimension, makeCommitment } from "../mocks/fixtures";

const dimensions = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
];
const commitments = [makeCommitment({ goals: ["Bug fixes"] })];

function mountInput(initialValues: Record<string, string> = {}) {
  return mount(TwoLineInput, { props: { dimensions, commitments, initialValues } });
}

describe("TwoLineInput", () => {
  it("renders the item input and the Enter hint", () => {
    const wrapper = mountInput();
    expect(wrapper.find("input").exists()).toBe(true);
    expect(wrapper.text()).toContain("⏎");
  });

  it("shows a duration token parsed from the item text", async () => {
    const wrapper = mountInput();
    await wrapper.find("input").setValue("Code review 1.5h");
    expect(wrapper.find("[data-test='dur-token']").text()).toContain("1h 30m");
  });

  it("shows a missing indicator per unfilled required dimension", () => {
    const wrapper = mountInput();
    const missing = wrapper.findAll("[data-test='missing']");
    expect(missing.length).toBe(2);
    expect(wrapper.text()).toContain("Category");
    expect(wrapper.text()).toContain("Goal");
  });

  it("emits submit with item, minutes, and dimensions on Enter", async () => {
    const wrapper = mountInput({ category: "Engineering" });
    await wrapper.find("input").setValue("Code review 1h");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")?.[0]).toEqual(["Code review", 60, { category: "Engineering" }]);
  });

  it("does NOT emit submit when there is no parseable duration", async () => {
    const wrapper = mountInput({ category: "Engineering" });
    await wrapper.find("input").setValue("Code review");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")).toBeFalsy();
    expect(wrapper.text()).toContain("Need a duration");
  });

  it("submits even when required dimensions are missing (soft hint)", async () => {
    const wrapper = mountInput(); // nothing filled
    await wrapper.find("input").setValue("Quick note 30m");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")?.[0]).toEqual(["Quick note", 30, {}]);
  });

  it("opens DimensionPopover on @ keydown", async () => {
    const wrapper = mountInput();
    await wrapper.find("input").trigger("keydown", { key: "@" });
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
  });

  it("removes a dimension token when its × is clicked", async () => {
    const wrapper = mountInput({ category: "Engineering" });
    expect(wrapper.find("[data-test='dim-token']").exists()).toBe(true);
    await wrapper.find("[data-test='dim-token-remove']").trigger("click");
    expect(wrapper.find("[data-test='dim-token']").exists()).toBe(false);
  });

  it("clearInput() empties the field", async () => {
    const wrapper = mountInput();
    await wrapper.find("input").setValue("Something 1h");
    (wrapper.vm as unknown as { clearInput: () => void }).clearInput();
    await wrapper.vm.$nextTick();
    expect((wrapper.find("input").element as HTMLInputElement).value).toBe("");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/TwoLineInput.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

```vue
<!-- src/components/TwoLineInput.vue -->
<script setup lang="ts">
import { ref, computed, inject, watch, type Ref } from "vue";
import type { Dimension, Commitment } from "../types";
import { parseDurationFromText, stripDurations, formatDuration } from "../utils/format";
import DimensionPopover from "./DimensionPopover.vue";

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  initialValues: Record<string, string>;
}>();

const emit = defineEmits<{
  submit: [item: string, durationMinutes: number, dimensions: Record<string, string>];
}>();

const text = ref("");
const inputEl = ref<HTMLInputElement | null>(null);
const popoverOpen = ref(false);
const dimValues = ref<Record<string, string>>({ ...props.initialValues });

watch(
  () => props.initialValues,
  (vals) => { if (Object.keys(vals).length > 0) dimValues.value = { ...vals }; },
  { immediate: true }
);

const parsedDuration = computed(() => {
  const t = text.value.trim();
  return t ? parseDurationFromText(t) : null;
});

const filledDims = computed(() => props.dimensions.filter(d => dimValues.value[d.key]));
const missingRequired = computed(() => props.dimensions.filter(d => d.required && !dimValues.value[d.key]));

function tokenClass(key: string): string {
  const map: Record<string, string> = {
    category: "bg-[var(--color-token-cat-bg)] text-[var(--color-token-cat-text)]",
    "business-line": "bg-[var(--color-token-biz-bg)] text-[var(--color-token-biz-text)]",
    "importance-urgency": "bg-[var(--color-token-imp-bg)] text-[var(--color-token-imp-text)]",
    goal: "bg-[var(--color-token-goal-bg)] text-[var(--color-token-goal-text)]",
  };
  return map[key] || map.category;
}

function removeDim(key: string) {
  const next = { ...dimValues.value };
  delete next[key];
  dimValues.value = next;
}

function onSelect(dimKey: string, value: string) {
  dimValues.value = { ...dimValues.value, [dimKey]: value };
}

function closePopover() {
  popoverOpen.value = false;
  inputEl.value?.focus();
}

function onKeydown(e: KeyboardEvent) {
  if (popoverOpen.value) {
    if (e.key === "Escape") { e.preventDefault(); closePopover(); }
    return;
  }
  if (e.key === "@") { e.preventDefault(); popoverOpen.value = true; return; }
  if (e.key === "Enter") { e.preventDefault(); handleSubmit(); return; }
}

function handleSubmit() {
  const trimmed = text.value.trim();
  if (!trimmed) return;
  const d = parsedDuration.value;
  if (!d) return; // duration required; template shows the hint
  const item = stripDurations(trimmed);
  emit("submit", item, d, { ...dimValues.value });
}

function clearInput() {
  text.value = "";
}

defineExpose({ clearInput });

const focusRequestId = inject<Ref<number>>("focusRequestId", ref(0));
watch(focusRequestId, () => {
  const active = document.activeElement;
  if (!active || active === document.body || active.tagName === "BODY") {
    inputEl.value?.focus();
  }
});
</script>

<template>
  <div class="relative">
    <div
      class="bg-[var(--color-surface)] border-2 border-[var(--color-border-form)] rounded-[var(--radius-card)] px-[16px] py-[10px]
             focus-within:border-[var(--color-brand-solid)] focus-within:shadow-[var(--shadow-focus-ring)] transition-all"
    >
      <!-- Line 1: item text -->
      <div class="flex gap-[8px] items-center">
        <span class="text-[var(--app-text-lg)] leading-none text-[var(--color-brand-solid)] flex-shrink-0">+</span>
        <input
          ref="inputEl"
          v-model="text"
          placeholder="What did you work on?"
          class="flex-1 border-none outline-none bg-transparent text-[var(--app-text-base)]
                 text-[var(--color-text-primary)] placeholder:text-[var(--color-placeholder)]
                 caret-[var(--color-brand-solid)] leading-[1.5] py-[2px]"
          @keydown="onKeydown"
        />
        <span class="mono text-[var(--app-text-2xs)] font-semibold text-[var(--color-text-secondary)]
                     border border-[var(--color-border-form)] rounded-[var(--radius-md)] px-[7px] py-[3px] flex-shrink-0">⏎</span>
      </div>

      <!-- Line 2: tokens + missing indicators -->
      <div class="flex gap-[4px] mt-[6px] flex-wrap items-center min-h-[4px] pl-[2px]">
        <span
          v-for="d in filledDims" :key="d.key"
          data-test="dim-token"
          class="text-[var(--app-text-micro)] font-medium px-[7px] py-[1px] rounded-[var(--radius-sm)] inline-flex items-center gap-[4px] leading-[1.6]"
          :class="tokenClass(d.key)"
        >
          {{ dimValues[d.key] }}
          <span data-test="dim-token-remove" class="cursor-pointer opacity-40 hover:opacity-100 text-[var(--app-text-xs)] leading-none" @click="removeDim(d.key)">×</span>
        </span>

        <span
          v-if="parsedDuration"
          data-test="dur-token"
          class="mono text-[var(--app-text-micro)] font-medium px-[7px] py-[1px] rounded-[var(--radius-sm)] inline-flex items-center gap-[4px] leading-[1.6]
                 bg-[var(--color-token-dur-bg)] text-[var(--color-token-dur-text)]"
        >{{ formatDuration(parsedDuration) }}</span>

        <span
          v-for="m in missingRequired" :key="'missing-' + m.key"
          data-test="missing"
          class="text-[var(--app-text-micro)] font-[450] px-[8px] py-[1px] rounded-[var(--radius-sm)]
                 border-[1.5px] border-dashed border-[var(--color-missing-border)] text-[var(--color-missing-text)]
                 inline-flex items-center gap-[3px] cursor-pointer hover:border-[var(--color-text-muted)] hover:text-[var(--color-text-secondary)]"
          @click="popoverOpen = true"
        >
          <span class="w-[5px] h-[5px] rounded-full bg-[var(--color-missing-dot)]"></span>{{ m.name }}
        </span>

        <span v-if="text.trim() && !parsedDuration" class="text-[var(--app-text-micro)] text-[var(--color-warning)]">
          Need a duration — type <code class="mono">1h</code>
        </span>
      </div>
    </div>

    <DimensionPopover
      v-if="popoverOpen"
      :dimensions="dimensions"
      :commitments="commitments"
      :dim-values="dimValues"
      class="absolute left-0 top-full mt-[4px] z-10"
      @select="onSelect"
      @close="closePopover"
    />

    <!-- Hints -->
    <div class="flex gap-[14px] mt-[4px] text-[var(--app-text-micro)] text-[var(--color-text-disabled)]">
      <span><kbd class="mono px-[5px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)] text-[var(--app-text-2xs)]">@</kbd> dim</span>
      <span><kbd class="mono px-[5px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)] text-[var(--app-text-2xs)]">⌘[</kbd> prev month</span>
      <span><kbd class="mono px-[5px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)] text-[var(--app-text-2xs)]">⌘]</kbd> next month</span>
    </div>
  </div>
</template>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/TwoLineInput.test.ts`
Expected: PASS (9 tests).

- [ ] **Step 5: Commit**

```bash
git add src/components/TwoLineInput.vue src/__tests__/components/TwoLineInput.test.ts
git commit -m "feat: add TwoLineInput with auto-duration-parse and dimension tokens"
```

### Task 2.2: EntryRowEdit

**Files:**
- Create: `src/components/composite/EntryRowEdit.vue`
- Test: `src/__tests__/components/composite/EntryRowEdit.test.ts`

Spec §4.6. Props: `entry`, `dimensions`, `commitments`. Emits `save [item, durationMinutes, dimensions]`, `cancel`, `delete`. Item `<input>`, a numeric duration `<input>` (minutes, label "min"), one removable chip per filled dimension, a `+ tag` chip that opens `DimensionPopover`, and Save/Cancel/Delete buttons. Duration is parsed with `resolveDelta` (so `+15`, `90`, `1.5*60` all work, matching the existing duration-edit behaviour).

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/components/composite/EntryRowEdit.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import EntryRowEdit from "../../../components/composite/EntryRowEdit.vue";
import { makeEntry, makeDimension, makeCommitment } from "../../mocks/fixtures";

const dimensions = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
];
const commitments = [makeCommitment({ goals: ["Bug fixes"] })];

function mountEdit(entryOverrides = {}) {
  const entry = makeEntry({ item: "Old item", duration: 45, dimensions: { category: "Engineering" }, ...entryOverrides });
  return mount(EntryRowEdit, { props: { entry, dimensions, commitments } });
}

describe("EntryRowEdit", () => {
  it("pre-fills item and duration from the entry", () => {
    const wrapper = mountEdit();
    const inputs = wrapper.findAll("input");
    expect((inputs[0].element as HTMLInputElement).value).toBe("Old item");
    expect((inputs[1].element as HTMLInputElement).value).toBe("45");
  });

  it("emits save with edited values", async () => {
    const wrapper = mountEdit();
    const inputs = wrapper.findAll("input");
    await inputs[0].setValue("New item");
    await inputs[1].setValue("60");
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")?.[0]).toEqual(["New item", 60, { category: "Engineering" }]);
  });

  it("resolves a delta duration like +15", async () => {
    const wrapper = mountEdit();
    await wrapper.findAll("input")[1].setValue("+15");
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")?.[0]).toEqual(["Old item", 60, { category: "Engineering" }]);
  });

  it("emits cancel", async () => {
    const wrapper = mountEdit();
    await wrapper.find("[data-test='cancel']").trigger("click");
    expect(wrapper.emitted("cancel")).toBeTruthy();
  });

  it("emits delete", async () => {
    const wrapper = mountEdit();
    await wrapper.find("[data-test='delete']").trigger("click");
    expect(wrapper.emitted("delete")).toBeTruthy();
  });

  it("removes a dimension chip and excludes it from save", async () => {
    const wrapper = mountEdit();
    await wrapper.find("[data-test='chip-remove']").trigger("click");
    await wrapper.find("[data-test='save']").trigger("click");
    expect(wrapper.emitted("save")?.[0]).toEqual(["Old item", 45, {}]);
  });

  it("opens DimensionPopover when + tag is clicked", async () => {
    const wrapper = mountEdit();
    await wrapper.find("[data-test='add-tag']").trigger("click");
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/composite/EntryRowEdit.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

```vue
<!-- src/components/composite/EntryRowEdit.vue -->
<script setup lang="ts">
import { ref } from "vue";
import type { Entry, Dimension, Commitment } from "../../types";
import { resolveDelta } from "../../utils/format";
import DimensionPopover from "../DimensionPopover.vue";

const props = defineProps<{
  entry: Entry;
  dimensions: Dimension[];
  commitments: Commitment[];
}>();

const emit = defineEmits<{
  save: [item: string, durationMinutes: number, dimensions: Record<string, string>];
  cancel: [];
  delete: [];
}>();

const item = ref(props.entry.item);
const durText = ref(String(props.entry.duration));
const dimValues = ref<Record<string, string>>({ ...props.entry.dimensions });
const popoverOpen = ref(false);

function chipClass(key: string): string {
  const map: Record<string, string> = {
    category: "bg-[var(--color-token-cat-bg)] text-[var(--color-token-cat-text)]",
    "business-line": "bg-[var(--color-token-biz-bg)] text-[var(--color-token-biz-text)]",
    "importance-urgency": "bg-[var(--color-token-imp-bg)] text-[var(--color-token-imp-text)]",
    goal: "bg-[var(--color-token-goal-bg)] text-[var(--color-token-goal-text)]",
  };
  return map[key] || map.category;
}

function filled() {
  return props.dimensions.filter(d => dimValues.value[d.key]);
}

function removeDim(key: string) {
  const next = { ...dimValues.value };
  delete next[key];
  dimValues.value = next;
}

function onSelect(dimKey: string, value: string) {
  dimValues.value = { ...dimValues.value, [dimKey]: value };
}

function save() {
  const minutes = resolveDelta(durText.value, props.entry.duration);
  emit("save", item.value.trim() || "(untitled)", minutes, { ...dimValues.value });
}
</script>

<template>
  <div
    class="bg-[var(--color-surface)] border border-[var(--color-brand-solid)] rounded-[var(--radius-form-lg)]
           shadow-[var(--shadow-focus-ring)] px-[14px] py-[9px] flex flex-col gap-[4px] relative"
  >
    <div class="flex gap-[8px] items-center">
      <input
        v-model="item"
        class="flex-1 text-[var(--app-text-base)] font-medium text-[var(--color-text-primary)] border-none outline-none bg-transparent py-[1px]"
        @keydown.enter.prevent="save"
      />
      <input
        v-model="durText"
        class="mono w-[56px] text-right text-[var(--app-text-sm)] text-[var(--color-text-primary)]
               border border-[var(--color-border-form)] rounded-[var(--radius-form)] px-[8px] py-[2px]
               outline-none focus:border-[var(--color-brand-solid)]"
        @keydown.enter.prevent="save"
      />
      <span class="text-[var(--app-text-xs-alt)] text-[var(--color-text-secondary)]">min</span>
    </div>

    <div class="flex gap-[3px] flex-wrap mt-[2px] items-center">
      <span
        v-for="d in filled()" :key="d.key"
        class="text-[var(--app-text-micro)] font-medium px-[7px] py-[1px] rounded-[var(--radius-sm)] inline-flex items-center gap-[5px]"
        :class="chipClass(d.key)"
      >
        {{ dimValues[d.key] }}
        <span data-test="chip-remove" class="cursor-pointer opacity-50 hover:opacity-100 text-[var(--app-text-xs-alt)] leading-none" @click="removeDim(d.key)">×</span>
      </span>
      <span
        data-test="add-tag"
        class="text-[var(--app-text-micro)] font-medium px-[7px] py-[1px] rounded-[var(--radius-sm)]
               border border-dashed border-[var(--color-border-form)] text-[var(--color-text-secondary)]
               cursor-pointer hover:border-[var(--color-text-muted)]"
        @click="popoverOpen = true"
      >+ tag</span>
    </div>

    <div class="flex gap-[8px] mt-[4px] items-center">
      <button data-test="save" class="text-[var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] bg-[var(--color-brand-solid)] text-white hover:bg-[var(--color-brand-link)]" @click="save">Save</button>
      <button data-test="cancel" class="text-[var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]" @click="emit('cancel')">Cancel</button>
      <button data-test="delete" class="text-[var(--app-text-micro)] font-semibold px-[10px] py-[2px] rounded-[var(--radius-form)] text-[var(--color-text-disabled)] hover:text-[var(--color-danger)] ml-auto" @click="emit('delete')">Delete</button>
    </div>

    <DimensionPopover
      v-if="popoverOpen"
      :dimensions="dimensions"
      :commitments="commitments"
      :dim-values="dimValues"
      class="absolute left-0 top-full mt-[4px] z-10"
      @select="onSelect"
      @close="popoverOpen = false"
    />
  </div>
</template>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/composite/EntryRowEdit.test.ts`
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/EntryRowEdit.vue src/__tests__/components/composite/EntryRowEdit.test.ts
git commit -m "feat: add EntryRowEdit inline editor"
```

### Task 2.3: Rewrite EntryRow

**Files:**
- Rewrite: `src/components/composite/EntryRow.vue`
- Test: `src/__tests__/components/composite/EntryRow.test.ts` (create if absent; overwrite if present)

Spec §4.5. Props stay `{ entry, index }`. Reads `useStore()` for `config.dimensions` and `commitments`. Display mode: item text (2-line clamp, `title` = full text), passive chips (`--color-chip-*`, 100px max-width ellipsis), mono duration, hover-revealed `⋯` trigger. Double-clicking the row OR clicking `⋯` enters edit mode, rendering `EntryRowEdit`. On `EntryRowEdit`'s `save`, diff against the original and emit `update [id, item, dur]` if item/duration changed and `updateDimensions [id, dims]` if dimensions changed; on `delete`, emit `delete [id]`; on `cancel`, exit edit mode. Emit contract unchanged so `MonthView` wiring is preserved.

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/components/composite/EntryRow.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { reactive } from "vue";
import EntryRow from "../../../components/composite/EntryRow.vue";
import { STORE_KEY } from "../../../stores/useStore";
import { makeEntry, makeConfig, makeCommitment } from "../../mocks/fixtures";

function mountRow(entryOverrides = {}) {
  const store = reactive({
    config: makeConfig(),
    commitments: [makeCommitment({ goals: ["Bug fixes"] })],
  });
  const entry = makeEntry({ item: "Review PR", duration: 90, dimensions: { category: "Coding" }, ...entryOverrides });
  return mount(EntryRow, {
    props: { entry, index: 0 },
    global: { provide: { [STORE_KEY as symbol]: store } },
  });
}

describe("EntryRow", () => {
  it("renders item text and formatted duration", () => {
    const wrapper = mountRow();
    expect(wrapper.text()).toContain("Review PR");
    expect(wrapper.text()).toContain("1h 30m");
  });

  it("renders a chip per filled dimension", () => {
    const wrapper = mountRow();
    expect(wrapper.text()).toContain("Coding");
  });

  it("enters edit mode on double-click", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='entry-row']").trigger("dblclick");
    expect(wrapper.findComponent({ name: "EntryRowEdit" }).exists()).toBe(true);
  });

  it("enters edit mode when the ⋯ trigger is clicked", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='edit-trigger']").trigger("click");
    expect(wrapper.findComponent({ name: "EntryRowEdit" }).exists()).toBe(true);
  });

  it("emits update on save when item/duration changed", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='entry-row']").trigger("dblclick");
    const editor = wrapper.findComponent({ name: "EntryRowEdit" });
    editor.vm.$emit("save", "Review PR #2", 120, { category: "Coding" });
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("update")?.[0]).toEqual([wrapper.props("entry").id, "Review PR #2", 120]);
    expect(wrapper.emitted("updateDimensions")).toBeFalsy();
  });

  it("emits updateDimensions on save when only dimensions changed", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='entry-row']").trigger("dblclick");
    const editor = wrapper.findComponent({ name: "EntryRowEdit" });
    editor.vm.$emit("save", "Review PR", 90, { category: "Coding", goal: "Bug fixes" });
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("updateDimensions")?.[0]).toEqual([wrapper.props("entry").id, { category: "Coding", goal: "Bug fixes" }]);
    expect(wrapper.emitted("update")).toBeFalsy();
  });

  it("emits delete from the editor", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='entry-row']").trigger("dblclick");
    wrapper.findComponent({ name: "EntryRowEdit" }).vm.$emit("delete");
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("delete")?.[0]).toEqual([wrapper.props("entry").id]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/composite/EntryRow.test.ts`
Expected: FAIL — current `EntryRow` has no `data-test='entry-row'`/`edit-trigger` and no `EntryRowEdit` child.

- [ ] **Step 3: Rewrite the implementation**

```vue
<!-- src/components/composite/EntryRow.vue -->
<script setup lang="ts">
import { ref, computed } from "vue";
import type { Entry } from "../../types";
import { formatDuration } from "../../utils/format";
import { useStore } from "../../stores/useStore";
import EntryRowEdit from "./EntryRowEdit.vue";

const props = defineProps<{ entry: Entry; index: number }>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();

const store = useStore();
const editing = ref(false);

const dimensions = computed(() => store.config?.dimensions || []);
const filledDims = computed(() => dimensions.value.filter(d => props.entry.dimensions[d.key]));

function chipClass(key: string): string {
  const map: Record<string, string> = {
    category: "bg-[var(--color-chip-cat-bg)] text-[var(--color-chip-cat-text)]",
    "business-line": "bg-[var(--color-chip-biz-bg)] text-[var(--color-chip-biz-text)]",
    "importance-urgency": "bg-[var(--color-chip-imp-bg)] text-[var(--color-chip-imp-text)]",
    goal: "bg-[var(--color-chip-goal-bg)] text-[var(--color-chip-goal-text)]",
  };
  return map[key] || map.category;
}

function onSave(item: string, durationMinutes: number, dims: Record<string, string>) {
  const itemChanged = item !== props.entry.item;
  const durChanged = durationMinutes !== props.entry.duration;
  const dimsChanged = JSON.stringify(dims) !== JSON.stringify(props.entry.dimensions);
  if (itemChanged || durChanged) emit("update", props.entry.id, item, durationMinutes);
  if (dimsChanged) emit("updateDimensions", props.entry.id, dims);
  editing.value = false;
}
</script>

<template>
  <EntryRowEdit
    v-if="editing"
    :entry="entry"
    :dimensions="dimensions"
    :commitments="store.commitments"
    @save="onSave"
    @cancel="editing = false"
    @delete="emit('delete', entry.id); editing = false"
  />
  <div
    v-else
    data-test="entry-row"
    class="group flex justify-between items-start gap-[8px] px-[14px] py-[9px] rounded-[var(--radius-form-lg)]
           border border-transparent hover:bg-[var(--color-surface-muted)] hover:border-[var(--color-divider)] transition-all"
    @dblclick="editing = true"
  >
    <div class="flex-1 min-w-0">
      <div
        class="text-[var(--app-text-base)] font-medium text-[var(--color-text-primary)] leading-[1.4] break-words overflow-hidden [display:-webkit-box] [-webkit-line-clamp:2] [-webkit-box-orient:vertical]"
        :title="entry.item"
      >{{ entry.item }}</div>
      <div v-if="filledDims.length" class="flex gap-[3px] mt-[3px] flex-wrap">
        <span
          v-for="d in filledDims" :key="d.key"
          class="text-[var(--app-text-micro)] font-[450] px-[6px] rounded-[var(--radius-sm)] leading-[1.7] max-w-[100px] overflow-hidden text-ellipsis whitespace-nowrap"
          :class="chipClass(d.key)"
          :title="entry.dimensions[d.key]"
        >{{ entry.dimensions[d.key] }}</span>
      </div>
    </div>
    <span class="mono text-[var(--app-text-sm)] text-[var(--color-text-primary)] flex-shrink-0 ml-[16px] pt-[1px]">
      {{ entry.duration > 0 ? formatDuration(entry.duration) : "—" }}
    </span>
    <span
      data-test="edit-trigger"
      class="text-[var(--color-text-secondary)] hover:text-[var(--color-brand-solid)] text-[14px] leading-none flex-shrink-0 ml-[8px] px-[2px] cursor-pointer opacity-0 group-hover:opacity-100 transition-opacity"
      title="Edit"
      @click="editing = true"
    >⋯</span>
  </div>
</template>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/composite/EntryRow.test.ts`
Expected: PASS (8 tests).

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/EntryRow.vue src/__tests__/components/composite/EntryRow.test.ts
git commit -m "refactor: rewrite EntryRow with 2-line clamp, hover edit, inline editor"
```

### Task 2.4: HeatmapCalendar

**Files:**
- Create: `src/components/HeatmapCalendar.vue`
- Test: `src/__tests__/components/HeatmapCalendar.test.ts`

Spec §4.1. Props: `year`, `month` (1-based), `selectedDate`, `monthEntries` (`Record<string, Entry[]>`), `availableMonths`. Emits `navigate {year, month}`, `selectDay date`, `requestMonths`. Renders: a nav row (`←` / clickable `Month Year ▾` / `→`), the `QuickJumpPopover` (toggled by the label; requests months first if `availableMonths === null`), a 7-column Monday-first heatmap grid (cells coloured by `heatLevel` of each day's total minutes, today/selected rings, future days dimmed and non-clickable), and a month total. Cell coloring is applied through `--heatmap-*` tokens.

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/components/HeatmapCalendar.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import HeatmapCalendar from "../../components/HeatmapCalendar.vue";
import { makeEntry } from "../mocks/fixtures";
import type { Entry } from "../../types";

function mountCal(monthEntries: Record<string, Entry[]> = {}, availableMonths: { year: number; month: number }[] | null = null) {
  return mount(HeatmapCalendar, {
    props: { year: 2026, month: 6, selectedDate: "2026-06-19", monthEntries, availableMonths },
  });
}

describe("HeatmapCalendar", () => {
  it("renders the month label", () => {
    expect(mountCal().text()).toContain("June");
    expect(mountCal().text()).toContain("2026");
  });

  it("renders a cell for each day of June (30 days)", () => {
    const cells = mountCal().findAll("[data-test='day-cell']");
    expect(cells.length).toBe(30);
  });

  it("emits navigate on the left and right arrows", async () => {
    const wrapper = mountCal();
    await wrapper.find("[data-test='prev-month']").trigger("click");
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2026, month: 5 }]);
    await wrapper.find("[data-test='next-month']").trigger("click");
    expect(wrapper.emitted("navigate")?.[1]).toEqual([{ year: 2026, month: 7 }]);
  });

  it("emits selectDay when a non-future day is clicked", async () => {
    const wrapper = mountCal();
    // Day 1 of June 2026 is in the past relative to selectedDate's month — click it
    const day1 = wrapper.findAll("[data-test='day-cell']")[0];
    await day1.trigger("click");
    expect(wrapper.emitted("selectDay")?.[0]).toEqual(["2026-06-01"]);
  });

  it("shows the month total of logged hours", () => {
    const monthEntries = {
      "2026-06-02": [makeEntry({ duration: 120 })],
      "2026-06-03": [makeEntry({ duration: 90 }), makeEntry({ duration: 30 })],
    };
    expect(mountCal(monthEntries).text()).toContain("4");      // 4h total
  });

  it("emits requestMonths when the label is clicked and months are not loaded", async () => {
    const wrapper = mountCal({}, null);
    await wrapper.find("[data-test='month-label']").trigger("click");
    expect(wrapper.emitted("requestMonths")).toBeTruthy();
  });

  it("shows QuickJumpPopover when the label is clicked and months are loaded", async () => {
    const wrapper = mountCal({}, [{ year: 2026, month: 6 }]);
    await wrapper.find("[data-test='month-label']").trigger("click");
    expect(wrapper.findComponent({ name: "QuickJumpPopover" }).exists()).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/HeatmapCalendar.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

```vue
<!-- src/components/HeatmapCalendar.vue -->
<script setup lang="ts">
import { ref, computed } from "vue";
import type { Entry } from "../types";
import type { AvailableMonth } from "../stores/useStore";
import { datesInMonth, parseDate } from "../utils/dates";
import { heatLevel } from "../utils/heatmap";
import QuickJumpPopover from "./QuickJumpPopover.vue";

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const props = defineProps<{
  year: number;
  month: number;
  selectedDate: string;
  monthEntries: Record<string, Entry[]>;
  availableMonths: AvailableMonth[] | null;
}>();

const emit = defineEmits<{
  navigate: [{ year: number; month: number }];
  selectDay: [date: string];
  requestMonths: [];
}>();

const showJump = ref(false);

function todayStr(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
}

const dates = computed(() => datesInMonth(`${props.year}-${String(props.month).padStart(2, "0")}-01`));

// Monday-first leading blank count for the first cell.
const leadingBlanks = computed(() => {
  const jsDay = parseDate(dates.value[0]).getDay(); // 0=Sun..6=Sat
  return (jsDay + 6) % 7;
});

function dayMinutes(date: string): number {
  return (props.monthEntries[date] || []).reduce((s, e) => s + e.duration, 0);
}

const monthTotalHours = computed(() => {
  let total = 0;
  for (const d of dates.value) total += dayMinutes(d);
  return Math.round((total / 60) * 10) / 10;
});

function isFuture(date: string): boolean {
  const now = new Date(); now.setHours(0, 0, 0, 0);
  const [y, m, d] = date.split("-").map(Number);
  const t = new Date(y, m - 1, d); t.setHours(0, 0, 0, 0);
  return t > now;
}

const cellBg: Record<string, string> = {
  empty: "bg-[var(--heatmap-empty)] text-[var(--heatmap-empty-text)]",
  light: "bg-[var(--heatmap-light)] text-[var(--heatmap-light-text)]",
  mid: "bg-[var(--heatmap-mid)] text-[var(--heatmap-mid-text)]",
  heavy: "bg-[var(--heatmap-heavy)] text-[var(--heatmap-heavy-text)] font-bold",
};

function cellClass(date: string): string {
  const base = cellBg[heatLevel(dayMinutes(date))];
  const rings: string[] = [];
  if (date === todayStr()) rings.push("shadow-[0_0_0_2px_var(--heatmap-today-ring)]");
  if (date === props.selectedDate) rings.push("shadow-[0_0_0_2px_var(--heatmap-selected-ring)]");
  return [base, ...rings, isFuture(date) ? "opacity-40 cursor-default" : "cursor-pointer hover:scale-110"].join(" ");
}

function dayNum(date: string): number {
  return parseInt(date.split("-")[2], 10);
}

function clickDay(date: string) {
  if (isFuture(date)) return;
  emit("selectDay", date);
}

function shift(delta: number) {
  let m = props.month + delta;
  let y = props.year;
  if (m < 1) { m = 12; y--; } else if (m > 12) { m = 1; y++; }
  emit("navigate", { year: y, month: m });
}

function onLabelClick() {
  if (props.availableMonths === null) { emit("requestMonths"); return; }
  showJump.value = !showJump.value;
}

function onJump(payload: { year: number; month: number }) {
  showJump.value = false;
  emit("navigate", payload);
}
</script>

<template>
  <div>
    <!-- Nav row -->
    <div class="flex items-center justify-between mb-[8px]">
      <span data-test="prev-month" class="text-[var(--app-text-xs)] text-[var(--color-text-secondary)] cursor-pointer px-[4px] py-[2px] hover:text-[var(--color-text-primary)]" title="Previous month (⌘[)" @click="shift(-1)">←</span>
      <span data-test="month-label" class="text-[var(--app-text-base)] font-bold text-[var(--color-text-primary)] cursor-pointer" @click="onLabelClick">
        {{ MONTH_NAMES[month - 1] }}
        <span class="font-normal text-[var(--app-text-xs-alt)] text-[var(--color-text-secondary)]">{{ year }} ▾</span>
      </span>
      <span data-test="next-month" class="text-[var(--app-text-xs)] text-[var(--color-text-secondary)] cursor-pointer px-[4px] py-[2px] hover:text-[var(--color-text-primary)]" title="Next month (⌘])" @click="shift(1)">→</span>
    </div>

    <QuickJumpPopover
      v-if="showJump && availableMonths !== null"
      :year="year" :month="month" :available-months="availableMonths"
      class="mb-[8px]"
      @jump="onJump"
    />

    <!-- Weekday headers -->
    <div class="grid grid-cols-7 gap-[3px] text-center text-[var(--app-text-2xs)] text-[var(--color-text-secondary)] mb-[4px]">
      <span>M</span><span>T</span><span>W</span><span>T</span><span>F</span><span>S</span><span>S</span>
    </div>

    <!-- Day grid -->
    <div class="grid grid-cols-7 gap-[3px] text-center">
      <span v-for="n in leadingBlanks" :key="'blank-' + n"></span>
      <span
        v-for="date in dates" :key="date"
        data-test="day-cell"
        class="mono w-[24px] h-[24px] rounded-[var(--radius-md)] flex items-center justify-center text-[var(--app-text-micro)] transition-all"
        :class="cellClass(date)"
        @click="clickDay(date)"
      >{{ dayNum(date) }}</span>
    </div>

    <!-- Month total -->
    <div class="mt-[6px] text-center text-[var(--app-text-xs-alt)] font-semibold text-[var(--color-text-primary)]">
      <span class="mono">{{ monthTotalHours }}h</span>
      <span class="font-normal text-[var(--app-text-micro)] text-[var(--color-text-secondary)]"> / month</span>
    </div>
  </div>
</template>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/HeatmapCalendar.test.ts`
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add src/components/HeatmapCalendar.vue src/__tests__/components/HeatmapCalendar.test.ts
git commit -m "feat: add HeatmapCalendar sidebar with nav, grid, month total"
```

---

## Phase 3: Containers & Integration

### Task 3.1: Rewrite EntryList

**Files:**
- Rewrite: `src/components/EntryList.vue`
- Test: `src/__tests__/components/EntryList.test.ts` (overwrite)

The summary row moves to `DayHeader`, so `EntryList` becomes the row list plus an empty state. It re-emits `EntryRow`'s events unchanged. `EntryRow` reads `useStore()`, so tests provide the store.

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/components/EntryList.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { reactive } from "vue";
import EntryList from "../../components/EntryList.vue";
import { STORE_KEY } from "../../stores/useStore";
import { makeEntry, makeConfig, makeCommitment } from "../mocks/fixtures";

function mountList(entries = [makeEntry({ item: "A", duration: 60 }), makeEntry({ item: "B", duration: 30 })]) {
  const store = reactive({ config: makeConfig(), commitments: [makeCommitment()] });
  return mount(EntryList, {
    props: { entries },
    global: { provide: { [STORE_KEY as symbol]: store } },
  });
}

describe("EntryList", () => {
  it("renders one EntryRow per entry", () => {
    const wrapper = mountList();
    expect(wrapper.findAllComponents({ name: "EntryRow" }).length).toBe(2);
  });

  it("shows an empty state when there are no entries", () => {
    const wrapper = mountList([]);
    expect(wrapper.text()).toContain("No entries");
  });

  it("re-emits update from a row", async () => {
    const wrapper = mountList();
    wrapper.findAllComponents({ name: "EntryRow" })[0].vm.$emit("update", "id1", "X", 45);
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("update")?.[0]).toEqual(["id1", "X", 45]);
  });

  it("re-emits delete from a row", async () => {
    const wrapper = mountList();
    wrapper.findAllComponents({ name: "EntryRow" })[0].vm.$emit("delete", "id1");
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("delete")?.[0]).toEqual(["id1"]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/EntryList.test.ts`
Expected: FAIL — current `EntryList` still renders the bordered total row and may not match the empty-state copy.

- [ ] **Step 3: Rewrite the implementation**

```vue
<!-- src/components/EntryList.vue -->
<script setup lang="ts">
import type { Entry } from "../types";
import EntryRow from "./composite/EntryRow.vue";

defineProps<{ entries: Entry[] }>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();
</script>

<template>
  <div class="flex-1 flex flex-col gap-[2px] overflow-y-auto pr-[4px]">
    <div v-if="entries.length === 0" class="p-8 text-center text-[var(--color-text-secondary)] text-[var(--app-text-sm)]">
      No entries yet. Log your first work item below.
    </div>
    <EntryRow
      v-for="(entry, index) in entries"
      :key="entry.id"
      :entry="entry"
      :index="index"
      @update="(id, item, dur) => emit('update', id, item, dur)"
      @delete="(id) => emit('delete', id)"
      @update-dimensions="(id, dims) => emit('updateDimensions', id, dims)"
    />
  </div>
</template>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/EntryList.test.ts`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add src/components/EntryList.vue src/__tests__/components/EntryList.test.ts
git commit -m "refactor: slim EntryList to row list + empty state"
```

### Task 3.2: Rewrite CommitmentsPanel

**Files:**
- Rewrite: `src/components/CommitmentsPanel.vue`
- Test: `src/__tests__/components/CommitmentsPanel.test.ts` (overwrite if present, else create)

Spec §4.3 and §9. Per-role expand/collapse of the goal list (`▾`/`▸`), a single brand-gradient progress bar (the old orange/yellow/green/red `barColor` logic is **removed** per §9), spent/allocation with `.mono`. The existing `CommitmentsEditor` edit flow is retained behind an "Edit" affordance.

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/components/CommitmentsPanel.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import CommitmentsPanel from "../../components/CommitmentsPanel.vue";
import { makeCommitmentProgress, makeCommitment } from "../mocks/fixtures";

function mountPanel() {
  return mount(CommitmentsPanel, {
    props: {
      progress: [makeCommitmentProgress({ role: "Developer", spent_minutes: 1230, allocation_minutes: 2400 })],
      commitments: [makeCommitment()],
      rootPath: "/x",
      selectedYear: 2026,
      selectedMonth: 6,
    },
  });
}

describe("CommitmentsPanel", () => {
  it("renders role name and mono spent/allocation", () => {
    const wrapper = mountPanel();
    expect(wrapper.text()).toContain("Developer");
    expect(wrapper.text()).toContain("20h 30m"); // 1230m
    expect(wrapper.text()).toContain("40"); // allocation hours
  });

  it("progress fill uses the brand gradient (single style, no status colors)", () => {
    const wrapper = mountPanel();
    const fill = wrapper.find("[data-test='progress-fill']");
    expect(fill.attributes("class") || "").not.toMatch(/bg-(orange|yellow|green|red)-/);
  });

  it("toggles the goal list for a role", async () => {
    const wrapper = mountPanel();
    const goalRows = () => wrapper.findAll("[data-test='goal-row']");
    const initial = goalRows().length;
    await wrapper.find("[data-test='role-toggle']").trigger("click");
    expect(goalRows().length).not.toBe(initial);
  });
});
```

(Note: `makeCommitmentProgress` default goals are `Ship feature X`/`Code review`; the toggle test only checks the count changes between expanded/collapsed states. If the panel defaults to expanded, the toggle collapses to 0; if collapsed, it expands to 2. Either satisfies `not.toBe(initial)`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/CommitmentsPanel.test.ts`
Expected: FAIL — current panel always shows goals (no `role-toggle`) and uses status `barColor`.

- [ ] **Step 3: Rewrite the implementation**

```vue
<!-- src/components/CommitmentsPanel.vue -->
<script setup lang="ts">
import { ref } from "vue";
import type { Commitment, CommitmentProgress } from "../types";
import { formatDuration } from "../utils/format";
import CommitmentsEditor from "./composite/CommitmentsEditor.vue";

const props = defineProps<{
  progress: CommitmentProgress[];
  commitments?: Commitment[];
  rootPath?: string;
  selectedYear: number;
  selectedMonth: number;
}>();

const emit = defineEmits<{ saved: [] }>();

// Roles start expanded; clicking the role header toggles its goal list.
const collapsed = ref<Record<string, boolean>>({});
function toggle(role: string) { collapsed.value[role] = !collapsed.value[role]; }

function pct(spent: number, alloc: number): string {
  if (alloc === 0) return "0%";
  return Math.min(100, Math.round((spent / alloc) * 100)) + "%";
}

const isEditing = ref(false);
function enterEdit() {
  if (!props.commitments || props.commitments.length === 0) return;
  isEditing.value = true;
}
function cancelEdit() { isEditing.value = false; }
</script>

<template>
  <div v-if="progress.length > 0 || (commitments && commitments.length > 0) || isEditing" data-test="commitments-panel">
    <div class="flex justify-between items-center mb-[10px]">
      <h3 class="text-[var(--app-text-micro)] font-bold text-[var(--color-text-secondary)] uppercase tracking-[0.5px]">Commitments</h3>
      <button
        v-if="!isEditing && commitments && commitments.length > 0"
        class="text-[var(--app-text-xs)] text-[var(--color-brand-link)] font-medium cursor-pointer"
        data-test="edit-btn"
        @click="enterEdit"
      >Edit</button>
    </div>

    <template v-if="!isEditing">
      <div v-for="s in progress" :key="s.role" class="mb-[16px] last:mb-0">
        <div
          data-test="role-toggle"
          class="flex justify-between items-center cursor-pointer rounded-[var(--radius-form-lg)] px-[2px] py-[1px] hover:bg-[var(--color-divider)]"
          @click="toggle(s.role)"
        >
          <span class="text-[var(--app-text-xs)] font-semibold text-[var(--color-text-primary)]">
            {{ s.role }} {{ collapsed[s.role] ? "▸" : "▾" }}
          </span>
          <span class="text-[var(--app-text-xs-alt)] font-semibold text-[var(--color-text-primary)]">
            <span class="mono">{{ (s.spent_minutes / 60).toFixed(1) }}</span><span class="mono font-normal text-[var(--color-text-secondary)]">/{{ (s.allocation_minutes / 60).toFixed(0) }}h</span>
          </span>
        </div>
        <div class="h-[4px] bg-[var(--color-divider)] rounded-[var(--radius-sm)] overflow-hidden mt-[4px]">
          <div
            data-test="progress-fill"
            class="h-full rounded-[var(--radius-sm)] transition-all"
            :style="{ width: pct(s.spent_minutes, s.allocation_minutes), background: 'linear-gradient(90deg, var(--color-brand-gradient-from), var(--color-brand-gradient-to))' }"
          />
        </div>
        <div v-if="!collapsed[s.role]" class="mt-[6px] flex flex-col gap-[1px]">
          <div
            v-for="g in s.goals" :key="g.name"
            data-test="goal-row"
            class="flex justify-between text-[var(--app-text-xs-alt)] text-[var(--color-text-secondary)] py-[3px] pl-[8px]"
          >
            <span class="overflow-hidden text-ellipsis whitespace-nowrap max-w-[130px]" :title="g.name">{{ g.name }}</span>
            <span v-if="g.spent_minutes > 0" class="mono font-medium text-[var(--color-text-primary)]">{{ formatDuration(g.spent_minutes) }}</span>
            <span v-else class="mono text-[var(--color-text-secondary)]">0</span>
          </div>
        </div>
      </div>
    </template>

    <CommitmentsEditor
      v-else
      :commitments="commitments || []"
      :root-path="rootPath || ''"
      :selected-year="selectedYear"
      :selected-month="selectedMonth"
      @saved="emit('saved')"
      @cancel="cancelEdit"
    />
  </div>
</template>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/CommitmentsPanel.test.ts`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/components/CommitmentsPanel.vue src/__tests__/components/CommitmentsPanel.test.ts
git commit -m "refactor: rewrite CommitmentsPanel with expand/collapse + gradient bar"
```

### Task 3.3: Rewrite MonthView (three-zone shell + append + keyboard nav)

**Files:**
- Rewrite: `src/components/MonthView.vue`
- Test: `src/__tests__/components/MonthView.test.ts` (overwrite)

`MonthView` becomes the orchestrator: it keeps all the existing data logic (`loadMonth`, `loadCommitmentProgress`, `loadDayNote`, `handleSelectDay`, `handleNavigate`, `handleRequestMonths`, `handleUpdateEntry`, `handleUpdateDimensions`, `handleDeleteEntry`, note save, file path) and **absorbs the append logic from the deleted `QuickEntry`** as `handleSubmit`. Layout becomes the three zones; navigation maps to `HeatmapCalendar`; input maps to `TwoLineInput`; summary maps to `DayHeader`. Adds global `⌘[`/`⌘]` month navigation.

- [ ] **Step 1: Write the failing test**

```typescript
// src/__tests__/components/MonthView.test.ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { reactive } from "vue";
import MonthView from "../../components/MonthView.vue";
import { STORE_KEY } from "../../stores/useStore";
import { makeConfig, makeCommitment, makeEntry } from "../mocks/fixtures";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

function makeStore() {
  return reactive({
    screen: "ready",
    rootPath: "/root",
    config: makeConfig(),
    commitments: [makeCommitment({ goals: ["Bug fixes"] })],
    commitmentProgress: [],
    today: { note: null, entries: [makeEntry({ item: "Existing", duration: 60 })] },
    lastDimensions: {},
    currentDate: "2026-06-19",
    monthEntries: { "2026-06-19": [makeEntry({ item: "Existing", duration: 60 })] },
    availableMonths: null,
  });
}

function mountView(store = makeStore()) {
  return mount(MonthView, {
    global: {
      provide: { [STORE_KEY as symbol]: store, focusRequestId: { value: 0 }, triggerUndoToast: () => {} },
    },
  });
}

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockResolvedValue({ note: null, entries: [] });
});

describe("MonthView", () => {
  it("renders the three zones: HeatmapCalendar, DayHeader, EntryList, TwoLineInput", () => {
    const wrapper = mountView();
    expect(wrapper.findComponent({ name: "HeatmapCalendar" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "DayHeader" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "EntryList" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "TwoLineInput" }).exists()).toBe(true);
    expect(wrapper.findComponent({ name: "CommitmentsPanel" }).exists()).toBe(true);
  });

  it("calls append_entry when TwoLineInput emits submit", async () => {
    invokeMock.mockResolvedValueOnce(makeEntry({ item: "New task", duration: 30 })); // append_entry
    const wrapper = mountView();
    wrapper.findComponent({ name: "TwoLineInput" }).vm.$emit("submit", "New task", 30, { category: "Coding" });
    await wrapper.vm.$nextTick();
    expect(invokeMock).toHaveBeenCalledWith(
      "append_entry",
      expect.objectContaining({ rootPath: "/root", date: "2026-06-19" }),
    );
  });

  it("only renders TwoLineInput when the selected day is today", () => {
    const store = makeStore();
    store.currentDate = "2026-06-10"; // not today (today is mocked-real; pick a past date in-month)
    const wrapper = mountView(store);
    // TwoLineInput is gated on isSelectedToday; for a past date it should be hidden
    expect(wrapper.findComponent({ name: "TwoLineInput" }).exists()).toBe(false);
  });
});
```

(Note: the "today" gating test assumes the suite runs on a date other than 2026-06-10. The real-date dependency mirrors the existing `MonthView`/`QuickEntry` behaviour, which already gates on `isSelectedToday`. If the suite ever runs exactly on 2026-06-10 this single assertion would need adjusting — acceptable, matches current code.)

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/__tests__/components/MonthView.test.ts`
Expected: FAIL — current `MonthView` renders `MonthNavigator`/`DayStrip`/`QuickEntry`, not the new components.

- [ ] **Step 3: Rewrite the implementation**

```vue
<!-- src/components/MonthView.vue -->
<script setup lang="ts">
import { inject, computed, watch, ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import HeatmapCalendar from "./HeatmapCalendar.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import DayHeader from "./DayHeader.vue";
import EntryList from "./EntryList.vue";
import TwoLineInput from "./TwoLineInput.vue";
import type { DayFile, Entry, CommitmentProgress } from "../types";
import { logError, logInfo } from "../utils/errorLog";
import { datesInMonth, yearMonthFromDate, parseDate } from "../utils/dates";

const store = useStore();
const inputRef = ref<InstanceType<typeof TwoLineInput> | null>(null);

const selectedYear = computed(() => yearMonthFromDate(store.currentDate).year);
const selectedMonth = computed(() => yearMonthFromDate(store.currentDate).month);

function todayStr(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
}
const isSelectedToday = computed(() => store.currentDate === todayStr());

const dayEntries = computed(() => store.today?.entries || []);
const dayTotalMinutes = computed(() => dayEntries.value.reduce((s, e) => s + e.duration, 0));

const dayTitle = computed(() => {
  const d = parseDate(store.currentDate);
  return d.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" });
});

const triggerUndoToast = inject<(undoFn: () => void) => void>("triggerUndoToast", () => {});

// ---- Month loading (unchanged from prior implementation) ----
async function loadMonth(year: number, month: number, defaultDay?: number) {
  const now = new Date();
  const isCurrentMonth = year === now.getFullYear() && month === now.getMonth() + 1;
  let day: number;
  if (defaultDay !== undefined) day = defaultDay;
  else if (isCurrentMonth) day = now.getDate();
  else day = new Date(year, month, 0).getDate();

  const dateStr = `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
  store.currentDate = dateStr;

  const dates = datesInMonth(dateStr);
  const map: Record<string, Entry[]> = {};
  for (const date of dates) {
    try {
      const df = (await invoke("get_entries", { rootPath: store.rootPath, date })) as DayFile;
      map[date] = df.entries;
    } catch (e) { logError("MonthView.loadMonth", e); map[date] = []; }
  }
  store.monthEntries = map;
  await loadCommitmentProgress(year, month);
  if (store.currentDate in map) {
    store.today = { note: null, entries: map[store.currentDate] };
    loadDayNote(store.currentDate);
  }
}

async function loadCommitmentProgress(year: number, month: number) {
  try {
    store.commitmentProgress = (await invoke("get_commitment_progress", { rootPath: store.rootPath, year, month })) as CommitmentProgress[];
  } catch (e) { logError("MonthView.loadCommitmentProgress", e); store.commitmentProgress = []; }
}

async function loadDayNote(dateStr: string) {
  try {
    const df = (await invoke("get_entries", { rootPath: store.rootPath, date: dateStr })) as DayFile;
    if (store.today) store.today.note = df.note;
  } catch (e) { logError("MonthView.loadDayNote", e); }
}

async function handleSelectDay(dateStr: string) {
  store.currentDate = dateStr;
  if (dateStr in store.monthEntries) {
    store.today = { note: null, entries: store.monthEntries[dateStr] };
    await loadDayNote(dateStr);
  }
}

async function handleNavigate({ year, month }: { year: number; month: number }) {
  await loadMonth(year, month);
}

async function handleRequestMonths() {
  if (store.availableMonths !== null) return;
  try {
    store.availableMonths = (await invoke("get_available_months", { rootPath: store.rootPath })) as { year: number; month: number }[];
  } catch (e) { logError("MonthView.handleRequestMonths", e); store.availableMonths = []; }
}

// ---- Append (absorbed from the deleted QuickEntry) ----
function sanitizeValues(vals: Record<string, string>): Record<string, string> {
  const validKeys = new Set((store.config?.dimensions || []).map(d => d.key));
  const cleaned: Record<string, string> = {};
  for (const [k, v] of Object.entries(vals)) if (validKeys.has(k) && v) cleaned[k] = v;
  return cleaned;
}

async function handleSubmit(item: string, durationMinutes: number, dimensions: Record<string, string>) {
  const finalDimensions = sanitizeValues(dimensions);
  const newEntry = { item, duration: String(durationMinutes), dimensions: finalDimensions };
  try {
    const result = await invoke("append_entry", { rootPath: store.rootPath, date: store.currentDate, entry: newEntry });
    store.lastDimensions = { ...finalDimensions };
    inputRef.value?.clearInput();
    if (store.today) {
      const entries = [...store.today.entries, result as Entry];
      store.today = { ...store.today, entries };
      store.monthEntries[store.currentDate] = entries;
    }
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
  } catch (e) { logError("MonthView.handleSubmit", e); }
}

// ---- Entry mutations (unchanged) ----
async function handleUpdateEntry(entryId: string, item: string, durationMinutes: number) {
  const entries = store.today?.entries;
  if (!entries) return;
  const entry = entries.find(e => e.id === entryId);
  if (!entry) return;
  const update: Record<string, unknown> = {};
  if (item !== entry.item) update.item = item;
  if (durationMinutes !== entry.duration) update.duration = String(durationMinutes);
  if (Object.keys(update).length === 0) return;
  try {
    const df = (await invoke("update_entry", { rootPath: store.rootPath, date: store.currentDate, entryId, update })) as DayFile;
    store.today = df;
    store.monthEntries[store.currentDate] = df.entries;
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
  } catch (e) { logError("MonthView.handleUpdateEntry", e); }
}

async function handleUpdateDimensions(entryId: string, dimensions: Record<string, string>) {
  try {
    const df = (await invoke("update_entry", { rootPath: store.rootPath, date: store.currentDate, entryId, update: { dimensions } })) as DayFile;
    store.today = df;
    store.monthEntries[store.currentDate] = df.entries;
    await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
  } catch (e) { logError("MonthView.handleUpdateDimensions", e); }
}

let pendingDeleteTimer: ReturnType<typeof setTimeout> | null = null;
async function handleDeleteEntry(entryId: string) {
  const entries = store.today?.entries;
  if (!entries) return;
  const idx = entries.findIndex(e => e.id === entryId);
  if (idx === -1) return;
  const [removed] = entries.splice(idx, 1);
  let cancelled = false;
  pendingDeleteTimer = setTimeout(async () => {
    if (cancelled) return;
    try {
      await invoke("delete_entry", { rootPath: store.rootPath, date: store.currentDate, entryId });
      store.monthEntries[store.currentDate] = [...entries];
      await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
    } catch (e) {
      logError("MonthView.handleDeleteEntry", e);
      if (entries.findIndex(e => e.id === entryId) === -1) entries.splice(idx, 0, removed);
    }
  }, 5000);
  triggerUndoToast(() => {
    cancelled = true;
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
    if (entries.findIndex(e => e.id === entryId) === -1) entries.splice(idx, 0, removed);
  });
}

// ---- Day note (inline; unchanged behaviour) ----
const noteRef = ref<HTMLDivElement>();
watch(() => store.today?.note, (n) => {
  if (noteRef.value && noteRef.value.textContent !== (n || "")) noteRef.value.textContent = n || "";
}, { immediate: true });

function onNotePaste(e: ClipboardEvent) {
  e.preventDefault();
  const text = e.clipboardData?.getData("text/plain") || "";
  const sel = window.getSelection();
  if (sel && sel.rangeCount > 0) {
    const range = sel.getRangeAt(0);
    range.deleteContents();
    range.insertNode(document.createTextNode(text));
    range.collapse(false);
  }
}
async function saveNote() {
  const text = noteRef.value?.textContent || "";
  try { await invoke("set_day_note", { rootPath: store.rootPath, date: store.currentDate, note: text }); }
  catch (e) { logError("MonthView.saveNote", e); }
}

// ---- File path ----
const dayFilePath = computed(() => {
  if (!store.rootPath) return "";
  const d = store.currentDate;
  return `${d.slice(0, 4)}/${d.slice(5, 7)}/${d}.md`;
});
const displayPath = computed(() => (store.rootPath ? `…/${dayFilePath.value}` : ""));
async function openInEditor() {
  if (!store.rootPath) return;
  try { await invoke("open_in_editor", { rootPath: store.rootPath, date: store.currentDate }); }
  catch (e) { logError("MonthView.openInEditor", e); }
}

// ---- Keyboard month navigation (⌘[ / ⌘]) ----
function shiftMonth(delta: number) {
  let m = selectedMonth.value + delta;
  let y = selectedYear.value;
  if (m < 1) { m = 12; y--; } else if (m > 12) { m = 1; y++; }
  loadMonth(y, m);
}
function onGlobalKeydown(e: KeyboardEvent) {
  if (!(e.metaKey || e.ctrlKey)) return;
  if (e.key === "[") { e.preventDefault(); shiftMonth(-1); }
  else if (e.key === "]") { e.preventDefault(); shiftMonth(1); }
}

onMounted(async () => {
  window.addEventListener("keydown", onGlobalKeydown);
  if (store.rootPath) {
    const { year, month } = yearMonthFromDate(store.currentDate);
    await loadMonth(year, month);
  }
});
onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
  if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
});

logInfo("MonthView", "mounted");
</script>

<template>
  <div class="flex gap-[24px] p-6 max-w-5xl mx-auto items-start min-h-screen">
    <!-- Sidebar -->
    <aside class="w-[220px] flex-shrink-0 flex flex-col gap-0 sticky top-6">
      <HeatmapCalendar
        :year="selectedYear"
        :month="selectedMonth"
        :selected-date="store.currentDate"
        :month-entries="store.monthEntries"
        :available-months="store.availableMonths"
        @navigate="handleNavigate"
        @select-day="handleSelectDay"
        @request-months="handleRequestMonths"
      />
      <div class="border-t border-[var(--color-divider)] my-[20px]"></div>
      <CommitmentsPanel
        :progress="store.commitmentProgress"
        :commitments="store.commitments"
        :root-path="store.rootPath"
        :selected-year="selectedYear"
        :selected-month="selectedMonth"
        @saved="loadCommitmentProgress(selectedYear, selectedMonth)"
      />
    </aside>

    <!-- Main -->
    <main class="flex-1 min-w-0 flex flex-col">
      <DayHeader
        :title="dayTitle"
        :is-today="isSelectedToday"
        :entry-count="dayEntries.length"
        :total-minutes="dayTotalMinutes"
      />

      <EntryList
        :entries="dayEntries"
        @update="handleUpdateEntry"
        @delete="handleDeleteEntry"
        @update-dimensions="handleUpdateDimensions"
      />

      <div class="mt-[16px] py-[8px]">
        <div
          ref="noteRef"
          class="text-[var(--app-text-xs)] italic text-[var(--color-text-secondary)] leading-[1.5] cursor-text px-[10px] py-[6px] rounded-[var(--radius-form-lg)] outline-none hover:bg-[var(--color-page-bg)]"
          contenteditable="true"
          data-placeholder="Add a note…"
          @blur="saveNote"
          @paste="onNotePaste"
        ></div>
      </div>

      <div v-if="isSelectedToday" class="mt-[12px]">
        <TwoLineInput
          ref="inputRef"
          :dimensions="store.config?.dimensions || []"
          :commitments="store.commitments"
          :initial-values="store.lastDimensions"
          @submit="handleSubmit"
        />
      </div>

      <div v-if="store.rootPath" class="mt-[10px] text-right">
        <button
          class="text-[var(--app-text-micro)] text-[var(--color-text-disabled)] hover:text-[var(--color-text-secondary)] cursor-pointer"
          :title="store.rootPath + '/' + dayFilePath"
          @click="openInEditor"
        >{{ displayPath }}</button>
      </div>
    </main>
  </div>
</template>

<style scoped>
[contenteditable]:empty::before {
  content: attr(data-placeholder);
  color: var(--color-placeholder);
}
</style>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/__tests__/components/MonthView.test.ts`
Expected: PASS (3 tests).

- [ ] **Step 5: Verify App.test still passes (App mounts MonthView unchanged)**

Run: `npx vitest run src/__tests__/components/App.test.ts`
Expected: PASS. If it references removed children or breaks, the only expected fix is leaving `App.vue` as-is (it already renders `<MonthView />` with no props). Do not change `App.vue` unless this test demands it.

- [ ] **Step 6: Commit**

```bash
git add src/components/MonthView.vue src/__tests__/components/MonthView.test.ts
git commit -m "refactor: rewrite MonthView as three-zone shell with append + keyboard nav"
```

---

## Phase 4: Cleanup

### Task 4.1: Delete obsolete components and their tests

**Files (delete):**
- `src/components/DayStrip.vue`, `src/components/MonthNavigator.vue`, `src/components/QuickEntry.vue`, `src/components/DimensionPanel.vue`, `src/components/EntryInput.vue`
- `src/components/composite/MentionMenu.vue`
- `src/components/base/AppInput.vue`, `src/components/base/AppChip.vue`, `src/components/base/AppSelect.vue`, `src/components/base/Popover.vue`
- Any matching tests under `src/__tests__/` (e.g. `DayStrip.test.ts`, `MonthNavigator.test.ts`, `QuickEntry.test.ts`, `EntryInput.test.ts`, `MentionMenu.test.ts`, `components/base/AppChip.test.ts`, `AppInput.test.ts`, `AppSelect.test.ts`, `Popover.test.ts`, `DimensionPanel.test.ts`)

- [ ] **Step 1: Confirm nothing still imports them**

Run:
```bash
grep -rEl "DayStrip|MonthNavigator|QuickEntry|DimensionPanel|EntryInput|MentionMenu|AppInput|AppChip|AppSelect|components/base/Popover" src --include='*.vue' --include='*.ts' | grep -v "__tests__"
```
Expected: **no output** (only the to-be-deleted files and their tests reference these names). If a retained file appears, fix that reference before deleting.

- [ ] **Step 2: Delete the files**

```bash
git rm src/components/DayStrip.vue src/components/MonthNavigator.vue src/components/QuickEntry.vue \
       src/components/DimensionPanel.vue src/components/EntryInput.vue \
       src/components/composite/MentionMenu.vue \
       src/components/base/AppInput.vue src/components/base/AppChip.vue \
       src/components/base/AppSelect.vue src/components/base/Popover.vue
# Delete matching tests (use git rm for each that exists):
git ls-files 'src/__tests__/**' | grep -Ei "DayStrip|MonthNavigator|QuickEntry|DimensionPanel|EntryInput|MentionMenu|AppInput|AppChip|AppSelect|/Popover" | xargs -r git rm
```

- [ ] **Step 3: Verify the full suite is green**

Run: `pnpm test && npx vue-tsc --noEmit`
Expected: all tests PASS, type-check CLEAN. (If `vue-tsc` reports an unused import or a dangling reference, remove it.)

- [ ] **Step 4: Commit**

```bash
git commit -m "chore: delete components superseded by UX redesign"
```

---

## Phase 5: Final Verification

### Task 5.1: Full automated suite

- [ ] **Step 1: Frontend tests + type-check**

Run: `pnpm test && npx vue-tsc --noEmit`
Expected: all PASS, CLEAN. Record the test file/count total.

- [ ] **Step 2: Backend tests (must remain untouched & green)**

Run: `cd src-tauri && cargo test`
Expected: all PASS (the redesign changed no Rust). If anything fails, the frontend work touched the backend by mistake — investigate before proceeding.

### Task 5.2: Manual smoke test

- [ ] **Step 1: Launch the app**

Run: `pnpm tauri dev`

- [ ] **Step 2: Walk the core flow and confirm each works**
  - App opens on Today; the `TwoLineInput` is focused.
  - Type `Sprint planning 1.5h` → a `1h 30m` duration token appears on line 2.
  - Press `@` → `DimensionPopover` opens; pick `category` → value → token chip appears; required-but-unfilled dims show dashed indicators.
  - Press `Enter` → entry appears at the bottom of the list; input clears.
  - Double-click an entry (or hover → click `⋯`) → inline editor; change duration, Save → list updates; commitments progress updates.
  - Click a heatmap day in the past → list switches; `TwoLineInput` is hidden for non-today.
  - `⌘[` / `⌘]` → month changes; heatmap re-renders.
  - Click the month label → `QuickJumpPopover`; jump to another month.
  - Expand/collapse a commitment role; the progress bar is the brand gradient.
  - Click the file path → opens in editor.

- [ ] **Step 2 (fallback): if `pnpm tauri dev` is unavailable in the environment**, run `pnpm dev` (Vite only) and verify the UI renders and `vue-tsc` is clean; note that Tauri `invoke` calls won't resolve without the backend.

---

## Phase 6: Demo Comparison

### Task 6.1: Region-by-region comparison against the demo

- [ ] **Step 1: Open `demos/UX-REDESIGN-DEMO.html` and the running app side by side. For each region, confirm the implementation matches the demo's layout and the spec's token values:**

| Region | Check |
|--------|-------|
| Sidebar nav | `← Month Year ▾ →` row; arrows navigate; label opens QuickJump |
| Heatmap grid | Monday-first; weekday headers M-S; cells colored by volume; today ring + selected ring; future dimmed; month total `Nh / month` |
| Commitments | role + `spent/alloc` (mono); brand-gradient bar; `▾/▸` expand; goal rows with mono spent, 130px ellipsis |
| Day header | title + Today badge + `N entries · Xh Ym` (mono) |
| Entry row | item (2-line clamp, tooltip); passive chips (100px ellipsis); mono duration; hover `⋯` |
| Entry edit | item input + duration `min` box + removable chips + `+ tag` + Save/Cancel/Delete; brand border + focus ring |
| Two-line input | `+` prefix; item input; `⏎` badge; token chips; missing dashed indicators; "Need a duration" hint; hints row |
| Dimension popover | DIM badge header; colored left bars; required/optional meta; val header with `←`; footer key hints |
| File path | bottom-right, muted, opens editor |

- [ ] **Step 2: Fix every deviation** (token mismatch, missing element, wrong layout). For each fix, re-run the affected component's test. Commit fixes in small batches:

```bash
git add -A && git commit -m "fix: align <region> with demo"
```

- [ ] **Step 3: Final green check**

Run: `pnpm test && npx vue-tsc --noEmit`
Expected: all PASS, CLEAN.

---

## Self-Review (completed by plan author)

**1. Spec coverage** — every spec §4 component maps to a task:

| Spec section | Task |
|---|---|
| §3 tokens | 0.1 |
| §4.1 HeatmapCalendar | 2.4 |
| §4.2 QuickJumpPopover | 1.2 |
| §4.3 CommitmentsPanel | 3.2 |
| §4.4 DayHeader | 1.4 |
| §4.5 EntryRow | 2.3 |
| §4.6 EntryRowEdit | 2.2 |
| §4.7 DayNote | 3.3 (inline in MonthView — kept inline by design; not a separate file) |
| §4.8 TwoLineInput | 2.1 |
| §4.9 DimensionPopover | 1.3 |
| §4.10 FilePath | 3.3 (inline in MonthView) |
| §5.1 shortcuts | `@`/`Enter`/`Esc` (2.1, 1.3); `⌘[`/`⌘]` (3.3); dbl-click edit (2.3) |
| §5.2 input flow | 2.1 + 3.3 |
| §5.3 edit flow | 2.2 + 2.3 + 3.3 (delete + undo toast) |
| §5.4 month nav | 2.4 + 3.3 |
| §6 overflow | 2.3 (clamp/ellipsis), 3.2 (goal ellipsis) |
| §9 deletions | 4.1 (components) + 3.2 (drop status barColor) + 3.1 (drop total row) |
| §10 component plan | Phases 1–4 |

**2. Deferred (documented, not silently dropped):**
- `⌘K` command palette (§5.1) — no component exists in §4 or the demo. Out of scope.
- `#` manual-duration trigger (§5.2 method B) — duration auto-parses from item text; `#` left as a future hint. The "Need a duration" path covers the no-duration case.

**3. Type/contract consistency** — verified: `submit [item, durationMinutes, dimensions]`, `save [item, durationMinutes, dimensions]`, `update [id, item, dur]`, `updateDimensions [id, dims]`, `delete [id]`, `navigate {year,month}`, `selectDay date`, `jump {year,month}`, `requestMonths` are used identically across producer and consumer. `clearInput()` is the exposed method on `TwoLineInput`, called by `MonthView`. Backend `invoke` signatures (`get_entries`, `append_entry`, `update_entry`, `delete_entry`, `set_day_note`, `get_commitment_progress`, `get_available_months`, `open_in_editor`) match the current code verbatim.

**4. Placeholder scan** — no TBD/TODO; every code step contains complete code.
