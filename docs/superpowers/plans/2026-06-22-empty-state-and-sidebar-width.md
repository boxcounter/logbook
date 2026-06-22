# Empty State & Sidebar Width Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two UX fixes: (1) conditional empty-state message in EntryList depending on whether the day is today, (2) widen sidebar from 220px to 280px.

**Architecture:** EntryList receives a new optional `isToday` boolean prop; its empty-state `<div>` renders different text for today vs non-today. MonthView passes `isSelectedToday` as that prop and bumps its sidebar width.

**Tech Stack:** Vue 3 + TypeScript, Vitest + @vue/test-utils

---

### Task 1: EntryList — add `isToday` prop and conditional empty state

**Files:**
- Modify: `src/components/EntryList.vue:6,17-18`

- [ ] **Step 1: Add `isToday` prop to `defineProps`**

  ```ts
  // src/components/EntryList.vue line 6 — replace the defineProps line
  defineProps<{ entries: Entry[]; justAddedId?: string | null; isToday?: boolean }>();
  ```

- [ ] **Step 2: Make the empty-state message conditional**

  ```html
  <!-- src/components/EntryList.vue line 17 — replace the static message -->
      <div v-if="entries.length === 0" class="p-2xl text-center text-[var(--color-text-secondary)] text-secondary">
        {{ isToday ? "No entries yet. Log your first work item below." : "No entries for this day." }}
      </div>
  ```

- [ ] **Step 3: Update EntryList test — verify non-today message**

  ```ts
  // src/__tests__/components/EntryList.test.ts — add new test after the existing empty-state test
  it("shows a different empty state for non-today days", () => {
    const wrapper = mountList([]);
    // mountList doesn't pass isToday, so it defaults to undefined/false → non-today message
    expect(wrapper.text()).toContain("No entries for this day.");
    expect(wrapper.text()).not.toContain("Log your first work item below");
  });

  it("shows the full CTA message when isToday is true", () => {
    const wrapper = mount(EntryList, {
      props: { entries: [], isToday: true },
      global: {
        provide: { [STORE_KEY as symbol]: reactive({ dimensions: makeDimensions(), fromTemplate: false, commitments: [makeCommitment()] }) },
      },
    });
    expect(wrapper.text()).toContain("No entries yet. Log your first work item below.");
  });
  ```

- [ ] **Step 4: Run EntryList tests**

  Run: `npx vitest run src/__tests__/components/EntryList.test.ts`
  Expected: 6 tests PASS (4 existing + 2 new)

- [ ] **Step 5: Commit**

  ```bash
  git add src/components/EntryList.vue src/__tests__/components/EntryList.test.ts
  git commit -m "feat(EntryList): conditional empty state — CTA only for today, plain message for other days"
  ```

---

### Task 2: MonthView — pass `isToday` to EntryList

**Files:**
- Modify: `src/components/MonthView.vue:385-391`

- [ ] **Step 1: Add `:is-today` binding to EntryList invocation**

  ```html
  <!-- src/components/MonthView.vue line 385-391 — add :is-today prop -->
        <EntryList
          :entries="dayEntries"
          :just-added-id="justAddedId"
          :is-today="isSelectedToday"
          @update="handleUpdateEntry"
          @delete="handleDeleteEntry"
          @update-dimensions="handleUpdateDimensions"
        />
  ```

- [ ] **Step 2: Verify MonthView test — EntryList receives the prop**

  The existing test "only renders EntryComposer when the selected day is today" (line 79-84) sets `currentDate` to a past date. After this change, the EntryList inside that wrapper also receives `:is-today="false"`. The test already passes because it doesn't assert on EntryList props — but let's add a quick check:

  ```ts
  // src/__tests__/components/MonthView.test.ts — add after "only renders EntryComposer when the selected day is today" test
  it("passes is-today=false to EntryList for a non-today date", () => {
    const store = makeStore();
    store.currentDate = "2026-06-10";
    const wrapper = mountView(store);
    const entryList = wrapper.findComponent({ name: "EntryList" });
    expect(entryList.props("isToday")).toBe(false);
  });

  it("passes is-today=true to EntryList for today", () => {
    const wrapper = mountView();
    const entryList = wrapper.findComponent({ name: "EntryList" });
    expect(entryList.props("isToday")).toBe(true);
  });
  ```

- [ ] **Step 3: Run MonthView tests**

  Run: `npx vitest run src/__tests__/components/MonthView.test.ts`
  Expected: 11 tests PASS (9 existing + 2 new)

- [ ] **Step 4: Commit**

  ```bash
  git add src/components/MonthView.vue src/__tests__/components/MonthView.test.ts
  git commit -m "feat(MonthView): pass isToday to EntryList for conditional empty state"
  ```

---

### Task 3: MonthView — widen sidebar to 280px

**Files:**
- Modify: `src/components/MonthView.vue:332`

- [ ] **Step 1: Change sidebar width class**

  ```html
  <!-- src/components/MonthView.vue line 332 — replace w-[220px] with w-[280px] -->
      <aside class="w-[280px] flex-shrink-0 flex flex-col gap-0 bg-[var(--color-surface-muted)] border-r border-[var(--color-divider)] px-lg py-xl">
  ```

  `★ Insight ─────────────────────────────────────`
  `w-[280px]` is a component dimension, not a t-shirt-size token — so the tailwind-token-usage test won't flag it. The token test only guards spacing/sizing tokens (like `max-w-md` collapsing to `--spacing-md`), not arbitrary pixel values for layout dimensions.
  `─────────────────────────────────────────────────`

- [ ] **Step 2: Run tailwind-token-usage test to confirm no false positive**

  Run: `npx vitest run src/__tests__/tailwind-token-usage.test.ts`
  Expected: PASS (no new violations)

- [ ] **Step 3: Run full MonthView test suite as a sanity check**

  Run: `npx vitest run src/__tests__/components/MonthView.test.ts src/__tests__/components/EntryList.test.ts`
  Expected: all tests PASS

- [ ] **Step 4: Commit**

  ```bash
  git add src/components/MonthView.vue
  git commit -m "feat(layout): widen sidebar from 220px to 280px"
  ```
