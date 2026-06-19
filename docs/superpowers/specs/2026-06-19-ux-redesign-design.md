# Logbook UX Redesign — Design Spec

> 关联 demo：`UX-REDESIGN-DEMO.html`
> 关联技术规格：`SPEC.md`
> 设计日期：2026-06-19

## 1. Context

### 问题诊断

当前 Logbook UX 的核心缺陷：**缺少信息层级设计**。

具体表现：
- 全局文字字号只差 1px（14/13/12/11），视觉分不出主次
- 维度录入有两条路径（@mention popover + Show/Hide 面板），逻辑割裂
- Entry List 里 item、chips、duration 三者的视觉权重无区分
- 输入区信息堆叠（输入框 + Duration 预览 + chips + Show/Hide 按钮），无优先级
- Commitments 面板文字区分度弱
- 删除按钮 hover 才可见，编辑无提示

### 设计目标

| 维度 | 目标 |
|------|------|
| 心理模型 | 仪表盘（开屏全局）+ 日记（自然记录）+ 数据库（结构化查询）三合一 |
| 使用模式 | 实时记，10-30s/条，一天 ~10 条，条目文字 8-50 字符 |
| 录入理想 | 全键盘操作、打开即 Today、自动 focus、轻反馈、Enter 提交 |
| 视觉气质 | 专业工具感——Linear / Arc / Things 3 式的克制、精准、呼吸感 |
| 信息层级 | 主（item text）→ 辅（duration）→ 标签（dimension chips）→ 提示（hints/missing） |
| 目标屏幕 | MacBook Pro 14" |

### 关键决策

- **字体方案**：数字、时间、快捷键用 SF Mono，其余 system-ui
- **必选维度**：软提示（虚线 chip），不拦截 Enter
- **编辑触发**：hover 显示 `⋯` 图标点击进入，双击整行也可。不用单击（防误触）
- **维度选择**：`@` 触发 popover → 维度菜单（紫色 header + 颜色条 + required/optional）→ 值菜单（暖灰 header + ← back + Esc 返回）
- **时长输入**：item 内自然写 `1h`/`45m` 自动解析，或 `#` 手动触发。token 无 `#` 前缀

## 2. Layout Architecture

### 三区一体模型

```
┌─────────────────────────────────────────────────────┐
│  Sidebar (220px)          │  Main Area (flex-1)      │
│                           │                          │
│  ← June 2026 ▾ →         │  Thursday, June 19 Today │
│  [heatmap calendar]       │  ─────────────────────── │
│  58.5h / month            │  Entry 1         1h  ⋯  │
│                           │  Entry 2      1h 30m ⋯  │
│  ─────────────────────    │  Entry 3         1h  ⋯  │
│  COMMITMENTS              │  ...                     │
│  Developer ▾              │  Entry 10         —  ⋯  │
│  ████████░░ 20.5/40h      │                          │
│    Sprint planning  3.5h  │  Day note: 今天主要...   │
│    重构用户认证...   8h   │                          │
│  PM ▸             12/20h  │  ┌──────────────────┐   │
│                           │  │ + [item text]  ⏎ │   │
│                           │  │ [token chips]    │   │
│                           │  └──────────────────┘   │
│                           │  @ dim  # time  ⌘[ ⌘]  │
│                           │        …/2026/06/19.md  │
└─────────────────────────────────────────────────────┘
```

### 区域职责

| 区域 | 内容 | 性质 |
|------|------|------|
| Sidebar | 月导航 + 热力图 + Commitments（role/进度/goals） | 上下文 / 辅助信息 |
| Main | 日标题 + Entry List + Day Note + 两行输入 + 文件路径 | 核心内容 + 行动点 |
| Input（在 Main 底部） | item 行 + token 行 + hints | 持续可见的录入入口 |

### 滚动行为

- Sidebar：sticky top（不随主区滚动）
- Main entry list：独立滚动（条目多时 input 区始终可见）
- 整体：窗口高度由内容撑开，无全局 scroll

## 3. Visual Design Tokens

> 核心原则：所有视觉值必须来自 token，组件内不写硬编码颜色/字号。
>
> **Token 值的唯一权威来源是 `docs/superpowers/specs/2026-06-19-ux-redesign-tokens.css`。**
> 本节只描述 token 的**语义分类和用途**，不列出具体值（避免双源漂移）。

### 3.1 Token 分类与用途

| 类别 | Token 前缀 | 用途 |
|------|-----------|------|
| Surface | `--color-page-bg`, `--color-surface`, `--color-surface-muted` | 页面背景、卡片背景、次级背景 |
| Text | `--color-text-primary`, `--color-text-secondary`, `--color-placeholder`, `--color-text-muted`, `--color-text-disabled` | 文字层级：主→辅→placeholder→禁用 |
| Border | `--color-border-form`, `--color-border-decorative`, `--color-divider` | 输入框边框、装饰性边框、分割线 |
| Brand | `--color-brand-*` | 品牌色：solid、gradient from/to、link、soft bg |
| Semantic | `--color-success`, `--color-danger`, `--color-warning` | 成功、危险、警告 |
| Input tokens | `--color-token-*-bg`, `--color-token-*-text` | 输入框内 token chip（较饱和，表示正在编辑） |
| Entry chips | `--color-chip-*-bg`, `--color-chip-*-text` | 列表内 chip（较淡，被动展示） |
| Missing | `--color-missing-*` | 必选维度缺失虚线提示 |
| Heatmap | `--heatmap-*` | 热力图色阶、文字色、环色 |
| Dim bars | `--dim-bar-*` | Popover 维度左侧颜色条 |
| Popover | `--color-popover-*` | 维度/值选择 popover 专用色 |
| Animation | `--anim-highlight-*` | 新 entry 高亮淡出 |

**字号 scale**: `--text-2xs`(9) → `--text-xs-alt`(11) → `--text-micro`(10) → `--text-xs`(12) → `--text-sm`(13) → `--text-base`(14) → `--text-lg`(18) → `--text-xl`(20)

**字重 scale**: `--weight-book`(400) → `--weight-medium`(500) → `--weight-semibold`(600) → `--weight-bold`(700) → `--weight-heavy`(800)

**间距 scale**: `--space-xs`(4) → `--space-sm`(8) → `--space-md`(12) → `--space-lg`(16) → `--space-xl`(24) → `--space-2xl`(28)

**圆角 scale**: `--radius-sm`(3) → `--radius-md`(4) → `--radius-form`(5) → `--radius-form-lg`(8) → `--radius-card`(12) → `--radius-lg`(14)

> **例外**: `font-weight: 450` 用于 entry chips 和 missing indicator，作为 400 和 500 之间的组件特例，不设独立 token。

## 4. Component Specs

> 每个组件的视觉属性精确对应 token。只列关键属性，省略布局属性（flex/gap/padding）除非是视觉的一部分。

### 4.1 HeatmapCalendar (Sidebar 顶部)

| 元素 | 字号 | 字重 | 颜色 | 特殊 |
|------|------|------|------|------|
| 月份名 | `--text-base` | `--weight-bold` | `--color-text-primary` | — |
| 年份 | 11px | 400 | `--color-text-secondary` | — |
| 星期头 M-S | 9px | 400 | `--color-text-secondary` | system-ui |
| 日期数字 | 10px | 400→700(heavy) | 按色阶 | `--font-mono` |
| 单元格 | 24×24px | — | bg 按色阶，text 按色阶 (`--heatmap-empty-text` / `--heatmap-light-text` / `--heatmap-mid-text` / `--heatmap-heavy-text`) | `--radius-md`, hover scale(1.15) |
| 今日环 | — | — | `--heatmap-today-ring` | `box-shadow: 0 0 0 2px` |
| 选中环 | — | — | `--heatmap-selected-ring` | `box-shadow: 0 0 0 2px` |
| 月份总计 | 11px | `--weight-semibold` | `--color-text-primary` | `/ month` 10px 400 `--color-text-secondary` |
| 导航箭头 | 12px | 400 | `--color-text-secondary` | hover → `--color-text-primary` |

### 4.2 QuickJumpPopover (Sidebar 顶部，点击月份标签弹出)

| 元素 | 字号 | 颜色 | 特殊 |
|------|------|------|------|
| Year select | 12px | `--color-text-primary` | border: `--color-border-form`, `--radius-form-lg` |
| Month select | 12px | `--color-text-primary` | 同上 |
| Go 提示 | 9px | `--color-text-secondary` | — |

默认隐藏，点击「June 2026 ▾」弹出，选择后关闭。

### 4.3 CommitmentsPanel (Sidebar 下半部)

| 元素 | 字号 | 字重 | 颜色 | 特殊 |
|------|------|------|------|------|
| Section label | `--text-micro` | `--weight-bold` | `--color-text-secondary` | uppercase, letter-spacing 0.5px |
| Role name | 12px | `--weight-semibold` | `--color-text-primary` | ▸ 折叠 / ▾ 展开 |
| Spent/allocation | 11px | `--weight-semibold` | `--color-text-primary` | spent `--font-mono`, allocation 400 `--color-text-secondary` |
| Progress bar | 高 4px | — | bg: `--color-divider`, fill: brand gradient | `--radius-sm` |
| Goal name | 11px | 400 | `--color-text-secondary` | `max-width:130px`, 超出省略，hover tooltip 全文 |
| Goal spent | 11px | `--weight-medium` | `--color-text-primary` | `--font-mono`. 0→`--color-text-secondary` |

点击 role 行展开/折叠 goal 列表。

### 4.4 DayHeader (Main 顶部)

| 元素 | 字号 | 字重 | 颜色 | 特殊 |
|------|------|------|------|------|
| Day title | `--text-xl` | `--weight-bold` | `--color-text-primary` | letter-spacing: -0.3px |
| Today badge | `--text-micro` | `--weight-semibold` | `--color-brand-link` | bg: `--color-brand-soft-bg`, radius 4px |
| Day summary | `--text-xs` | 400 | `--color-text-secondary` | entry count `--font-mono`, total `--font-mono` |
| Divider line | — | — | `--color-divider` | 1px solid, padding-bottom: 14px |

### 4.5 EntryRow (Main 列表项)

| 元素 | 字号 | 字重 | 颜色 | 特殊 |
|------|------|------|------|------|
| Item text | `--text-base` | `--weight-medium` | `--color-text-primary` | 最多 2 行，超出省略，hover tooltip 全文 |
| Duration | 13px | 400 | `--color-text-primary` | `--font-mono`, `tabular-nums`, ml 16px |
| Chip (cat) | `--text-micro` | 450 | `--color-chip-cat-text` | bg: `--color-chip-cat-bg`, max-width:100px, 省略 |
| Chip (biz) | `--text-micro` | 450 | `--color-chip-biz-text` | bg: `--color-chip-biz-bg`, 同上 |
| Chip (imp) | `--text-micro` | 450 | `--color-chip-imp-text` | bg: `--color-chip-imp-bg`, 同上 |
| Chip (goal) | `--text-micro` | 450 | `--color-chip-goal-text` | bg: `--color-chip-goal-bg`, 同上 |
| Edit trigger ⋯ | 14px | 400 | `--color-text-secondary` | 默认 opacity:0, row hover→1, icon hover→`--color-brand-solid` |
| Row hover | — | — | bg: `--color-surface-muted` | border: `--color-divider` 出现 |

**状态**：
- **默认**：背景透明，border 透明，⋮ 不可见
- **Hover**：bg `--color-divider`，border `--color-divider`，⋮ 出现
- **编辑中**：bg `--color-surface`，border `--color-brand-solid`，shadow `--shadow-focus-ring`
- **刚添加**：bg `--color-brand-soft-bg`，border `--color-brand-soft-bg`，1.5s 动画消退

### 4.6 EntryRowEdit (编辑模式，内嵌在 EntryRow 中触发)

| 元素 | 字号 | 字重 | 颜色 | 特殊 |
|------|------|------|------|------|
| Item input | `--text-base` | `--weight-medium` | `--color-text-primary` | border:none, 透明 bg, 全宽 |
| Duration input | 13px | 400 | `--color-text-primary` | `--font-mono`, width:56px, text-align:right, 1px border `--color-border-form`, focus→`--color-brand-solid` |
| Duration unit | 11px | 400 | `--color-text-secondary` | "min" |
| Edit chip | `--text-micro` | 500 | 按类型 | × 可删除（opacity:0.5, hover→1） |
| +tag chip | `--text-micro` | 500 | `--color-text-secondary` | border:1px dashed `--color-border-form`, hover→`--color-text-secondary` |
| Save btn | `--text-micro` | `--weight-semibold` | #fff | bg: `--color-brand-solid`, `--radius-form` |
| Cancel btn | `--text-micro` | `--weight-semibold` | `--color-text-secondary` | 透明 bg, hover→`--color-text-secondary` |
| Delete btn | `--text-micro` | `--weight-semibold` | `--color-text-disabled` | 透明 bg, ml:auto, hover→`--color-danger` |

### 4.7 DayNote (Main 中，Entry List 和 Input 之间)

| 元素 | 字号 | 颜色 | 特殊 |
|------|------|------|------|
| Note text | `--text-xs` | `--color-text-secondary` | italic, cursor:text, hover bg→`--color-page-bg`, hover color→`--color-text-secondary` |

### 4.8 TwoLineInput (Main 底部)

| 元素 | 字号 | 字重 | 颜色 | 特殊 |
|------|------|------|------|------|
| "+" prefix | `--text-lg` | 400 | `--color-brand-solid` | flex-shrink:0 |
| Item input | `--text-base` | 400 | `--color-text-primary` | border:none, 透明 bg, caret `--color-brand-solid` |
| Placeholder | `--text-base` | 400 | `--color-placeholder` | — |
| Enter badge | 9px | `--weight-semibold` | `--color-text-secondary` | border `--color-border-form`, radius 4px, 默认 opacity:0.5, focused→1 |
| Input card | — | — | bg: `--color-surface`, border:2px `--color-border-form` | `--radius-card`, focused→border `--color-brand-solid` + `--shadow-focus-ring` |
| Token (cat) | `--text-micro` | `--weight-medium` | `--color-token-cat-text` | bg: `--color-token-cat-bg` |
| Token (biz) | `--text-micro` | `--weight-medium` | `--color-token-biz-text` | bg: `--color-token-biz-bg` |
| Token (imp) | `--text-micro` | `--weight-medium` | `--color-token-imp-text` | bg: `--color-token-imp-bg` |
| Token (goal) | `--text-micro` | `--weight-medium` | `--color-token-goal-text` | bg: `--color-token-goal-bg` |
| Token (dur) | `--text-micro` | `--weight-medium` | `--color-token-dur-text` | bg: `--color-token-dur-bg`, `--font-mono` |
| Token × | 12px | — | opacity:0.4 | hover→opacity:1 |
| Missing indicator | `--text-micro` | 450 | `--color-missing-text` | border:1.5px dashed `--color-missing-border`, 可点击 |
| Missing dot | 5×5px | — | `--color-missing-dot` | border-radius:50% |
| Hints row | `--text-micro` | 400 | `--color-text-disabled` | hover→`--color-text-secondary`, kbd 内 `--font-mono` |

### 4.9 DimensionPopover (输入 @ 后弹出)

| 元素 | 字号 | 字重 | 颜色 | 特殊 |
|------|------|------|------|------|
| Dim header | `--text-micro` | `--weight-bold` | `--color-brand-solid` | bg: `#fafaff`, uppercase, "DIM" 徽章 |
| Dim item | 13px | 400 | `--color-text-primary` | 左侧颜色条(width:3px, `--radius-sm`), hover bg:`--color-divider` |
| Dim item selected | 13px | `--weight-semibold` | `--color-brand-solid` | bg: `--color-popover-item-selected-bg` |
| Dim bar (cat) | 3×18px | — | `--dim-bar-cat` | — |
| Dim bar (biz) | 3×18px | — | `--dim-bar-biz` | — |
| Dim bar (imp) | 3×18px | — | `--dim-bar-imp` | — |
| Dim bar (goal) | 3×18px | — | `--dim-bar-goal` | — |
| Required meta | `--text-micro` | `--weight-medium` | `--color-warning` | — |
| Optional meta | `--text-micro` | 400 | `--color-text-disabled` | — |
| Val header | `--text-micro` | `--weight-bold` | `#78716c` | bg: `#fafaf9`, ← back button |
| Val item | 13px | 400 | `--color-text-primary` | hover bg:`--color-divider` |
| Val item selected | 13px | `--weight-semibold` | `--color-brand-solid` | bg: `#fafaff` |
| Footer | 9px | 400 | `--color-text-disabled` | `--font-mono` for keyboard hints |

### 4.10 FilePath (Main 底部，Input 下方)

| 元素 | 字号 | 颜色 | 特殊 |
|------|------|------|------|
| Path text | `--text-micro` | `--color-text-disabled` | text-align:right, cursor:pointer, 点击 open in editor |

## 5. Interaction Model

### 5.1 键盘快捷键

| 快捷键 | 作用 | 上下文 |
|--------|------|--------|
| `@` | 触发维度选择 popover | 输入框 focused |
| `#` | 触发手动时长输入 | 输入框 focused |
| `Enter` | 提交 entry | 输入框 focused |
| `Esc` | 关闭 popover / 从值选择返回维度选择 | popover 打开 |
| `↵` (在 popover 中)  | 确认选中 | popover 打开 |
| `⌘[` | 上一个月 | 全局 |
| `⌘]` | 下一个月 | 全局 |
| `⌘K` | 命令面板 | 全局 |
| 双击 entry row | 进入编辑模式 | 条目 hover 态 |

### 5.2 输入流程（完整）

1. 打开 App → 自动 Today 视图，输入框 focused
2. 用户在 item 行打字（item text）
3. 输入时长：
   - 方式 A：在 item 里写 `1h` / `45m` / `1.5h` → 自动解析，token 行出现 `1h` chip
   - 方式 B：按 `#` → 输入数字 → token 行出现 `1h` chip（无 # 前缀）
4. 输入维度：
   - 按 `@` → 维度选择 popover 弹出
   - 维度菜单：紫色 header + 左侧颜色条。`required`/`optional` 标注
   - 选中维度 → 值菜单：暖灰 header + ← back 按钮。Esc 返回维度选择
   - 选中值 → token 在第二行显示
   - 重复 @ 添加更多维度
5. 必选维度未填：虚线 chip `● category` 轻声提示，不拦截 Enter
6. 删掉 duration token：系统重新扫描 item text
   - 有 duration pattern → 重新解析显示
   - 无 duration pattern → token 行显示 「Need a duration — type "1h" or press #」
7. Enter 提交 → 输入框清空，entry 出现在列表顶部，1.5s 蓝色高亮消退

### 5.3 编辑流程

1. Hover entry row → 行尾出现 `⋯`
2. 点击 `⋯`（或双击整行）→ 进入编辑模式
3. 编辑模式：item 变输入框，duration 变数字框，chips 可删除，`+tag` 可添加新维度
4. Save → 提交修改，退出编辑模式
5. Cancel / Esc → 放弃修改，退出编辑模式
6. Delete → 删除 entry，5s undo toast

### 5.4 月份导航

- ← → 箭头：翻月
- 点击「June 2026 ▾」：弹出年/月双下拉 QuickJumpPopover
- `⌘[` / `⌘]`：键盘翻月
- 热力图日期单元格：点击切换日期

## 6. 文本溢出处理

| 场景 | 最大长度 | 处理 |
|------|----------|------|
| Entry item text | 2 行 | `-webkit-line-clamp: 2`, overflow: hidden |
| Entry chip | 100px | `text-overflow: ellipsis`, white-space: nowrap |
| Sidebar goal name | 130px | `text-overflow: ellipsis`, white-space: nowrap |
| 以上全部 | — | hover tooltip 显示全文 |

## 7. 状态与反馈

| 状态 | 视觉表现 |
|------|----------|
| 输入框无焦点 | border `--color-border-form`, ⏎ opacity:0.5 |
| 输入框有焦点 | border `--color-brand-solid`, shadow `--shadow-focus-ring`, ⏎ opacity:1 |
| 必选维度缺失 | 虚线 chip `● dimension_name`（软提示，不拦截） |
| 无 duration | token 行橙色提示文字 |
| Entry 刚添加 | 蓝色高亮（bg `--color-brand-soft-bg`），1.5s 消退 |
| Entry 编辑中 | 蓝色边框 + focus ring |
| 月份切换 | 热力图重绘，day 条目重新加载 |

## 8. 不变部分

以下来自 SPEC.md，不在本次 UX 改版范围内：

- 数据模型（Entry, Commitment, Config, DayFile）
- Rust 后端命令（14 个，见 SPEC.md）
- 文件格式（Markdown + frontmatter）
- 配置结构（config.yaml, _monthly.md）
- 图表技术栈（Chart.js，Phase 3 统计视图）
- Tauri 2.x + Vue 3 + TypeScript 技术栈

## 9. 删除项

以下现有 UI 元素在新设计中移除：

| 移除项 | 原因 |
|--------|------|
| DimensionPanel（Show/Hide 切换） | 被 @ 维度 popover + token 行取代 |
| Duration 预览行 | 被 token 行中的 dur chip 取代 |
| Missing required 红色错误文字 | 改为虚线 chip 软提示 |
| EntryRow 双击编辑 item / 双击编辑 duration | 统一为一个编辑模式（item + duration + chips 一起编辑） |
| 左栏单独的 MonthNavigator 卡片 | 整合到侧栏热力图上方 |
| DayStrip 横向滚动日期条 | 被侧栏热力图取代 |
| CommitmentsPanel 原有进度条颜色逻辑（橙/黄/绿/红） | 改为统一的 brand gradient 进度条 |

## 10. 技术注意事项

### Token 迁移

当前 `tokens.css` 将被重写。现有 token 名大部分保留，新增 token 按上述 3.1-3.7 命名。保持向后兼容：

- 原有的 `--color-*` token 名不变，值可能调整
- 新增 `--color-token-*`（输入区 token）和 `--color-chip-*`（列表 chip）系列
- 新增 `--heatmap-*` 系列
- 原有 `--app-text-*` 字号 token 替换为 `--text-*` 系列（更简洁）

### 组件拆分

新组件（需新建）：
- `HeatmapCalendar.vue` — 热力图日历
- `QuickJumpPopover.vue` — 年月快速跳转
- `TwoLineInput.vue` — 两行输入组件
- `DimensionPopover.vue` — 维度/值选择 popover
- `EntryRowEdit.vue` — Entry 编辑模式（可能内嵌在 EntryRow 中）

需重构的组件：
- `MonthView.vue` → 新布局
- `EntryList.vue` / `EntryRow.vue` → 新视觉规格
- `CommitmentsPanel.vue` → 新视觉规格 + goal 展开/折叠
- `App.vue` → 新的快捷键系统

可保留的组件：
- `SetupScreen.vue`
- `ConfigErrorBanner.vue`
- `Toast.vue`
- `AppButton.vue`（可能微调）
- `ProgressBar.vue`（内部使用，非直接用户可见）

删除的组件：
- `DayStrip.vue`
- `MonthNavigator.vue`
- `QuickEntry.vue`
- `DimensionPanel.vue`
- `EntryInput.vue`
- `MentionMenu.vue`
- `AppInput.vue`
- `AppChip.vue`
- `AppSelect.vue`
- `Popover.vue`
