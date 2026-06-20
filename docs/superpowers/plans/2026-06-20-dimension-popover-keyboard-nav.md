# DimensionPopover 键盘导航 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给 `DimensionPopover`（`@` 维度/值菜单）加键盘导航：`CTRL+N/P` + `↑↓` 移动高亮，`Enter` 选中高亮项，默认高亮第一个未填维度。

**Architecture:** 全部逻辑收在 `DimensionPopover.vue` 内部，扩展它现有的 window 级 capture-phase keydown 监听（当前只处理 Esc）。两个父组件（`TwoLineInput`、`EntryRowEdit`）无需改动即同时获得能力。`Enter` 用 `stopPropagation` 抢在父组件之前处理，实现「popover 开着时 Enter 选中高亮项而非提交」。

**Tech Stack:** Vue 3 `<script setup>` + TypeScript；Vitest + @vue/test-utils。

**Spec:** `docs/superpowers/specs/2026-06-20-dimension-popover-keyboard-nav-design.md`

---

## File Structure

- Modify: `src/components/DimensionPopover.vue` — 新增 `activeIndex` 状态、默认高亮、导航/Enter 键处理、模板高亮渲染、footer 提示。
- Modify: `src/__tests__/components/DimensionPopover.test.ts` — 新增键盘导航测试。
- Modify: `SPEC.md` — §5.2 同步「popover 开启时 Enter 选中高亮项」。

所有交互逻辑单文件内聚，无需新增文件或 composable。

---

### Task 1: 高亮状态 + 默认高亮第一个未填维度 + 模板渲染

**Files:**
- Modify: `src/components/DimensionPopover.vue`
- Test: `src/__tests__/components/DimensionPopover.test.ts`

- [ ] **Step 1: Write the failing tests**

在 `src/__tests__/components/DimensionPopover.test.ts` 的 `describe` 块内末尾追加。先在文件顶部 import 区确认已有 `nextTick` 可用——本测试用 `wrapper.vm.$nextTick()` 即可，无需额外 import。

```ts
  // ---- keyboard navigation ----

  // Returns the index of the currently highlighted dim-item (-1 if none).
  function activeDimIndex(wrapper: ReturnType<typeof mountPop>): number {
    return wrapper.findAll("[data-test='dim-item']").findIndex(
      (n) => n.attributes("data-active") === "true"
    );
  }

  it("highlights the first unfilled dimension on open", async () => {
    // category filled → first unfilled is Goal (index 1)
    const wrapper = mountPop({ category: "Engineering" });
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(1);
  });

  it("highlights index 0 when no dimension is filled", async () => {
    const wrapper = mountPop();
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(0);
  });

  it("syncs highlight to a dim-item on mouseenter", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[2].trigger("mouseenter");
    expect(activeDimIndex(wrapper)).toBe(2);
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- DimensionPopover`
Expected: 3 new tests FAIL（`data-active` 属性不存在 → `activeDimIndex` 返回 -1；mouseenter 无效）。

- [ ] **Step 3: Add highlight state + default-highlight helper**

在 `src/components/DimensionPopover.vue` 的 `<script setup>` 中，`const activeDimKey = ref<string | null>(null);`（第 18 行）之后新增：

```ts
const activeIndex = ref(0);

// First dimension still missing a value. `justFilled` lets callers treat a
// key as filled before props.dimValues reflects the just-emitted select.
function firstUnfilledIndex(justFilled?: string): number {
  const idx = props.dimensions.findIndex(
    (d) => d.key !== justFilled && !props.dimValues[d.key]
  );
  return idx === -1 ? 0 : idx;
}
```

在现有 `onMounted(...)`（第 80 行）里，把单行改为同时初始化高亮：

```ts
onMounted(() => {
  activeIndex.value = firstUnfilledIndex();
  window.addEventListener("keydown", onWindowKeydown, true);
});
```

- [ ] **Step 4: Render highlight markers + mouseenter sync in dim phase**

把 dim 阶段的 `v-for`（第 100 行起的那个 `<div v-for="d in dimensions" ...>`）改为带索引，并加 `data-active`、`@mouseenter`、ring 高亮类。将该元素替换为：

```html
      <div
        v-for="(d, i) in dimensions" :key="d.key"
        data-test="dim-item"
        :data-active="i === activeIndex"
        class="px-[14px] py-[9px] text-[length:var(--app-text-sm)]
               flex items-center gap-[10px] cursor-pointer border-b border-[var(--color-surface-muted)]
               last:border-b-0 hover:bg-[var(--color-divider)]"
        :class="[
          dimValues[d.key] ? 'bg-[var(--color-popover-item-selected-bg)] text-[var(--color-brand-solid)] font-semibold' : 'text-[var(--color-text-primary)]',
          i === activeIndex ? 'ring-1 ring-inset ring-[var(--color-brand-solid)]' : '',
        ]"
        @mouseenter="activeIndex = i"
        @click="selectDim(d.key)"
      >
        <span class="w-[3px] h-[18px] rounded-[var(--radius-sm)] flex-shrink-0" :class="barClass(d.key)"></span>
        {{ d.name }}
        <span
          class="ml-auto text-[length:var(--app-text-micro)]"
          :class="d.required ? 'text-[var(--color-warning)] font-medium' : 'text-[var(--color-text-disabled)]'"
        >{{ d.required ? "required" : "optional" }}</span>
      </div>
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `npm test -- DimensionPopover`
Expected: 全部 PASS（含原有点击/Esc 测试不回归）。

- [ ] **Step 6: Commit**

```bash
git add src/components/DimensionPopover.vue src/__tests__/components/DimensionPopover.test.ts
git commit -m "feat(popover): 高亮状态 + 默认高亮第一个未填维度"
```

---

### Task 2: CTRL+N/P 与 ↑↓ 移动高亮（dim 阶段，循环）

**Files:**
- Modify: `src/components/DimensionPopover.vue`
- Test: `src/__tests__/components/DimensionPopover.test.ts`

- [ ] **Step 1: Write the failing tests**

追加到 `describe` 块内（复用 Task 1 的 `activeDimIndex` 辅助函数）：

```ts
  it("ArrowDown / Ctrl+N move highlight down with wrap", async () => {
    const wrapper = mountPop(); // active index 0, 3 dims
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(1);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "n", ctrlKey: true }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(2);

    // wrap 2 -> 0
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(0);
  });

  it("ArrowUp / Ctrl+P move highlight up with wrap", async () => {
    const wrapper = mountPop(); // active index 0
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(2); // wrap 0 -> 2

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "p", ctrlKey: true }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(1);
  });

  it("prevents default on navigation keys", async () => {
    const wrapper = mountPop();
    const ev = new KeyboardEvent("keydown", { key: "ArrowDown", cancelable: true });
    window.dispatchEvent(ev);
    await wrapper.vm.$nextTick();
    expect(ev.defaultPrevented).toBe(true);
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- DimensionPopover`
Expected: 3 新测试 FAIL（导航键未处理，高亮不动）。

- [ ] **Step 3: Add list-length helper + move() + key handling**

在 `firstUnfilledIndex` 之后新增 helper：

```ts
function listLength(): number {
  return phase.value === "dim" ? props.dimensions.length : activeValues.value.length;
}

function move(delta: number) {
  const n = listLength();
  if (!n) return;
  activeIndex.value = (activeIndex.value + delta + n) % n;
}
```

把现有 `onWindowKeydown`（当前只处理 Escape）整体替换为：

```ts
// Window-level capture-phase handler (spec §5.1/§5.2 + keyboard nav design):
// Esc — val→dim / dim→close. Arrows / Ctrl+N/P move the highlight. Enter selects
// the highlighted item (added in later tasks). capture + stopPropagation makes the
// popover own these keys regardless of focus, ahead of the parent's handlers.
function onWindowKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    e.preventDefault();
    e.stopPropagation();
    if (phase.value === "val") goBack();
    else emit("close");
    return;
  }
  const down = e.key === "ArrowDown" || (e.ctrlKey && (e.key === "n" || e.key === "N"));
  const up = e.key === "ArrowUp" || (e.ctrlKey && (e.key === "p" || e.key === "P"));
  if (down) { e.preventDefault(); e.stopPropagation(); move(1); return; }
  if (up) { e.preventDefault(); e.stopPropagation(); move(-1); return; }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- DimensionPopover`
Expected: 全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/components/DimensionPopover.vue src/__tests__/components/DimensionPopover.test.ts
git commit -m "feat(popover): CTRL+N/P 与方向键移动高亮（循环）"
```

---

### Task 3: Enter 在 dim 阶段选中 + val 阶段默认高亮已选值

**Files:**
- Modify: `src/components/DimensionPopover.vue`
- Test: `src/__tests__/components/DimensionPopover.test.ts`

- [ ] **Step 1: Write the failing tests**

追加到 `describe` 块内。新增 val 阶段高亮辅助：

```ts
  function activeValIndex(wrapper: ReturnType<typeof mountPop>): number {
    return wrapper.findAll("[data-test='val-item']").findIndex(
      (n) => n.attributes("data-active") === "true"
    );
  }

  it("Enter in dim phase enters the highlighted dimension's value menu", async () => {
    const wrapper = mountPop(); // highlight on Category (index 0)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.find("[data-test='back-btn']").exists()).toBe(true); // val phase
    expect(wrapper.text()).toContain("Engineering");
  });

  it("val phase highlights the already-selected value", async () => {
    const wrapper = mountPop({ category: "PM" }); // values: Engineering, PM
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // → category val phase
    await wrapper.vm.$nextTick();
    expect(activeValIndex(wrapper)).toBe(1); // "PM"
  });

  it("val phase highlights index 0 when no value selected yet", async () => {
    const wrapper = mountPop();
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click");
    await wrapper.vm.$nextTick();
    expect(activeValIndex(wrapper)).toBe(0);
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- DimensionPopover`
Expected: FAIL（Enter 未选中；val-item 无 `data-active`）。

- [ ] **Step 3: Add defaultValIndex + Enter branch + selectDim reset + val-item markers**

在 helper 区新增：

```ts
function defaultValIndex(): number {
  const cur = activeDimKey.value ? props.dimValues[activeDimKey.value] : undefined;
  const i = cur ? activeValues.value.indexOf(cur) : -1;
  return i >= 0 ? i : 0;
}
```

修改 `selectDim`，进入 val 阶段时重算高亮：

```ts
function selectDim(key: string) {
  activeDimKey.value = key;
  phase.value = "val";
  activeIndex.value = defaultValIndex();
}
```

在 `onWindowKeydown` 的 `up` 分支之后、函数结束前，新增 Enter 分支：

```ts
  if (e.key === "Enter") {
    e.preventDefault();
    e.stopPropagation();
    if (phase.value === "dim") {
      const d = props.dimensions[activeIndex.value];
      if (d) selectDim(d.key);
    } else {
      const v = activeValues.value[activeIndex.value];
      if (v !== undefined) selectVal(v);
    }
    return;
  }
```

把 val 阶段的 `v-for`（`<div v-for="v in activeValues" ...>`）替换为带索引 + 高亮：

```html
      <div
        v-for="(v, i) in activeValues" :key="v"
        data-test="val-item"
        :data-active="i === activeIndex"
        class="px-[14px] py-[9px] text-[length:var(--app-text-sm)]
               cursor-pointer border-b border-[var(--color-surface-muted)] last:border-b-0
               hover:bg-[var(--color-divider)]"
        :class="[
          activeDimKey && dimValues[activeDimKey] === v ? 'bg-[var(--color-popover-item-selected-bg)] text-[var(--color-brand-solid)] font-semibold' : 'text-[var(--color-text-primary)]',
          i === activeIndex ? 'ring-1 ring-inset ring-[var(--color-brand-solid)]' : '',
        ]"
        @mouseenter="activeIndex = i"
        @click="selectVal(v)"
      >{{ v }}</div>
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- DimensionPopover`
Expected: 全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/components/DimensionPopover.vue src/__tests__/components/DimensionPopover.test.ts
git commit -m "feat(popover): Enter 选中高亮维度 + val 阶段默认高亮已选值"
```

---

### Task 4: Enter 在 val 阶段选值 + 返回 dim 高亮下一个未填 + goBack/footer

**Files:**
- Modify: `src/components/DimensionPopover.vue`
- Test: `src/__tests__/components/DimensionPopover.test.ts`

- [ ] **Step 1: Write the failing tests**

追加到 `describe` 块内：

```ts
  it("Enter in val phase emits select for the highlighted value and prevents default", async () => {
    const wrapper = mountPop({ category: "PM" }); // not all required filled (goal missing)
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // category val, active = PM(1)
    const ev = new KeyboardEvent("keydown", { key: "Enter", cancelable: true });
    window.dispatchEvent(ev);
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("select")?.[0]).toEqual(["category", "PM"]);
    expect(ev.defaultPrevented).toBe(true);
  });

  it("returning to dim phase after a select highlights the next unfilled dimension", async () => {
    // category preset; pick a category value → returns to dim (goal still missing)
    const wrapper = mountPop({ category: "PM" });
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // category val
    await wrapper.findAll("[data-test='val-item']")[0].trigger("click"); // pick Engineering → back to dim
    await wrapper.vm.$nextTick();
    // category now filled (just selected) → next unfilled is Goal (index 1)
    expect(activeDimIndex(wrapper)).toBe(1);
  });

  it("Esc back from val to dim re-highlights the first unfilled dimension", async () => {
    const wrapper = mountPop({ category: "Engineering" });
    await wrapper.findAll("[data-test='dim-item']")[1].trigger("click"); // Goal val phase
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(activeDimIndex(wrapper)).toBe(1); // Goal still unfilled
  });

  it("shows the ⌃N/⌃P move hint in the footer", () => {
    const wrapper = mountPop();
    expect(wrapper.text()).toContain("move");
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- DimensionPopover`
Expected: FAIL（返回 dim 后高亮未重算、footer 无 "move"；select-emit 测试可能已通过，其余 FAIL）。

- [ ] **Step 3: Reset highlight on goBack + selectVal return branch + footer hints**

修改 `goBack`：

```ts
function goBack() {
  phase.value = "dim";
  activeDimKey.value = null;
  activeIndex.value = firstUnfilledIndex();
}
```

修改 `selectVal` 的「未全部填完」分支（`else` 块），返回 dim 时把刚填的 key 视作已填来定位下一个未填项：

```ts
function selectVal(value: string) {
  if (!activeDimKey.value) return;
  const justFilledKey = activeDimKey.value;
  emit("select", justFilledKey, value);
  const allFilled = props.dimensions
    .filter(d => d.required)
    .every(d => d.key === justFilledKey || props.dimValues[d.key]);
  if (allFilled) {
    emit("close");
  } else {
    phase.value = "dim";
    activeDimKey.value = null;
    activeIndex.value = firstUnfilledIndex(justFilledKey);
  }
}
```

在 dim 阶段 footer（含 `↵ select` / `esc close` 的那段 `<div>`）内，于 `esc` 提示之前插入 move 提示：

```html
        <span><kbd class="mono px-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">⌃N/⌃P</kbd> move</span>
```

在 val 阶段 footer（含 `↵ select` / `esc back to dims`）内，同样于 `esc` 提示之前插入：

```html
        <span><kbd class="mono px-[4px] border border-[var(--color-border-form)] rounded-[var(--radius-sm)] bg-[var(--color-surface)]">⌃N/⌃P</kbd> move</span>
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- DimensionPopover`
Expected: 全部 PASS。

- [ ] **Step 5: Run full suite to check no regression in parents**

Run: `npm test`
Expected: 全绿（`TwoLineInput` / `EntryRowEdit` / `MonthView` 等既有测试不回归）。

- [ ] **Step 6: Commit**

```bash
git add src/components/DimensionPopover.vue src/__tests__/components/DimensionPopover.test.ts
git commit -m "feat(popover): Enter 选值 + 返回 dim 高亮下一未填 + footer 提示"
```

---

### Task 5: SPEC.md 同步 + 一致性检查

**Files:**
- Modify: `SPEC.md`

- [ ] **Step 1: 检查 SPEC.md 中 §5.2 / Enter 相关描述**

Run: `grep -n "Enter" SPEC.md`
若 §5.2 不在 `SPEC.md`（spec 编号源自 Vault 设计文档），则在 `SPEC.md` 的「特殊处理」或前端交互相关段落补一句说明即可。

- [ ] **Step 2: 补注 Enter 行为**

在 `SPEC.md` 描述录入/Enter 行为处，补一行：

```markdown
- DimensionPopover 开启时，Enter 改为「选中当前高亮项」（dim→进入值菜单 / val→填值），不提交 entry；关闭 popover（Esc）后 Enter 恢复提交。键盘导航：CTRL+N/P 或 ↑↓ 移动高亮，默认高亮第一个未填维度。
```

若无合适锚点，则放在前端架构「特殊处理」列表末尾。

- [ ] **Step 3: 跑一致性检查**

按项目规则调用 `/check-consistency` skill，确认 SPEC.md ↔ 代码一致。

- [ ] **Step 4: Commit**

```bash
git add SPEC.md
git commit -m "docs(spec): 同步 DimensionPopover Enter/键盘导航行为"
```

---

## 手动验证（实现完成后）

`npm run tauri dev` 启动，在录入框：

1. 打字后敲 `@` → popover 弹出，第一个未填维度高亮。
2. `CTRL+N`/`CTRL+P`/`↑`/`↓` 移动高亮，到端循环。
3. `Enter` 进入该维度值菜单，已选值默认高亮。
4. 值菜单里 `CTRL+N/P` 移动，`Enter` 填值 → 返回 dim，下一个未填维度高亮。
5. 全部必填填完后选值 → popover 关闭。
6. popover 开着时 `Enter` 不再秒提交 entry；`Esc` 关闭后 `Enter` 正常提交。
7. 在某条 entry 的行内编辑里点 `+ tag` 打开 popover，同样的键盘行为生效。

---

## Self-Review

- **Spec coverage:** Enter=选中高亮（Task 3/4）；CTRL+N/P+↑↓（Task 2）；默认高亮第一个未填（Task 1）+ val 默认高亮已选值（Task 3）+ 返回 dim 高亮下一未填（Task 4）；鼠标/键盘统一 mouseenter（Task 1/3）；视觉 ring（Task 1/3）；footer 提示（Task 4）；两父组件自动生效（无需改动，手动验证第 7 步）；测试计划全覆盖；SPEC 同步（Task 5）。无遗漏。
- **Placeholder scan:** 无 TBD/TODO；每个代码步骤含完整代码。
- **Type consistency:** `firstUnfilledIndex(justFilled?)` / `defaultValIndex()` / `listLength()` / `move(delta)` 命名贯穿一致；`activeIndex` 单一来源；`onWindowKeydown` 单处定义、逐任务扩展。
