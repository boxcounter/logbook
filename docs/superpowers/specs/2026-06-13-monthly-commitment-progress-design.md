# Monthly Commitment Progress — Design Spec

> 将 CommitmentsPanel 从每日进度切换为月累计进度，支持月中跟踪和调整各 role 的时间分配。

## 问题

当前 CommitmentsPanel 把月 allocation 除以 20 得出每日预算，再跟当天 entries 对比。用户预期：每个 role 按月初设定的预算（如 Developer 6 月 30h），跟踪本月累计实际投入，判断自己是否在履行各角色职责，并在月中做出调整。

## 设计决策汇总

| 决策 | 选择 |
|------|------|
| 进度条目标线 | 固定月预算（不按 elapsed 折算） |
| 选中月份 | 跟随 DateNavigator 选中月份 |
| 计算位置 | Rust 端新增 command，不在前端聚合 |
| 手动编辑 day file 后的刷新 | 前端主动刷新 + 重启后生效，不扩展 watcher |
| 颜色方案 | 方案 B — 颜色参照时间进度，宽度参照固定预算 |
| 历史月份 elapsed | 已完成月份用 100% |

## 架构

### 数据流

```
TodayView
├── 从 DateNavigator 拿到当前选中日期 → 解析 year/month
├── invoke("get_commitment_progress", { rootPath, year, month })
│   └── Rust: 读 _monthly.md → 扫描当月所有 day files → 按 goal 聚合 → 按 role 汇总
├── 结果写入 store.commitmentProgress
├── 传给 CommitmentsPanel 渲染
└── 刷新时机:
    ├── init 后 → loadPeriod() → get_commitment_progress
    ├── entry 增删改成功后 → get_commitment_progress
    ├── _monthly.md watcher 变更 → emit 事件 → 重新加载（已有）
    └── day files 手动编辑 → 不实时更新，重启后生效
```

### Rust 端

**新增 command（`commands.rs`）：**

```rust
#[tauri::command]
pub fn get_commitment_progress(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<Vec<CommitmentProgress>, String>
```

**新增数据结构（`models.rs`）：**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentProgress {
    pub role: String,
    pub allocation_minutes: u32,   // 月预算，小时 × 60
    pub spent_minutes: u32,        // 本月累计已用分钟
    pub goals: Vec<GoalProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalProgress {
    pub name: String,
    pub spent_minutes: u32,
}
```

**计算逻辑：**
1. 读 `_monthly.md` 获取 `Vec<Commitment>`
2. 遍历当月目录下所有 day files（`{year}/{month:02}/*.md`，排除 `_monthly.md`）
3. 对每个 entry 的 `dimensions.goal`，匹配到对应 Commitment 的 `goals` 列表 → 归属到对应 role
4. `allocation_minutes = commitment.allocation * 60`
5. 无法匹配到任何 role 的 goal entry 忽略

### 前端

**TypeScript 类型（`types.ts`）：**

```typescript
export interface CommitmentProgress {
  role: string;
  allocation_minutes: number;
  spent_minutes: number;
  goals: GoalProgress[];
}

export interface GoalProgress {
  name: string;
  spent_minutes: number;
}
```

**Store 新增字段（`useStore.ts`）：**

```typescript
commitmentProgress: CommitmentProgress[]
```

**CommitmentsPanel 改造：**

- Props: `commitments: Commitment[]` + `entries: Entry[]` → `progress: CommitmentProgress[]`
- 删除内部聚合逻辑（`WORKING_DAYS_PER_MONTH`、`dailyAllocation`、goals spent 计算）
- 直接使用 `progress` 中已聚合好的 `allocationMinutes`、`spentMinutes`、`goals`

**颜色函数（替代 `barColor`）：**

```typescript
function barColor(spent: number, alloc: number, selectedYear: number, selectedMonth: number): string {
  if (alloc === 0) return "bg-gray-300";

  const spentRatio = spent / alloc;

  // 超预算 → 红（最高优先级）
  if (spentRatio > 1) return "bg-red-500";

  const elapsed = elapsedRatio(selectedYear, selectedMonth);

  // 颜色参照时间进度，宽度参照固定预算
  if (spentRatio < elapsed * 0.6) return "bg-orange-500";  // 显著落后于时间进度
  if (spentRatio > elapsed * 1.4) return "bg-yellow-500";  // 超前消耗
  return "bg-green-500";                                    // 节奏正常
}

function elapsedRatio(year: number, month: number): number {
  const now = new Date();
  const isCurrentMonth = year === now.getFullYear() && month === now.getMonth() + 1;
  if (isCurrentMonth) {
    const daysInMonth = new Date(year, month, 0).getDate(); // month is 1-based here
    return now.getDate() / daysInMonth;
  }
  return 1.0; // 历史月或未来月
}
```

判定逻辑表：

| 条件 | 颜色 | 信号含义 |
|------|------|----------|
| spent > allocation | 红 | 已超预算 |
| spend% < elapsed% × 0.6 | 橙 | 投入显著不足，需要补 |
| spend% 在 [elapsed%×0.6, elapsed%×1.4] | 绿 | 节奏正常 |
| spend% > elapsed% × 1.4 且未超 | 黄 | 超前消耗，可能超标 |

**TodayView 改动：**
- `loadPeriod()` 之后调用 `get_commitment_progress`，结果写入 store
- entry 增删改成功后刷新 commitment progress

**CommitmentsPanel 模板渲染（不变结构）：**

```
Developer                            12.5h / 30.0h
████████████░░░░░░░░░░░░░░  42% 橙
  Slax Reader MVP              5.2h
  Code Review                  3.8h
  Team 1:1                     2.0h
  Hiring                       1.5h
```

- 角色名 + `spent / allocation`（小时显示）
- 进度条宽度 = `spent / allocation`，颜色按上方判定
- 下方 goals 列表，0h 的目标灰色显示

### 边界情况

- 当月无 `_monthly.md` 或 commitments 为空 → 不渲染面板（和现在一致）
- 某 role 无 goal 匹配到任何 entry → 进度条 0%，颜色按 elapsed 判定
- allocation 为 0 → 灰色条（防御性处理）

## 不涉及

- 不修改 `Commitment` 数据结构（`allocation` 始终是 hours/month）
- 不新增 `set_commitments` command
- 不扩展文件 watcher 覆盖 day files
- 不影响 Phase 3 的 `get_stats`（CommitmentStats 可后续与 CommitmentProgress 合并）
