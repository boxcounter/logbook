# Logbook UX Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the entire frontend UI from a form-based dual-column layout into a three-zone professional tool (sidebar heatmap + entry list + two-line input), matching the design spec and demo.

**Architecture:** Three-zone layout with a 220px sidebar (heatmap calendar + commitments), a flex-1 main area (day header + entry list + day note), and a persistent two-line input at the bottom. All visual values sourced from CSS custom properties in `tokens.css`. Rust backend (14 commands) and data model unchanged.

**Tech Stack:** Vue 3 + Composition API + TypeScript + Tailwind CSS v4 + Vitest + @vue/test-utils. Backend: Tauri 2.x + Rust (unchanged).

**Design references:**
- `docs/superpowers/specs/2026-06-19-ux-redesign-design.md` — component spec tables (canonical visual values)
- `docs/superpowers/specs/2026-06-19-ux-redesign-tokens.css` — token blueprint (overwrites `src/assets/tokens.css`)
- `UX-REDESIGN-DEMO.html` — visual source of truth

---

## File Structure

### New files (9)
- `src/components/HeatmapCalendar.vue` — sidebar calendar heatmap
- `src/components/QuickJumpPopover.vue` — year/month dual-select popover
- `src/components/DimensionPopover.vue` — @-triggered dimension/value picker
- `src/components/TwoLineInput.vue` — two-line entry input (replaces EntryInput + QuickEntry + DimensionPanel)
- `src/__tests__/components/HeatmapCalendar.test.ts`
- `src/__tests__/components/QuickJumpPopover.test.ts`
- `src/__tests__/components/DimensionPopover.test.ts`
- `src/__tests__/components/TwoLineInput.test.ts`
- `scripts/lint-tokens.sh` — CSS variable compliance checker

### Rewritten files (7)
- `src/assets/tokens.css` — replaced with new token definitions
- `src/components/MonthView.vue` — new 3-zone layout
- `src/components/EntryList.vue` — new visual spec
- `src/components/composite/EntryRow.vue` — new visual spec + hover edit trigger + inline edit
- `src/components/CommitmentsPanel.vue` — goal expand/collapse + new visual spec
- `src/App.vue` — new layout shell + keyboard shortcuts
- `src/stores/useStore.ts` — minor: add `availableMonths` retains, `monthEntries` retains

### Modified files (2)
- `src/main.ts` — no change expected (verify `tokens.css` import path)
- `vitest.config.ts` — no change expected

### Deleted files (16)
Components: `DayStrip.vue`, `MonthNavigator.vue`, `QuickEntry.vue`, `DimensionPanel.vue`, `EntryInput.vue`, `MentionMenu.vue`
Base: `AppInput.vue`, `AppChip.vue`, `AppSelect.vue`, `Popover.vue`
Composite: `CommitmentsEditor.vue`
Tests: `DayStrip.test.ts`, `MonthNavigator.test.ts`, `QuickEntry.test.ts`, `EntryInput.test.ts`, `MentionMenu.test.ts`

### Retained files (7)
- `SetupScreen.vue`, `ConfigErrorBanner.vue` — unchanged
- `AppButton.vue`, `ProgressBar.vue`, `Toast.vue` — keep, may tweak later
- `SetupScreen.test.ts`, `ConfigErrorBanner.test.ts` — keep as-is

---

## Phase 0: Token Foundation

### Task 0.1: Overwrite tokens.css

**Files:**
- Modify: `src/assets/tokens.css`

- [ ] **Step 1: Copy token blueprint to src**

```bash
cp docs/superpowers/specs/2026-06-19-ux-redesign-tokens.css src/assets/tokens.css
```

- [ ] **Step 2: Verify the file is valid CSS**

```bash
npx tailwindcss --input src/assets/main.css --output /dev/null 2>&1
```

Expected: no errors (may have warnings about unused variables — that's fine at this stage).

- [ ] **Step 3: Verify existing tests still pass (prove backward compat)**

```bash
pnpm test
```

Expected: all existing tests pass. If any fail, the old tokens had values hardcoded in tests — fix the test expectations to match new token values.

- [ ] **Step 4: Commit**

```bash
git add src/assets/tokens.css
git commit -m "refactor: replace tokens.css with UX redesign token definitions"
```

---

## Phase 1: New Base Components

### Task 1.1: Create HeatmapCalendar

**Files:**
- Create: `src/components/HeatmapCalendar.vue`
- Create: `src/__tests__/components/HeatmapCalendar.test.ts`

- [ ] **Step 1: Write failing test**

```typescript
// src/__tests__/components/HeatmapCalendar.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import HeatmapCalendar from "../../../components/HeatmapCalendar.vue";

const MONTH_DATES = [
  "2026-06-01", "2026-06-02", "2026-06-03", "2026-06-04", "2026-06-05",
  "2026-06-06", "2026-06-07", "2026-06-08", "2026-06-09", "2026-06-10",
  "2026-06-11", "2026-06-12", "2026-06-13", "2026-06-14", "2026-06-15",
  "2026-06-16", "2026-06-17", "2026-06-18", "2026-06-19", "2026-06-20",
  "2026-06-21", "2026-06-22", "2026-06-23", "2026-06-24", "2026-06-25",
  "2026-06-26", "2026-06-27", "2026-06-28", "2026-06-29", "2026-06-30",
];

function makeDailyMinutes(dates: string[]): Record<string, number> {
  const map: Record<string, number> = {};
  for (const d of dates) map[d] = 0;
  return map;
}

describe("HeatmapCalendar", () => {
  it("renders 30 date cells for June", () => {
    const wrapper = mount(HeatmapCalendar, {
      props: {
        year: 2026,
        month: 6,
        dates: MONTH_DATES,
        dailyMinutes: makeDailyMinutes(MONTH_DATES),
        selectedDate: "2026-06-19",
        currentDate: "2026-06-19",
        monthTotalMinutes: 3510,
      },
    });
    const cells = wrapper.findAll(".heatmap-cell");
    // 30 days + first week empty cells (June starts on a Monday → 0 empty)
    expect(cells.length).toBe(30);
  });

  it("marks today with heatmap-today class", () => {
    const wrapper = mount(HeatmapCalendar, {
      props: {
        year: 2026,
        month: 6,
        dates: MONTH_DATES,
        dailyMinutes: makeDailyMinutes(MONTH_DATES),
        selectedDate: "2026-06-17",
        currentDate: "2026-06-19",
        monthTotalMinutes: 3510,
      },
    });
    const todayCell = wrapper.find('[data-date="2026-06-19"]');
    expect(todayCell.classes()).toContain("heatmap-today");
  });

  it("marks selected date with heatmap-selected class", () => {
    const wrapper = mount(HeatmapCalendar, {
      props: {
        year: 2026,
        month: 6,
        dates: MONTH_DATES,
        dailyMinutes: makeDailyMinutes(MONTH_DATES),
        selectedDate: "2026-06-17",
        currentDate: "2026-06-19",
        monthTotalMinutes: 3510,
      },
    });
    const selectedCell = wrapper.find('[data-date="2026-06-17"]');
    expect(selectedCell.classes()).toContain("heatmap-selected");
  });

  it("emits select-date when a non-future cell is clicked", async () => {
    const wrapper = mount(HeatmapCalendar, {
      props: {
        year: 2026,
        month: 6,
        dates: MONTH_DATES,
        dailyMinutes: makeDailyMinutes(MONTH_DATES),
        selectedDate: "2026-06-19",
        currentDate: "2026-06-19",
        monthTotalMinutes: 3510,
      },
    });
    await wrapper.find('[data-date="2026-06-15"]').trigger("click");
    expect(wrapper.emitted("select-date")?.[0]).toEqual(["2026-06-15"]);
  });

  it("emits navigate when arrows are clicked", async () => {
    const wrapper = mount(HeatmapCalendar, {
      props: {
        year: 2026,
        month: 6,
        dates: MONTH_DATES,
        dailyMinutes: makeDailyMinutes(MONTH_DATES),
        selectedDate: "2026-06-19",
        currentDate: "2026-06-19",
        monthTotalMinutes: 3510,
      },
    });
    await wrapper.find('[data-action="prev-month"]').trigger("click");
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2026, month: 5 }]);

    await wrapper.find('[data-action="next-month"]').trigger("click");
    expect(wrapper.emitted("navigate")?.[1]).toEqual([{ year: 2026, month: 7 }]);
  });

  it("emits request-months when month label is clicked and availableMonths is null", async () => {
    const wrapper = mount(HeatmapCalendar, {
      props: {
        year: 2026,
        month: 6,
        dates: MONTH_DATES,
        dailyMinutes: makeDailyMinutes(MONTH_DATES),
        selectedDate: "2026-06-19",
        currentDate: "2026-06-19",
        monthTotalMinutes: 3510,
        availableMonths: null,
      },
    });
    await wrapper.find('[data-action="toggle-popover"]').trigger("click");
    expect(wrapper.emitted("request-months")).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run src/__tests__/components/HeatmapCalendar.test.ts
```

Expected: FAIL — component doesn't exist.

- [ ] **Step 3: Implement HeatmapCalendar**

```vue
<!-- src/components/HeatmapCalendar.vue -->
<script setup lang="ts">
import { computed } from "vue";
import type { AvailableMonth } from "../stores/useStore";

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const props = defineProps<{
  year: number;
  month: number;
  dates: string[];
  dailyMinutes: Record<string, number>;
  selectedDate: string;
  currentDate: string;
  monthTotalMinutes: number;
  availableMonths?: AvailableMonth[] | null;
}>();

const emit = defineEmits<{
  "select-date": [date: string];
  "navigate": [{ year: number; month: number }];
  "request-months": [];
}>();

const showPopover = ref(false);

function heatmapLevel(minutes: number): string {
  if (minutes === 0) return "heatmap-empty";
  if (minutes <= 90) return "heatmap-light";
  if (minutes <= 240) return "heatmap-mid";
  return "heatmap-heavy";
}

function isToday(dateStr: string): boolean {
  return dateStr === props.currentDate;
}

function isFuture(dateStr: string): boolean {
  const now = new Date();
  now.setHours(0, 0, 0, 0);
  const [y, m, d] = dateStr.split("-").map(Number);
  const target = new Date(y, m - 1, d);
  target.setHours(0, 0, 0, 0);
  return target > now;
}

function dayNumber(dateStr: string): number {
  return parseInt(dateStr.split("-")[2], 10);
}

function handleClick(dateStr: string) {
  if (isFuture(dateStr)) return;
  emit("select-date", dateStr);
}

function shiftMonth(delta: number) {
  let newMonth = props.month + delta;
  let newYear = props.year;
  if (newMonth < 1) { newMonth = 12; newYear--; }
  else if (newMonth > 12) { newMonth = 1; newYear++; }
  emit("navigate", { year: newYear, month: newMonth });
}

function handleLabelClick() {
  if (props.availableMonths === null || props.availableMonths === undefined) {
    emit("request-months");
    return;
  }
  showPopover.value = !showPopover.value;
}

const availableYears = computed(() => {
  if (!props.availableMonths) return [];
  const years = [...new Set(props.availableMonths.map(m => m.year))];
  years.sort((a, b) => b - a);
  return years;
});

const selectedPopoverYear = ref(props.year);

const monthsForYear = computed(() => {
  if (!props.availableMonths) return [];
  return props.availableMonths
    .filter(m => m.year === selectedPopoverYear.value)
    .map(m => m.month)
    .sort((a, b) => a - b);
});

function onQuickJump(month: number) {
  emit("navigate", { year: selectedPopoverYear.value, month });
  showPopover.value = false;
}

// Day-of-week offset: first day of month (0=Sun, 1=Mon, ..., 6=Sat)
const startOffset = computed(() => {
  const first = props.dates[0];
  if (!first) return 0;
  const d = new Date(first);
  return d.getDay() === 0 ? 6 : d.getDay() - 1; // convert to Mon=0
});
</script>

<template>
  <div>
    <!-- Month navigator -->
    <div style="display:flex; align-items:center; justify-content:space-between; margin-bottom:8px;">
      <span data-action="prev-month" style="font-size:12px; color:var(--color-text-secondary); cursor:pointer; padding:2px 4px;" @click="shiftMonth(-1)">←</span>
      <span data-action="toggle-popover" style="font-size:var(--text-base); font-weight:var(--weight-bold); color:var(--color-text-primary); cursor:pointer;" @click="handleLabelClick">
        {{ MONTH_NAMES[month - 1] }} <span style="font-weight:var(--weight-book); font-size:var(--text-xs-alt); color:var(--color-text-secondary);">{{ year }} ▾</span>
      </span>
      <span data-action="next-month" style="font-size:12px; color:var(--color-text-secondary); cursor:pointer; padding:2px 4px;" @click="shiftMonth(1)">→</span>
    </div>

    <!-- Quick-jump popover -->
    <div v-if="showPopover && availableMonths" style="background:var(--color-surface); border:1px solid var(--color-border-form); border-radius:10px; box-shadow:var(--shadow-quickjump); padding:10px 12px; margin-bottom:8px; display:flex; gap:8px; align-items:center;">
      <select v-model="selectedPopoverYear" style="font-size:var(--text-xs); border:1px solid var(--color-border-form); border-radius:6px; padding:4px 8px; background:var(--color-surface); color:var(--color-text-primary); outline:none;">
        <option v-for="y in availableYears" :key="y" :value="y">{{ y }}</option>
      </select>
      <select style="font-size:var(--text-xs); border:1px solid var(--color-border-form); border-radius:6px; padding:4px 8px; background:var(--color-surface); color:var(--color-text-primary); outline:none;" @change="onQuickJump(parseInt(($event.target as HTMLSelectElement).value, 10))">
        <option v-for="m in monthsForYear" :key="m" :value="m" :selected="m === month && selectedPopoverYear === year">{{ MONTH_NAMES[m - 1] }}</option>
      </select>
    </div>

    <!-- Day-of-week headers -->
    <div style="display:grid; grid-template-columns:repeat(7,1fr); gap:3px; text-align:center; font-size:var(--text-2xs); color:var(--color-text-secondary);">
      <span>M</span><span>T</span><span>W</span><span>T</span><span>F</span><span>S</span><span>S</span>
    </div>

    <!-- Heatmap grid -->
    <div style="display:grid; grid-template-columns:repeat(7,1fr); gap:3px; text-align:center;">
      <span v-for="i in startOffset" :key="'empty-'+i"></span>
      <span
        v-for="dateStr in dates"
        :key="dateStr"
        :data-date="dateStr"
        class="heatmap-cell"
        :class="[
          isFuture(dateStr) ? 'heatmap-empty' : heatmapLevel(dailyMinutes[dateStr] || 0),
          isToday(dateStr) ? 'heatmap-today' : '',
          dateStr === selectedDate ? 'heatmap-selected' : '',
        ]"
        :style="{
          width: '24px', height: '24px', borderRadius: 'var(--radius-md)',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          fontSize: 'var(--text-micro)', cursor: isFuture(dateStr) ? 'default' : 'pointer',
          opacity: isFuture(dateStr) ? 0.3 : 1,
          fontFamily: 'var(--font-mono)',
        }"
        @click="handleClick(dateStr)"
      >
        {{ dayNumber(dateStr) }}
      </span>
    </div>

    <div style="margin-top:6px; font-size:var(--text-xs-alt); color:var(--color-text-primary); text-align:center; font-weight:var(--weight-semibold);">
      <span style="font-family:var(--font-mono);">{{ (monthTotalMinutes / 60).toFixed(1) }}h</span>
      <span style="font-weight:var(--weight-book); font-size:var(--text-micro); color:var(--color-text-secondary);"> / month</span>
    </div>
  </div>
</template>
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
npx vitest run src/__tests__/components/HeatmapCalendar.test.ts
```

Expected: all 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/HeatmapCalendar.vue src/__tests__/components/HeatmapCalendar.test.ts
git commit -m "feat: add HeatmapCalendar component with heatmap color scale"
```

---

### Task 1.2: Create DimensionPopover

**Files:**
- Create: `src/components/DimensionPopover.vue`
- Create: `src/__tests__/components/DimensionPopover.test.ts`

- [ ] **Step 1: Write failing test**

```typescript
// src/__tests__/components/DimensionPopover.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import DimensionPopover from "../../../components/DimensionPopover.vue";
import { makeDimension, makeCommitment } from "../../mocks/fixtures";
import type { Dimension, Commitment } from "../../../types";

const dimensions: Dimension[] = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering", "PM"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
  makeDimension({ name: "Business Line", key: "business-line", source: "static", values: ["Platform", "Slax"], required: false }),
];

const commitments: Commitment[] = [
  makeCommitment({ role: "Developer", goals: ["Ship feature X", "Bug fixes"] }),
];

describe("DimensionPopover", () => {
  it("starts in dim phase and renders all dimensions", () => {
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments, dimValues: {} },
    });
    expect(wrapper.text()).toContain("Category");
    expect(wrapper.text()).toContain("Goal");
    expect(wrapper.text()).toContain("Business Line");
    expect(wrapper.text()).toContain("required");
    expect(wrapper.text()).toContain("optional");
  });

  it("emits select-dim when a dimension is clicked", async () => {
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments, dimValues: {} },
    });
    await wrapper.findAll(".popover-item")[0].trigger("click");
    expect(wrapper.emitted("select-dim")?.[0]).toEqual(["category"]);
  });

  it("switches to val phase when in val mode", async () => {
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments, dimValues: {}, activeDimKey: "goal", phase: "val" },
    });
    expect(wrapper.text()).toContain("Ship feature X");
    expect(wrapper.text()).toContain("Bug fixes");
  });

  it("emits select-val when a value is clicked in val phase", async () => {
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments, dimValues: {}, activeDimKey: "goal", phase: "val" },
    });
    await wrapper.findAll(".popover-item")[0].trigger("click");
    expect(wrapper.emitted("select-val")?.[0]).toEqual(["goal", "Ship feature X"]);
  });

  it("emits close on Esc in dim phase", async () => {
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments, dimValues: {} },
    });
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("emits back on Esc in val phase", async () => {
    const wrapper = mount(DimensionPopover, {
      props: { dimensions, commitments, dimValues: {}, activeDimKey: "goal", phase: "val" },
    });
    await wrapper.trigger("keydown", { key: "Escape" });
    expect(wrapper.emitted("back")).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run src/__tests__/components/DimensionPopover.test.ts
```

- [ ] **Step 3: Implement DimensionPopover**

```vue
<!-- src/components/DimensionPopover.vue -->
<script setup lang="ts">
import { computed } from "vue";
import type { Dimension, Commitment } from "../types";

const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
  dimValues: Record<string, string>;
  activeDimKey?: string | null;
  phase?: "dim" | "val";
}>();

const emit = defineEmits<{
  "select-dim": [dimKey: string];
  "select-val": [dimKey: string, value: string];
  "close": [];
  "back": [];
}>();

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of props.commitments) for (const g of c.goals) goals.add(g);
  return [...goals];
});

function dimBarClass(key: string): string {
  const map: Record<string, string> = {
    goal: "goals-bar",
    category: "cat-bar",
    "business-line": "biz-bar",
    "importance-urgency": "imp-bar",
  };
  return map[key] || "cat-bar";
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    e.preventDefault();
    if (props.phase === "val") emit("back");
    else emit("close");
  }
}
</script>

<template>
  <div
    class="popover"
    style="background:var(--color-surface); border:1px solid var(--color-border-form); border-radius:var(--radius-card); box-shadow:var(--shadow-popover); overflow:hidden; width:240px;"
    @keydown="handleKeydown"
    tabindex="0"
  >
    <!-- Dim phase header -->
    <div v-if="phase !== 'val'" style="padding:8px 14px; font-size:var(--text-micro); font-weight:var(--weight-bold); text-transform:uppercase; letter-spacing:0.5px; background:var(--color-popover-dim-header-bg); color:var(--color-popover-dim-header-text); border-bottom:1px solid var(--color-divider); display:flex; align-items:center; gap:8px;">
      <span style="background:var(--color-brand-solid); color:#fff; padding:1px 6px; border-radius:3px; font-size:var(--text-2xs);">DIM</span>
      Pick a dimension
    </div>

    <!-- Val phase header -->
    <div v-else style="padding:8px 14px; font-size:var(--text-micro); font-weight:var(--weight-bold); background:var(--color-popover-val-header-bg); color:var(--color-popover-val-header-text); border-bottom:1px solid var(--color-divider); display:flex; align-items:center; gap:8px;">
      <span style="cursor:pointer; font-weight:var(--weight-bold); font-size:var(--text-xs);" @click="$emit('back')">←</span>
      <span style="color:var(--color-chip-goal-text); font-weight:var(--weight-semibold);">{{ dimensions.find(d => d.key === activeDimKey)?.name || '' }}</span>
    </div>

    <!-- Dim phase: list dimensions -->
    <template v-if="phase !== 'val'">
      <div
        v-for="d in dimensions" :key="d.key"
        class="popover-item"
        style="padding:9px 14px; font-size:var(--text-sm); cursor:pointer; color:var(--color-text-primary); display:flex; align-items:center; gap:10px; border-bottom:1px solid var(--color-divider);"
        @click="$emit('select-dim', d.key)"
      >
        <span :class="dimBarClass(d.key)" style="width:3px; height:18px; border-radius:var(--radius-sm); flex-shrink:0;"></span>
        {{ d.name }}
        <span style="margin-left:auto; font-size:var(--text-micro);" :class="d.required ? 'meta-required' : 'meta-optional'">
          {{ d.required ? 'required' : 'optional' }}
        </span>
      </div>
      <!-- Footer -->
      <div style="padding:6px 14px; font-size:var(--text-2xs); color:var(--color-text-disabled); border-top:1px solid var(--color-divider); display:flex; gap:12px; font-family:var(--font-mono);">
        <span><kbd style="padding:1px 4px; border:1px solid var(--color-border-form); border-radius:var(--radius-sm); background:var(--color-surface); font-size:8px;">↵</kbd> select</span>
        <span><kbd style="padding:1px 4px; border:1px solid var(--color-border-form); border-radius:var(--radius-sm); background:var(--color-surface); font-size:8px;">esc</kbd> close</span>
      </div>
    </template>

    <!-- Val phase: list values -->
    <template v-else>
      <div
        v-for="v in (activeDimKey && dimensions.find(d => d.key === activeDimKey)?.source === 'monthly' ? goalOptions : dimensions.find(d => d.key === activeDimKey)?.values || [])"
        :key="v"
        class="popover-item"
        style="padding:9px 14px; font-size:var(--text-sm); cursor:pointer; color:var(--color-text-primary); border-bottom:1px solid var(--color-divider);"
        @click="$emit('select-val', activeDimKey!, v)"
      >
        {{ v }}
      </div>
      <div style="padding:6px 14px; font-size:var(--text-2xs); color:var(--color-text-disabled); border-top:1px solid var(--color-divider); display:flex; gap:12px; font-family:var(--font-mono);">
        <span><kbd style="padding:1px 4px; border:1px solid var(--color-border-form); border-radius:var(--radius-sm); background:var(--color-surface); font-size:8px;">↵</kbd> select</span>
        <span><kbd style="padding:1px 4px; border:1px solid var(--color-border-form); border-radius:var(--radius-sm); background:var(--color-surface); font-size:8px;">esc</kbd> back to dims</span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.goals-bar { background: var(--dim-bar-goal); }
.cat-bar { background: var(--dim-bar-cat); }
.biz-bar { background: var(--dim-bar-biz); }
.imp-bar { background: var(--dim-bar-imp); }
.meta-required { color: var(--color-warning); font-weight: var(--weight-medium); }
.meta-optional { color: var(--color-text-disabled); }
</style>
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/__tests__/components/DimensionPopover.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add src/components/DimensionPopover.vue src/__tests__/components/DimensionPopover.test.ts
git commit -m "feat: add DimensionPopover with dim/val two-phase flow"
```

---

### Task 1.3: Create TwoLineInput

**Files:**
- Create: `src/components/TwoLineInput.vue`
- Create: `src/__tests__/components/TwoLineInput.test.ts`

- [ ] **Step 1: Write failing test**

```typescript
// src/__tests__/components/TwoLineInput.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import TwoLineInput from "../../../components/TwoLineInput.vue";
import { makeDimension, makeCommitment } from "../../mocks/fixtures";
import type { Dimension, Commitment } from "../../../types";

const dimensions: Dimension[] = [
  makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering"], required: true }),
  makeDimension({ name: "Goal", key: "goal", source: "monthly", required: true }),
];

const commitments: Commitment[] = [
  makeCommitment({ goals: ["Feature X"] }),
];

describe("TwoLineInput", () => {
  it("renders input field and Enter hint", () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions, commitments, initialValues: {} },
    });
    expect(wrapper.find("input").exists()).toBe(true);
    expect(wrapper.text()).toContain("⏎");
  });

  it("parses duration from item text and displays dur token", async () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions, commitments, initialValues: {} },
    });
    const input = wrapper.find("input");
    await input.setValue("Code review 1.5h");
    // Token row should show 1h 30m
    expect(wrapper.text()).toContain("1h 30m");
  });

  it("emits submit with item, duration, and dimensions on Enter", async () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions, commitments, initialValues: { category: "Engineering" } },
    });
    const input = wrapper.find("input");
    await input.setValue("Code review 1h");
    await input.trigger("keydown", { key: "Enter" });
    const submitEvent = wrapper.emitted("submit");
    expect(submitEvent).toBeTruthy();
    expect(submitEvent![0]).toEqual(["Code review", 60, { category: "Engineering" }]);
  });

  it("shows missing indicator for unfilled required dimensions", () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions, commitments, initialValues: {} },
    });
    expect(wrapper.find(".missing-indicator").exists()).toBe(true);
    expect(wrapper.text()).toContain("category");
  });

  it("opens popover on @ key", async () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions, commitments, initialValues: {} },
    });
    const input = wrapper.find("input");
    await input.trigger("keydown", { key: "@" });
    // Popover should be visible
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);
  });

  it("clears input via exposed clearInput method", async () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions, commitments, initialValues: {} },
    });
    const input = wrapper.find("input");
    await input.setValue("Something 1h");
    (wrapper.vm as any).clearInput();
    expect(input.element.value).toBe("");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run src/__tests__/components/TwoLineInput.test.ts
```

- [ ] **Step 3: Implement TwoLineInput**

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

const input = ref("");
const dimValues = ref<Record<string, string>>({ ...props.initialValues });

watch(() => props.initialValues, (vals) => {
  if (Object.keys(vals).length > 0) dimValues.value = { ...vals };
}, { immediate: true });

const inputEl = ref<HTMLInputElement | null>(null);

// Duration: auto-parsed from item text
const parsedDuration = computed(() => {
  if (!input.value.trim()) return null;
  return parseDurationFromText(input.value.trim());
});

// Chips: dimension values + duration
const allRequiredFilled = computed(() =>
  props.dimensions.filter(d => d.required).every(d => dimValues.value[d.key])
);

const missingRequired = computed(() =>
  props.dimensions.filter(d => d.required && !dimValues.value[d.key])
);

// Popover state
const popoverVisible = ref(false);
const popoverPhase = ref<"dim" | "val">("dim");
const activeDimKey = ref<string | null>(null);

function openDimMenu() {
  popoverPhase.value = "dim";
  activeDimKey.value = null;
  popoverVisible.value = true;
}

function onSelectDim(dimKey: string) {
  activeDimKey.value = dimKey;
  popoverPhase.value = "val";
}

function onSelectVal(dimKey: string, value: string) {
  dimValues.value = { ...dimValues.value, [dimKey]: value };
  popoverVisible.value = false;
  // Re-open if more required dims remain
  if (!allRequiredFilled.value) {
    setTimeout(() => {
      popoverPhase.value = "dim";
      activeDimKey.value = null;
      popoverVisible.value = true;
    }, 50);
  }
}

function closePopover() {
  popoverVisible.value = false;
}

function onPopoverBack() {
  popoverPhase.value = "dim";
  activeDimKey.value = null;
}

// Token chip helpers
function chipColor(key: string): string {
  const map: Record<string, string> = {
    category: "cat", "business-line": "biz",
    "importance-urgency": "imp", goal: "goal",
  };
  return map[key] || "cat";
}

function removeDim(key: string) {
  dimValues.value = { ...dimValues.value, [key]: "" };
}

// Keyboard handler
function onKeydown(e: KeyboardEvent) {
  if (e.key === "@") {
    e.preventDefault();
    openDimMenu();
    return;
  }
  if (e.key === "#") {
    e.preventDefault();
    // Focus the duration helper: append " #" to trigger manual mode
    // For now, rely on auto-parse from text
    return;
  }
  if (e.key === "Enter") {
    e.preventDefault();
    handleSubmit();
    return;
  }
}

// Submit
function handleSubmit() {
  const trimmed = input.value.trim();
  if (!trimmed) return;
  const d = parsedDuration.value;
  if (!d) return;
  const item = stripDurations(trimmed);
  emit("submit", item, d, { ...dimValues.value });
}

function clearInput() {
  input.value = "";
}

defineExpose({ clearInput, inputEl });

// Focus injection
const focusRequestId = inject<Ref<number>>("focusRequestId", ref(0));
watch(focusRequestId, () => {
  const active = document.activeElement;
  if (!active || active === document.body || active.tagName === "BODY") {
    inputEl.value?.focus();
  }
});
</script>

<template>
  <div style="position:relative;">
    <!-- Input card -->
    <div class="input-card" :class="{ focused: true }" style="background:var(--color-surface); border:2px solid var(--color-border-form); border-radius:var(--radius-card); padding:10px 16px;">
      <!-- Line 1: item text -->
      <div style="display:flex; gap:8px; align-items:center;">
        <span style="color:var(--color-brand-solid); font-size:var(--text-lg); line-height:1; flex-shrink:0;">+</span>
        <input
          ref="inputEl"
          v-model="input"
          placeholder="What did you work on?"
          style="flex:1; border:none; outline:none; font-size:var(--text-base); color:var(--color-text-primary); background:transparent; caret-color:var(--color-brand-solid); line-height:1.5; padding:2px 0;"
          @keydown="onKeydown"
        >
        <span style="font-size:var(--text-2xs); color:var(--color-text-secondary); padding:3px 7px; border:1px solid var(--color-border-form); border-radius:var(--radius-md); font-weight:var(--weight-semibold); flex-shrink:0; font-family:var(--font-mono);">⏎</span>
      </div>

      <!-- Line 2: tokens + missing indicators -->
      <div style="display:flex; gap:4px; margin-top:6px; flex-wrap:wrap; align-items:center; min-height:4px; padding-left:2px;">
        <!-- Dimension tokens -->
        <span
          v-for="dim in props.dimensions.filter(d => dimValues[d.key])"
          :key="dim.key"
          :class="'input-token ' + chipColor(dim.key)"
          style="font-size:var(--text-micro); padding:1px 7px; border-radius:var(--radius-sm); font-weight:var(--weight-medium); display:inline-flex; align-items:center; gap:4px; line-height:1.6;"
        >
          {{ dimValues[dim.key] }}
          <span style="cursor:pointer; opacity:0.4; font-size:var(--text-xs); line-height:1;" @click="removeDim(dim.key)">&times;</span>
        </span>
        <!-- Duration token -->
        <span
          v-if="parsedDuration"
          class="input-token dur-token"
          style="font-size:var(--text-micro); padding:1px 7px; border-radius:var(--radius-sm); font-weight:var(--weight-medium); display:inline-flex; align-items:center; gap:4px; line-height:1.6; font-family:var(--font-mono);"
        >
          {{ formatDuration(parsedDuration) }}
          <span style="cursor:pointer; opacity:0.4; font-size:var(--text-xs); line-height:1;" @click="/* duration auto-re-parses on next input */">&times;</span>
        </span>
        <!-- Missing required indicators -->
        <span
          v-for="m in missingRequired"
          :key="'missing-'+m.key"
          class="missing-indicator"
          style="font-size:var(--text-micro); padding:1px 8px; border-radius:var(--radius-sm); border:1.5px dashed var(--color-missing-border); color:var(--color-missing-text); font-weight:450; cursor:pointer; display:inline-flex; align-items:center; gap:3px;"
          @click="openDimMenu"
        >
          <span style="width:5px; height:5px; border-radius:50%; background:var(--color-missing-dot);"></span>
          {{ m.name }}
        </span>
      </div>
    </div>

    <!-- Popover -->
    <DimensionPopover
      v-if="popoverVisible"
      :dimensions="dimensions"
      :commitments="commitments"
      :dim-values="dimValues"
      :active-dim-key="activeDimKey"
      :phase="popoverPhase"
      @select-dim="onSelectDim"
      @select-val="onSelectVal"
      @close="closePopover"
      @back="onPopoverBack"
      style="position:absolute; left:0; top:100%; margin-top:4px; z-index:10;"
    />

    <!-- Hints row -->
    <div style="display:flex; gap:14px; margin-top:4px; font-size:var(--text-micro); color:var(--color-text-disabled);">
      <span><kbd style="padding:1px 5px; border:1px solid var(--color-border-form); border-radius:var(--radius-sm); background:var(--color-surface); font-size:var(--text-2xs); font-family:var(--font-mono);">@</kbd> dim</span>
      <span><kbd style="padding:1px 5px; border:1px solid var(--color-border-form); border-radius:var(--radius-sm); background:var(--color-surface); font-size:var(--text-2xs); font-family:var(--font-mono);">#</kbd> time</span>
      <span><kbd style="padding:1px 5px; border:1px solid var(--color-border-form); border-radius:var(--radius-sm); background:var(--color-surface); font-size:var(--text-2xs); font-family:var(--font-mono);">⌘[</kbd> prev</span>
      <span><kbd style="padding:1px 5px; border:1px solid var(--color-border-form); border-radius:var(--radius-sm); background:var(--color-surface); font-size:var(--text-2xs); font-family:var(--font-mono);">⌘]</kbd> next</span>
    </div>
  </div>
</template>

<style scoped>
.input-token.cat { background: var(--color-token-cat-bg); color: var(--color-token-cat-text); }
.input-token.biz { background: var(--color-token-biz-bg); color: var(--color-token-biz-text); }
.input-token.imp { background: var(--color-token-imp-bg); color: var(--color-token-imp-text); }
.input-token.goal { background: var(--color-token-goal-bg); color: var(--color-token-goal-text); }
.dur-token { background: var(--color-token-dur-bg); color: var(--color-token-dur-text); }
</style>
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/__tests__/components/TwoLineInput.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add src/components/TwoLineInput.vue src/__tests__/components/TwoLineInput.test.ts
git commit -m "feat: add TwoLineInput with auto-duration-parse and inline tokens"
```

---

### Task 1.4: Create QuickJumpPopover

**Files:**
- Create: `src/components/QuickJumpPopover.vue`
- Create: `src/__tests__/components/QuickJumpPopover.test.ts`

- [ ] **Step 1: Write failing test**

```typescript
// src/__tests__/components/QuickJumpPopover.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import QuickJumpPopover from "../../../components/QuickJumpPopover.vue";

const availableMonths = [
  { year: 2026, month: 5 }, { year: 2026, month: 6 }, { year: 2026, month: 7 },
  { year: 2025, month: 12 },
];

describe("QuickJumpPopover", () => {
  it("renders year and month selects", () => {
    const wrapper = mount(QuickJumpPopover, {
      props: { year: 2026, month: 6, availableMonths },
    });
    const selects = wrapper.findAll("select");
    expect(selects.length).toBe(2);
  });

  it("emits navigate when month is selected", async () => {
    const wrapper = mount(QuickJumpPopover, {
      props: { year: 2026, month: 6, availableMonths },
    });
    const monthSelect = wrapper.findAll("select")[1];
    await monthSelect.setValue("5");
    await monthSelect.trigger("change");
    expect(wrapper.emitted("navigate")?.[0]).toEqual([{ year: 2026, month: 5 }]);
  });

  it("filters months by selected year", async () => {
    const wrapper = mount(QuickJumpPopover, {
      props: { year: 2026, month: 6, availableMonths },
    });
    const yearSelect = wrapper.findAll("select")[0];
    await yearSelect.setValue("2025");
    await yearSelect.trigger("change");
    // After year change, month select should only show months for 2025
    const monthSelect = wrapper.findAll("select")[1];
    const options = monthSelect.findAll("option");
    expect(options.length).toBe(1); // only December 2025
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run src/__tests__/components/QuickJumpPopover.test.ts
```

- [ ] **Step 3: Implement QuickJumpPopover**

```vue
<!-- src/components/QuickJumpPopover.vue -->
<script setup lang="ts">
import { ref, computed } from "vue";

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const props = defineProps<{
  year: number;
  month: number;
  availableMonths: { year: number; month: number }[];
}>();

const emit = defineEmits<{
  navigate: [{ year: number; month: number }];
}>();

const selectedYear = ref(props.year);

const availableYears = computed(() => {
  const years = [...new Set(props.availableMonths.map(m => m.year))];
  years.sort((a, b) => b - a);
  return years;
});

const monthsForYear = computed(() =>
  props.availableMonths
    .filter(m => m.year === selectedYear.value)
    .map(m => m.month)
    .sort((a, b) => a - b)
);

function onMonthChange(month: number) {
  emit("navigate", { year: selectedYear.value, month });
}
</script>

<template>
  <div style="background:var(--color-surface); border:1px solid var(--color-border-form); border-radius:10px; box-shadow:var(--shadow-quickjump); padding:10px 12px; display:flex; gap:8px; align-items:center;">
    <select v-model="selectedYear" style="font-size:var(--text-xs); border:1px solid var(--color-border-form); border-radius:6px; padding:4px 8px; background:var(--color-surface); color:var(--color-text-primary); outline:none;">
      <option v-for="y in availableYears" :key="y" :value="y">{{ y }}</option>
    </select>
    <select style="font-size:var(--text-xs); border:1px solid var(--color-border-form); border-radius:6px; padding:4px 8px; background:var(--color-surface); color:var(--color-text-primary); outline:none;" @change="onMonthChange(parseInt(($event.target as HTMLSelectElement).value, 10))">
      <option v-for="m in monthsForYear" :key="m" :value="m" :selected="m === month && selectedYear === year">{{ MONTH_NAMES[m - 1] }}</option>
    </select>
    <span style="font-size:var(--text-2xs); color:var(--color-text-secondary); white-space:nowrap;">Go</span>
  </div>
</template>
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/__tests__/components/QuickJumpPopover.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add src/components/QuickJumpPopover.vue src/__tests__/components/QuickJumpPopover.test.ts
git commit -m "feat: add QuickJumpPopover for year/month navigation"
```

---

## Phase 2: Rewrite Core Components

### Task 2.1: Rewrite EntryRow with new visual spec + hover edit trigger

**Files:**
- Modify: `src/components/composite/EntryRow.vue` (full rewrite)

- [ ] **Step 1: Read current EntryRow test to understand existing signatures**

```bash
# No action needed — existing test is at src/__tests__/components/composite/EntryRow.test.ts
```

- [ ] **Step 2: Update the test file for new behavior**

```typescript
// Rewrite src/__tests__/components/composite/EntryRow.test.ts
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import EntryRow from "../../../components/composite/EntryRow.vue";
import { makeEntry, makeConfig } from "../../mocks/fixtures";
import { STORE_KEY } from "../../../stores/useStore";
import { createTestStore } from "../../mocks/store";

function mountRow(entryOverrides: Record<string, unknown> = {}) {
  const store = createTestStore({ config: makeConfig() });
  const entry = makeEntry(entryOverrides as any);
  const wrapper = mount(EntryRow, {
    props: { entry, index: 0 },
    global: { provide: { [STORE_KEY as symbol]: store } },
  });
  return { wrapper, store, entry };
}

describe("EntryRow", () => {
  it("renders item text, formatted duration", () => {
    const { wrapper } = mountRow({ item: "Write tests", duration: 75 });
    expect(wrapper.text()).toContain("Write tests");
    expect(wrapper.text()).toContain("1h 15m");
  });

  it("renders dimension chips with muted colors", () => {
    const { wrapper } = mountRow({ dimensions: { category: "Engineering", "business-line": "Slax" } });
    const chips = wrapper.findAll(".entry-chip");
    expect(chips.length).toBe(2);
    expect(chips[0].text()).toContain("Engineering");
  });

  it("edit trigger is hidden by default (opacity:0)", () => {
    const { wrapper } = mountRow();
    const trigger = wrapper.find(".edit-trigger");
    expect(trigger.exists()).toBe(true);
    expect(trigger.attributes("style")).toContain("opacity: 0");
  });

  it("emits update when edit is saved", async () => {
    const { wrapper } = mountRow({ id: "e1", item: "Old", duration: 30 });
    // Enter edit mode by emitting directly (clicking ⋯ requires hover which is hard in jsdom)
    await wrapper.find(".edit-trigger").trigger("click");
    const itemInput = wrapper.find(".edit-item-input");
    await itemInput.setValue("New item");
    await wrapper.find(".edit-save").trigger("click");
    expect(wrapper.emitted("update")?.[0]).toEqual(["e1", "New item", 30]);
  });

  it("emits delete", async () => {
    const { wrapper } = mountRow({ id: "e2" });
    await wrapper.find(".edit-trigger").trigger("click");
    await wrapper.find(".edit-delete").trigger("click");
    expect(wrapper.emitted("delete")?.[0]).toEqual(["e2"]);
  });

  it("Cancel exits edit mode without emitting", async () => {
    const { wrapper } = mountRow({ id: "e3" });
    await wrapper.find(".edit-trigger").trigger("click");
    await wrapper.find(".edit-cancel").trigger("click");
    expect(wrapper.find(".edit-item-input").exists()).toBe(false);
  });

  it("double-click also enters edit mode", async () => {
    const { wrapper } = mountRow({ item: "Double click me" });
    await wrapper.find(".entry-row-content").trigger("dblclick");
    expect(wrapper.find(".edit-item-input").exists()).toBe(true);
  });

  it("chips are truncated with max-width and ellipsis", () => {
    const { wrapper } = mountRow({ dimensions: { category: "Engineering" } });
    const chip = wrapper.find(".entry-chip");
    expect(chip.attributes("style")).toContain("max-width: 100px");
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
npx vitest run src/__tests__/components/composite/EntryRow.test.ts
```

Expected: FAIL — component doesn't match new test expectations.

- [ ] **Step 4: Rewrite EntryRow**

```vue
<!-- src/components/composite/EntryRow.vue -->
<script setup lang="ts">
import { ref, computed } from 'vue';
import type { Entry } from '../../types';
import { formatDuration } from '../../utils/format';
import { useStore } from '../../stores/useStore';

const props = defineProps<{
  entry: Entry;
  index: number;
}>();

const emit = defineEmits<{
  update: [entryId: string, item: string, durationMinutes: number];
  delete: [entryId: string];
  updateDimensions: [entryId: string, dimensions: Record<string, string>];
}>();

const store = useStore();
const editing = ref(false);

const itemInput = ref('');
const durInput = ref('');

function enterEdit() {
  itemInput.value = props.entry.item;
  durInput.value = String(props.entry.duration);
  editing.value = true;
}

function saveEdit() {
  editing.value = false;
  const newItem = itemInput.value.trim() || '(untitled)';
  const newDur = parseInt(durInput.value, 10) || props.entry.duration;
  if (newItem !== props.entry.item || newDur !== props.entry.duration) {
    emit('update', props.entry.id, newItem, newDur);
  }
}

function cancelEdit() {
  editing.value = false;
}

function chipClass(key: string): string {
  const map: Record<string, string> = { category: 'cat', 'business-line': 'biz', 'importance-urgency': 'imp', goal: 'goal' };
  return map[key] || 'cat';
}

const orderedDimensions = computed(() => store.config?.dimensions || []);

const goalOptions = computed(() => {
  const goals = new Set<string>();
  for (const c of store.commitments) for (const g of c.goals) goals.add(g);
  return [...goals];
});

function onDimChange(dimKey: string, value: string) {
  emit('updateDimensions', props.entry.id, { ...props.entry.dimensions, [dimKey]: value });
}

function removeEditChip(key: string) {
  emit('updateDimensions', props.entry.id, { ...props.entry.dimensions, [key]: '' });
}

// Determine which dims can be added (not already set)
const addableDims = computed(() =>
  orderedDimensions.value.filter(d => !props.entry.dimensions[d.key])
);
</script>

<template>
  <div
    class="entry-row-content"
    :class="editing ? 'entry-row editing' : 'entry-row'"
    :style="editing ? { background: 'var(--color-surface)', borderColor: 'var(--color-brand-solid)', boxShadow: 'var(--shadow-focus-ring)', cursor: 'default' } : {}"
  >
    <!-- Display mode -->
    <template v-if="!editing">
      <div style="flex:1; min-width:0;">
        <div
          class="entry-item"
          :title="entry.item.length > 25 ? entry.item : undefined"
          style="font-size:var(--text-base); font-weight:var(--weight-medium); color:var(--color-text-primary); line-height:1.4; word-break:break-word; overflow:hidden; display:-webkit-box; -webkit-line-clamp:2; -webkit-box-orient:vertical;"
        >
          {{ entry.item }}
        </div>
        <div v-if="Object.keys(entry.dimensions).length" style="display:flex; gap:3px; margin-top:3px; flex-wrap:wrap;">
          <span
            v-for="dim in orderedDimensions.filter(d => entry.dimensions[d.key])"
            :key="dim.key"
            :class="'entry-chip ' + chipClass(dim.key)"
            :title="entry.dimensions[dim.key]"
            style="font-size:var(--text-micro); padding:0px 6px; border-radius:var(--radius-sm); font-weight:450; line-height:1.7; max-width:100px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;"
          >
            {{ entry.dimensions[dim.key] }}
          </span>
        </div>
      </div>
      <span class="entry-duration" style="font-size:var(--text-sm); color:var(--color-text-primary); flex-shrink:0; margin-left:16px; font-family:var(--font-mono); tabular-nums:lining-nums;" @dblclick.stop="enterEdit">
        {{ formatDuration(entry.duration) }}
      </span>
      <span
        class="edit-trigger"
        title="Click or double-click to edit"
        style="opacity:0; font-size:14px; color:var(--color-text-secondary); cursor:pointer; flex-shrink:0; margin-left:8px; padding:0 2px;"
        @click="enterEdit"
      >⋯</span>
    </template>

    <!-- Edit mode -->
    <template v-else>
      <div class="edit-row" style="display:flex; flex-direction:column; gap:4px; width:100%;">
        <div style="display:flex; gap:8px; align-items:center;">
          <input
            v-model="itemInput"
            class="edit-item-input"
            style="font-size:var(--text-base); border:none; outline:none; background:transparent; color:var(--color-text-primary); font-weight:var(--weight-medium); width:100%; padding:1px 0;"
            @keydown.enter.prevent="saveEdit"
            @keydown.escape.prevent="cancelEdit"
            autofocus
          >
          <input
            v-model="durInput"
            style="font-size:var(--text-sm); border:1px solid var(--color-border-form); border-radius:var(--radius-form); padding:2px 8px; outline:none; color:var(--color-text-primary); width:56px; text-align:right; font-family:var(--font-mono); tabular-nums:lining-nums;"
            @keydown.enter.prevent="saveEdit"
            @keydown.escape.prevent="cancelEdit"
          >
          <span style="font-size:var(--text-xs-alt); color:var(--color-text-secondary);">min</span>
        </div>
        <div class="edit-chips" style="display:flex; gap:3px; flex-wrap:wrap; margin-top:2px;">
          <span
            v-for="dim in orderedDimensions.filter(d => entry.dimensions[d.key])"
            :key="dim.key"
            :class="'edit-chip ' + chipClass(dim.key)"
            style="font-size:var(--text-micro); padding:1px 7px; border-radius:var(--radius-sm); display:inline-flex; align-items:center; gap:5px; cursor:pointer;"
          >
            {{ entry.dimensions[dim.key] }}
            <span style="font-size:var(--text-xs-alt); cursor:pointer; opacity:0.5;" @click="removeEditChip(dim.key)">&times;</span>
          </span>
          <!-- +tag for addable dims -->
          <span
            v-if="addableDims.length"
            class="edit-chip placeholder"
            style="font-size:var(--text-micro); padding:1px 7px; border-radius:var(--radius-sm); border:1px dashed var(--color-border-form); color:var(--color-text-secondary); background:transparent; cursor:pointer;"
          >
            + tag
          </span>
        </div>
        <div class="edit-actions" style="display:flex; gap:8px; margin-top:4px; align-items:center;">
          <button class="edit-save" style="font-size:var(--text-micro); padding:2px 10px; border-radius:var(--radius-form); font-weight:var(--weight-semibold); cursor:pointer; border:none; background:var(--color-brand-solid); color:#fff;" @click="saveEdit">Save</button>
          <button class="edit-cancel" style="font-size:var(--text-micro); padding:2px 10px; border-radius:var(--radius-form); font-weight:var(--weight-semibold); cursor:pointer; border:none; background:transparent; color:var(--color-text-secondary);" @click="cancelEdit">Cancel</button>
          <button class="edit-delete" style="font-size:var(--text-micro); padding:2px 10px; border-radius:var(--radius-form); font-weight:var(--weight-semibold); cursor:pointer; border:none; background:transparent; color:var(--color-text-disabled); margin-left:auto;" @click="$emit('delete', entry.id)">Delete</button>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.entry-row {
  padding: 9px 14px; border-radius: 8px; border: 1px solid transparent;
  display: flex; justify-content: space-between; align-items: flex-start;
  transition: all 0.15s; cursor: default;
}
.entry-row:hover { background: var(--color-surface-muted); border-color: var(--color-divider); }
.entry-row:hover .edit-trigger { opacity: 1 !important; }
.edit-trigger:hover { color: var(--color-brand-solid) !important; }
.edit-row { display: flex; flex-direction: column; gap: 4px; width: 100%; }

.entry-chip.cat { background: var(--color-chip-cat-bg); color: var(--color-chip-cat-text); }
.entry-chip.biz { background: var(--color-chip-biz-bg); color: var(--color-chip-biz-text); }
.entry-chip.imp { background: var(--color-chip-imp-bg); color: var(--color-chip-imp-text); }
.entry-chip.goal { background: var(--color-chip-goal-bg); color: var(--color-chip-goal-text); }

.edit-chip.goal { background: var(--color-token-goal-bg); color: var(--color-token-goal-text); }
.edit-chip.cat { background: var(--color-token-cat-bg); color: var(--color-token-cat-text); }
.edit-chip.biz { background: var(--color-token-biz-bg); color: var(--color-token-biz-text); }
.edit-chip.imp { background: var(--color-token-imp-bg); color: var(--color-token-imp-text); }
.edit-chip.placeholder { border: 1px dashed var(--color-border-form); color: var(--color-text-secondary); background: transparent; cursor: pointer; }
.edit-chip.placeholder:hover { border-color: var(--color-text-secondary); }
</style>
```

- [ ] **Step 5: Run test to verify it passes**

```bash
npx vitest run src/__tests__/components/composite/EntryRow.test.ts
```

Expected: all tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/composite/EntryRow.vue src/__tests__/components/composite/EntryRow.test.ts
git commit -m "refactor: rewrite EntryRow with hover edit trigger, muted chips, inline edit mode"
```

---

### Task 2.2: Rewrite EntryList

**Files:**
- Modify: `src/components/EntryList.vue` (update to match new EntryRow API)
- Modify: `src/__tests__/components/EntryList.test.ts` (minimal changes, EntryRow emits same event names)

- [ ] **Step 1: Update EntryList to match new design**

The current EntryList just wraps EntryRow with a total row. The API (props/emits) stays the same. Only need to update the visual wrapper:

```vue
<!-- src/components/EntryList.vue (key changes only) -->
<!-- Replace the outer div with: -->
<div style="background:var(--color-surface); border-radius:var(--radius-card); box-shadow:var(--shadow-card);">
  <div v-if="entries.length === 0" style="padding:32px; text-align:center; color:var(--color-text-secondary); font-size:var(--text-sm);">
    No entries yet. Log your first work item above.
  </div>
  <div v-else style="padding:0 16px;">
    <EntryRow
      v-for="(entry, index) in entries"
      :key="entry.id"
      :entry="entry"
      :index="index"
      @update="(id, item, dur) => $emit('update', id, item, dur)"
      @delete="(id) => $emit('delete', id)"
      @update-dimensions="(id, dims) => $emit('updateDimensions', id, dims)"
    />
    <!-- Inline total row -->
    <div style="display:flex; justify-content:space-between; font-size:var(--text-sm); color:var(--color-text-secondary); padding:12px 0; border-top:2px solid var(--color-divider); margin-top:2px;">
      <span>{{ entries.length }} {{ entries.length === 1 ? 'entry' : 'entries' }}</span>
      <span style="font-weight:var(--weight-bold); font-size:15px; color:var(--color-brand-link); font-family:var(--font-mono);">{{ formatDuration(totalMinutes) }}</span>
    </div>
  </div>
</div>
```

- [ ] **Step 2: Run existing EntryList tests**

```bash
npx vitest run src/__tests__/components/EntryList.test.ts
```

If any tests fail, fix to match new DOM structure (class names changed minimally).

- [ ] **Step 3: Commit**

```bash
git add src/components/EntryList.vue
git commit -m "refactor: update EntryList visual to match new tokens"
```

---

### Task 2.3: Rewrite CommitmentsPanel with goal expand/collapse

**Files:**
- Modify: `src/components/CommitmentsPanel.vue` (full rewrite)
- Modify: `src/__tests__/components/CommitmentsPanel.test.ts` (update for new behavior)

- [ ] **Step 1: Update test**

```typescript
// Key new tests to add to CommitmentsPanel.test.ts
it("expands role to show goals on click", async () => {
  const wrapper = mount(CommitmentsPanel, {
    props: { progress: [makeCommitmentProgress({ goals: [{ name: "Goal A", spent_minutes: 60 }] })], commitments: [makeCommitment()], rootPath: "", selectedYear: 2026, selectedMonth: 6 },
  });
  const roleRow = wrapper.find('[data-test="role-row"]');
  await roleRow.trigger("click");
  expect(wrapper.find('[data-test="goal-row"]').exists()).toBe(true);
});

it("shows ▾ for expanded role, ▸ for collapsed", () => {
  // Verify the expand/collapse indicator
});
```

- [ ] **Step 2: Rewrite CommitmentsPanel**

Key changes:
- Click role row to expand/collapse goal list
- Use muted token colors for text
- Progress bar uses brand gradient only (remove the orange/yellow/green logic)
- `/40h` etc. in mono font

Refer to `docs/superpowers/specs/2026-06-19-ux-redesign-design.md` section 4.3 for exact values.

- [ ] **Step 3: Run tests**

```bash
npx vitest run src/__tests__/components/CommitmentsPanel.test.ts
```

- [ ] **Step 4: Commit**

```bash
git add src/components/CommitmentsPanel.vue src/__tests__/components/CommitmentsPanel.test.ts
git commit -m "refactor: rewrite CommitmentsPanel with goal expand/collapse and new visual spec"
```

---

## Phase 3: Views and Integration

### Task 3.1: Rewrite MonthView with 3-zone layout

**Files:**
- Modify: `src/components/MonthView.vue` (complete rewrite)
- Modify: `src/__tests__/components/MonthView.test.ts` (update for new layout)

- [ ] **Step 1: Rewrite MonthView**

The new MonthView assembles three zones:
- Zone 1 (sidebar, 220px): HeatmapCalendar + CommitmentsPanel
- Zone 2 (main): DayHeader + EntryList + DayNote
- Zone 3 (bottom): TwoLineInput (only on today) + input hints + file path

Key data flow:
- `loadMonth(year, month)` — same as current, builds `monthEntries` map and computes `dailyMinutes` for heatmap
- `handleSelectDay(dateStr)` — updates `store.currentDate`, refreshes entry list
- `handleNavigate({ year, month })` — calls `loadMonth`

```vue
<!-- src/components/MonthView.vue (skeleton) -->
<script setup lang="ts">
import { computed, inject, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import HeatmapCalendar from "./HeatmapCalendar.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import TwoLineInput from "./TwoLineInput.vue";
import EntryList from "./EntryList.vue";
import type { DayFile, Entry, CommitmentProgress } from "../types";
import { logError } from "../utils/errorLog";
import { datesInMonth, yearMonthFromDate } from "../utils/dates";
import { formatDuration } from "../utils/format";

const store = useStore();
const selectedYear = computed(() => yearMonthFromDate(store.currentDate).year);
const selectedMonth = computed(() => yearMonthFromDate(store.currentDate).month);
const monthDates = computed(() => datesInMonth(store.currentDate));

const isSelectedToday = computed(() => {
  const now = new Date();
  const today = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  return store.currentDate === today;
});

// Daily minutes for heatmap
const dailyMinutes = computed(() => {
  const map: Record<string, number> = {};
  for (const [date, entries] of Object.entries(store.monthEntries)) {
    map[date] = entries.reduce((sum, e) => sum + e.duration, 0);
  }
  return map;
});

const monthTotalMinutes = computed(() =>
  Object.values(dailyMinutes.value).reduce((sum, m) => sum + m, 0)
);

// ... loadMonth, handleSelectDay, handleNavigate, loadCommitmentProgress, saveNote, etc.
// (same logic as current MonthView, unchanged)
</script>

<template>
  <div style="display:flex; gap:16px; padding:24px; max-width:5xl; margin:0 auto; align-items:flex-start;">
    <!-- Sidebar -->
    <div style="width:220px; flex-shrink:0; display:flex; flex-direction:column; gap:12px; position:sticky; top:24px;">
      <HeatmapCalendar
        :year="selectedYear"
        :month="selectedMonth"
        :dates="monthDates"
        :daily-minutes="dailyMinutes"
        :selected-date="store.currentDate"
        :current-date="new Date().toISOString().slice(0,10)"
        :month-total-minutes="monthTotalMinutes"
        :available-months="store.availableMonths"
        @select-date="handleSelectDay"
        @navigate="handleNavigate"
        @request-months="handleRequestMonths"
      />
      <CommitmentsPanel
        :progress="store.commitmentProgress"
        :commitments="store.commitments"
        :root-path="store.rootPath"
        :selected-year="selectedYear"
        :selected-month="selectedMonth"
        @saved="loadCommitmentProgress(selectedYear, selectedMonth)"
      />
    </div>

    <!-- Main -->
    <div style="flex:1; min-width:0; display:flex; flex-direction:column; gap:12px;">
      <!-- Day header -->
      <div style="display:flex; justify-content:space-between; align-items:baseline; padding-bottom:14px; border-bottom:1px solid var(--color-divider);">
        <div>
          <span style="font-size:var(--text-xl); font-weight:var(--weight-bold); color:var(--color-text-primary); letter-spacing:-0.3px;">
            {{ new Date(store.currentDate).toLocaleDateString('en-US', { weekday: 'long', month: 'long', day: 'numeric' }) }}
          </span>
          <span v-if="isSelectedToday" style="margin-left:6px; font-size:var(--text-micro); padding:2px 8px; border-radius:var(--radius-md); background:var(--color-brand-soft-bg); color:var(--color-brand-link); font-weight:var(--weight-semibold);">Today</span>
        </div>
        <span style="font-size:var(--text-xs); color:var(--color-text-secondary);">
          <span style="font-family:var(--font-mono);">{{ store.today?.entries.length || 0 }}</span> entries ·
          <span style="font-family:var(--font-mono);">{{ formatDuration(store.today?.entries.reduce((s, e) => s + e.duration, 0) || 0) }}</span>
        </span>
      </div>

      <!-- Day note (contenteditable) -->
      <div ref="noteRef" class="day-note" contenteditable="true" @blur="saveNote" @paste="onNotePaste" @input="onNoteInput"></div>

      <!-- Entry list -->
      <EntryList
        :entries="store.today?.entries || []"
        @update="(id, item, dur) => handleUpdateEntry(id, item, dur)"
        @delete="(id) => handleDeleteEntry(id)"
        @update-dimensions="(id, dims) => handleUpdateDimensions(id, dims)"
      />

      <!-- Input (today only) -->
      <TwoLineInput
        v-if="isSelectedToday"
        ref="entryInputRef"
        :dimensions="store.config?.dimensions || []"
        :commitments="store.commitments"
        :initial-values="store.lastDimensions"
        @submit="handleAppend"
      />

      <!-- File path -->
      <div v-if="store.rootPath" style="text-align:right; margin-top:8px;">
        <span style="font-size:var(--text-micro); color:var(--color-text-disabled); cursor:pointer;" @click="openInEditor">
          …/{{ store.currentDate.slice(0,4) }}/{{ store.currentDate.slice(5,7) }}/{{ store.currentDate }}.md
        </span>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Update MonthView test**

Update `MonthView.test.ts` to test new layout: sidebar exists, EntryList renders, TwoLineInput visible only when isSelectedToday is true, file path visible.

- [ ] **Step 3: Run tests**

```bash
npx vitest run src/__tests__/components/MonthView.test.ts
```

- [ ] **Step 4: Commit**

```bash
git add src/components/MonthView.vue src/__tests__/components/MonthView.test.ts
git commit -m "refactor: rewrite MonthView with 3-zone layout"
```

---

### Task 3.2: Rewrite App.vue

**Files:**
- Modify: `src/App.vue` (new keyboard shortcuts, simplified screen routing)
- Modify: `src/__tests__/components/App.test.ts` (minimal — MonthView is now default ready screen)

- [ ] **Step 1: Update App.vue**

Key changes:
- Add `⌘[` / `⌘]` global keyboard listeners for month navigation (delegate to MonthView)
- Remove old component imports (MonthNavigator is now inside HeatmapCalendar)
- Keep SetupScreen, ConfigErrorBanner, Toast unchanged

```typescript
// Add to App.vue <script setup>:
function onGlobalKeydown(e: KeyboardEvent) {
  if (e.metaKey && e.key === '[') {
    e.preventDefault();
    // Emit via provide or store to MonthView
    navigateMonthRef.value?.shiftMonth(-1);
  }
  if (e.metaKey && e.key === ']') {
    e.preventDefault();
    navigateMonthRef.value?.shiftMonth(1);
  }
}

// In onMounted:
document.addEventListener('keydown', onGlobalKeydown);
// In onUnmounted:
document.removeEventListener('keydown', onGlobalKeydown);
```

- [ ] **Step 2: Update App.test.ts**

```bash
npx vitest run src/__tests__/components/App.test.ts
```

Verify screen routing still works (loading → setup → error → ready → MonthView).

- [ ] **Step 3: Commit**

```bash
git add src/App.vue src/__tests__/components/App.test.ts
git commit -m "refactor: rewrite App.vue with global keyboard shortcuts and new MonthView"
```

---

## Phase 4: Cleanup

### Task 4.1: Delete obsolete components and tests

- [ ] **Step 1: Delete old component files**

```bash
rm src/components/DayStrip.vue
rm src/components/MonthNavigator.vue
rm src/components/QuickEntry.vue
rm src/components/DimensionPanel.vue
rm src/components/EntryInput.vue
rm src/components/composite/MentionMenu.vue
rm src/components/composite/CommitmentsEditor.vue
rm src/components/base/AppInput.vue
rm src/components/base/AppChip.vue
rm src/components/base/AppSelect.vue
rm src/components/base/Popover.vue
```

- [ ] **Step 2: Delete old test files**

```bash
rm src/__tests__/components/DayStrip.test.ts
rm src/__tests__/components/MonthNavigator.test.ts
rm src/__tests__/components/QuickEntry.test.ts
rm src/__tests__/components/EntryInput.test.ts
rm src/__tests__/components/composite/MentionMenu.test.ts
rm src/__tests__/components/composite/CommitmentsEditor.test.ts
rm src/__tests__/components/base/AppInput.test.ts
rm src/__tests__/components/base/AppChip.test.ts
rm src/__tests__/components/base/AppSelect.test.ts
rm src/__tests__/components/base/Popover.test.ts
```

- [ ] **Step 3: Run full test suite to verify nothing broke**

```bash
pnpm test
```

Expected: all remaining tests pass (no import errors from deleted files).

- [ ] **Step 4: Run vue-tsc to verify no type errors from deleted imports**

```bash
npx vue-tsc --noEmit
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: delete obsolete components replaced by UX redesign"
```

---

### Task 4.2: Create CSS variable compliance lint script

**Files:**
- Create: `scripts/lint-tokens.sh`

- [ ] **Step 1: Write the lint script**

```bash
#!/bin/bash
# scripts/lint-tokens.sh
# Checks that Vue components use CSS variables instead of hardcoded values.
# Exit 0 = clean, Exit 1 = violations found.

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VIOLATIONS=0

echo "=== CSS Token Compliance Check ==="

# Check for hardcoded colors (hex values in Vue component styles)
echo ""
echo "Checking for hardcoded hex colors in Vue components..."
for f in $(find "$ROOT/src/components" -name "*.vue"); do
  # Extract style blocks and check for hex colors that aren't in var()
  # Exclude: comments, SVG data URIs, and genuine var() usage
  HITS=$(grep -nP '#[0-9a-fA-F]{6}|#[0-9a-fA-F]{3}' "$f" \
    | grep -v 'var(' \
    | grep -v 'svg' \
    | grep -v 'data:' \
    | grep -v 'url(' \
    | grep -v '//' \
    || true)
  if [ -n "$HITS" ]; then
    echo "  $f:"
    echo "$HITS" | while read line; do
      echo "    $line"
    done
    VIOLATIONS=$((VIOLATIONS + 1))
  fi
done

# Check for font sizes outside the token scale
echo ""
echo "Checking for font sizes outside token scale (9, 10, 11, 12, 13, 14, 18, 20)..."
for f in $(find "$ROOT/src/components" -name "*.vue"); do
  # Find font-size declarations not using var() and not in the allowed list
  HITS=$(grep -noP 'font-size:\s*(?!var\()(?!\s*9px|\s*10px|\s*11px|\s*12px|\s*13px|\s*14px|\s*18px|\s*20px)\d+px' "$f" || true)
  if [ -n "$HITS" ]; then
    echo "  $f:"
    echo "$HITS" | while read line; do
      echo "    $line"
    done
    VIOLATIONS=$((VIOLATIONS + 1))
  fi
done

if [ $VIOLATIONS -gt 0 ]; then
  echo ""
  echo "❌ Found $VIOLATIONS file(s) with token violations."
  exit 1
else
  echo ""
  echo "✅ All components comply with token definitions."
  exit 0
fi
```

- [ ] **Step 2: Make executable and test**

```bash
chmod +x scripts/lint-tokens.sh
bash scripts/lint-tokens.sh
```

Expected: clean (may have some false positives that need filtering — adjust grep patterns).

- [ ] **Step 3: Commit**

```bash
git add scripts/lint-tokens.sh
git commit -m "feat: add CSS token compliance lint script"
```

---

## Phase 5: Final Verification

### Task 5.1: Full test suite

- [ ] **Step 1: Run frontend tests**

```bash
pnpm test
```

Target: 209+ tests pass (current baseline was 209).

- [ ] **Step 2: Run Rust tests**

```bash
cargo test
```

Expected: 60/60 pass (unchanged backend).

- [ ] **Step 3: Run type check**

```bash
npx vue-tsc --noEmit
```

Expected: clean.

### Task 5.2: Manual smoke test

- [ ] **Step 1: Launch the app**

```bash
pnpm tauri dev
```

- [ ] **Step 2: Verify key flows**
  - Open app → see today's date, heatmap with current month
  - Input: type "Test entry 1h" → see duration token appear in line 2 → Enter → entry appears in list with 1.5s highlight
  - Input: type "@" → DimensionPopover appears → select category → select value → token appears in line 2
  - Edit: hover entry row → see "⋯" → click → enter edit mode → change item → Save
  - Month nav: click ← → arrows → heatmap updates
  - Quick jump: click "June 2026 ▾" → popover appears → change year/month
  - Commitments: click role → goals expand/collapse

- [ ] **Step 3: Verify no console errors**

Check Tauri dev console for any errors.

---

## Phase 6: Demo Comparison

### Task 6.1: Structured comparison against UX-REDESIGN-DEMO.html

**Prerequisite:** App must be running (`pnpm tauri dev`) and demo open in browser (`open UX-REDESIGN-DEMO.html`).

Compare the running app against the demo, element by element. Document every deviation, no matter how small.

- [ ] **Step 1: Layout structure**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Sidebar width | 220px | | |
| Main area | flex-1, fills remaining space | | |
| Sidebar background | `var(--color-surface-muted)` = #fafbfc | | |
| Main area padding | 24px top/bottom, 28px left/right | | |
| Gap between sidebar and main | 16px | | |

- [ ] **Step 2: Heatmap calendar**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Navigation arrows | 12px, color `var(--color-text-secondary)` | | |
| Month label | 14px, weight 700, color `var(--color-text-primary)` | | |
| Year in label | 11px, weight 400, color `var(--color-text-secondary)` | | |
| Weekday headers | 9px, color `var(--color-text-secondary)`, mono | | |
| Cell size | 24×24px, radius 4px | | |
| Cell text | 10px, mono, color by heat level | | |
| Today ring | `box-shadow: 0 0 0 2px #6366f1` | | |
| Selected ring | `box-shadow: 0 0 0 2px #94a3b8` | | |
| Month total | 11px, weight 600, mono for number | | |
| Hover scale | `transform: scale(1.15)` on hover | | |
| Future dates | opacity 0.3-0.5, cursor default, no click | | |

- [ ] **Step 3: Commitments panel**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Section label | 10px, weight 700, uppercase, letter-spacing 0.5px | | |
| Role name | 12px, weight 600 | | |
| Spent/allocation | 11px, weight 600 for spent, 400 for /Xh, mono | | |
| Progress bar | 4px height, brand gradient fill | | |
| Goal name | 11px, color `var(--color-text-secondary)`, max-width 130px, ellipsis | | |
| Goal spent | 11px, weight 500, mono | | |
| Expand/collapse | ▾ expanded, ▸ collapsed | | |
| Role hover | bg change on hover | | |

- [ ] **Step 4: Day header**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Day title | 20px, weight 700, letter-spacing -0.3px | | |
| Today badge | 10px, weight 600, bg `var(--color-brand-soft-bg)`, color `var(--color-brand-link)` | | |
| Day summary | 12px, color `var(--color-text-secondary)`, mono for numbers | | |
| Divider line | `border-bottom: 1px solid var(--color-divider)`, 14px padding-bottom | | |

- [ ] **Step 5: Entry list and rows**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Row padding | 9px top/bottom, 14px left/right | | |
| Row border-radius | 8px | | |
| Row hover bg | `var(--color-surface-muted)` (#fafbfc) | | |
| Item text | 14px, weight 500, color `var(--color-text-primary)`, max 2 lines | | |
| Duration | 13px, color `var(--color-text-primary)`, mono, tabular-nums | | |
| Chip size | 10px, max-width 100px, ellipsis | | |
| Chip colors (muted) | cat: bg #f5f6fa text #5b63a6; biz: bg #f7f5fa text #7b5ea7; etc. | | |
| Edit trigger | 14px, opacity 0, opacity 1 on row hover, color `var(--color-brand-solid)` on icon hover | | |
| Just-added animation | bg `var(--anim-highlight-bg)`, border `var(--anim-highlight-border)`, 1.5s fade | | |
| Empty state | "No entries yet. Log your first work item above." centered | | |
| Total row | 2px border-top, entry count + total duration in brand color, mono | | |

- [ ] **Step 6: Edit mode**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Edit row border | `var(--color-brand-solid)`, `box-shadow: var(--shadow-focus-ring)` | | |
| Item input | 14px, weight 500, border none, transparent bg | | |
| Duration input | 13px, mono, width 56px, text-align right, border-radius 5px | | |
| Duration unit | "min", 11px, color `var(--color-text-secondary)` | | |
| Edit chips | input-token colors (saturated, not muted) | | |
| +tag chip | dashed border, `var(--color-text-secondary)` | | |
| Save button | 10px, weight 600, bg `var(--color-brand-solid)`, white text | | |
| Cancel button | 10px, weight 600, transparent bg, `var(--color-text-secondary)` | | |
| Delete button | 10px, weight 600, transparent bg, `var(--color-text-disabled)`, hover → `var(--color-danger)` | | |

- [ ] **Step 7: Two-line input**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Input card | 2px border, `var(--color-border-form)`, radius 12px, padding 10px 16px | | |
| Input card focused | border `var(--color-brand-solid)`, `box-shadow: var(--shadow-focus-ring)` | | |
| "+" prefix | 18px, color `var(--color-brand-solid)` | | |
| Item input | 14px, transparent bg, caret `var(--color-brand-solid)` | | |
| Placeholder | color `var(--color-placeholder)` | | |
| Enter badge | 9px, weight 600, border, opacity 0.5 default → 1 focused, mono | | |
| Input tokens | 10px, weight 500, saturated colors | | |
| Duration token | mono, bg #fff7ed, color #c2410c | | |
| Token remove × | 12px, opacity 0.4, hover → 1 | | |
| Missing indicator | 10px, weight 450, dashed border, dot + dim name | | |
| Hints row | 10px, color `var(--color-text-disabled)`, mono for kbd | | |

- [ ] **Step 8: Dimension popover**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Popover container | bg white, 1px border, radius 12px, `box-shadow: var(--shadow-popover)`, width 240px | | |
| Dim header | 10px, weight 700, uppercase, bg #fafaff, color #6366f1, "DIM" badge | | |
| Dim items | 13px, 3px color bar on left, required/optional meta on right | | |
| Dim item selected | bg #fafaff, color `var(--color-brand-solid)`, weight 600 | | |
| Val header | 10px, weight 700, bg #fafaf9, color #78716c, ← back button | | |
| Val items | 13px, no color bars | | |
| Footer | 9px, color `var(--color-text-disabled)`, mono for kbd hints | | |
| Dim bar colors | goal: #86efac, cat: #a5b4fc, biz: #c4b5fd, imp: #5eead4 | | |
| Required meta | 10px, weight 500, color `var(--color-warning)` | | |
| Optional meta | 10px, color `var(--color-text-disabled)` | | |

- [ ] **Step 9: Day note**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Note text | 12px, color `var(--color-text-secondary)`, italic | | |
| Hover | bg changes to `var(--color-page-bg)`, color → `var(--color-text-secondary)` | | |

- [ ] **Step 10: File path**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Path text | 10px, color `var(--color-text-disabled)`, right-aligned | | |
| Hover | cursor pointer | | |

- [ ] **Step 11: Quick jump popover**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Container | bg white, 1px border, radius 10px, `box-shadow: var(--shadow-quickjump)` | | |
| Selects | 12px, border `var(--color-border-form)`, radius 6px | | |
| "Go" hint | 9px, color `var(--color-text-secondary)` | | |
| Visibility | hidden by default, shown on month label click | | |

- [ ] **Step 12: Global typography audit**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| All durations use `var(--font-mono)` | Every duration value in entry list, input tokens, commitments, heatmap | | |
| All keyboard hints use `var(--font-mono)` | All kbd elements in hints, popover footers | | |
| No hardcoded colors | Run `bash scripts/lint-tokens.sh` — zero violations | | |
| Body font | `var(--font-body)` (system-ui) for all non-data text | | |

- [ ] **Step 13: Interaction behavior**

| Check | Demo expectation | App actual | Match? |
|-------|-----------------|------------|--------|
| Open app → today focused | Input gets focus on load / window focus | | |
| @ triggers popover | Popover appears below input, not covering it | | |
| Esc in val phase → back to dim | Popover returns to dimension list | | |
| Esc in dim phase → close | Popover closes | | |
| Enter submits | Entry appears in list with 1.5s highlight | | |
| Hover row → ⋯ appears | Edit trigger visible on hover, hidden otherwise | | |
| Click ⋯ → edit mode | Row transforms to edit mode with inputs | | |
| Double-click row → edit mode | Same as clicking ⋯ | | |
| ⌘[ / ⌘] navigates months | Heatmap and entries update | | |
| Click heatmap cell → select day | Entry list loads that day's entries | | |
| Click role in commitments → toggle goals | Goals expand/collapse on click | | |

### Task 6.2: Fix all deviations

For every "No" in the tables above:

- [ ] **Step 1: Document each deviation** with a screenshot description (what the app shows vs what the demo shows)
- [ ] **Step 2: Fix each deviation** — adjust CSS, component markup, or token references
- [ ] **Step 3: Re-run comparison** — verify the fix resolved the deviation
- [ ] **Step 4: Commit fixes**

```bash
git add -A
git commit -m "fix: resolve visual deviations found in demo comparison"
```

---

## Verification

After all phases complete:

```bash
pnpm test              # all frontend tests pass
cargo test             # 60/60 Rust tests pass
npx vue-tsc --noEmit  # clean
bash scripts/lint-tokens.sh  # clean
# Phase 6 demo comparison: all 13 check tables pass
```

## Notes

- **Backend is frozen.** All 14 Rust commands remain unchanged. The data model (Entry, Commitment, Config, DayFile, types.ts) is untouched.
- **useStore.ts** interface stays the same (`screen`, `rootPath`, `config`, `today`, `commitments`, `commitmentProgress`, `lastDimensions`, `currentDate`, `monthEntries`, `availableMonths`). No new reactive state needed — `dailyMinutes` is a computed in MonthView.
- **Existing AppButton, ProgressBar, Toast** are retained without changes.
- **Tailwind CSS v4** remains for utility classes (flex, gap, padding, etc.). Visual values come from tokens.css CSS variables via `var()`.
- **Test patterns** follow the existing codebase: `createTestStore()` + `makeEntry()` + `makeConfig()` from test mocks, `setupTauriMocks()` for Tauri command mocking.
- **Design spec section 4** (component spec tables) is the authoritative reference for exact font-size, font-weight, color token per element.
