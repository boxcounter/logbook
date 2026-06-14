# 页面布局改版 — 设计规格

> 2026-06-14 · 状态：设计中

## 概述

将主窗口布局从 Day/Week/Month 三粒度模式改为固定月视图模式。窗口保持左右两栏：左栏显示月概览（月份导航 + Commitments），右栏显示选中月份内的日级详情（日期条、日备注、快捷录入、条目列表）。

## 动机

- 当前三粒度模型（Day/Week/Month）中，Week 和 Month 粒度使用频率低，徒增复杂度
- Month 是自然的组织单元——Commitments 按月定义，文件按 `年/月/` 目录存储
- Commitments 在当前布局中放在右侧边栏，位置次要；移到左栏后获得固定、显眼的位置
- 弱化 Week 简化心智模型，减少 UI 表面积

## 组件架构

```
App.vue
├── SetupScreen.vue                （不变）
├── ConfigErrorBanner.vue          （不变）
└── MonthView.vue                  （新建，替代 TodayView.vue）
    ├── MonthSidebar.vue           （新建，左 1/3，sticky 定位）
    │   ├── MonthNavigator.vue     （新建）
    │   └── CommitmentsPanel.vue   （复用，不变）
    └── DayDetail.vue              （新建，右 2/3）
        ├── DayStrip.vue           （新建）
        ├── DayNote                （内联，从 DateNavigator 拆出）
        ├── QuickEntry.vue         （复用）
        ├── EntryList.vue          （复用，底部加内联合计行）
        └── 文件路径链接            （内联）
```

### 删除的组件

| 组件 | 原因 |
|------|------|
| TodayView.vue | 由 MonthView.vue 替代 |
| DateNavigator.vue | 拆分为 MonthNavigator + DayStrip + DayNote |
| SummaryBar.vue | 由 EntryList 底部的内联合计行替代 |

## 布局

```
┌─────────────────┬──────────────────────────────────────────┐
│  左栏 (1/3)     │  右栏 (2/3)                              │
│                 │                                          │
│ ┌─────────────┐ │ ┌──────────────────────────────────────┐ │
│ │ ← 6月 2026 →│ │ │ DayStrip: 1  2  3 ... 30（横向滚动  │ │
│ │   [▼ 选择]  │ │ │ 有 entry 的日期标蓝点，每 7 天稍宽间距，│ │
│ └─────────────┘ │ │ 未来日期灰色）                       │ │
│                 │ └──────────────────────────────────────┘ │
│ ┌─────────────┐ │ ┌──────────────────────────────────────┐ │
│ │ Commitments │ │ │ DayNote（contenteditable）           │ │
│ │             │ │ └──────────────────────────────────────┘ │
│ │ Developer   │ │ ┌──────────────────────────────────────┐ │
│ │ 12.5h/20h   │ │ │ QuickEntry（仅选中日期为当天时可见） │ │
│ │ ██████░░░░  │ │ └──────────────────────────────────────┘ │
│ │             │ │ ┌──────────────────────────────────────┐ │
│ │ Designer    │ │ │ EntryList                            │ │
│ │ 3.2h/10h    │ │ │  ├ Entry 1                          │ │
│ │ ███░░░░░░░  │ │ │  ├ Entry 2                          │ │
│ │             │ │ │  └ Entry 3                          │ │
│ │ PM          │ │ │  ─────────────────────              │ │
│ │ 8.0h/15h    │ │ │  3 entries / 7.0h  （内联合计）     │ │
│ │ █████░░░░░  │ │ └──────────────────────────────────────┘ │
│ └─────────────┘ │ …/2026/06/2026-06-14.md                 │
└─────────────────┴──────────────────────────────────────────┘
```

## 组件详细设计

### MonthNavigator

- 显示 `← <月份名> <年份> →`，左右箭头逐月翻页
- 点击月份文字打开快速跳转弹窗，内含两个 `<select>` 下拉：
  - 年份选择：仅列磁盘上有数据的年份
  - 月份选择：仅列该年有数据的月份
- 下拉值变更时立即跳转
- 点击弹窗外区域关闭弹窗

### DayStrip

- 单行横向滚动，显示当月全部日期（28-31 个 cell）
- 视觉效果规则：
  - 每个 cell：日期数字，可点击
  - 选中的日期：蓝底白字
  - 当天：加粗或下划线（与选中态区分）
  - 未来日期（当月内大于今天的日期）：灰色文字，不可点击
  - 有 entry 的日期：数字下方显示蓝色小圆点
  - 无 entry 的日期：不显示圆点
  - 每第 7 天右侧：稍宽的间距（约 6-8px），形成自然的 7 日一组视觉效果（week 分界）
- 默认滚动位置：当月滚动到当天可见；过去月份滚动到月末
- 点击日期 → 更新 `store.currentDate`，从已加载的 `monthEntries` 取数据

### DayNote

- 从当前 DateNavigator 的 contenteditable div 拆出
- 行为不变：blur 时调用 `set_day_note`
- 位于 DayStrip 和 QuickEntry 之间
- placeholder 文字："Add a note…"

### EntryList（修改）

- 复用现有组件，底部新增内联合计行：`<N> entries / <X>h`
- 不再接收 SummaryBar 作为兄弟组件——合计行为 EntryList 模板的一部分

### QuickEntry、CommitmentsPanel、EntryItem、EntryGroup、DimensionPanel、EntryInput

- 不变。原样复用。

## Store 变更

```typescript
// 移除
granularity: Granularity                        // 不再需要
periodEntries: Record<string, Entry[]>           // 由 monthEntries 替代

// 新增
monthEntries: Record<string, Entry[]>            // 当月所有日期的 entries
```

- `currentDate` 保留——始终存储当前选中的日期（YYYY-MM-DD 格式）
- `currentYear`、`currentMonth` 通过 computed 从 `currentDate` 派生

## 数据流

### 月份加载（由 MonthNavigator 触发）

1. 设置 `currentDate`：目标为当月 → 取今天；目标为过去月份 → 取该月最后一天
2. 遍历当月每一天，调用 `invoke("get_entries")` → 填入 `store.monthEntries`
3. 调用 `invoke("get_commitment_progress")` → 填入 `store.commitmentProgress`
4. 从 `monthEntries[currentDate]` 设置 `store.today`

### 日期选择（由 DayStrip 点击触发）

1. 更新 `store.currentDate`
2. 从 `store.monthEntries[currentDate]` 取数据设置 `store.today`（无网络请求）

### Entry 增删改

1. 维持现有乐观更新逻辑
2. 后端确认后：更新对应日期的 `monthEntries` + 刷新 `commitmentProgress`

### 应用启动 / 窗口聚焦

- 维持现有行为——init 时确定当前日期，加载该月 entries + commitments
- 窗口聚焦时，检查日期是否跨天变化，必要时重新 init

## 快速跳转的数据来源

为支持快速跳转下拉只列有数据的年月，后端需新增命令：

```rust
get_available_months(root_path: String) -> Vec<AvailableMonth>
// AvailableMonth { year: i32, month: u32 }
```

该命令扫描 `{root_path}/` 目录，找出所有含有至少一个 `.md` 文件的 `YYYY/MM/` 子目录。按时间倒序排列（最新在前）。

若该命令性能有问题，可在 init 时运行一次并缓存于 store 中；文件监听已覆盖 config 和 monthly 文件，月份可用性仅在目录创建或删除时变化。

## 不变的部分

- 除新增 `get_available_months` 外的所有 Rust 后端命令
- Tauri 事件系统（config-changed、commitments-changed）
- 文件监听
- SetupScreen、ConfigErrorBanner 行为
- Undo toast
- EntryInput、DimensionPanel、mention 支持
- CSS/Tailwind 方案

## 删除的部分

- `Granularity` 类型及所有粒度相关逻辑
- `datesInPeriod` 中 week/month 分支（仅保留 month 分支用于加载）
- `SummaryBar.vue` 组件
- `DateNavigator.vue` 组件
- `TodayView.vue` 组件
- `weekLabel` 工具函数

## 测试策略

### 新建组件测试
- `MonthNavigator.test.ts`：箭头导航 emit 正确的月份/年份，快速跳转下拉过滤可用月份
- `DayStrip.test.ts`：渲染正确的天数、圆点标记、未来日期灰色、选择 emit、7 天一组的间距
- `MonthView.test.ts`：集成测试——月份加载填充 entries，日期点击更新视图

### 需更新的组件测试
- `EntryList.test.ts`：验证内联合计行显示正确的数量和时长
- `App.test.ts`：验证 screen === "ready" 时渲染 MonthView

### 需删除的测试
- `DateNavigator.test.ts`：由 MonthNavigator + DayStrip 测试替代
- `SummaryBar.test.ts`：删除
- `TodayView.test.ts`：由 MonthView 测试替代

### 工具函数测试
- `dates.test.ts`：移除 `datesInPeriod` 的 week/month 测试，移除 `weekLabel` 测试
