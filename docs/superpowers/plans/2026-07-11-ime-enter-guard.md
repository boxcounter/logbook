# IME Enter Guard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 防止中文（及其他 IME）输入法用 Enter 选词时误触发编辑保存 / entry 提交。

**Architecture:** 在 `EntryRowEdit.vue` 和 `EntryComposer.vue` 的 Enter 处理最前面加 `e.isComposing` 守卫，与项目已有 4 处（useDayNote / DimensionPopover / CommitmentsModal / DimensionEditorModal）保持一致。不引入 compositionstart/end 监听。

**Tech Stack:** Vue 3 + TypeScript + vitest + @vue/test-utils

## Global Constraints

- 守卫写法必须与项目现有 4 处一致：`if (e.isComposing) return;`（spec 明确要求「不发明新机制」）
- 遵循 `docs/interaction-principles.md` 第 4 条第 46 行
- 设计 token 规则不变（本次不涉及间距 / 字号）

**Spec:** `docs/superpowers/specs/2026-07-11-ime-enter-guard-design.md`

**Worktree:** `/Users/boxcounter/Code/Boxcounter/logbook/.worktrees/fix-ime-enter-guard`（分支 `fix-ime-enter-guard`）

---

### Task 1: EntryRowEdit — IME Enter 守卫

**Files:**
- Modify: `src/components/composite/EntryRowEdit.vue`（`onEnter` 函数约第 50 行；两个 `<input>` 的 `@keydown.enter.prevent` 模板绑定约第 171、180 行）
- Test: `src/__tests__/components/composite/EntryRowEdit.test.ts`

**Interfaces:**
- Consumes: 无（独立改动）
- Produces: `onEnter(e: KeyboardEvent)` 新签名（内部使用，不对外暴露）

- [ ] **Step 1: Write the failing test**

在 `src/__tests__/components/composite/EntryRowEdit.test.ts` 的 `describe("EntryRowEdit", ...)` 块末尾（最后一个 `it(...)` 之后、闭合 `});` 之前）加这个测试：

```ts
  it("does NOT save when Enter is pressed during IME composition (e.g. Chinese pinyin)", async () => {
    const wrapper = mountEdit();
    // Simulate the Enter that selects an IME candidate word — isComposing is true.
    const input = wrapper.find("input");
    input.element.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true, isComposing: true }),
    );
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("save")).toBeFalsy();
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm test -- src/__tests__/components/composite/EntryRowEdit.test.ts`
Expected: FAIL — 该测试报错（当前 `onEnter` 不检查 `isComposing`，会调用 `save()`，`emitted("save")` 为 truthy，断言失败）。如果测试因其他原因失败（如 import 错误），先修复再继续。

- [ ] **Step 3: Modify the template — remove `.prevent` from both inputs**

在 `src/components/composite/EntryRowEdit.vue` 模板中，两个 `<input>` 的 Enter 绑定从 `@keydown.enter.prevent="onEnter"` 改为 `@keydown.enter="onEnter"`。

**item input（约第 171 行）**，改动前：
```vue
        @keydown.enter.prevent="onEnter"
```
改动后：
```vue
        @keydown.enter="onEnter"
```

**duration input（约第 180 行）**，同样改动：
```vue
        @keydown.enter.prevent="onEnter"
```
改为：
```vue
        @keydown.enter="onEnter"
```

注意：两处文本完全相同，需分别定位（item input 在 `v-model="item"` 的 input 上，duration input 在 `v-model="durText"` 的 input 上）。用各自 input 的上下文逐个 Edit。

- [ ] **Step 4: Modify `onEnter` — add event param + isComposing guard + preventDefault**

在 `src/components/composite/EntryRowEdit.vue` `<script setup>` 中，改动前（约第 49-53 行）：

```ts
// Enter normally saves; while confirming it means "keep editing".
function onEnter() {
  if (confirming.value) { confirming.value = false; return; }
  save();
}
```

改动后：

```ts
// Enter normally saves; while confirming it means "keep editing".
// Guard against IME composition (e.g. Chinese pinyin candidate selection).
function onEnter(e: KeyboardEvent) {
  if (e.isComposing) return;
  e.preventDefault();
  if (confirming.value) { confirming.value = false; return; }
  save();
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `pnpm test -- src/__tests__/components/composite/EntryRowEdit.test.ts`
Expected: PASS（全部测试，含新加的 IME 测试）。440 + 1 = 441 tests。

- [ ] **Step 6: Commit**

```bash
git add src/components/composite/EntryRowEdit.vue src/__tests__/components/composite/EntryRowEdit.test.ts
git commit -m "fix: guard Enter against IME composition in EntryRowEdit"
```

---

### Task 2: EntryComposer — IME Enter 守卫

**Files:**
- Modify: `src/components/EntryComposer.vue`（`onKeydown` 的 Enter 分支约第 90 行）
- Test: `src/__tests__/components/EntryComposer.test.ts`

**Interfaces:**
- Consumes: 无（独立改动）
- Produces: 无（内部改动）

- [ ] **Step 1: Write the failing test**

在 `src/__tests__/components/EntryComposer.test.ts` 的 `describe("EntryComposer", ...)` 块末尾（最后一个 `it(...)` 之后、闭合 `});` 之前）加这个测试：

```ts
  it("does NOT submit when Enter is pressed during IME composition (e.g. Chinese pinyin)", async () => {
    const wrapper = mountInput();
    await setDims(wrapper, { category: "Engineering", goal: "Bug fixes" });
    const input = wrapper.find("input");
    await input.setValue("Code review 1h");
    // Simulate the Enter that selects an IME candidate word — isComposing is true.
    input.element.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true, isComposing: true }),
    );
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("submit")).toBeFalsy();
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm test -- src/__tests__/components/EntryComposer.test.ts`
Expected: FAIL — 当前 `onKeydown` Enter 分支不检查 `isComposing`，调用 `handleSubmit()`，`emitted("submit")` 为 truthy，断言失败。

- [ ] **Step 3: Add isComposing guard to the Enter branch**

在 `src/components/EntryComposer.vue` 的 `onKeydown` 函数中，改动前（约第 90-95 行）：

```ts
  if (e.key === "Enter") {
    e.preventDefault();
    if (popoverOpen.value) closePopover();
    handleSubmit();
    return;
  }
```

改动后：

```ts
  if (e.key === "Enter") {
    // Guard against IME composition (e.g. Chinese pinyin candidate selection).
    if (e.isComposing) return;
    e.preventDefault();
    if (popoverOpen.value) closePopover();
    handleSubmit();
    return;
  }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm test -- src/__tests__/components/EntryComposer.test.ts`
Expected: PASS（全部测试，含新加的 IME 测试）。

- [ ] **Step 5: Run full test suite**

Run: `pnpm test`
Expected: PASS — 全部 442 tests（440 原有 + 2 新增），0 failures。

- [ ] **Step 6: Commit**

```bash
git add src/components/EntryComposer.vue src/__tests__/components/EntryComposer.test.ts
git commit -m "fix: guard Enter against IME composition in EntryComposer"
```

---

## Self-Review

**1. Spec coverage:**
- spec 改动 1（EntryRowEdit onEnter 签名 + isComposing 守卫 + .prevent 移入函数）→ Task 1 Steps 3-4 ✓
- spec 改动 2（EntryComposer onKeydown Enter 分支 isComposing 守卫）→ Task 2 Step 3 ✓
- spec 测试方案（两个组件各一个 isComposing:true 测试）→ Task 1 Step 1 + Task 2 Step 1 ✓
- spec「不做的事」→ 计划未引入 compositionstart/end，未改 Esc / @，未改其他 4 处 ✓

**2. Placeholder scan:** 无 TBD/TODO，所有代码块完整。

**3. Type consistency:** `onEnter(e: KeyboardEvent)` 签名在 Task 1 Step 4 定义；Task 1 Step 3 模板 `@keydown.enter="onEnter"` Vue 会自动传入原生 `KeyboardEvent`，类型一致。`onKeydown(e: KeyboardEvent)` 在 EntryComposer 已有，Task 2 只加守卫行，签名不变。
