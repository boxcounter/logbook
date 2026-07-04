# Hide Default Template Indicator for Future Months Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hide "Using default template (no custom dimensions this month)" indicator when viewing a future month.

**Architecture:** Add `isFutureMonth` computed to MonthView.vue, append `&& !isFutureMonth` to the existing v-if. Update the existing test to cover future-month hiding.

**Tech Stack:** Vue 3 + TypeScript + Vitest + jsdom

---

### File Map

| File | Responsibility | Action |
|------|---------------|--------|
| `src/components/MonthView.vue` | Month view with default template indicator | Modify: add computed, update v-if |
| `src/__tests__/components/MonthView.test.ts` | Unit tests | Modify: extend existing test |

---

### Task 1: Hide default template indicator for future months

**Files:**
- Modify: `src/components/MonthView.vue:37-39,186`
- Test: `src/__tests__/components/MonthView.test.ts:104-111`

- [ ] **Step 1: Write the failing test**

Update the existing test at line 104-111 to also cover the future-month case:

```ts
  it("shows the default-template indicator only when usingDefaultDimensions is true and month is not in the future", () => {
    const off = mountView();
    expect(off.text()).not.toContain("Using default template");

    const store = makeStore();
    store.usingDefaultDimensions = true;
    const on = mountView(store);
    expect(on.text()).toContain("Using default template");

    // Future month: should NOT show even when usingDefaultDimensions is true
    const futureStore = makeStore();
    futureStore.usingDefaultDimensions = true;
    // Set to next year, January — definitely future
    futureStore.currentDate = `${new Date().getFullYear() + 1}-01-01`;
    const future = mountView(futureStore);
    expect(future.text()).not.toContain("Using default template");
  });
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test -- src/__tests__/components/MonthView.test.ts -t "shows the default-template indicator"
```

Expected: FAIL — still shows "Using default template" for the future month.

- [ ] **Step 3: Add `isFutureMonth` computed**

In `src/components/MonthView.vue`, add after the `isSelectedToday` computed (after line 39):

```ts
const isFutureMonth = computed(() => {
  const now = new Date();
  const currentYear = now.getFullYear();
  const currentMonth = now.getMonth() + 1;
  return selectedYear.value > currentYear ||
    (selectedYear.value === currentYear && selectedMonth.value > currentMonth);
});
```

- [ ] **Step 4: Update the v-if condition**

In `src/components/MonthView.vue`, change line 186 from:

```html
      <p v-if="store.usingDefaultDimensions" class="mb-sm text-micro text-[var(--color-text-disabled)]">
```

To:

```html
      <p v-if="store.usingDefaultDimensions && !isFutureMonth" class="mb-sm text-micro text-[var(--color-text-disabled)]">
```

- [ ] **Step 5: Run tests to verify pass**

```bash
pnpm test -- src/__tests__/components/MonthView.test.ts
```

Expected: All tests pass, including the updated one.

- [ ] **Step 6: Run full test suite**

```bash
pnpm test
```

Expected: All tests pass.

- [ ] **Step 7: Run typecheck**

```bash
npx vue-tsc --noEmit
```

Expected: No errors.

- [ ] **Step 8: Commit**

```bash
git add src/components/MonthView.vue src/__tests__/components/MonthView.test.ts
git commit -m "feat: hide default template indicator for future months"
```
