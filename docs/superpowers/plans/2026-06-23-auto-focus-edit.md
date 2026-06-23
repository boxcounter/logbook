# Auto-focus on Edit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When entering edit mode on an entry row, auto-focus the input matching the click target (item or duration) with the cursor at the end.

**Architecture:** EntryRow detects the click target via `data-edit-target` attributes and `closest()`, passes `focusTarget` prop to EntryRowEdit, which focuses the matching input on mount with `setSelectionRange` to place the cursor at the end.

**Tech Stack:** Vue 3 + TypeScript + vitest + @vue/test-utils

---

### Task 1: Add click-target detection in EntryRow

**Files:**
- Modify: `src/components/composite/EntryRow.vue`

- [ ] **Step 1: Add `data-edit-target` attributes to the display row**

Add `data-edit-target="item"` to the item container div and `data-edit-target="duration"` to the duration span. This ensures `closest()` finds the correct target regardless of whether the click lands on the item text, a dimension chip, or whitespace inside the item area.

In `EntryRow.vue`, change lines 61 and 75 (add `data-edit-target` and `data-test` attributes):

Line 61 — add `data-edit-target="item"`:
```html
	    <div class="flex-1 min-w-0" data-edit-target="item">
```

Line 62 — add `data-test="item-display"` for test targeting:
```html
	      <div
	        data-test="item-display"
	        class="text-body font-medium …"
```

Line 75 — add both attributes to the duration span:
```html
	    <span
	      data-test="duration-display"
	      data-edit-target="duration"
	      class="mono text-secondary …"
	    >{{ entry.duration > 0 ? formatDuration(entry.duration) : "—" }}</span>
```

Key: placing `data-edit-target="item"` on the parent `<div>` that contains both the item text and the chips div means `closest()` from a chip click walks up to the item container — correct default behavior.

- [ ] **Step 2: Add `focusTarget` ref and `onDblClick` handler**

Add to the `<script setup>` section after line 18 (`const editing = ref(false);`):

```ts
const focusTarget = ref<'item' | 'duration'>('item');

function onDblClick(e: MouseEvent) {
  const target = (e.target as HTMLElement).closest('[data-edit-target]') as HTMLElement | null;
  focusTarget.value = (target?.dataset.editTarget as 'item' | 'duration') || 'item';
  editing.value = true;
}
```

- [ ] **Step 3: Update the "…" button handler**

Replace the inline `@click="editing = true"` on the "…" span (line 82) with a function call:

In the `<script setup>`, add after `onDblClick`:

```ts
function onEditTrigger() {
  focusTarget.value = 'item';
  editing.value = true;
}
```

In the template, change line 82 from:
```html
	      @click="editing = true"
```
to:
```html
	      @click="onEditTrigger"
```

- [ ] **Step 4: Bind the dblclick handler and pass focusTarget prop**

Replace the inline `@dblclick="editing = true"` on line 59 with `@dblclick="onDblClick"`.

Pass `:focus-target="focusTarget"` to the `EntryRowEdit` component on line 44:

```html
	  <EntryRowEdit
	    v-if="editing"
	    :entry="entry"
	    :dimensions="dimensions"
	    :commitments="store.commitments"
	    :focus-target="focusTarget"
	    @save="onSave"
	    @cancel="editing = false"
	    @delete="emit('delete', entry.id); editing = false"
	  />
```

- [ ] **Step 5: Verify the build compiles**

Run: `npx vue-tsc --noEmit`
Expected: no new type errors.

- [ ] **Step 6: Commit**

```bash
git add src/components/composite/EntryRow.vue
git commit -m "feat: detect edit click target and pass focusTarget to EntryRowEdit"
```

---

### Task 2: Add focus-on-mount in EntryRowEdit

**Files:**
- Modify: `src/components/composite/EntryRowEdit.vue`

- [ ] **Step 1: Add `focusTarget` prop and template refs**

Add to the `defineProps` block (line 8-12), after `commitments`:

```ts
  focusTarget?: 'item' | 'duration';
```

Add template refs after line 25 (`const rootEl = ref<HTMLElement>();`):

```ts
const itemInputEl = ref<HTMLInputElement>();
const durInputEl = ref<HTMLInputElement>();
```

- [ ] **Step 2: Bind template refs to the inputs**

On the item input (line 143), add `ref="itemInputEl"`:

```html
	      <input
	        ref="itemInputEl"
	        v-model="item"
```

On the duration input (line 148), add `ref="durInputEl"`:

```html
	      <input
	        ref="durInputEl"
	        v-model="durText"
```

- [ ] **Step 3: Add focus logic in onMounted**

In the existing `onMounted` (line 74-78), add focus logic after the event listener registrations:

```ts
onMounted(async () => {
  document.addEventListener("mousedown", onDocMousedown, true);
  document.addEventListener("focusin", onDocFocusin, true);
  document.addEventListener("keydown", onDocKeydown);

  await nextTick();
  const target = props.focusTarget === 'duration' ? durInputEl.value : itemInputEl.value;
  target?.focus();
  if (target) {
    target.setSelectionRange(target.value.length, target.value.length);
  }
});
```

Add `nextTick` to the import on line 3:

```ts
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
```

- [ ] **Step 4: Verify the build compiles**

Run: `npx vue-tsc --noEmit`
Expected: no new type errors.

- [ ] **Step 5: Commit**

```bash
git add src/components/composite/EntryRowEdit.vue
git commit -m "feat: auto-focus item or duration input on edit entry mount"
```

---

### Task 3: Add EntryRow tests for click-target detection

**Files:**
- Modify: `src/__tests__/components/composite/EntryRow.test.ts`

- [ ] **Step 1: Write tests for focusTarget prop passing**

Add the following tests to the `describe("EntryRow", ...)` block, before the closing `});` on line 90:

```ts
  it("passes focusTarget='item' when double-clicking the item text", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='item-display']").trigger("dblclick");
    const editor = wrapper.findComponent({ name: "EntryRowEdit" });
    expect(editor.props("focusTarget")).toBe("item");
  });

  it("passes focusTarget='duration' when double-clicking the duration", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='duration-display']").trigger("dblclick");
    const editor = wrapper.findComponent({ name: "EntryRowEdit" });
    expect(editor.props("focusTarget")).toBe("duration");
  });

  it("passes focusTarget='item' when double-clicking a dimension chip (closest traversal)", async () => {
    // The chip is inside the item-target div, so closest() walks up to data-edit-target="item"
    const wrapper = mountRow();
    // Find the first dimension chip text
    const chip = wrapper.find(".text-micro");
    await chip.trigger("dblclick");
    const editor = wrapper.findComponent({ name: "EntryRowEdit" });
    expect(editor.props("focusTarget")).toBe("item");
  });

  it("passes focusTarget='item' when clicking the … trigger", async () => {
    const wrapper = mountRow();
    await wrapper.find("[data-test='edit-trigger']").trigger("click");
    const editor = wrapper.findComponent({ name: "EntryRowEdit" });
    expect(editor.props("focusTarget")).toBe("item");
  });
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `npx vitest run src/__tests__/components/composite/EntryRow.test.ts`
Expected: all tests pass (new + existing).

- [ ] **Step 3: Commit**

```bash
git add src/__tests__/components/composite/EntryRow.test.ts
git commit -m "test: verify focusTarget prop based on double-click target"
```

---

### Task 4: Add EntryRowEdit tests for focus behavior

**Files:**
- Modify: `src/__tests__/components/composite/EntryRowEdit.test.ts`

- [ ] **Step 1: Add a mount helper that accepts focusTarget**

Add after the existing `mountEdit` function (line 22):

```ts
function mountEditWithFocus(focusTarget: 'item' | 'duration' = 'item') {
  const entry = makeEntry({ item: "Old item", duration: 45, dimensions: { ...fullDims } });
  return mount(EntryRowEdit, {
    props: { entry, dimensions, commitments, focusTarget },
    attachTo: document.body,
  });
}
```

`attachTo: document.body` is required for `element.focus()` to set `document.activeElement` in jsdom.

- [ ] **Step 2: Write tests for focus behavior**

Add to the `describe("EntryRowEdit", ...)` block, before the closing `});` on line 215:

```ts
  it("focuses the item input on mount when focusTarget is 'item'", () => {
    const wrapper = mountEditWithFocus('item');
    const inputs = wrapper.findAll("input");
    expect(document.activeElement).toBe(inputs[0].element);
  });

  it("focuses the duration input on mount when focusTarget is 'duration'", () => {
    const wrapper = mountEditWithFocus('duration');
    const inputs = wrapper.findAll("input");
    expect(document.activeElement).toBe(inputs[1].element);
  });

  it("defaults to focusing the item input when focusTarget is omitted", () => {
    const entry = makeEntry({ item: "Old item", duration: 45, dimensions: { ...fullDims } });
    const wrapper = mount(EntryRowEdit, {
      props: { entry, dimensions, commitments },
      attachTo: document.body,
    });
    const inputs = wrapper.findAll("input");
    expect(document.activeElement).toBe(inputs[0].element);
  });

  it("places the cursor at the end of the existing text", async () => {
    const spy = vi.spyOn(HTMLInputElement.prototype, 'setSelectionRange');
    const entry = makeEntry({ item: "Review PR", duration: 45, dimensions: { ...fullDims } });
    const wrapper = mount(EntryRowEdit, {
      props: { entry, dimensions, commitments, focusTarget: 'item' },
      attachTo: document.body,
    });
    // setSelectionRange should be called with (len, len) to place cursor at end
    const itemInput = wrapper.findAll("input")[0].element as HTMLInputElement;
    expect(spy).toHaveBeenCalledWith(itemInput.value.length, itemInput.value.length);
    spy.mockRestore();
  });
```

Add `vi` to the vitest import on line 2:

```ts
import { describe, it, expect, afterEach, vi } from "vitest";
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `npx vitest run src/__tests__/components/composite/EntryRowEdit.test.ts`
Expected: all tests pass (new + existing).

- [ ] **Step 4: Run full test suite to check for regressions**

Run: `npx vitest run`
Expected: all tests pass.

- [ ] **Step 5: Run full build check**

Run: `npm run build`
Expected: build succeeds (this includes vue-tsc type-checking).

- [ ] **Step 6: Commit**

```bash
git add src/__tests__/components/composite/EntryRowEdit.test.ts
git commit -m "test: verify auto-focus behavior on EntryRowEdit mount"
```
