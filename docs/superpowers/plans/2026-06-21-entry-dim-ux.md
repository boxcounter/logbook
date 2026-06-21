# Entry 输入框 & dimension 菜单交互改进 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 移除新建 entry 输入框对上次 dimension 的预填，并让 dimension 菜单中「光标(active)」与「已选(selected)」在视觉上不再混淆。

**Architecture:** 两个独立改动。Issue 1 删除 `store.lastDimensions` 这条 dead state 及 `TwoLineInput` 的 `initialValues` prop，使输入框始终从空白开始。Issue 2 在 `DimensionPopover` 里把「背景填充」收敛为只表示光标（实心品牌色 + 白字），已选项改用品牌色文字 + `✓`，并删除两个不再使用的 token。

**Tech Stack:** Tauri 2.x + Vue 3 + TypeScript + Tailwind CSS v4.3 + Vitest + @vue/test-utils。

参考 spec：`docs/superpowers/specs/2026-06-21-entry-dim-ux-design.md`

---

## File Structure

修改（无新建文件）：

- `src/stores/useStore.ts` — 删除 `lastDimensions` 字段及初始化。
- `src/components/MonthView.vue` — 删除提交后写入 `lastDimensions`，`<TwoLineInput>` 不再传 `:initial-values`。
- `src/components/TwoLineInput.vue` — 删除 `initialValues` prop、相关 watch，简化 `dimValues` 初值与 Esc 清空逻辑。
- `src/components/DimensionPopover.vue` — dim/val 两处 `:class` 与右侧渲染改为方案 A 三态。
- `src/assets/tokens.css` — 删除 `--color-popover-item-active-bg` / `--color-popover-item-selected-bg`（light + dark 共 4 行）。
- `src/__tests__/useStore.test.ts` — 改断言。
- `src/__tests__/components/TwoLineInput.test.ts` — 去掉 `initialValues` prop，新增 `setDims` 辅助函数并改写依赖预填的用例。
- `src/__tests__/components/MonthView.test.ts` — mock store 去掉 `lastDimensions`。
- `src/__tests__/components/DimensionPopover.test.ts` — 改写 4 个高亮样式用例为方案 A。

执行环境：当前已在 worktree `.claude/worktrees/entry-dim-ux`（分支 `worktree-entry-dim-ux`）。所有命令在该 worktree 根目录执行。

测试命令约定：单文件 `npx vitest run <path>`；全量 `npm test`；类型 + 构建 `npm run build`（会用 `vue-tsc` 严格 typecheck 测试文件，故测试也必须类型正确）。

---

## Task 1: 删除 dimension 预填（Issue 1）

### 1a. 移除 `store.lastDimensions`

**Files:**
- Test: `src/__tests__/useStore.test.ts:13`
- Modify: `src/stores/useStore.ts:17,37`

- [ ] **Step 1: 改测试，断言字段已不存在**

把 `src/__tests__/useStore.test.ts` 第 13 行：

```ts
    expect(store.lastDimensions).toEqual({});
```

替换为：

```ts
    expect("lastDimensions" in store).toBe(false);
```

- [ ] **Step 2: 运行，确认失败**

Run: `npx vitest run src/__tests__/useStore.test.ts`
Expected: FAIL — `defaults` 用例报 `expected true to be false`（字段仍存在）。

- [ ] **Step 3: 从 store 删除该字段**

`src/stores/useStore.ts`：删除接口里的第 17 行

```ts
  lastDimensions: Record<string, string>;
```

并删除 `createStore()` 返回对象里的第 37 行

```ts
    lastDimensions: {},
```

- [ ] **Step 4: 运行，确认通过**

Run: `npx vitest run src/__tests__/useStore.test.ts`
Expected: PASS（2 个用例全绿）。

### 1b. `TwoLineInput` 移除 `initialValues` prop

**Files:**
- Test: `src/__tests__/components/TwoLineInput.test.ts`
- Modify: `src/components/TwoLineInput.vue:8-30,85-94`

- [ ] **Step 5: 改测试 —— 去掉 prop、加 `setDims` 辅助、改写依赖预填的用例**

在 `src/__tests__/components/TwoLineInput.test.ts` 中：

(1) 把 `mountInput` 与新增辅助函数（第 17-19 行）替换为：

```ts
function mountInput() {
  return mount(TwoLineInput, { props: { dimensions, commitments } });
}

// Drive dimension selections through the popover (replaces the removed
// initialValues prop), then close it so Esc/Enter return to the input.
async function setDims(
  wrapper: ReturnType<typeof mountInput>,
  dims: Record<string, string>,
) {
  await wrapper.find("input").trigger("keydown", { key: "@" });
  const pop = wrapper.findComponent({ name: "DimensionPopover" });
  for (const [k, v] of Object.entries(dims)) await pop.vm.$emit("select", k, v);
  await pop.vm.$emit("close");
  await wrapper.vm.$nextTick();
}
```

(2) 「emits submit … (all required filled)」用例（原第 50-55 行）替换为：

```ts
  it("emits submit with item, minutes, and dimensions on Enter (all required filled)", async () => {
    const wrapper = mountInput();
    await setDims(wrapper, { category: "Engineering", goal: "Bug fixes" });
    await wrapper.find("input").setValue("Code review 1h");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")?.[0]).toEqual(["Code review", 60, { category: "Engineering", goal: "Bug fixes" }]);
  });
```

(3) 「does NOT emit submit when there is no parseable duration」用例（原第 57-63 行）把 `mountInput({ category: "Engineering" })` 改为 `mountInput()`（预填与本用例无关）：

```ts
  it("does NOT emit submit when there is no parseable duration", async () => {
    const wrapper = mountInput();
    await wrapper.find("input").setValue("Code review");
    await wrapper.find("input").trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("submit")).toBeFalsy();
    expect(wrapper.text()).toContain("Need a duration");
  });
```

(4) 「Enter while the popover is open selects the highlight」用例（原第 89-106 行）把挂载改为不带 prop：

```ts
  it("Enter while the popover is open selects the highlight instead of submitting", async () => {
    const wrapper = mount(TwoLineInput, {
      props: { dimensions, commitments },
      attachTo: document.body,
    });
    const input = wrapper.find("input");
    await input.setValue("Code review 1h");
    await input.trigger("keydown", { key: "@" }); // open popover (handled by the input's own onKeydown)
    expect(wrapper.findComponent({ name: "DimensionPopover" }).exists()).toBe(true);

    // Real bubbling Enter so the popover's window capture-phase listener intercepts it.
    input.element.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    await wrapper.vm.$nextTick();

    // Popover owns Enter: it entered the highlighted dimension's value menu; the entry is NOT submitted.
    expect(wrapper.emitted("submit")).toBeFalsy();
    expect(wrapper.find("[data-test='back-btn']").exists()).toBe(true); // popover advanced to val phase
  });
```

(5) 「removes a dimension token when its × is clicked」用例（原第 118-123 行）改为用 `setDims` 建立 token：

```ts
  it("removes a dimension token when its × is clicked", async () => {
    const wrapper = mountInput();
    await setDims(wrapper, { category: "Engineering" });
    expect(wrapper.find("[data-test='dim-token']").exists()).toBe(true);
    await wrapper.find("[data-test='dim-token-remove']").trigger("click");
    expect(wrapper.find("[data-test='dim-token']").exists()).toBe(false);
  });
```

(6) 其余仍写 `initialValues: {}` 或 `initialValues: ...` 的挂载点，删除该 prop：
- 第 90-92 行已在 (4) 处理。
- 「focuses the input on a focus request…」（原第 137-141 行）、「does not steal focus…」（原第 155-159 行）、「exposes focusInput()」（原第 170-173 行）、两个独立 Esc 用例（原第 180-182、208-210 行）：把它们 props 里的 `initialValues: {}` 整项删掉。例如：

```ts
    const wrapper = mount(TwoLineInput, {
      props: { dimensions: [], commitments: [] },
    });
```

（「Esc clears a selected dimension token even with no text」用例原第 190-205 行已不依赖 prop，保持不变，但其 `mountInput({})` 调用要改为 `mountInput()`。）

- [ ] **Step 6: 运行测试，确认失败**

Run: `npx vitest run src/__tests__/components/TwoLineInput.test.ts`
Expected: FAIL —— 预填用例失败，因为源码仍用 `props.initialValues` 初始化而测试不再传值（dimValues 初值与断言不符）。

- [ ] **Step 7: 改源码 —— 删除 prop / watch，简化初值与 Esc**

`src/components/TwoLineInput.vue`：

(1) props 定义（第 8-12 行）删除 `initialValues`：

```ts
const props = defineProps<{
  dimensions: Dimension[];
  commitments: Commitment[];
}>();
```

(2) `dimValues` 初值（第 21 行）改为空对象：

```ts
const dimValues = ref<Record<string, string>>({});
```

(3) 删除整个 `watch(() => props.initialValues, ...)` 块（第 26-30 行）。

(4) Esc 清空逻辑（第 85-94 行）改为不再引用 `props.initialValues`：

```ts
  if (e.key === "Escape") {
    if (popoverOpen.value) return;
    const hasContent =
      text.value.trim() !== "" || Object.keys(dimValues.value).length > 0;
    if (!hasContent) return;
    e.preventDefault();
    text.value = "";
    dimValues.value = {};
    submitAttempted.value = false;
    return;
  }
```

注：`props` 变量仍被 `dimensions` / `commitments` 使用，保留 `const props = defineProps...` 不变（只删 `initialValues` 一项）。

- [ ] **Step 8: 运行测试，确认通过**

Run: `npx vitest run src/__tests__/components/TwoLineInput.test.ts`
Expected: PASS（全部用例绿）。

### 1c. `MonthView` 解除对 `lastDimensions` 的引用

**Files:**
- Modify: `src/components/MonthView.vue:123,374`
- Test: `src/__tests__/components/MonthView.test.ts:29`

- [ ] **Step 9: 改 MonthView 测试的 mock store**

`src/__tests__/components/MonthView.test.ts` 第 29 行删除：

```ts
    lastDimensions: {},
```

- [ ] **Step 10: 改 MonthView 源码**

`src/components/MonthView.vue`：

(1) 删除 `handleSubmit` 内第 123 行：

```ts
    store.lastDimensions = { ...finalDimensions };
```

(2) `<TwoLineInput>`（第 370-376 行）删除 `:initial-values` 那一行，变为：

```html
        <TwoLineInput
          ref="inputRef"
          :dimensions="store.config?.dimensions || []"
          :commitments="store.commitments"
          @submit="handleSubmit"
        />
```

- [ ] **Step 11: 全量测试 + 构建**

Run: `npm test`
Expected: PASS（全绿）。

Run: `npm run build`
Expected: 成功，无 `vue-tsc` 类型错误（确认无残留 `lastDimensions` / `initialValues` 引用）。

- [ ] **Step 12: 提交**

```bash
git add src/stores/useStore.ts src/components/MonthView.vue src/components/TwoLineInput.vue \
  src/__tests__/useStore.test.ts src/__tests__/components/TwoLineInput.test.ts src/__tests__/components/MonthView.test.ts
git commit -m "feat(entry): 新建输入框不再预填上次 dimension，删除 lastDimensions"
```

---

## Task 2: dimension 菜单实心光标高亮（Issue 2 / 方案 A）

光标态（含 hover）= 实心 `--color-brand-solid` 填充 + 白字；已选态 = 品牌色文字 + `font-semibold` + `✓`，无背景填充；普通态 = `--color-text-primary`。光标态优先于已选态。dim 阶段已选项右侧由 required/optional 改显「值 + ✓」（值过长截断）。

### 2a. 改写高亮样式测试（红）

**Files:**
- Test: `src/__tests__/components/DimensionPopover.test.ts:207-247`

- [ ] **Step 1: 用方案 A 期望替换 4 个样式用例**

把 `src/__tests__/components/DimensionPopover.test.ts` 中从注释 `// ---- highlight style (fill, not ring) ----`（第 207 行）到文件末尾 `});` 之前的 4 个 `it(...)` 块（第 209-247 行）整段替换为：

```ts
  it("highlights the active item with a solid brand fill and white text", () => {
    const wrapper = mountPop(); // active = index 0 (Category)
    const item = wrapper.findAll("[data-test='dim-item']")[0];
    expect(item.classes()).toContain("bg-[var(--color-brand-solid)]");
    expect(item.classes()).toContain("text-white");
    expect(item.classes()).not.toContain("ring-1");
  });

  it("a filled, non-active dimension uses brand text + ✓ with no background fill", async () => {
    // category filled → default active is Goal (index 1); Category (0) is filled & not active
    const wrapper = mountPop({ category: "Engineering" });
    await wrapper.vm.$nextTick();
    const cat = wrapper.findAll("[data-test='dim-item']")[0];
    expect(cat.classes()).toContain("text-[var(--color-brand-solid)]");
    expect(cat.classes()).toContain("font-semibold");
    expect(cat.classes()).not.toContain("bg-[var(--color-brand-solid)]");
    expect(cat.text()).toContain("Engineering"); // value shown on the right
    expect(cat.text()).toContain("✓");
  });

  it("cursor on a filled item shows the solid fill and white text (cursor wins over selected)", async () => {
    const wrapper = mountPop({ category: "Engineering" }); // active = Goal(1)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp" })); // 1 -> 0 (Category)
    await wrapper.vm.$nextTick();
    const cat = wrapper.findAll("[data-test='dim-item']")[0];
    expect(cat.classes()).toContain("bg-[var(--color-brand-solid)]");
    expect(cat.classes()).toContain("text-white");
    expect(cat.classes()).not.toContain("text-[var(--color-brand-solid)]");
  });

  it("val phase: active value uses the solid fill; the selected value uses brand text + ✓", async () => {
    const wrapper = mountPop({ category: "Engineering" }); // category values: Engineering, PM
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // enter Category val; active = Engineering(0)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" })); // 0 -> 1 (PM)
    await wrapper.vm.$nextTick();
    const vals = wrapper.findAll("[data-test='val-item']");
    expect(vals[1].classes()).toContain("bg-[var(--color-brand-solid)]"); // PM active
    expect(vals[1].classes()).toContain("text-white");
    expect(vals[0].classes()).toContain("text-[var(--color-brand-solid)]"); // Engineering selected, not active
    expect(vals[0].classes()).not.toContain("bg-[var(--color-brand-solid)]");
    expect(vals[0].text()).toContain("✓");
  });
```

- [ ] **Step 2: 运行，确认失败**

Run: `npx vitest run src/__tests__/components/DimensionPopover.test.ts`
Expected: FAIL —— 这 4 个用例红（当前实现仍输出 `bg-[var(--color-popover-item-active-bg)]` / `...-selected-bg`，且无 `✓`）。其余导航用例仍应绿（它们依赖 `data-active`，不受样式改动影响）。

### 2b. 实现 dim 阶段模板

**Files:**
- Modify: `src/components/DimensionPopover.vue:151-173`

- [ ] **Step 3: 改 dim-item 的 `:class` 与右侧渲染**

把 dim 阶段的 `v-for` 块（第 151-173 行）替换为：

```html
      <div
        v-for="(d, i) in dimensions" :key="d.key"
        data-test="dim-item"
        :data-active="i === activeIndex"
        class="px-[14px] py-[9px] text-[length:var(--app-text-sm)]
               flex items-center gap-[10px] cursor-pointer border-b border-[var(--color-surface-muted)]
               last:border-b-0"
        :class="
          i === activeIndex
            ? 'bg-[var(--color-brand-solid)] text-white'
            : (dimValues[d.key]
                ? 'text-[var(--color-brand-solid)] font-semibold'
                : 'text-[var(--color-text-primary)]')
        "
        @mouseenter="activeIndex = i"
        @click="selectDim(d.key)"
      >
        <span class="w-[3px] h-[18px] rounded-[var(--radius-sm)] flex-shrink-0" :class="barClass(d.key)"></span>
        {{ d.name }}
        <span
          v-if="dimValues[d.key]"
          class="ml-auto flex items-center gap-[4px] text-[length:var(--app-text-micro)] max-w-[110px]"
        >
          <span class="truncate">{{ dimValues[d.key] }}</span>
          <span class="flex-shrink-0">✓</span>
        </span>
        <span
          v-else
          class="ml-auto text-[length:var(--app-text-micro)]"
          :class="i === activeIndex
            ? 'text-white/80'
            : (d.required ? 'text-[var(--color-warning)] font-medium' : 'text-[var(--color-text-disabled)]')"
        >{{ d.required ? "required" : "optional" }}</span>
      </div>
```

### 2c. 实现 val 阶段模板

**Files:**
- Modify: `src/components/DimensionPopover.vue:194-208`

- [ ] **Step 4: 改 val-item 的 `:class` 与 `✓` 渲染**

把 val 阶段的 `v-for` 块（第 194-208 行）替换为：

```html
      <div
        v-for="(v, i) in activeValues" :key="v"
        data-test="val-item"
        :data-active="i === activeIndex"
        class="px-[14px] py-[9px] text-[length:var(--app-text-sm)]
               flex items-center cursor-pointer border-b border-[var(--color-surface-muted)] last:border-b-0"
        :class="
          i === activeIndex
            ? 'bg-[var(--color-brand-solid)] text-white'
            : (activeDimKey && dimValues[activeDimKey] === v
                ? 'text-[var(--color-brand-solid)] font-semibold'
                : 'text-[var(--color-text-primary)]')
        "
        @mouseenter="activeIndex = i"
        @click="selectVal(v)"
      >
        <span class="truncate">{{ v }}</span>
        <span v-if="activeDimKey && dimValues[activeDimKey] === v" class="ml-auto flex-shrink-0">✓</span>
      </div>
```

- [ ] **Step 5: 运行 DimensionPopover 测试，确认通过**

Run: `npx vitest run src/__tests__/components/DimensionPopover.test.ts`
Expected: PASS（含改写后的 4 个样式用例与全部导航用例）。

### 2d. 删除不再使用的 token

**Files:**
- Modify: `src/assets/tokens.css:85-86,165-166`

- [ ] **Step 6: 确认无引用后删除 4 行**

先确认源码已无引用（仅测试历史除外）：

Run: `grep -rn "popover-item-active-bg\|popover-item-selected-bg" src/`
Expected: 无任何输出（DimensionPopover.vue 与测试均已改写完毕）。

删除 `src/assets/tokens.css` light 区块第 85-86 行：

```css
  --color-popover-item-selected-bg: #fafaff;
  --color-popover-item-active-bg: #eef2ff;
```

删除 dark 区块第 165-166 行：

```css
    --color-popover-item-selected-bg: #1e1b3a;
    --color-popover-item-active-bg: #2e2a52;
```

- [ ] **Step 7: 全量测试 + 构建**

Run: `npm test`
Expected: PASS（全绿）。

Run: `npm run build`
Expected: 成功，无类型错误。

- [ ] **Step 8: 提交**

```bash
git add src/components/DimensionPopover.vue src/assets/tokens.css src/__tests__/components/DimensionPopover.test.ts
git commit -m "feat(popover): 实心光标高亮(方案A)——光标实心填充、已选改品牌色文字+✓"
```

---

## 验收（人工）

`npm run dev` 后在「今天」页：

1. 新建一条 entry 并提交 → 输入框清空，且下一条**不再**预填上次的 dimension。
2. 输入框按 `@` 打开 dimension 菜单 → 用 ⌃N/⌃P 移动，光标行始终是**唯一**实心品牌色填充行（白字），不会与已选行混淆。
3. 已选过值的维度行显示「值 + ✓」、品牌色文字、无背景填充；光标落到该行时变实心填充 + 白字。
4. 切换系统深色模式复测 1-3，对比度正常。

## 自查（plan ↔ spec）

- Issue 1：lastDimensions 字段(1a)、TwoLineInput prop/watch/Esc(1b)、MonthView 写入与传参(1c)、三个测试文件全部覆盖。✓
- Issue 2：dim 阶段(2b)、val 阶段(2c)、token 清理(2d)、4 个样式测试改写(2a) 全部覆盖。✓
- 类型一致性：`setDims` 签名与用法一致；新增 class 字符串（`bg-[var(--color-brand-solid)]`、`text-white`、`text-white/80`、`truncate`、`max-w-[110px]`）均为本项目已用或 Tailwind v4 内置。✓
- 无 placeholder：所有步骤含完整代码与命令。✓
