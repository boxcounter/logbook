# Logbook — Technical Spec

> 关联设计文档：Vault `1_Projects/Logbook/README.md`

## 技术栈

| 层 | 选型 |
|---|------|
| 桌面框架 | Tauri 2.x |
| 后端 | Rust (`#[tauri::command]` + `serde_yaml`) |
| 文件监听 | `notify` crate |
| 前端 | Vue 3 + Composition API + TypeScript |
| 样式 | Tailwind CSS |
| 图表 | Chart.js（按需引入：Doughnut + Bar controllers） |
| Frontmatter 解析 | 手动提取 `---` 边界 + `serde_yaml`（格式简单，不引入额外 crate） |

## Rust 后端

### 命令清单

```
init(app: AppHandle) → InitResult
set_root_path(app: AppHandle, path: String) → Result<InitResult, String>
get_entries(root_path: String, date: String) → DayFile
append_entry(root_path: String, date: String, entry: NewEntry) → Entry
update_entry(root_path: String, date: String, entry_id: String, update: UpdateEntry) → DayFile
delete_entry(root_path: String, date: String, entry_id: String) → DayFile
set_day_note(root_path: String, date: String, note: String) → DayFile
get_stats(root_path: String, year: i32, month: u32) → MonthStats
get_commitments(root_path: String, year: i32, month: u32) → Vec<Commitment>
```

`validate_config`、`validate_monthly`、`watch_files` 是内部函数，通过 `init` 和 Tauri `setup` hook 调用，不暴露为命令。Commitments 通过直接编辑 `_monthly.md` 文件写入（文件监听自动重新读取），不提供 `set_commitments` 命令。`root_path` 由前端状态持有，每次调用时传入。

### 数据结构

```rust
// Config
struct Config { dimensions: Vec<Dimension> }
struct Dimension {
    name: String,               // "Business line"
    key: String,                // "business-line"
    source: String,             // "static" (default) | "monthly"
    values: Option<Vec<String>>,  // source = "static" 时必填
}

// Monthly planning
struct MonthlyFile {
    commitments: Vec<Commitment>,
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
struct NewEntry {
    item: String,
    duration: String,           // 前端已扫描求和、去重片段、合并为总分钟数字符串（如 "60"）；Rust parse_duration 做最终转换
    dimensions: HashMap<String, String>,
}
struct UpdateEntry {
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

// Config validation
enum ConfigError {
    DimensionsNotArray,
    MissingName { index: usize },
    MissingKey { index: usize },
    MissingValues { index: usize },       // source = "static" 且无 values
    ValuesNotArray { index: usize },
    ValuesEmpty { index: usize },
    KeyInvalidChars { index: usize, key: String },
    InvalidSource { index: usize, found: String },
}
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
- Monthly file: `{root_path}/{year}/{month:02}/_monthly.md`
- Config: `{root_path}/config.yaml`
- 写入: 先写 temp 再 rename（原子写入）
- Frontmatter: 定位 `---` 边界，`serde_yaml::from_str()` 解析中间内容
- 空文件返回空 DayFile，不存在自动创建

### Commitments 统计

Rust 端通过 Goal 维度关联：

1. 读 `_monthly.md` 拿到 `Vec<Commitment>`
2. 遍历当月所有 day files，按 `dimensions.goal` 聚合 duration
3. 每个 Goal 归属到 Role（Goal 在哪个 Commitment 的 goals 里就归属哪个 Role——Goal 名在 roles 之间保证唯一）
4. 算每个 Role 的 Spent、各 Goal 的占比

### Config 校验补充

- 最多 1 个 `source: "monthly"` 的维度（当前只支持 Goal）
- `source: "monthly"` 的维度不检查 `values` 字段

## 前端架构

### 组件树（Vue 3 SFC + `<script setup>`）

```
App.vue
├── TabBar.vue
├── SetupScreen.vue                     // 首次启动，folder picker
├── ConfigErrorBanner.vue
├── TodayView.vue
│   ├── DateNavigator.vue
│   ├── CommitmentsPanel.vue            // Allocation / Spent / Balance 进度条
│   ├── QuickEntry.vue
│   │   ├── EntryInput.vue
│   │   └── DimensionPanel.vue
│   ├── EntryList.vue → EntryItem.vue
│   └── SummaryBar.vue
└── StatsView.vue
    ├── MonthSelector.vue
    ├── MonthTotal.vue
    ├── CommitmentsPanel.vue            // 同上组件，可复用
    ├── TrendChart.vue (Chart.js Bar)
    ├── DonutChart.vue (Chart.js Doughnut)   // 每个维度一张
    └── EntryDetailPanel.vue
```

### 状态管理

`reactive()` + `provide/inject`。根组件创建 reactive store，`provide()` 注入子组件树。

### 图表

- **DonutChart**: Chart.js `DoughnutController` + `ArcElement`，点击扇区 emit 事件。所有维度统一使用此组件。
- **TrendChart**: Chart.js `BarController` + `BarElement` + `CategoryScale` + `LinearScale`。支持按维度 stack（下拉选维度 key）。

### 特殊处理

- Goal 维度：值列表不从 config 取，从 Rust 端 `get_commitments` 返回的 goals 并集构建
- CommitmentsPanel：Today 页和 Stats 页复用同一组件。Today 页始终可见（录入框上方）

## 数据流

- **启动**: App mount → `invoke('init')`。Rust 端读 `root_path.txt`：
  - 无文件 → 返回 `NeedsSetup` → 前端弹文件夹选择器 → `set_root_path` → 重新 init
  - config/_monthly.md 有错 → 返回 `ConfigError(errors)` → 前端显示 ErrorBanner
  - 正常 → 返回 `Ready { config, today, commitments }` → 渲染 Today
- **文件监听**: Tauri `setup` hook 中启动 `notify` 线程，watch config.yaml + 当月 `_monthly.md`。变更时重新校验，emit `config-changed` 或 `commitments-changed` 事件推前端。
- **录入**: 用户输入 → 前端扫描全文 duration（regex 求和） → 去除匹配片段得到 item → 添加继承的维度 → `invoke('append_entry', ...)` → Rust 解析 duration 字符串为 u32，写文件 → 返回 Entry → 前端 refresh 列表 + Commitments
- **统计**: 切换月份 → `invoke('get_stats', ...)` → Rust 遍历月目录下所有 .md → 内存聚合（含 Commitments） → 返回 MonthStats → 更新图表

## 实现阶段

| Phase | 内容 |
|-------|------|
| 1 | 脚手架 + init/get_entries/append_entry/update_entry/delete_entry/set_day_note + Commitments 读 + Today day 粒度 + Day note + Undo toast + Config 校验 + ErrorBanner |
| 2 | Week/Month 粒度 |
| 3 | get_stats + 所有图表（环形图 + 趋势 + Commitments） + 图表联动 |
| 4 | 键盘快捷键 + 动画 + 容错 |
