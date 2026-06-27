# 未使用的 base 原语 / 死代码 — 已清理

> 来源：2026-06-26 `/review-project` 审查。2026-06-27 执行清理。

## 验证（2026-06-27，重新跑过四类 grep）

| 检查 | 结果 |
|------|------|
| PascalCase 引用 `AppButton` / `ProgressBar` / `mentionHelpers` | 均仅命中各自测试文件 |
| kebab-case 模板用法 `<app-button>` / `<progress-bar>` | 0 命中 |
| 全局注册 | 0 命中 |
| 文件被 import（排除测试） | 0 命中 |

同时确认 `provideStore()` 在 `useStore.ts` 中零调用者（`main.ts` 直接用 `app.provide(STORE_KEY, createStore())`）。

## 清理结果

| 删除项 | 文件 | 理由 |
|---|---|---|
| `mentionHelpers.ts` | `src/utils/mentionHelpers.ts` + 测试 | 三个函数零消费者，`DimensionPopover` 有自己的等价实现 |
| `provideStore()` | `src/stores/useStore.ts`（仅删函数 + `provide` import） | 零调用者，`main.ts` 直接 `app.provide` |
| `AppButton.vue` | `src/components/base/AppButton.vue` + 测试 | pill-only 设计语言与 app 按钮生态不匹配，13 个按钮中仅 2 个可替换，替换覆盖面 <20% |
| `ProgressBar.vue` | `src/components/base/ProgressBar.vue` + 测试 | 两个消费点需求不统一（高度、圆角、动画、过预算处理），抽象成本 > 重复成本 |

## 关联的重复代码（未处理，另开会话）

以下 P2 重复项不在本次清理范围：

- dimension→颜色 class 映射在 `EntryComposer` / `EntryRowEdit` / `EntryRow` / `DimensionPopover` 多处重复
- `todayStr()` 在 `App.vue` / `MonthView.vue` / `HeatmapCalendar.vue` 重复，等价 `utils/dates.formatDate`
- `MONTH_NAMES` 在 `HeatmapCalendar` 与 `QuickJumpPopover` 各定义一份
