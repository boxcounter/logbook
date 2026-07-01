# UX Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Three independent UX improvements: widen goal text in commitments sidebar, move version number to OS window title, and support right-click copy of file path.

**Architecture:** All changes are in the Vue frontend layer — no Rust/Tauri backend changes. The three tasks touch different parts of MonthView.vue and CommitmentsPanel.vue and can be implemented in any order. One minor test mock update is needed for the window title change.

**Tech Stack:** Vue 3 + TypeScript + Tauri 2.x + Vitest + @vue/test-utils

**Spec:** `docs/superpowers/specs/2026-07-01-ux-polish-design.md`

---

### Task 1: Widen Goal Text in Commitments Sidebar

**Files:**
- Modify: `src/components/MonthView.vue:356`
- Modify: `src/components/CommitmentsPanel.vue:68-76`

- [ ] **Step 1: Widen sidebar from 280px to 320px**

In `src/components/MonthView.vue` line 356, change the sidebar `<aside>` width class:

```diff
-    <aside class="w-[280px] flex-shrink-0 flex flex-col gap-0 bg-[var(--color-surface-muted)] border-r border-[var(--color-divider)] px-lg py-xl">
+    <aside class="w-[320px] flex-shrink-0 flex flex-col gap-0 bg-[var(--color-surface-muted)] border-r border-[var(--color-divider)] px-lg py-xl">
```

- [ ] **Step 2: Replace goal name fixed max-w with flex-grow layout**

In `src/components/CommitmentsPanel.vue`, replace the goal row template (lines 68-76):

```diff
          <div
            v-for="g in s.goals" :key="g.name"
            data-test="goal-row"
            class="flex justify-between text-secondary text-[var(--color-text-secondary)] py-xs pl-sm"
          >
-            <span class="overflow-hidden text-ellipsis whitespace-nowrap max-w-[130px]" :title="g.name">{{ g.name }}</span>
+            <span class="overflow-hidden text-ellipsis whitespace-nowrap flex-1 min-w-0" :title="g.name">{{ g.name }}</span>
-            <span v-if="g.spent_minutes > 0" class="mono font-medium text-[var(--color-text-primary)]">{{ formatDurationCompact(g.spent_minutes) }}</span>
-            <span v-else class="mono text-[var(--color-text-secondary)]">0</span>
+            <span v-if="g.spent_minutes > 0" class="mono font-medium text-[var(--color-text-primary)] flex-shrink-0 ml-sm">{{ formatDurationCompact(g.spent_minutes) }}</span>
+            <span v-else class="mono text-[var(--color-text-secondary)] flex-shrink-0 ml-sm">0</span>
          </div>
```

- [ ] **Step 3: Run existing tests to verify no regressions**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook/.claude/worktrees/pure-scribbling-kurzweil && npx vitest run src/__tests__/components/CommitmentsPanel.test.ts src/__tests__/components/MonthView.test.ts
```

Expected: All existing tests pass. No test references the old `max-w-[130px]` or `w-[280px]` values.

- [ ] **Step 4: Commit**

```bash
git add src/components/MonthView.vue src/components/CommitmentsPanel.vue
git commit -m "feat: widen sidebar to 320px and use flex-grow for goal text"
```

---

### Task 2: Version Number in OS Window Title

**Files:**
- Modify: `src/components/MonthView.vue:1-6` (imports), `:35` (ref declaration), `:338` (onMounted), `:438-439` (template)
- Modify: `src/__tests__/mocks/tauri.ts:63` (add setTitle to mock)

- [ ] **Step 1: Update the Tauri mock to include setTitle**

In `src/__tests__/mocks/tauri.ts` line 63, add `setTitle` to the `getCurrentWindow` mock return value so any component test that renders MonthView doesn't crash:

```diff
-const getCurrentWindow = vi.fn().mockReturnValue({ onFocusChanged });
+const getCurrentWindow = vi.fn().mockReturnValue({ onFocusChanged, setTitle: vi.fn() });
```

Note: `onFocusChanged` already returns a mock function elsewhere; no need to change `Mock` type since `getCurrentWindow` already returns a plain object cast via `as Mock`-compatible inference.

- [ ] **Step 2: Add getCurrentWindow import, remove getVersion import**

In `src/components/MonthView.vue`, update the `<script setup>` imports:

```diff
  import { inject, computed, watch, ref, onMounted, onUnmounted, nextTick } from "vue";
  import { invoke } from "@tauri-apps/api/core";
- import { getVersion } from "@tauri-apps/api/app";
+ import { getCurrentWindow } from "@tauri-apps/api/window";
  import { useStore } from "../stores/useStore";
```

- [ ] **Step 3: Remove appVersion ref, update onMounted to set window title**

In `src/components/MonthView.vue`, remove the `appVersion` ref (line 35):

```diff
-const appVersion = ref("");
```

In `onMounted` (line 338), replace the `getVersion()` call that stored the version in the ref with a call that sets the OS window title:

```diff
  onMounted(async () => {
    window.addEventListener("keydown", onGlobalKeydown);
-   getVersion().then(v => { appVersion.value = v; }).catch(() => {});
+   getVersion().then(v => { getCurrentWindow().setTitle("Logbook v" + v); }).catch(() => {});
    if (store.rootPath) {
```

- [ ] **Step 4: Remove the version `<span>` from the template**

In `src/components/MonthView.vue` lines 438-439, remove the version display element:

```diff
       <div v-if="store.rootPath" class="mt-sm text-right flex justify-end items-baseline gap-md">
-        <span v-if="appVersion" class="text-micro text-[var(--color-text-disabled)]">v{{ appVersion }}</span>
         <button
           class="text-micro text-[var(--color-text-disabled)] hover:text-[var(--color-text-secondary)] cursor-pointer"
           :title="store.rootPath + '/' + dayFilePath"
           @click="revealDayFile"
         >{{ displayPath }}</button>
       </div>
```

The `gap-md` on the parent div can stay (it becomes a no-op with a single child), or be removed for cleanliness. Keep it — harmless and avoids an extra whitespace diff.

- [ ] **Step 5: Add getVersion mock to MonthView.test.ts**

In `src/__tests__/components/MonthView.test.ts`, add the mock near the other vi.mock calls (after line 11):

```typescript
vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn().mockResolvedValue("0.0.0") }));
```

Also add the `getCurrentWindow` mock if not already covered (the test file doesn't currently mock `@tauri-apps/api/window`, so we need to add it):

```typescript
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: vi.fn().mockReturnValue({ setTitle: vi.fn() }) }));
```

- [ ] **Step 6: Run tests**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook/.claude/worktrees/pure-scribbling-kurzweil && npx vitest run src/__tests__/components/MonthView.test.ts
```

Expected: All existing tests pass.

- [ ] **Step 7: Verify version display is gone from DOM**

Add a quick test to MonthView.test.ts confirming the version span no longer renders:

```typescript
it("does not render version string in the DOM (version is now in OS window title)", () => {
  const wrapper = mountView();
  expect(wrapper.text()).not.toMatch(/v\d+\.\d+\.\d+/);
});
```

- [ ] **Step 8: Commit**

```bash
git add src/components/MonthView.vue src/__tests__/mocks/tauri.ts src/__tests__/components/MonthView.test.ts
git commit -m "feat: move version number to OS window title via setTitle"
```

---

### Task 3: Right-Click to Copy File Path

**Files:**
- Modify: `src/components/MonthView.vue` (script: add copyFilePath + timer + cleanup; template: add @contextmenu.prevent)

- [ ] **Step 1: Add copyFilePath logic and timer cleanup in `<script setup>`**

In `src/components/MonthView.vue`, add the `copiedFeedback` ref, `copyTimer` variable, and `copyFilePath` function. Place them near the existing file path section (around line 279, right after the `revealDayFile` function):

```typescript
// ---- File path right-click copy ----
const copiedFeedback = ref(false);
let copyTimer: ReturnType<typeof setTimeout> | null = null;
async function copyFilePath(e: MouseEvent) {
  e.preventDefault();
  if (!store.rootPath) return;
  await navigator.clipboard.writeText(store.rootPath + "/" + dayFilePath.value);
  copiedFeedback.value = true;
  if (copyTimer) clearTimeout(copyTimer);
  copyTimer = setTimeout(() => { copiedFeedback.value = false; }, 1500);
}
```

Add the timer cleanup to the existing `onUnmounted` hook (after line 347, before the closing `}`):

In the existing `onUnmounted` block:

```diff
  onUnmounted(() => {
    window.removeEventListener("keydown", onGlobalKeydown);
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
    if (highlightTimer) clearTimeout(highlightTimer);
+   if (copyTimer) clearTimeout(copyTimer);
  });
```

- [ ] **Step 2: Update the file path button in the template**

Replace the existing button (lines 440-444) to add the `@contextmenu.prevent` handler and conditional "Copied!" text:

```diff
         <button
           class="text-micro text-[var(--color-text-disabled)] hover:text-[var(--color-text-secondary)] cursor-pointer"
           :title="store.rootPath + '/' + dayFilePath"
           @click="revealDayFile"
+          @contextmenu.prevent="copyFilePath"
-        >{{ displayPath }}</button>
+        >{{ copiedFeedback ? 'Copied!' : displayPath }}</button>
```

- [ ] **Step 3: Run tests**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook/.claude/worktrees/pure-scribbling-kurzweil && npx vitest run src/__tests__/components/MonthView.test.ts
```

Expected: All existing tests pass.

- [ ] **Step 4: Add a test for right-click copy behavior**

In `src/__tests__/components/MonthView.test.ts`, add to the main `describe("MonthView", ...)` block:

```typescript
it("copies full file path to clipboard on right-click and shows Copied! feedback", async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", {
    value: { writeText },
    writable: true,
  });

  const wrapper = mountView();
  const btn = wrapper.find("button[title*='/']");
  expect(btn.exists()).toBe(true);

  // Before right-click: displays compact path (starts with "…/")
  expect(btn.text()).toMatch(/^…\//);

  await btn.trigger("contextmenu");
  await wrapper.vm.$nextTick();

  // Should copy the full path (no "…/" prefix)
  expect(writeText).toHaveBeenCalledWith(
    expect.stringMatching(/^\/root\/\d{4}\/\d{2}\/\d{4}-\d{2}-\d{2}\.md$/),
  );

  // Button text should change to "Copied!"
  expect(btn.text()).toBe("Copied!");
});
```

- [ ] **Step 5: Run the new test**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook/.claude/worktrees/pure-scribbling-kurzweil && npx vitest run src/__tests__/components/MonthView.test.ts -t "copies full file path"
```

Expected: New test passes.

- [ ] **Step 6: Run full test suite to verify no regressions**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook/.claude/worktrees/pure-scribbling-kurzweil && npx vitest run
```

Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/components/MonthView.vue src/__tests__/components/MonthView.test.ts
git commit -m "feat: right-click file path to copy full path to clipboard"
```

---

### Task 4 (Integration): Build Check

**Files:** None (verification only)

- [ ] **Step 1: Run the full build check**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook/.claude/worktrees/pure-scribbling-kurzweil && npm run build
```

Expected: Build succeeds with no type errors. `vue-tsc` checks test files with `noUnusedLocals`, so any unused imports left behind will surface here.

- [ ] **Step 2: Verify the test suite post-build**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook/.claude/worktrees/pure-scribbling-kurzweil && npm run verify
```

Expected: All checks pass (lint + tests + build).
