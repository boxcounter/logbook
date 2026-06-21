# Logbook — Technical Spec

> 关联设计文档：Vault `1_Projects/Logbook/README.md`

## 技术栈

| 层 | 选型 |
|---|------|
| 桌面框架 | Tauri 2.x |
| 后端 | Rust (`#[tauri::command]` + `yaml_serde`) |
| 文件监听 | `notify` crate |
| 前端 | Vue 3 + Composition API + TypeScript |
| 样式 | Tailwind CSS |
| 图表 | Chart.js（按需引入：Doughnut + Bar controllers，Phase 3） |
| Frontmatter 解析 | 手动提取 `---` 边界 + `yaml_serde` |

## Rust 后端

### 命令清单（15 个，已实现）

```
init(app: AppHandle) → InitResult
set_root_path(app: AppHandle, path: String) → Result<InitResult, String>
get_entries(root_path: String, date: String) → Result<DayFile, String>
append_entry(root_path: String, date: String, entry: CreateEntryInput) → Result<Entry, String>
update_entry(root_path: String, date: String, entry_id: String, update: UpdateEntryInput) → Result<DayFile, String>
delete_entry(root_path: String, date: String, entry_id: String) → Result<DayFile, String>
set_day_note(root_path: String, date: String, note: String) → Result<DayFile, String>
get_commitments(root_path: String, year: i32, month: u32) → Result<Vec<Commitment>, String>
get_month_dimensions(root_path: String, year: i32, month: u32) → Result<MonthDimensions, String>  // 纯读：该月生效维度 + from_template 标志，不实例化
set_commitments(root_path: String, year: i32, month: u32, commitments: Vec<Commitment>) → Result<Vec<Commitment>, String>
get_commitment_progress(root_path: String, year: i32, month: u32) → Result<Vec<CommitmentProgress>, String>
get_available_months(root_path: String) → Result<Vec<AvailableMonth>, String>  // 扫描有数据的年月，懒加载
reveal_day_file(root_path: String, date: String) → Result<(), String>  // 在文件管理器中打开目录并选中日文件
create_starter_files(path: String) → Result<(), String>  // 空目录创建初始文件
log_error(message: String)                              // 前端 error → error.log
log_info(message: String)                               // 前端 info → info.log
```

Phase 3 将新增：`get_stats(root_path: String, year: i32, month: u32) → MonthStats`

`validate_dimensions`、`validate_monthly`、`watch_files`、`resolve_month_dimensions`、`ensure_month_instantiated` 是内部函数，不暴露为命令。维度集合按月存放（见下）：每个月首次写入（append/update/delete/set_day_note/set_commitments 任一）时，`ensure_month_instantiated` 把 `template.yaml` 当时的维度快照进该月 `_monthly.md` 的 `dimensions:` 块（保留已有 commitments）；纯读不写。`set_commitments` 写回时只替换 commitments，保留 dimensions 块。Commitments 通过 `set_commitments(root_path, year, month, commitments)` 写入（校验 + goal 改名批量更新 entry + 原子写 `_monthly.md`；文件监听随后重新读取）；外部直接编辑 `_monthly.md` 仍由文件监听重新读取。校验：role 名非空且唯一、allocation > 0、goal 名非空且全局唯一、删除有 entry 引用的 goal 拒绝。`root_path` 由前端状态持有，每次调用时传入。

### 数据结构

```rust
// Template (template.yaml) — 全局默认维度集；新月份首次写入时快照进 _monthly.md
struct Template { dimensions: Vec<Dimension> }
struct Dimension {
    name: String,               // "Business line"
    key: String,                // "business-line"
    source: String,             // "static" (default) | "monthly"
    values: Option<Vec<String>>,  // source = "static" 时必填
    required: bool,             // false when absent (serde default)
}

// Monthly planning (_monthly.md) — dimensions 块按月存放（snapshot），空 = 未实例化
struct MonthlyFile {
    dimensions: Vec<Dimension>, // serde default 空；非空 ⟺ 该月已实例化
    commitments: Vec<Commitment>,
}

// 某月生效维度（get_month_dimensions 返回）
struct MonthDimensions {
    dimensions: Vec<Dimension>, // 月度块若非空，否则 template
    from_template: bool,        // true = 该月尚未实例化（展示的是模板预览）
}
struct Commitment {
    role: String,               // "Developer"
    allocation: u32,            // hours
    goals: Vec<String>,
}

// Entries
struct DayFile { note: Option<String>, entries: Vec<Entry> }
struct Entry {
    id: String,                  // UUID v4
    item: String,
    duration: u32,              // minutes
    dimensions: HashMap<String, String>,
}
struct CreateEntryInput {
    item: String,
    duration: String,           // 前端已扫描求和、去重片段、合并为总分钟数字符串（如 "60"）；Rust parse_duration 做最终转换
    dimensions: HashMap<String, String>,
}
struct UpdateEntryInput {
    item: Option<String>,
    duration: Option<String>,
    dimensions: Option<HashMap<String, String>>,
}

// Stats
struct MonthStats {
    year: i32, month: u32, total_minutes: u32,
    daily_totals: Vec<DailyTotal>,
    dimension_stats: HashMap<String, DimensionStats>,
    commitments: Vec<CommitmentStats>,
}
struct DailyTotal { day: u8, minutes: u32 }
struct DimensionStats { values: Vec<ValueStats> }
struct ValueStats { value: String, minutes: u32, percentage: f32 }
struct CommitmentStats {
    role: String,
    allocation: u32,            // minutes
    spent: u32,                 // minutes
    goals: Vec<GoalStats>,
}
struct GoalStats { goal: String, minutes: u32, percentage: f32, entries: Vec<EntrySummary> }
struct EntrySummary { date: String, item: String, duration: u32 }

// Config & monthly validation errors
struct ConfigErrorDetail {
    kind: String,      // "MissingName" | "MissingKey" | "MissingValues" | "ValuesEmpty"
    message: String,   // | "KeyInvalidChars" | "InvalidSource" | "MultipleMonthly"
                       // | "MissingRole" | "ZeroAllocation" | "DuplicateGoal"
                       // | "ParseError" | "ConfigReadError"
}

// Init result (serde tag = "status")
enum InitResult {
    NeedsSetup,
    ConfigError(Vec<ConfigErrorDetail>),
    Ready { root_path: String, dimensions: Vec<Dimension>, from_template: bool, today: DayFile, commitments: Vec<Commitment> },  // dimensions = 当前月生效维度
}

// Commitment progress (computed)
struct CommitmentProgress {
    role: String, allocation_minutes: u32, spent_minutes: u32,
    goals: Vec<GoalProgress>,
}
struct GoalProgress { name: String, spent_minutes: u32 }

// Available months (lazy-loaded for quick-jump popover)
struct AvailableMonth { year: i32, month: u32 }
```

### Duration 解析

```rust
fn parse_duration(input: &str) -> Result<u32, String>
// 全文扫描所有 duration pattern，求和
// "1.5h" → 90, "30m" → 30, "80" → 80, "2h" → 120
// "准备会议（15m），面聊（45m）" → 60
```

### 文件操作

- Day file: `{root_path}/{year}/{month:02}/{date}.md`，其中 `{date}` 为完整 ISO 日期如 `2026-06-07`
- Monthly file: `{root_path}/{year}/{month:02}/_monthly.md`（含按月 `dimensions:` 块 + `commitments:`）
- Template: `{root_path}/template.yaml`（全局默认维度集；旧 `config.yaml` 已弃用，不再读取）
- 写入: 先写 temp 再 rename（原子写入）
- Frontmatter: 定位 `---` 边界，`yaml_serde::from_str()` 解析中间内容
- 空文件返回空 DayFile，不存在自动创建

### Commitments 统计

Rust 端通过 Goal 维度关联：

1. 读 `_monthly.md` 拿到 `Vec<Commitment>`
2. 遍历当月所有 day files，按 `dimensions.goal` 聚合 duration
3. 每个 Goal 归属到 Role（Goal 在哪个 Commitment 的 goals 里就归属哪个 Role——Goal 名在 roles 之间保证唯一）
4. 算每个 Role 的 Spent、各 Goal 的占比

### 维度校验补充

- `validate_dimensions(&[Dimension])` 同时校验 `template.yaml` 与每月 `_monthly.md` 的 `dimensions:` 块
- 最多 1 个 `source: "monthly"` 的维度（当前只支持 Goal）
- `source: "monthly"` 的维度不检查 `values` 字段

### 月度维度模板（snapshot 语义）

- 某月生效维度 = 该月 `dimensions:` 块（非空）否则 `template.yaml`（`resolve_month_dimensions`，缺文件时容错返回空）
- 首次写入触发 `ensure_month_instantiated`：无 `dimensions:` 块时快照 template（保留 commitments），已实例化则 no-op
- 改 `template.yaml` 不回溯影响已实例化的月份；纯读（含 `get_month_dimensions`、`init`）不实例化
- 详见 `docs/superpowers/specs/2026-06-21-monthly-dimension-templates-design.md`

## 前端架构

### 组件树（Vue 3 SFC + `<script setup>`）

```
App.vue
├── SetupScreen.vue                     // 首次启动，folder picker
├── ConfigErrorBanner.vue
└── MonthView.vue                       // 固定月视图：左 1/3 月概览 + 右 2/3 日详情
    ├── MonthNavigator.vue              // ← → 箭头 + 快速跳转双下拉（年/月）
    ├── CommitmentsPanel.vue            // Allocation / Spent / Balance 进度条
    ├── DayStrip.vue                    // 横向滚动 1-31 日期条，蓝点标记有 entry 的日期
    ├── QuickEntry.vue
    │   ├── EntryInput.vue
    │   └── DimensionPanel.vue
    └── EntryList.vue → EntryItem.vue   // 始终 day 模式，底部内联合计行

// Phase 3（planned）:
// └── StatsView.vue
//     ├── TabBar.vue
//     ├── MonthSelector.vue
//     ├── MonthTotal.vue
//     ├── CommitmentsPanel.vue（复用）
//     ├── TrendChart.vue (Chart.js Bar)
//     ├── DonutChart.vue (Chart.js Doughnut)
//     └── EntryDetailPanel.vue
```

### 状态管理

`reactive()` + `provide/inject`。根组件创建 reactive store，`provide()` 注入子组件树。

### 图表

- **DonutChart**: Chart.js `DoughnutController` + `ArcElement`，点击扇区 emit 事件。所有维度统一使用此组件。
- **TrendChart**: Chart.js `BarController` + `BarElement` + `CategoryScale` + `LinearScale`。支持按维度 stack（下拉选维度 key）。

### 特殊处理

- Goal 维度：值列表不从 template/月度维度块取，从 Rust 端 `get_commitments` 返回的 goals 并集构建
- CommitmentsPanel：Today 页和 Stats 页复用同一组件。Today 页始终可见（录入框上方）
- DimensionPopover 键盘导航：`CTRL+N`/`CTRL+P` 或 `↑`/`↓` 移动高亮（循环），默认高亮第一个还没填 value 的维度（从 val 阶段返回时高亮下一个未填项）。popover 开启时 `Enter` 改为「选中当前高亮项」（dim 阶段进入值菜单 / val 阶段填值），不再提交 entry / 保存编辑；按 `Esc` 关闭 popover 后 `Enter` 恢复提交。`EntryComposer` 与 `EntryRowEdit` 复用同一 popover，行为一致。

## 数据流

- **启动**: App mount → `invoke('init')`。Rust 端读 `root_path.txt`：
  - 无文件 → 返回 `NeedsSetup` → 前端弹文件夹选择器 → `set_root_path` → 重新 init
  - config/_monthly.md 有错 → 返回 `ConfigError(errors)` → 前端显示 ErrorBanner
  - 正常 → 返回 `Ready { dimensions, from_template, today, commitments }`（dimensions = 当前月生效维度）→ 渲染 Today
- **文件监听**: Tauri `setup` hook 中启动 `notify` 线程，watch template.yaml + 当月 `_monthly.md`。变更时重新校验，emit `config-changed` 或 `commitments-changed` 事件推前端。
- **录入**: 用户输入 → 前端扫描全文 duration（regex 求和） → 去除匹配片段得到 item → 添加继承的维度 → `invoke('append_entry', ...)` → Rust 解析 duration 字符串为 u32，写文件 → 返回 Entry → 前端 refresh 列表 + Commitments
- **统计**: 切换月份 → `invoke('get_stats', ...)` → Rust 遍历月目录下所有 .md → 内存聚合（含 Commitments） → 返回 MonthStats → 更新图表

## 实现阶段

| Phase | 内容 |
|-------|------|
| 1 | 脚手架 + init/get_entries/append_entry/update_entry/delete_entry/set_day_note + Commitments 读 + Day note + Undo toast + Config 校验 + ErrorBanner + 固定月视图（MonthView/MonthNavigator/DayStrip） + 内联合计行 |
| 2 | 快速跳转双下拉（年/月） + 懒加载 get_available_months |
| 3 | get_stats + 所有图表（环形图 + 趋势 + Commitments） + 图表联动 |
| 4 | 键盘快捷键 + 动画 + 容错 |
