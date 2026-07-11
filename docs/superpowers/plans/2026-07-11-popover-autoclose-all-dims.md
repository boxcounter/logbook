# DimensionPopover 自动关闭：所有可填维度填完 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `DimensionPopover` 的 auto-close 条件从「所有 required 维度填完」改为「所有可见且可选值非空的维度填完」，顺带修复 deleted-required / 空值-required 永久阻断关闭的 bug。

**Architecture:** 单文件改动。在 `DimensionPopover.vue` 新增 `hasFillableValues` 辅助函数判定某维度是否有可选值，并改写 `selectVal` 中的 `allFilled` 判断为遍历 `visibleDims`（排除 deleted）且排除无可选值的维度。

**Tech Stack:** Vue 3 `<script setup>` + TypeScript，vitest + @vue/test-utils

## Global Constraints

- 测试命令：`pnpm test`（vitest + jsdom）
- 测试遵循 TDD：先写失败测试，跑红 → 改实现 → 跑绿
- 不触碰父组件（EntryComposer.vue、EntryRowEdit.vue）、Rust 端、配置 schema
- 现有测试全部必须保持通过
- 设计依据：`docs/superpowers/specs/2026-07-11-popover-autoclose-all-dims-design.md`

---

### Task 1: 新增测试用例锁定新语义（先红）

**Files:**
- Modify: `src/__tests__/components/DimensionPopover.test.ts`（在文件末尾、最后一个 `it` 之后、`describe` 闭合 `});` 之前追加 3 个用例）
- Test: 同上

**Interfaces:**
- Consumes: `makeDimension`、`makeCommitment` from `../mocks/fixtures`（文件顶部已 import）；`mount` / `enableAutoUnmount` 已配置
- Produces: 3 个失败测试，驱动 Task 2 的实现

**背景：** DimensionPopover 是受控组件——它 `emit("select")` 后，父组件才会把新 `dimValues` 通过 prop 回传。测试中需在每次选值后 `await wrapper.setProps({ dimValues: ... })` 模拟父组件回传，否则 popover 看不到刚选的值。

**注意：** 文件顶部共享的 `dimensions` 数组（L10-14）含一个 optional `business-line`（有值 `["Slax"]`）。新语义下 optional 未填会阻断 close。因此现有用例 L51「emits close once all required dimensions are filled」会转红——它的 `mountPop({ category: "Engineering" })` 没预填 business-line，选完 goal 后 business-line 仍未填，不再 emit close。这个用例本意是测「必填填完即关」的旧语义，新语义下需更新为预填 optional（见 Step 1）。

- [ ] **Step 1: 更新现有 L51 用例 + 追加 3 个新测试**

先更新 L51 用例，把 optional 维度预填，使其反映新语义（全填完才关）：

将 L51-57 的：

```ts
  it("emits close once all required dimensions are filled after a selection", async () => {
    // category already filled; selecting goal value fills the last required dim
    const wrapper = mountPop({ category: "Engineering" });
    await wrapper.findAll("[data-test='dim-item']")[1].trigger("click"); // Goal
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click"); // Bug fixes
    expect(wrapper.emitted("close")).toBeTruthy();
  });
```

替换为：

```ts
  it("emits close once all fillable dims are filled after a selection", async () => {
    // category + business-line (optional) prefilled; selecting goal fills the last fillable dim
    const wrapper = mountPop({ category: "Engineering", "business-line": "Slax" });
    await wrapper.findAll("[data-test='dim-item']")[1].trigger("click"); // Goal
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click"); // Bug fixes
    expect(wrapper.emitted("close")).toBeTruthy();
  });
```

然后在 `describe("DimensionPopover", () => {` 块内、最后一个 `it(...)` 之后追加（`describe` 闭合 `});` 之前）：

```ts

  // ---- auto-close: all fillable dims filled (not just required) ----

  it("closes after all visible dims (incl. optional) are filled, ignoring deleted-required", async () => {
    const dims = [
      makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering"], required: true }),
      makeDimension({ name: "Mode", key: "mode", source: "static", values: ["Deep Work"], required: true }),
      makeDimension({ name: "Biz", key: "biz", source: "static", values: ["Slax"], required: false }),
      // deleted-required: not visible, must not block close
      makeDimension({ name: "Hidden", key: "hidden", source: "static", values: ["h"], required: true, deleted: true }),
    ];
    const wrapper = mount(DimensionPopover, {
      props: { dimensions: dims, commitments: [], dimValues: {} },
    });

    async function pick(dimIdx: number, valIdx: number, next: Record<string, string>) {
      await wrapper.findAll("[data-test='dim-item']")[dimIdx].trigger("click");
      await wrapper.findAll("[data-test='val-item']")[valIdx].trigger("click");
      await wrapper.setProps({ dimValues: next });
    }

    await pick(0, 0, { category: "Engineering" });
    expect(wrapper.emitted("close")).toBeFalsy();
    await pick(1, 0, { category: "Engineering", mode: "Deep Work" });
    expect(wrapper.emitted("close")).toBeFalsy(); // optional "biz" still unfilled
    await pick(2, 0, { category: "Engineering", mode: "Deep Work", biz: "Slax" });
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("does not close when an optional dim with available values is left unfilled", async () => {
    const dims = [
      makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering"], required: true }),
      makeDimension({ name: "Biz", key: "biz", source: "static", values: ["Slax"], required: false }),
    ];
    const wrapper = mount(DimensionPopover, {
      props: { dimensions: dims, commitments: [], dimValues: {} },
    });
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // category
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click"); // Engineering
    await wrapper.setProps({ dimValues: { category: "Engineering" } });
    expect(wrapper.emitted("close")).toBeFalsy(); // optional "biz" still unfilled -> no close
  });

  it("closes when a required dim with empty values is the only unfilled dim", async () => {
    const dims = [
      makeDimension({ name: "Category", key: "category", source: "static", values: ["Engineering"], required: true }),
      // required but no values to pick -> must not block close
      makeDimension({ name: "Empty", key: "empty", source: "static", values: [], required: true }),
    ];
    const wrapper = mount(DimensionPopover, {
      props: { dimensions: dims, commitments: [], dimValues: {} },
    });
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // category
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click"); // Engineering
    await wrapper.setProps({ dimValues: { category: "Engineering" } });
    expect(wrapper.emitted("close")).toBeTruthy(); // "empty" has no values -> ignored
  });
```

- [ ] **Step 2: 跑测试确认 3 个新用例失败（红）**

```bash
pnpm test -- --run src/__tests__/components/DimensionPopover.test.ts
```

Expected:
- `closes after all visible dims (incl. optional) are filled, ignoring deleted-required` — FAIL（改前 category+mode 两个 required 填完即关，第二个断言 `toBeFalsy()` 不满足）
- `does not close when an optional dim with available values is left unfilled` — FAIL（改前 category required 填完即 emit close）
- `closes when a required dim with empty values is the only unfilled dim` — FAIL（改前 "empty" required 未填，allFilled 永远 false）
- 现有用例全部 PASS

- [ ] **Step 3: Commit（测试先红）**

```bash
git add src/__tests__/components/DimensionPopover.test.ts
git commit -m "test(popover): add red tests for all-fillable-dims auto-close"
```

---

### Task 2: 改写 selectVal 判断（转绿）

**Files:**
- Modify: `src/components/DimensionPopover.vue`（L130-144 `selectVal`；新增 `hasFillableValues` 辅助函数）

**Interfaces:**
- Consumes: `visibleDims` computed（L23）、`goalOptions` computed（L47-51）、`props.commitments`、`props.dimValues`
- Produces: `selectVal` 在「所有可见且可选值非空的维度填完」时 emit close

- [ ] **Step 1: 新增 `hasFillableValues` 辅助函数**

在 `DimensionPopover.vue` 的 `firstUnfilledIndex` 函数之后（约 L33）、`listLength` 之前插入：

```ts
// Whether a dimension has any value the user could pick. Mirrors the value
// sources in `activeValues` (role from commitments, goals from commitments,
// static from d.values) — used only for auto-close judgement, not rendering.
function hasFillableValues(d: Dimension): boolean {
  if (d.source === "commitments:role") {
    return props.commitments.length > 0;
  }
  if (d.source === "commitments:role:goals") {
    return goalOptions.value.length > 0;
  }
  return (d.values ?? []).length > 0;
}
```

- [ ] **Step 2: 改写 `selectVal` 的 auto-close 判断**

将 `selectVal`（L130-144）中的这段：

```ts
  const allFilled = props.dimensions
    .filter(d => d.required)
    .every(d => d.key === justFilledKey || props.dimValues[d.key]);
  if (allFilled) {
```

替换为：

```ts
  const allFillableFilled = visibleDims.value
    .filter(d => d.key !== justFilledKey && !props.dimValues[d.key])
    .every(d => !hasFillableValues(d));
  if (allFillableFilled) {
```

- [ ] **Step 3: 跑测试确认全绿**

```bash
pnpm test -- --run src/__tests__/components/DimensionPopover.test.ts
```

Expected: 全部通过（L51 更新后用例 + 3 个新用例 + 其余现有用例）

- [ ] **Step 4: 跑全量测试确认无回归**

```bash
pnpm test
```

Expected: 全部通过，无失败

- [ ] **Step 5: Commit**

```bash
git add src/components/DimensionPopover.vue
git commit -m "fix(popover): auto-close when all fillable dims are filled

Previously auto-close checked only required dims from props.dimensions
without excluding deleted ones, so a deleted-required dim would silently
block close forever. Now iterate visibleDims and ignore dims with no
available values."
```
