# Unify Value Input to Inline Editing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace DimensionEditorModal's transient value input with inline editing, matching CommitmentsModal's goal pattern.

**Architecture:** Single-component refactor. Remove `newValue` ref and transient input template block. Add `+ Add Value` button that pushes empty string to values array, `onValueEnter` for keyboard insertion, and save-time empty-value filtering.

**Tech Stack:** Vue 3 SFC, TypeScript

## Global Constraints

- Design token: spacing use `--spacing-*`, font-size use `text-title/body/secondary/micro`, no bare px or Tailwind size defaults
- Interaction principles: no silent input loss, consistent dismissal (Esc/click-outside/focus-out), keyboard-first
- Naming: components by responsibility, `*Input` DTO suffix, same concept same word across Rust/TS
- Test framework: vitest + @vue/test-utils + jsdom

---

### Task 1: Remove obsolete tests

**Files:**
- Modify: `src/__tests__/components/composite/DimensionEditorModal.test.ts`

**Note:** These tests reference the transient input pattern being removed. Delete them first to establish a failing baseline, then Task 5 adds replacement tests.

- [ ] **Step 1: Remove "adds a new value" test (L117-128)**

Delete the entire test:
```typescript
  it("adds a new value", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Select Biz (index 1) — static with values
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    const newValInput = wrapper.find('input[placeholder="New value"]');
    await newValInput.setValue("Design");
    await wrapper.find('[data-test="add-value"]').trigger("click");
    const valueInputs = wrapper.findAll('[data-test="value-input"]');
    const values = valueInputs.map((el) => (el.element as HTMLInputElement).value);
    expect(values).toContain("Design");
  });
```

- [ ] **Step 2: Remove "clears new value input after adding" test (L143-153)**

Delete the entire test:
```typescript
  it("clears new value input after adding", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    const newValInput = wrapper.find('input[placeholder="New value"]');
    await newValInput.setValue("Design");
    await wrapper.find('[data-test="add-value"]').trigger("click");
    await nextTick();
    const after = wrapper.find('input[placeholder="New value"]');
    expect((after.element as HTMLInputElement).value).toBe("");
  });
```

- [ ] **Step 3: Remove "shows hint when new value input has text" test (L155-172)**

Delete the entire test:
```typescript
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
    await nextTick();
    // Hint should disappear (newValue cleared by addValue)
    expect(wrapper.text()).not.toContain("Press Enter or click + to add");
  });
```

- [ ] **Step 4: Commit obsolete test removal**

```bash
git add src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "test: remove obsolete tests for transient value input in DimensionEditorModal"
```

---

### Task 2: Add replacement tests

**Files:**
- Modify: `src/__tests__/components/composite/DimensionEditorModal.test.ts`

**Produced:** `test: void ← adds value via inline input and + Add Value button`, `test: void ← Enter inserts new empty value row`, `test: void ← Enter on last empty value is no-op`, `test: void ← save filters empty value strings`

- [ ] **Step 1: Add test — "+ Add Value" button inserts an empty inline row**

Insert after the existing "deletes a value" test (after line 141):
```typescript
  it("+ Add Value button inserts an empty inline value row", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Select Biz (index 1) — static with values
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    const before = wrapper.findAll('[data-test="value-input"]').length;
    await wrapper.find('[data-test="add-value-btn"]').trigger("click");
    await nextTick();
    const after = wrapper.findAll('[data-test="value-input"]').length;
    expect(after).toBe(before + 1);
    // The new row should be empty
    const inputs = wrapper.findAll('[data-test="value-input"]');
    const lastVal = (inputs[inputs.length - 1].element as HTMLInputElement).value;
    expect(lastVal).toBe("");
  });
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts -t "+ Add Value button"
```
Expected: FAIL (element `[data-test="add-value-btn"]` does not exist yet)

- [ ] **Step 3: Add test — Enter on value input inserts new empty row below**

Insert after the test from Step 1:
```typescript
  it("Enter on value input inserts new empty row below", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Select Biz (index 1) — static with values: ["Product", "Marketing", "Engineering"]
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    const before = wrapper.findAll('[data-test="value-input"]').length;
    // Press Enter on first value input ("Product")
    const firstInput = wrapper.findAll('[data-test="value-input"]')[0];
    await firstInput.trigger("keydown.enter");
    await nextTick();
    const after = wrapper.findAll('[data-test="value-input"]').length;
    expect(after).toBe(before + 1);
    // The inserted row should be empty and positioned after index 0
    const inputs = wrapper.findAll('[data-test="value-input"]');
    expect((inputs[1].element as HTMLInputElement).value).toBe("");
    // Original "Product" still at index 0, "Marketing" shifted to index 2
    expect((inputs[0].element as HTMLInputElement).value).toBe("Product");
    expect((inputs[2].element as HTMLInputElement).value).toBe("Marketing");
  });
```

- [ ] **Step 4: Run test to verify it fails**

```bash
pnpm vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts -t "Enter on value"
```
Expected: FAIL (no `onValueEnter` handler yet)

- [ ] **Step 5: Add test — Enter on last empty value is no-op (guard)**

Insert after the test from Step 3:
```typescript
  it("Enter on last empty value is a no-op", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Select Biz (index 1) — static with values
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    // Add an empty value row first
    await wrapper.find('[data-test="add-value-btn"]').trigger("click");
    await nextTick();
    const before = wrapper.findAll('[data-test="value-input"]').length;
    // Press Enter on the last (empty) input
    const inputs = wrapper.findAll('[data-test="value-input"]');
    const lastInput = inputs[inputs.length - 1];
    await lastInput.trigger("keydown.enter");
    await nextTick();
    const after = wrapper.findAll('[data-test="value-input"]').length;
    expect(after).toBe(before); // No new row inserted
  });
```

- [ ] **Step 6: Run test to verify it fails**

```bash
pnpm vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts -t "Enter on last"
```
Expected: FAIL (no `onValueEnter` handler yet)

- [ ] **Step 7: Add test — save filters empty value strings**

Insert after the existing "emits saved with updated dimensions on Save" test:
```typescript
  it("filters empty values when saving", async () => {
    const wrapper = mountModal({ open: true, dimensions: MOCK_DIMENSIONS });
    // Select Biz (index 1) — static with values
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    // Push an empty value to the array (simulating inline editing leaving an empty row)
    await wrapper.find('[data-test="add-value-btn"]').trigger("click");
    await nextTick();
    await wrapper.find('[data-test="save"]').trigger("click");
    await nextTick();

    const callArgs = (invoke as any).mock.calls.find((c: any[]) => c[0] === "save_dimensions")[1];
    const bizDim = callArgs.dimensions.find((d: any) => d.key === "biz");
    expect(bizDim.values).not.toContain("");
    expect(bizDim.values).toEqual(["Product", "Marketing", "Engineering"]);
  });
```

- [ ] **Step 8: Commit new failing tests**

```bash
git add src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "test: add tests for inline value editing in DimensionEditorModal"
```

---

### Task 3: Implement script changes

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue`

**Consumes:** N/A (script-only refactor)
**Produces:** `function onValueEnter(index: number): void`, empty-value filtering in `save()`

- [ ] **Step 1: Remove `newValue` ref**

Delete line 23:
```typescript
const newValue = ref("");
```
(entire line removed)

- [ ] **Step 2: Remove `newValue` reset in watch**

In the watch callback (around line 46), delete:
```typescript
  newValue.value = "";
```

- [ ] **Step 3: Remove `addValue` function**

Delete lines 96-101 (the entire `addValue` function):
```typescript
function addValue() {
  const val = newValue.value.trim();
  if (!val || !selectedDimension.value?.values) return;
  selectedDimension.value.values = [...selectedDimension.value.values, val];
  newValue.value = "";
}
```

- [ ] **Step 4: Add `onValueEnter` function**

Insert after the `removeValue` function (after line 106):
```typescript
function onValueEnter(index: number) {
  if (!selectedDimension.value?.values) return;
  const values = selectedDimension.value.values;
  if (index === values.length - 1 && values[index].trim() === "") return;
  values.splice(index + 1, 0, "");
}
```

- [ ] **Step 5: Add empty-value filtering in `save()`**

In the `save()` function, before the `invoke` call, insert filtering logic. Change:
```typescript
    const result = await invoke<Dimension[]>("save_dimensions", {
      rootPath: props.rootPath,
      year: props.year,
      month: props.month,
      dimensions: draft.value,
    });
```
To:
```typescript
    const cleaned = draft.value.map(d => {
      if (d.source === "static" && d.values) {
        return { ...d, values: d.values.filter(v => v.trim() !== "") };
      }
      return d;
    });
    const result = await invoke<Dimension[]>("save_dimensions", {
      rootPath: props.rootPath,
      year: props.year,
      month: props.month,
      dimensions: cleaned,
    });
```

- [ ] **Step 6: Commit script changes**

```bash
git add src/components/composite/DimensionEditorModal.vue
git commit -m "refactor: replace transient value input with inline editing in DimensionEditorModal — script"
```

---

### Task 4: Implement template changes

**Files:**
- Modify: `src/components/composite/DimensionEditorModal.vue`

**Consumes:** `onValueEnter(index: number)` from Task 3

- [ ] **Step 1: Add Enter handler to value inputs**

On each value `<input>` (line 419), add `@keydown.enter.exact.prevent`:
```html
                      <input
                        data-test="value-input"
                        :value="val"
                        :disabled="selectedDimension.deleted"
                        @input="updateValue(i, $event)"
                        @keydown.enter.exact.prevent="onValueEnter(i)"
                        class="flex-1 px-sm py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
                               text-body text-[var(--color-text-primary)]
                               bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]
                               disabled:opacity-40"
                      />
```

- [ ] **Step 2: Delete transient input block**

Delete the entire block from `<!-- New value input (hidden when deleted) -->` comment (line 439) through `</template>` on line 458, inclusive of lines 439-458:
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

- [ ] **Step 3: Add "+ Add Value" button**

Insert after the `</VueDraggable>` closing tag (after line 437, before the `</template>` on line 459):
```html
                  <button
                    v-if="!selectedDimension.deleted"
                    data-test="add-value-btn"
                    class="self-start mt-sm text-secondary font-medium text-[var(--color-brand-link)] cursor-pointer hover:underline"
                    @click="selectedDimension.values = [...selectedDimension.values, '']"
                  >+ Add Value</button>
```

- [ ] **Step 4: Commit template changes**

```bash
git add src/components/composite/DimensionEditorModal.vue
git commit -m "refactor: replace transient value input with inline editing in DimensionEditorModal — template"
```

---

### Task 5: Update remaining tests to match new markup

**Files:**
- Modify: `src/__tests__/components/composite/DimensionEditorModal.test.ts`

**Consumes:** new `[data-test="add-value-btn"]` and removal of `input[placeholder="New value"]`

- [ ] **Step 1: Update "does not show values section for commitments:goals dimensions" test**

Replace the check for `input[placeholder="New value"]` with `[data-test="add-value-btn"]`. Change line 106:
```typescript
    expect(wrapper.find('input[placeholder="New value"]').exists()).toBe(false);
```
To:
```typescript
    expect(wrapper.find('[data-test="add-value-btn"]').exists()).toBe(false);
```

- [ ] **Step 2: Update "hides add-value section when selected dimension is deleted" test**

Replace references from `[data-test="add-value"]` and `input[placeholder="New value"]` to the new `[data-test="add-value-btn"]`. Replace the entire test (lines 525-538):
```typescript
  it("hides add-value section when selected dimension is deleted", async () => {
    // Use Biz (index 1, static with values)
    const wrapper = mountModal();
    const bizRow = wrapper.findAll('[data-test="dim-row"]')[1];
    await bizRow.trigger("click");
    // First verify + Add Value button exists
    expect(wrapper.find('[data-test="add-value-btn"]').exists()).toBe(true);
    // Delete Biz
    await wrapper.find('[data-test="delete-dim"]').trigger("click");
    await nextTick();
    // + Add Value button should be gone
    expect(wrapper.find('[data-test="add-value-btn"]').exists()).toBe(false);
  });
```

- [ ] **Step 3: Commit test updates**

```bash
git add src/__tests__/components/composite/DimensionEditorModal.test.ts
git commit -m "test: update test selectors for inline value editing in DimensionEditorModal"
```

---

### Task 6: Verify all tests pass

- [ ] **Step 1: Run frontend tests**

```bash
pnpm vitest run src/__tests__/components/composite/DimensionEditorModal.test.ts
```
Expected: all tests PASS, new tests covering Enter behavior and save filtering all green.

- [ ] **Step 2: Run full test suite**

```bash
pnpm test
```
Expected: all tests PASS (vitest + cargo test).

- [ ] **Step 3: Run typecheck**

```bash
pnpm typecheck
```
Expected: PASS, no type errors from removed `newValue` ref.

- [ ] **Step 4: Manual smoke test**

```bash
pnpm tauri dev
```
Verify:
- Open Dimensions modal, select a `static` dimension
- Click `+ Add Value` → new empty inline row appears
- Type in the row → value persists in the row
- Press Enter on a non-empty row → new empty row appears below
- Press Enter on the last empty row → no-op
- Delete a value with `×` button
- Save → reopen → no empty values persisted
- `commitments:goals` dimension: no `+ Add Value` button visible

---

### Task 7: Cleanup superseded spec

**Files:**
- Delete: `docs/superpowers/specs/2026-07-04-dimension-value-add-hint-design.md`

- [ ] **Step 1: Remove the old spec**

```bash
rm docs/superpowers/specs/2026-07-04-dimension-value-add-hint-design.md
git add docs/superpowers/specs/2026-07-04-dimension-value-add-hint-design.md
git commit -m "docs: remove superseded dimension-value-add-hint spec"
```

---

### Task 8: Final verification and lint

- [ ] **Step 1: Lint check**

```bash
pnpm lint
```
Expected: PASS, no lint errors.

- [ ] **Step 2: Verify no dangling references to deleted code**

```bash
rg -n "newValue|addValue|New value|add-value[^-]" src/ --include '*.ts' --include '*.vue' --include '*.svelte'
```
Expected: no output (or only comments/irrelevant hits outside the component). The `data-test="add-value-btn"` should be the only `add-value` hit.

- [ ] **Step 3: Create final commit if any lint fixes needed**
