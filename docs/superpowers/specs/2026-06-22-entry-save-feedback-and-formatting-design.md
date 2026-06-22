# Entry 保存反馈 & 时间格式 & 配额输入宽度 — 设计说明

**日期：** 2026-06-22

## 背景

三项小的 UI 优化：

1. Entry 列表编辑保存后无反馈，用户不知道是否生效
2. Commitments 区域的时间展示保留分钟数占用空间，节省下来能让 role/goal 名称更宽裕
3. RoleCard 时间配额输入框太窄，三位数（如 150）显示不全

---

## 1. 保存反馈 Toast

### 现状

`MonthView.handleUpdateEntry` / `handleUpdateDimensions` 调用 `invoke("update_entry", …)` 后静默更新 store，无任何视觉反馈。

### 方案

新增轻量 "Saved" toast，独立于现有 Undo toast。

- **组件**：复用现有 `<Toast>`，不传 `undoLabel` → 无 Undo 按钮，仅展示消息和关闭按钮
- **触发**：`App.vue` `provide("triggerSavedToast", fn)`，`MonthView` inject 后在 update 成功时调用
- **持续**：2 秒自动消失，无需用户操作
- **位置**：与 Undo toast 相同——底部居中

### 改动清单

| 文件 | 改动 |
|------|------|
| `src/App.vue` | 新增 `showSavedToast` ref、`savedToastMessage` ref、`triggerSavedToast` 函数；`provide("triggerSavedToast")` |
| `src/components/MonthView.vue` | `inject("triggerSavedToast")`；`handleUpdateEntry` / `handleUpdateDimensions` 成功后调用 |
| `src/components/base/Toast.vue` | 无需改动（`undoLabel` 缺省时 `v-if` 自动隐藏按钮） |

### 取舍理由

- **为什么放 App 层，而不是 EntryRowEdit 内部**：toast 固定底部居中，与 Undo toast 统一位置，避免每个编辑行各自弹 toast 造成视觉碎片。
- **为什么 2 秒而非 5 秒**：保存是低风险操作，无需 Undo，仅需确认"已生效"。

---

## 2. Commitments 时间格式改为紧凑形式

### 现状

`formatDuration(minutes)` 统一输出 `"1h 30m"`。

### 方案

新增 `formatDurationCompact(minutes: number): string`：
- `0` → `"0"`，`30` → `"0.5h"`，`90` → `"1.5h"`，`120` → `"2h"`，`150` → `"2.5h"`
- 保留一位小数，`.0` 抹掉；0 分钟直接返回 `"0"`（不返回 `"0h"`，与现有调用点 `v-if="spent > 0"` / `v-else` → `"0"` 保持一致）
- 转换逻辑：minutes === 0 时返回 `"0"`；否则 `Math.round(minutes / 6) / 10`，再 strip trailing `.0` + `"h"`

### 替换范围

| 文件 | 行号 | 当前调用 | 改为 |
|------|------|---------|------|
| `CommitmentsPanel.vue` | L57（role spent）、L74（goal spent） | `formatDuration` | `formatDurationCompact` |
| `CommitmentsModal.vue` | L157（Logged total） | `formatDuration` | `formatDurationCompact` |
| `RoleCard.vue` | L77（overBy）、L164（roleSpent） | `formatDuration` | `formatDurationCompact` |
| `GoalRow.vue` | L29（logged 展示）、L32（title 属性） | `formatDuration` | `formatDurationCompact` |

### 不改的范围

| 文件 | 原因 |
|------|------|
| `EntryRow.vue` L76 | Entry 列表——明确保留详细格式 |
| `DayHeader.vue` L20 | 日总计——已确认保持现状 |
| `EntryComposer.vue` L188 | 录入预览——已确认保持现状 |

---

## 3. 配额输入宽度

### 现状

`RoleCard.vue` L118：`<input class="w-[42px]" …>` —— 约容纳 2 位数，三位数（如 150）截断。

### 方案

`w-[42px]` → `w-[52px]`。足以容纳 3 位数 + padding。

> 按 CLAUDE.md 设计 token 规范，组件尺寸（`w-`/`h-`/`min-`/`max-`）不受语义 token 约束，可用任意 px。

### 改动清单

| 文件 | 改动 |
|------|------|
| `src/components/composite/RoleCard.vue` L118 | `w-[42px]` → `w-[52px]` |

---

## 测试

- `format.test.ts`：新增 `describe("formatDurationCompact")`，覆盖 `0`→`"0"`、`30`→`"0.5h"`、`90`→`"1.5h"`、`120`→`"2h"`、`150`→`"2.5h"`
- `RoleCard.test.ts` / `GoalRow.test.ts` / `CommitmentsPanel.test.ts` / `CommitmentsModal.test.ts`：若现有断言包含 `formatDuration` 输出的 duration 字符串，更新为紧凑格式
