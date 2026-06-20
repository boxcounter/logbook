# DimensionPopover 高亮样式重设计 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 DimensionPopover 的 active 高亮从 `ring-1 ring-inset`（全项目唯一的 `ring-*` 用法）换成品牌色填充背景（方案 A），并统一 hover。

**Architecture:** 新增 design token `--color-popover-item-active-bg`（浅/深两套），在 `DimensionPopover.vue` 两处 `:class` 里用「单一背景」绑定区分 active / filled / 普通，移除 `ring-*` 与冗余 `hover:bg-divider`。行为逻辑零改动。

**Tech Stack:** Vue 3 `<script setup>` + Tailwind v4（arbitrary values）+ Vitest。

**Spec:** `docs/superpowers/specs/2026-06-20-popover-highlight-style-design.md`

---

## File Structure

- Modify: `src/assets/tokens.css` — 新增 `--color-popover-item-active-bg`（:root + dark media）。
- Modify: `src/components/DimensionPopover.vue` — dim/val 两处 item 的 `class`/`:class`。
- Modify: `src/__tests__/components/DimensionPopover.test.ts` — 样式断言。

---

### Task 1: 新增 active 背景 token

**Files:**
- Modify: `src/assets/tokens.css`

- [ ] **Step 1: 在 :root 新增浅色 token**

在 `src/assets/tokens.css` 的 `--color-popover-item-selected-bg: #fafaff;`（约 line 85）之后新增一行：

```css
  --color-popover-item-active-bg: #eef2ff;
```

- [ ] **Step 2: 在 dark media 新增深色 token**

在 `@media (prefers-color-scheme: dark)` 块内的 `--color-popover-item-selected-bg: #1e1b3a;`（约 line 165）之后新增一行：

```css
    --color-popover-item-active-bg: #2e2a52;
```

- [ ] **Step 3: 确认 token 已加入**

无专门校验颜色 token 定义的测试（`tailwind-token-usage` 只管 `--app-text-*` 字号写法，与此无关）。做个 sanity 检查：

Run: `grep -n "popover-item-active-bg" src/assets/tokens.css`
Expected: 两行命中（:root 浅色 + dark media 深色）。

- [ ] **Step 4: Commit**

```bash
git add src/assets/tokens.css
git commit -m "feat(tokens): 新增 --color-popover-item-active-bg（浅/深）"
```

---

### Task 2: 切换 active 视觉为填充背景 + 统一 hover

**Files:**
- Modify: `src/components/DimensionPopover.vue`
- Test: `src/__tests__/components/DimensionPopover.test.ts`

- [ ] **Step 1: Write the failing tests**

追加到 `src/__tests__/components/DimensionPopover.test.ts` 的 `describe` 块内（复用已有的 `mountPop`）：

```ts
  // ---- highlight style (fill, not ring) ----

  it("highlights the active item with the active background, not a ring", () => {
    const wrapper = mountPop(); // active = index 0 (Category)
    const item = wrapper.findAll("[data-test='dim-item']")[0];
    expect(item.classes()).toContain("bg-[var(--color-popover-item-active-bg)]");
    expect(item.classes()).not.toContain("ring-1");
    expect(item.classes()).not.toContain("hover:bg-[var(--color-divider)]");
  });

  it("shows the selected background on a filled, non-active dimension", () => {
    // category filled → default active is Goal (index 1); Category (0) is filled & not active
    const wrapper = mountPop({ category: "Engineering" });
    const cat = wrapper.findAll("[data-test='dim-item']")[0];
    expect(cat.classes()).toContain("bg-[var(--color-popover-item-selected-bg)]");
    expect(cat.classes()).not.toContain("bg-[var(--color-popover-item-active-bg)]");
    expect(cat.classes()).toContain("text-[var(--color-brand-solid)]");
  });

  it("stacks active background and brand text when the cursor is on a filled item", async () => {
    const wrapper = mountPop({ category: "Engineering" }); // active = Goal(1)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp" })); // 1 -> 0 (Category)
    await wrapper.vm.$nextTick();
    const cat = wrapper.findAll("[data-test='dim-item']")[0];
    expect(cat.classes()).toContain("bg-[var(--color-popover-item-active-bg)]");
    expect(cat.classes()).not.toContain("bg-[var(--color-popover-item-selected-bg)]");
    expect(cat.classes()).toContain("text-[var(--color-brand-solid)]");
    expect(cat.classes()).toContain("font-semibold");
  });

  it("val phase: active value uses active bg, the already-selected value uses selected bg", async () => {
    const wrapper = mountPop({ category: "Engineering" }); // category values: Engineering, PM
    await wrapper.findAll("[data-test='dim-item']")[0].trigger("click"); // enter Category val; active = Engineering(0)
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown" })); // 0 -> 1 (PM)
    await wrapper.vm.$nextTick();
    const vals = wrapper.findAll("[data-test='val-item']");
    expect(vals[1].classes()).toContain("bg-[var(--color-popover-item-active-bg)]"); // PM active
    expect(vals[0].classes()).toContain("bg-[var(--color-popover-item-selected-bg)]"); // Engineering selected, not active
    expect(vals[0].classes()).not.toContain("bg-[var(--color-popover-item-active-bg)]");
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- DimensionPopover`
Expected: 新测试 FAIL（当前用的是 `ring-1`，且静态 class 含 `hover:bg-[var(--color-divider)]`，无 active 背景类）。

- [ ] **Step 3: 改 dim 阶段 item**

在 `src/components/DimensionPopover.vue`，把 dim 阶段 item 的静态 `class` 末尾的 `last:border-b-0 hover:bg-[var(--color-divider)]` 改为仅 `last:border-b-0`：

```html
        class="px-[14px] py-[9px] text-[length:var(--app-text-sm)]
               flex items-center gap-[10px] cursor-pointer border-b border-[var(--color-surface-muted)]
               last:border-b-0"
```

把它的 `:class`（当前为 selected/primary + ring 两项）替换为：

```html
        :class="[
          i === activeIndex
            ? 'bg-[var(--color-popover-item-active-bg)]'
            : (dimValues[d.key] ? 'bg-[var(--color-popover-item-selected-bg)]' : ''),
          dimValues[d.key] ? 'text-[var(--color-brand-solid)] font-semibold' : 'text-[var(--color-text-primary)]',
        ]"
```

- [ ] **Step 4: 改 val 阶段 item**

把 val 阶段 item 的静态 `class` 末尾的 `last:border-b-0\n               hover:bg-[var(--color-divider)]` 改为仅 `last:border-b-0`：

```html
        class="px-[14px] py-[9px] text-[length:var(--app-text-sm)]
               cursor-pointer border-b border-[var(--color-surface-muted)] last:border-b-0"
```

把它的 `:class` 替换为：

```html
        :class="[
          i === activeIndex
            ? 'bg-[var(--color-popover-item-active-bg)]'
            : (activeDimKey && dimValues[activeDimKey] === v ? 'bg-[var(--color-popover-item-selected-bg)]' : ''),
          activeDimKey && dimValues[activeDimKey] === v ? 'text-[var(--color-brand-solid)] font-semibold' : 'text-[var(--color-text-primary)]',
        ]"
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `npm test -- DimensionPopover`
Expected: 全部 PASS（含所有既有导航/默认高亮/Enter 测试——它们基于 `data-active`，未受影响）。

- [ ] **Step 6: Run full suite + typecheck**

Run: `npm test`
Expected: 全绿。

Run: `npx vue-tsc --noEmit`
Expected: 无错误。

- [ ] **Step 7: Commit**

```bash
git add src/components/DimensionPopover.vue src/__tests__/components/DimensionPopover.test.ts
git commit -m "feat(popover): active 高亮改为品牌色填充背景，移除 ring 与冗余 hover"
```

---

## 手动验证

`npm run tauri dev`，录入框敲 `@`：

1. 弹出后 active 项是淡紫填充（不是细描边）。
2. `⌃N/⌃P` / `↑↓` 移动，填充块跟着走；鼠标悬停某项也变同样填充（hover 与 active 统一）。
3. 已填维度仍是浅紫底 + 品牌色加粗文字；active 落在已填项上时叠加正常。
4. 进入值菜单同样表现。
5. （可选）系统切深色模式，active 块为更亮一档的靛蓝，仍比「已填」突出。

---

## Self-Review

- **Spec coverage:** 新 token 浅/深（Task 1）；dim/val 单一背景绑定 + 移除 ring/hover（Task 2 Step 3/4）；active/filled 叠加（test「stacks active background…」）；val 阶段 active vs selected（test「val phase…」）；导航行为不变（沿用既有测试，`data-active` 保留）。覆盖完整。
- **Placeholder scan:** 无 TBD/TODO；每步含完整代码与命令。深色值 #2e2a52 为确定值（spec 注明可微调，非占位）。
- **Type consistency:** token 名 `--color-popover-item-active-bg` 在 tokens.css 与组件 class 中一致；`i === activeIndex` 沿用文件现有写法。
