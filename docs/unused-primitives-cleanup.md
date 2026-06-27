# 未使用的 base 原语 / 死代码 — 待清理

> 来源：2026-06-26 `/review-project` 审查。本文档是**待办记录**，留待另一个会话处理；不在当前会话动这些文件。

## 结论

三个模块在生产代码里**零引用**，但都有配套且通过的测试——因此它们抬高了绿色测试计数，却在为永不运行的代码背书（审查元批评所称的「虚假覆盖」）。

## 验证方式（2026-06-26，四种交叉验证）

| 检查 | 结果 |
|------|------|
| PascalCase 引用 `AppButton` / `ProgressBar` / `mentionHelpers` | 仅命中各自测试文件 |
| kebab-case 模板用法 `<app-button>` / `<progress-bar>` | 0 命中 |
| 全局注册 `app.component(...)` / `components: {...}` | 0 命中（项目用 `<script setup>` 局部导入，无全局注册） |
| 文件被 import（排除测试）`base/AppButton`、`base/ProgressBar`、`utils/mentionHelpers` | 0 命中 |

> 处理前请**重新跑一遍上述四类 grep** 确认仍未被引用（代码可能已变化）。

## 三个文件的现状

- **`src/components/base/AppButton.vue`** — 在 commit `3cbf418` 作为设计系统组件加入，此后从未被任何界面导入。各处按钮（`SetupScreen` / `RecoveryScreen` / `EntryRowEdit` 等）都各自手写 `<button class="...">`。
- **`src/components/base/ProgressBar.vue`** — `CommitmentsPanel` 与 `RoleCard` 各自手写进度条 div，未用此原语。
- **`src/utils/mentionHelpers.ts`**（`dimBarColor` / `getValueCount` / `firstUnfilledRequiredIndex`）— 被遗弃的早期版本；出货用的 `DimensionPopover.vue` 有自己的 `firstUnfilledIndex` / `barClass` 实现。**最无歧义的死代码。**

## 待决策

- **`mentionHelpers.ts`**：确定死代码，可连同 `src/__tests__/mentionHelpers.test.ts` 直接删除。
- **`AppButton.vue` / `ProgressBar.vue`**：`base/` 设计系统原语，有**两条相反的合理路线**，需人来定，不要单方面选：
  1. **删除** —— 连同各自测试一并移除；承认现状是各处内联手写。
  2. **接线复用** —— 把这两个原语接入界面，移除 `SetupScreen` / `RecoveryScreen` / `CommitmentsPanel` / `RoleCard` 等处的内联按钮/进度条重复。

## 关联的重复代码（审查 P2，一并考虑）

若选「接线复用」，这些内联重复是同一类问题，宜一起收敛：
- dimension→颜色 class 映射在 `EntryComposer` / `EntryRowEdit` / `EntryRow` / `DimensionPopover`（+ 死代码 `mentionHelpers.dimBarColor`）多处重复。
- `todayStr()` 在 `App.vue` / `MonthView.vue` / `HeatmapCalendar.vue` 重复，且与 `utils/dates.formatDate` 等价。
- 进度百分比计算 `Math.min(100, round(spent/alloc*100))` 在 `CommitmentsPanel` / `RoleCard` / `ProgressBar` 重复。
- `MONTH_NAMES` 在 `HeatmapCalendar` 与 `QuickJumpPopover` 各定义一份。
- `src/stores/useStore.ts` 的 `provideStore()` 亦为死代码（main.ts 直接 `app.provide`）。
