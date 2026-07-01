# Dimension Auto-Coloring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace hardcoded 4-key dimension colors with algorithmic hue-wheel spreading — any set of custom dimensions gets automatically assigned distinguishable colors, zero configuration.

**Architecture:** A single utility (`src/utils/dimensionColor.ts`) takes the full dimensions array, sorts active (non-deleted) dimensions by key, spreads them evenly across a 360° hue wheel, and produces `hsl()` color strings for each of 5 roles: bar, chip display bg/text, chip edit bg/text. Deleted dimensions get a fixed neutral gray. All 4 consuming components switch from `:class` (Tailwind) to `:style` (inline) because Tailwind cannot scan dynamically-computed class strings.

**Tech Stack:** TypeScript, Vue 3, vitest

---

### Task 1: Rewrite `dimensionColor.ts` utility

**Files:**
- Modify: `src/utils/dimensionColor.ts`

- [ ] **Step 1: Replace entire file content**

```typescript
// Dimension auto-coloring: spreads active dimensions evenly across the hue
// wheel, sorted by key so drag-reorder doesn't change colors. Deleted
// dimensions get a fixed neutral gray. All output is hsl() strings for use
// in inline :style — Tailwind can't scan dynamically-computed class names.

import type { Dimension } from "../types";

const BASE = 210; // starting hue — a pleasant blue for the single-dimension case

// ---- hue assignment ----------------------------------------------------

/** Map dimension key → hue (degrees, 0–360), or null for deleted dimensions.
 *  Active dimensions are sorted by key and spread evenly across the wheel. */
export function dimensionHues(
  dimensions: Dimension[],
): Map<string, number | null> {
  const active = dimensions
    .filter((d) => !d.deleted)
    .map((d) => d.key)
    .sort();
  const n = active.length;
  const map = new Map<string, number | null>();

  for (const d of dimensions) {
    if (d.deleted) {
      map.set(d.key, null);
    }
  }
  for (let i = 0; i < n; i++) {
    const hue = n === 1 ? BASE : (BASE + (i * 360) / n) % 360;
    map.set(active[i], Math.round(hue));
  }

  return map;
}

// ---- color producers (5 roles) -----------------------------------------

function hsl(h: number, s: number, l: number): string {
  return `hsl(${h} ${s}% ${l}%)`;
}

/** Bar color (3px left indicator).  hue=null → gray. */
export function dimBar(hue: number | null): string {
  return hue === null ? hsl(0, 0, 75) : hsl(hue, 58, 70);
}

/** Display-chip style (EntryRow, passive state). */
export function dimChipStyle(hue: number | null): {
  background: string;
  color: string;
} {
  return hue === null
    ? { background: hsl(0, 0, 96), color: hsl(0, 0, 45) }
    : { background: hsl(hue, 42, 96), color: hsl(hue, 40, 42) };
}

/** Token-chip style (EntryRowEdit, active editing state). */
export function dimTokenChipStyle(hue: number | null): {
  background: string;
  color: string;
} {
  return hue === null
    ? { background: hsl(0, 0, 95), color: hsl(0, 0, 40) }
    : { background: hsl(hue, 66, 95), color: hsl(hue, 60, 37) };
}
```

- [ ] **Step 2: Verify type-check**

```bash
npx vue-tsc --noEmit
```

Expected: clean (the old `dimBarVar`/`dimBarColor`/`dimBarClass` exports are gone — if any component still imports them, vue-tsc catches it).

- [ ] **Step 3: Commit**

```bash
git add src/utils/dimensionColor.ts
git commit -m "refactor: rewrite dimensionColor with hue-wheel spreading"
```

---

### Task 2: Write unit tests for the new utility

**Files:**
- Modify: `src/__tests__/dimensionColor.test.ts`

- [ ] **Step 1: Replace entire test file**

```typescript
import { describe, it, expect } from "vitest";
import {
  dimensionHues,
  dimBar,
  dimChipStyle,
  dimTokenChipStyle,
} from "../utils/dimensionColor";
import type { Dimension } from "../types";

function dim(overrides: Partial<Dimension> = {}): Dimension {
  return {
    name: overrides.key ?? "Test",
    key: overrides.key ?? "test",
    source: "static",
    required: false,
    deleted: false,
    values: ["a"],
    ...overrides,
  };
}

describe("dimensionHues", () => {
  it("returns empty map for empty input", () => {
    expect(dimensionHues([]).size).toBe(0);
  });

  it("single active dimension gets BASE hue", () => {
    const hues = dimensionHues([dim({ key: "goal" })]);
    expect(hues.get("goal")).toBe(210);
  });

  it("two active dimensions are 180° apart", () => {
    const hues = dimensionHues([dim({ key: "a" }), dim({ key: "b" })]);
    expect(hues.get("a")).toBe(210);
    expect(hues.get("b")).toBe(30); // (210 + 180) % 360
  });

  it("three active dimensions are 120° apart", () => {
    const hues = dimensionHues([
      dim({ key: "alpha" }),
      dim({ key: "beta" }),
      dim({ key: "gamma" }),
    ]);
    // Sorted by key: alpha, beta, gamma
    expect(hues.get("alpha")).toBe(210);
    expect(hues.get("beta")).toBe(330); // (210 + 120) % 360
    expect(hues.get("gamma")).toBe(90); // (210 + 240) % 360
  });

  it("sorts by key, not input order", () => {
    // Input in reverse key order — output should be key-sorted
    const hues = dimensionHues([
      dim({ key: "c" }),
      dim({ key: "a" }),
      dim({ key: "b" }),
    ]);
    expect(hues.get("a")).toBe(210);
    expect(hues.get("b")).toBe(330);
    expect(hues.get("c")).toBe(90);
  });

  it("adding a dimension changes hues (re-spread)", () => {
    const two = dimensionHues([dim({ key: "a" }), dim({ key: "b" })]);
    const three = dimensionHues([
      dim({ key: "a" }),
      dim({ key: "b" }),
      dim({ key: "c" }),
    ]);
    // With 2 dims: a→210, b→30. With 3: a→210, b→330, c→90.
    // b changes from 30° to 330°.
    expect(two.get("b")).not.toBe(three.get("b"));
  });

  it("deleted dimensions get null hue", () => {
    const hues = dimensionHues([
      dim({ key: "a" }),
      dim({ key: "z", deleted: true }),
    ]);
    expect(hues.get("a")).toBe(210); // only active dim
    expect(hues.get("z")).toBeNull();
  });

  it("deleted dimensions do not affect active hue count", () => {
    // 1 active + 2 deleted → still treated as N=1 (BASE)
    const hues = dimensionHues([
      dim({ key: "active" }),
      dim({ key: "del1", deleted: true }),
      dim({ key: "del2", deleted: true }),
    ]);
    expect(hues.get("active")).toBe(210);
  });
});

describe("dimBar", () => {
  it("produces an hsl string", () => {
    const bar = dimBar(210);
    expect(bar).toMatch(/^hsl\(\d+ 58% 70%\)$/);
  });

  it("uses gray for null hue", () => {
    expect(dimBar(null)).toBe("hsl(0 0% 75%)");
  });
});

describe("dimChipStyle", () => {
  it("returns background and color for an active hue", () => {
    const style = dimChipStyle(210);
    expect(style.background).toMatch(/^hsl\(\d+ 42% 96%\)$/);
    expect(style.color).toMatch(/^hsl\(\d+ 40% 42%\)$/);
  });

  it("returns gray for null hue", () => {
    const style = dimChipStyle(null);
    expect(style.background).toBe("hsl(0 0% 96%)");
    expect(style.color).toBe("hsl(0 0% 45%)");
  });
});

describe("dimTokenChipStyle", () => {
  it("returns background and color for an active hue", () => {
    const style = dimTokenChipStyle(210);
    expect(style.background).toMatch(/^hsl\(\d+ 66% 95%\)$/);
    expect(style.color).toMatch(/^hsl\(\d+ 60% 37%\)$/);
  });

  it("returns gray for null hue", () => {
    const style = dimTokenChipStyle(null);
    expect(style.background).toBe("hsl(0 0% 95%)");
    expect(style.color).toBe("hsl(0 0% 40%)");
  });
});
```

- [ ] **Step 2: Run tests — fail (old imports broken)**

```bash
npx vitest run src/__tests__/dimensionColor.test.ts
```

Expected: some old-style tests may still import removed exports. If Task 1 is already committed, these tests will reference the new exports and pass.

- [ ] **Step 3: Run all tests**

```bash
npx vitest run
```

Expected: all pass (EntryRow / EntryRowEdit tests don't assert on color classes, so they should still pass).

- [ ] **Step 4: Commit**

```bash
git add src/__tests__/dimensionColor.test.ts
git commit -m "test: rewrite dimensionColor tests for hue-wheel spreading"
```

---

### Task 3: Update DimensionPopover.vue

**Files:**
- Modify: `src/components/DimensionPopover.vue`

- [ ] **Step 1: Replace imports and barClass**

Replace the import line:
```typescript
import { dimBarClass } from "../utils/dimensionColor";
```
with:
```typescript
import { dimensionHues, dimBar } from "../utils/dimensionColor";
```

Replace the `barClass` function (line ~55-58):
```typescript
function barClass(key: string): string {
  return dimBarClass(key);
}
```
with a computed hues map:
```typescript
const hues = computed(() => dimensionHues(props.dimensions));
function barColor(key: string): string {
  return dimBar(hues.value.get(key) ?? null);
}
```

- [ ] **Step 2: Replace the template color bar**

Find the color bar `<span>` (line ~168) that uses `:class="barClass(d.key)"`:
```html
<span class="w-[3px] h-[18px] rounded-[var(--radius-sm)] flex-shrink-0" :class="barClass(d.key)"></span>
```
Replace with:
```html
<span class="w-[3px] h-[18px] rounded-[var(--radius-sm)] flex-shrink-0" :style="{ background: barColor(d.key) }"></span>
```

- [ ] **Step 3: Verify**

```bash
npx vue-tsc --noEmit && npx vitest run
```

- [ ] **Step 4: Commit**

```bash
git add src/components/DimensionPopover.vue
git commit -m "refactor: DimensionPopover uses hue-wheel bar colors"
```

---

### Task 4: Update DimensionEditorModal.vue

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue`

- [ ] **Step 1: Replace import**

Replace:
```typescript
import { dimBarColor } from "../../utils/dimensionColor";
```
with:
```typescript
import { dimensionHues, dimBar } from "../../utils/dimensionColor";
```

- [ ] **Step 2: Add computed hues**

After the `draft` / `selectedDimension` computed section (around line 54), add:
```typescript
const draftHues = computed(() => dimensionHues(draft.value));
```

- [ ] **Step 3: Replace left-panel color bar**

Find the color bar:
```html
:style="{ background: dimBarColor(dim.key) }"
```
Replace with:
```html
:style="{ background: dimBar(draftHues.value.get(dim.key) ?? null) }"
```

- [ ] **Step 4: Verify nothing else uses old imports**

```bash
grep -n "dimBarColor\|dimBarVar\|dimBarClass" src/components/composite/DimensionEditorModal.vue
```

Expected: no matches.

- [ ] **Step 5: Verify**

```bash
npx vue-tsc --noEmit && npx vitest run
```

- [ ] **Step 6: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue
git commit -m "refactor: DimensionEditorModal uses hue-wheel bar colors"
```

---

### Task 5: Update EntryRow.vue

**Files:**
- Modify: `src/components/composite/EntryRow.vue`

- [ ] **Step 1: Add import**

At the top of the `<script setup>` block, after the existing imports, add:
```typescript
import { dimensionHues, dimChipStyle } from "../../utils/dimensionColor";
```

- [ ] **Step 2: Add computed hues**

After the `filledDims` computed, add:
```typescript
const chipHues = computed(() => dimensionHues(dimensions.value));
function chipStyle(key: string) {
  return dimChipStyle(chipHues.value.get(key) ?? null);
}
```

- [ ] **Step 3: Replace chip class with inline style**

Find the chip template (around line 82-86):
```html
<span
  v-for="d in filledDims" :key="d.key"
  class="text-micro font-[450] px-sm rounded-[var(--radius-sm)] max-w-[100px] overflow-hidden text-ellipsis whitespace-nowrap"
  :class="chipClass(d.key)"
  :title="entry.dimensions[d.key]"
>{{ entry.dimensions[d.key] }}</span>
```
Replace with:
```html
<span
  v-for="d in filledDims" :key="d.key"
  class="text-micro font-[450] px-sm rounded-[var(--radius-sm)] max-w-[100px] overflow-hidden text-ellipsis whitespace-nowrap"
  :style="chipStyle(d.key)"
  :title="entry.dimensions[d.key]"
>{{ entry.dimensions[d.key] }}</span>
```

- [ ] **Step 4: Remove old chipClass function**

Delete the entire `chipClass` function (lines 35-43).

- [ ] **Step 5: Verify**

```bash
npx vue-tsc --noEmit && npx vitest run
```

- [ ] **Step 6: Commit**

```bash
git add src/components/composite/EntryRow.vue
git commit -m "refactor: EntryRow chip colors use hue-wheel inline style"
```

---

### Task 6: Update EntryRowEdit.vue

**Files:**
- Modify: `src/components/composite/EntryRowEdit.vue`

- [ ] **Step 1: Add import**

```typescript
import { dimensionHues, dimTokenChipStyle } from "../../utils/dimensionColor";
```

- [ ] **Step 2: Add computed hues**

After the `filled` function, add:
```typescript
const editHues = computed(() => dimensionHues(props.dimensions));
function tokenChipStyle(key: string) {
  return dimTokenChipStyle(editHues.value.get(key) ?? null);
}
```

- [ ] **Step 3: Replace chip class with inline style**

Find the edit chip template (around line 172-178):
```html
<span
  v-for="d in filled()" :key="d.key"
  class="text-micro font-medium px-sm py-2xs rounded-[var(--radius-sm)] inline-flex items-center gap-xs"
  :class="chipClass(d.key)"
>
  {{ dimValues[d.key] }}
  <span ...>×</span>
</span>
```
Replace `:class="chipClass(d.key)"` with `:style="tokenChipStyle(d.key)"`.

- [ ] **Step 4: Remove old chipClass function**

Delete the entire `chipClass` function (lines 106-114).

- [ ] **Step 5: Verify**

```bash
npx vue-tsc --noEmit && npx vitest run
```

- [ ] **Step 6: Commit**

```bash
git add src/components/composite/EntryRowEdit.vue
git commit -m "refactor: EntryRowEdit token chips use hue-wheel inline style"
```

---

### Task 7: Clean up stale CSS tokens

**Files:**
- Modify: `src/assets/tokens.css`

- [ ] **Step 1: Confirm no remaining references**

```bash
grep -rn "dim-bar-\|color-chip-\|color-token-" src/ --include="*.vue" --include="*.ts" | grep -v node_modules | grep -v __tests__ | grep -v tokens.css
```

Expected: no matches (all consumers now use the new utility).

- [ ] **Step 2: Remove the old token blocks**

Delete these three blocks from `src/assets/tokens.css`:

Lines 48-56 (entry chip display tokens):
```css
--color-chip-cat-bg: #f5f6fa;
--color-chip-cat-text: #5b63a6;
--color-chip-biz-bg: #f7f5fa;
--color-chip-biz-text: #7b5ea7;
--color-chip-imp-bg: #f5faf9;
--color-chip-imp-text: #3d7a73;
--color-chip-goal-bg: #f5faf6;
--color-chip-goal-text: #4a7c59;
```

Lines 37-44 (entry chip edit tokens):
```css
--color-token-cat-bg: #eef2ff;
--color-token-cat-text: #4338ca;
--color-token-biz-bg: #f5f3ff;
--color-token-biz-text: #6d28d9;
--color-token-imp-bg: #f0fdfa;
--color-token-imp-text: #0f766e;
--color-token-goal-bg: #f0fdf4;
--color-token-goal-text: #15803d;
```

Lines 76-79 (dimension bar tokens):
```css
--dim-bar-goal: #86efac;
--dim-bar-cat: #a5b4fc;
--dim-bar-biz: #c4b5fd;
--dim-bar-imp: #5eead4;
```

- [ ] **Step 3: Verify build still produces correct CSS**

```bash
pnpm build
```

Expected: build succeeds; CSS output shrinks slightly (fewer tokens). Verify no dim-bar/chip-color/token-color strings in output:
```bash
grep -c "dim-bar\|chip-cat\|chip-biz\|token-cat" dist/assets/*.css
```
Expected: 0.

- [ ] **Step 4: Commit**

```bash
git add src/assets/tokens.css
git commit -m "chore: remove stale hardcoded dimension color tokens"
```

---

### Task 8: End-to-end verification

**Files:**
- (No new files — verification run)

- [ ] **Step 1: Full test suite**

```bash
cd src-tauri && cargo test
npx vue-tsc --noEmit
npx vitest run
pnpm build
```

Expected: all pass.

- [ ] **Step 2: Manual smoke test**

```bash
pnpm tauri dev
```

Verify:
- App loads, dimensions appear in popover with distinct colors
- ⚙ DimensionEditorModal shows color bars matching popover
- Entry chips (display + edit) use the same color family as the bar
- Add a new dimension → colors re-spread, all still distinguishable
- Delete a dimension → remaining colors re-spread
- Soft-deleted dimension → bar and chips are gray

- [ ] **Step 3: Commit any final fixes**

```bash
git add -A && git commit -m "chore: e2e verification fixes"
```
