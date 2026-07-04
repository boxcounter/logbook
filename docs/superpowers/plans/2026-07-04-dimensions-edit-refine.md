# Dimensions Edit Refine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Three small refinements to DimensionEditorModal: filter deleted dimensions from template save, add spacing between separator and "Save as template" button, and add `commitments:role` info card matching the existing `commitments:goals` one.

**Architecture:** All changes in a single file — `src/components/composite/DimensionEditorModal.vue` (three template/logic edits). Tests in `src/__tests__/components/composite/DimensionEditorModal.test.ts` (update existing test, add two new tests).

**Tech Stack:** Vue 3 + TypeScript + Vitest + jsdom

---

### File Map

| File | Responsibility | Action |
|------|---------------|--------|
| `src/components/composite/DimensionEditorModal.vue` | Dimensions editing modal | Modify: filter deleted dims, add spacing, add role card |
| `src/__tests__/components/composite/DimensionEditorModal.test.ts` | Unit tests for DimensionEditorModal | Modify: update save-as-template test, add 2 new tests |

---

### Task 1: Filter deleted dimensions from save-as-template

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue:190-193`
- Test: `src/__tests__/components/composite/DimensionEditorModal.test.ts:253-262`

- [ ] **Step 1: Write the failing test**

Add a test to verify deleted dimensions are filtered out. Place it right after the existing "saveAsTemplate invokes save_dimensions_template" test (after line 262):

```ts
// Test imports at top of file already include everything needed.
// No new imports required.

it("saveAsTemplate filters out deleted dimensions", async () => {
  const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
  // Goal (index 0) is selected by default. Mark it as deleted.
  const deleteBtn = wrapper.find('[data-test="delete-dim"]');
  await deleteBtn.trigger("click");
  await nextTick();

  // Save as template
  await wrapper.find('[data-test="save-as-template"]').trigger("click");
  await nextTick();
  await nextTick();

  // Only the 2 non-deleted dimensions (Biz, Importance) should be passed
  expect(invoke).toHaveBeenCalledWith("save_dimensions_template", expect.objectContaining({
    rootPath: "/test",
    dimensions: [
      expect.objectContaining({ name: "Goal", deleted: true }),
      expect.objectContaining({ name: "Biz", deleted: false }),
      expect.objectContaining({ name: "Importance", deleted: false }),
    ],
  }));
});
```

Wait — this test would FAIL after the fix because `dimensions` should NOT include the deleted Goal. Let me fix the test:

```ts
it("saveAsTemplate filters out deleted dimensions", async () => {
  const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
  // Goal (index 0) is selected by default. Mark it as deleted.
  const deleteBtn = wrapper.find('[data-test="delete-dim"]');
  await deleteBtn.trigger("click");
  await nextTick();

  // Save as template
  await wrapper.find('[data-test="save-as-template"]').trigger("click");
  await nextTick();
  await nextTick();

  // Only the 2 non-deleted dimensions should be passed
  const callArgs = (invoke as any).mock.calls.find((c: any[]) => c[0] === "save_dimensions_template")[1];
  const passedDims: { key: string; deleted?: boolean }[] = callArgs.dimensions;
  expect(passedDims).toHaveLength(2);
  expect(passedDims.map((d: any) => d.key)).toEqual(["biz", "importance-urgency"]);
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test -- src/__tests__/components/composite/DimensionEditorModal.test.ts -t "filters out deleted"
```

Expected: FAIL — 3 dimensions passed instead of 2.

- [ ] **Step 3: Implement the fix**

In `src/components/composite/DimensionEditorModal.vue`, modify lines 190-194:

```ts
// Before:
await invoke("save_dimensions_template", {
  rootPath: props.rootPath,
  dimensions: draft.value,
});

// After:
const active = draft.value.filter(d => !d.deleted);
await invoke("save_dimensions_template", {
  rootPath: props.rootPath,
  dimensions: active,
});
```

- [ ] **Step 4: Run tests to verify pass**

```bash
pnpm test -- src/__tests__/components/composite/DimensionEditorModal.test.ts
```

Expected: All existing tests pass, new test passes.

- [ ] **Step 5: Also update the existing test to reflect correct behavior**

The existing test at line 253-262 passes MOCK_DIMENSIONS without any deleted items, so `expect.objectContaining({ dimensions: MOCK_DIMENSIONS })` already receives the correct filtered array (since none are deleted). This test requires no change. Verify it still passes:

```bash
pnpm test -- src/__tests__/components/composite/DimensionEditorModal.test.ts -t "saveAsTemplate invokes save_dimensions_template"
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "fix: save-as-template filters deleted dimensions"
```

---

### Task 2: Add spacing between separator and "Save as template" button

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue:233-238`

- [ ] **Step 1: Apply the spacing fix**

In `src/components/composite/DimensionEditorModal.vue`, modify line 235 — add `ml-2xs` to the button's class:

```html
<!-- Before: -->
<button
  data-test="save-as-template"
  class="text-secondary font-semibold text-[var(--color-brand-link)] cursor-pointer disabled:opacity-50 disabled:cursor-default"

<!-- After: -->
<button
  data-test="save-as-template"
  class="ml-2xs text-secondary font-semibold text-[var(--color-brand-link)] cursor-pointer disabled:opacity-50 disabled:cursor-default"
```

- [ ] **Step 2: Verify the token passes the tailwind token usage test**

```bash
pnpm test -- src/__tests__/tailwind-token-usage.test.ts
```

Expected: PASS — `ml-2xs` is a valid `--spacing-*` token.

- [ ] **Step 3: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue
git commit -m "fix: add spacing between separator and Save as template button"
```

---

### Task 3: Add commitments:role info card

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue:462`
- Test: `src/__tests__/components/composite/DimensionEditorModal.test.ts:93`

- [ ] **Step 1: Write the failing test**

Add a test right after the "shows commitments:goals info card" test (after line 93):

```ts
it("shows commitments:role info card", async () => {
  const roleDim: Dimension = { name: "Role", key: "role", source: "commitments:role", values: undefined, required: false, deleted: false };
  const wrapper = mountModal({ open: true, dimensions: [roleDim, ...MOCK_DIMENSIONS] });
  // Click the Role row (index 0) to select it
  const roleRow = wrapper.findAll('[data-test="dim-row"]')[0];
  await roleRow.trigger("click");
  expect(wrapper.text()).toContain("Values are derived from commitment roles");
});
```

Note: `Dimension` type is already imported at the top of the test file (line 7).

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test -- src/__tests__/components/composite/DimensionEditorModal.test.ts -t "shows commitments:role info card"
```

Expected: FAIL — the text "Values are derived from commitment roles" is not found.

- [ ] **Step 3: Implement the info card**

In `src/components/composite/DimensionEditorModal.vue`, add after the `commitments:goals` template block (after line 462 — the `</template>` that closes `commitments:goals`):

```html
<!-- Add right after the commitments:goals </template> (after line 462) -->
<template v-if="selectedDimension.source === 'commitments:role'">
  <div class="border border-[var(--color-border-form)] rounded-[var(--radius-form-lg)] bg-[var(--color-surface-muted)] p-md">
    <p class="text-secondary text-[var(--color-text-muted)]">Values are derived from commitment roles.</p>
  </div>
</template>
```

- [ ] **Step 4: Run tests to verify pass**

```bash
pnpm test -- src/__tests__/components/composite/DimensionEditorModal.test.ts
```

Expected: All tests pass, including the new one.

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "feat: add commitments:role info card in dimension editor"
```

---

### Task 4: Full test verification

- [ ] **Run the full test suite**

```bash
pnpm test
```

Expected: All tests pass.

- [ ] **Run lint and typecheck**

```bash
pnpm lint
pnpm typecheck
```

Expected: No errors.
