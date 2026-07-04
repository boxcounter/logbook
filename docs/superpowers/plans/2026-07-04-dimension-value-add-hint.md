# Dimension Value Add Hint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a hint "Press Enter or click + to add" below the new value input when the user has typed something but hasn't committed yet.

**Architecture:** Single line addition in DimensionEditorModal.vue — a conditional `<p>` tag after the dashed input row. One new test for show/hide behavior.

**Tech Stack:** Vue 3 + TypeScript + Vitest + jsdom

---

### File Map

| File | Responsibility | Action |
|------|---------------|--------|
| `src/components/composite/DimensionEditorModal.vue` | Dimension editing modal | Modify: add hint line after value input row |
| `src/__tests__/components/composite/DimensionEditorModal.test.ts` | Unit tests | Modify: add test for hint show/hide |

---

### Task 1: Add value add hint

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue:454`
- Test: `src/__tests__/components/composite/DimensionEditorModal.test.ts:147`

- [ ] **Step 1: Write the failing test**

Add a test right after the "clears new value input after adding" test (after line 147):

```ts
it("shows hint when new value input has text", async () => {
  const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
  // Select Biz (index 1) — static with values
  const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
  await bizRow.trigger("click");
  const newValInput = wrapper.find('input[placeholder="New value"]');
  // No hint initially
  expect(wrapper.text()).not.toContain("Press Enter or click + to add");
  // Type something
  await newValInput.setValue("Design");
  // Hint should appear
  expect(wrapper.text()).toContain("Press Enter or click + to add");
  // Commit the value
  await wrapper.find('[data-test="add-value"]').trigger("click");
  // Hint should disappear (newValue cleared by addValue)
  expect(wrapper.text()).not.toContain("Press Enter or click + to add");
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test -- src/__tests__/components/composite/DimensionEditorModal.test.ts -t "shows hint when new value"
```

Expected: FAIL — "Press Enter or click + to add" not found.

- [ ] **Step 3: Implement the hint**

In `src/components/composite/DimensionEditorModal.vue`, add after the closing `</div>` of the input row (line 454), before the closing `</template>` (line 455):

```html
                  <!-- New value input (hidden when deleted) -->
                  <template v-if="!selectedDimension.deleted">
                    <div class="flex items-center gap-sm mt-sm">
                      <span class="text-[var(--color-text-disabled)] select-none px-2xs invisible">⠿</span>
                      <input
                        v-model="newValue"
                        placeholder="New value"
                        class="flex-1 px-sm py-xs border border-dashed border-[var(--color-border-form)] rounded-[var(--radius-form)]
                               text-body text-[var(--color-text-primary)] placeholder-[var(--color-placeholder)]
                               bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
                        @keydown.enter.exact.prevent="addValue"
                      />
                      <button
                        data-test="add-value"
                        class="text-secondary font-semibold text-[var(--color-brand-link)] px-sm py-xs cursor-pointer"
                        @click="addValue"
                      >+</button>
                    </div>
                    <p v-if="newValue.trim()" class="text-micro text-[var(--color-text-muted)] mt-xs">Press Enter or click + to add</p>
                  </template>
```

The only new line is:
```html
<p v-if="newValue.trim()" class="text-micro text-[var(--color-text-muted)] mt-xs">Press Enter or click + to add</p>
```

- [ ] **Step 4: Run tests to verify pass**

```bash
pnpm test -- src/__tests__/components/composite/DimensionEditorModal.test.ts
```

Expected: All tests pass, including the new one.

- [ ] **Step 5: Run full test suite**

```bash
pnpm test
```

Expected: All tests pass.

- [ ] **Step 6: Run typecheck**

```bash
npx vue-tsc --noEmit
```

Expected: No errors.

- [ ] **Step 7: Commit**

```bash
git add src/components/composite/DimensionEditorModal.vue src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "feat: show hint when new value input has uncommitted text"
```
