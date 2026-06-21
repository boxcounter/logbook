# Entry 输入框 & dimension 菜单交互改进

日期：2026-06-21
分支：`worktree-entry-dim-ux`

两个独立的 UX 问题，合并到一个变更里处理。

---

## Issue 1 — 移除 dimension 预填

### 问题

新建 entry 后，输入框会预填上一条 entry 的 dimension values（来自 `store.lastDimensions`）。用户的真实工作流里相邻 entry 的维度通常不同，预填反而多一步：得先取消已填的值再重选。

### 根因

`store.lastDimensions` 在每次提交后写入（`MonthView.vue:123`），并作为 `initial-values` 喂给下一个 `TwoLineInput`（`MonthView.vue:374`）。除预填外，该字段无任何其他消费方——是纯粹的 dead state。

### 方案

彻底删除 `lastDimensions`，每次新建输入框从空白开始。

改动：

- **`src/stores/useStore.ts`**：删除 `lastDimensions: Record<string, string>` 字段声明与 `lastDimensions: {}` 初始化。
- **`src/components/MonthView.vue`**：
  - 删除 `handleSubmit` 里的 `store.lastDimensions = { ...finalDimensions };`。
  - `<TwoLineInput>` 不再传 `:initial-values`。
- **`src/components/TwoLineInput.vue`**：
  - 删除 `initialValues` prop。
  - `dimValues` 初始化为 `{}`。
  - 删除 `watch(() => props.initialValues, ...)` 块。
  - Esc 清空逻辑：`hasContent` 改为「`text` 非空 或 `dimValues` 有任意键」；重置改为 `dimValues.value = {}`、`text.value = ""`、`submitAttempted.value = false`。

### 安全性确认

`TwoLineInput` 全项目仅 `MonthView` 使用，且只用于「新建 entry」。编辑既有 entry 走 `EntryRowEdit`，不涉及 dimension，因此删除 `initialValues` prop 不影响编辑路径。

### 测试影响

`npm run build` 会用 `vue-tsc` 严格 typecheck 测试文件，因此须同步：

- `src/__tests__/useStore.test.ts`：去掉 `expect(store.lastDimensions).toEqual({})`。
- `src/__tests__/components/MonthView.test.ts`：mock store 去掉 `lastDimensions: {}`。
- `src/__tests__/components/TwoLineInput.test.ts`：去掉传 `initialValues` prop 的挂载参数；如有「预填」相关用例则删除或改写为「初始为空」。

---

## Issue 2 — 区分光标(active)与已选(selected)：方案 A 实心光标

### 问题

dimension 菜单里，「已有值」的条目背景（`--color-popover-item-selected-bg` = `#fafaff`）与「当前 ⌃N/⌃P 光标」的高亮背景（`--color-popover-item-active-bg` = `#eef2ff`）都是淡靛蓝填充，色差太小，分不清光标停在哪一行。

### 根因

两种状态都用「背景填充」这一个视觉通道，且同属靛蓝色系（commit 498403d 将 active 改为品牌色填充后两者更接近）。

### 方案（A — 实心光标）

让背景填充只代表「光标」一个含义；「已选」改用文字色 + `✓` 标记，不再占用背景填充。`DimensionPopover.vue` 的 dim 阶段（`v-for` dim-item）与 val 阶段（`v-for` val-item）统一为三态：

| 状态 | 背景 | 文字 | 右侧 |
|---|---|---|---|
| **光标 active**（含 mouseenter hover） | 实心 `--color-brand-solid` 填充 | 白字 | 该行原右侧内容，淡色（白/浅靛蓝），保持可读 |
| **已选 selected**（非光标） | 无填充 | `--color-brand-solid` + `font-semibold` | dim 阶段：已选**值** + `✓`；val 阶段：值后加 `✓` |
| 普通 | 无 | `--color-text-primary` | required/optional badge（dim 阶段） |

优先级：光标态压倒已选态。光标落在已选项上时仍渲染为实心品牌 + 白字。任意时刻整列只有一行有背景填充。

实现要点：

- 两处 `:class` 数组改写。active 分支输出 `bg-[var(--color-brand-solid)] text-white`（覆盖其余文字/背景类）。
- dim 阶段：已选项右侧由「required/optional badge」改为「值 + `✓`」；值过长用 `truncate` / `max-w` 截断，避免撑破 240px 宽度。未选项仍显示 required/optional。
- val 阶段：已选值文本后追加 `✓`。
- `data-active` 属性保持不变（`DimensionPopover.test.ts` 依赖它定位高亮行）。
- dark mode：实心 `--color-brand-solid`(#6366f1) + 白字、已选品牌色文字均沿用现有 token，无需新增。

### 附带清理

- 删除 `src/assets/tokens.css` 中不再被引用的 `--color-popover-item-active-bg`、`--color-popover-item-selected-bg`（light + dark 共 4 处定义）。删除前 grep 确认无其他引用。
- 本变更修订了 `docs/superpowers/specs/2026-06-20-popover-highlight-style-design.md` 的高亮策略；如需可在该文件加一行指向本 spec。

### 测试影响

- `DimensionPopover.test.ts`：若有断言具体背景 class（如 `bg-[var(--color-popover-item-selected-bg)]`）须改为新类；基于 `data-active` 的断言不受影响。新增/调整：已选项渲染 `✓`、active 项为实心品牌背景。

---

## 验收

- `npm run build`（含 `vue-tsc` 严格检查）通过。
- `npm test` 全绿。
- 手测：新建 entry 输入框初始为空、提交后下一条仍为空；dimension 菜单中 ⌃N/⌃P 移动时光标行始终是唯一实心填充行，已选行带 `✓` 且无背景填充。

## 不做（YAGNI）

- 不保留 `lastDimensions` 作「重复上一条」之类的潜在用途。
- 不为高亮新增 token；复用 `--color-brand-solid`。
