# Role 维度 — 设计规格

> 问题：无 goal 的日常耗时不计入 role spent → balance 始终等于 allocation → 虚假剩余时间。

## 数据模型

### Entry.dimensions 新增 role key

```
dimensions: {
    role: "Developer",       // 可选，新增
    goal: "Ship feature X",  // 可选，现有
}
```

role 的值来源为 `commitments.yaml` 中声明的 role 列表（动态，按月存放），不在 `dimensions.template.yaml` 的 `dimensions` 里声明。语义和 goal 的 `source: "monthly"` 相同。

### Entry 新增 attribution 字段

```rust
enum Attribution {
    Ok,            // 正常归属
    Unattributed,  // 无 role 且无 goal（或 goal 未声明）
    Mismatch,      // 有 role 有 goal，但 goal 不在该 role 下声明
}
```

后端读 entry 时（结合当月 commitments 的 goal→role 映射）判定并填入。前端 `EntryRow` 直接读此字段决定 amber 标记。`serde` 序列化为 `"ok"` / `"unattributed"` / `"mismatch"` 小写字符串传给前端。

## 归属规则

```
// role spent = 显式 role 维度 + 隐式推导（有 goal 无 role 时，goal → role 映射）
role_spent[role] = sum(entries with dimensions.role == role)
                 + sum(entries without role dim, but with dimensions.goal that maps to role)
goal_spent[goal]  = sum(entries with dimensions.goal == goal)  // 不变
```

| entry 状态 | role spent | bar segment | EntryRow 标记 |
|-----------|-----------|-------------|--------------|
| `role: Dev`，goal 匹配 Dev 下声明的 goal | 归入 Dev | dark（goal 耗时） | 无 |
| `role: Dev`，无 goal 或 goal 不匹配 Dev 的声明 | 归入 Dev | light（general 耗时） | mismatch 时 amber |
| 无 role，`goal: Ship X`（已在 Dev 下声明） | 归入 Dev（fallback） | dark | 无 |
| 无 role，无 goal（或 goal 未声明） | 未归属 | — | amber |

## Command API 变更

### Entry — 携带归属状态

`Entry` 新增 `attribution` 字段，后端读 entry 时（结合当月 commitments）直接判定：

```rust
struct Entry {
    id: String,
    item: String,
    duration: u32,
    dimensions: HashMap<String, String>,
    attribution: Attribution,  // 新增
}

enum Attribution {
    Ok,            // 正常归属（有 role，或通过 goal → role 映射）
    Unattributed,  // 无 role 且无 goal（或 goal 未声明）
    Mismatch,      // 有 role 有 goal，但 goal 不在该 role 下声明
}
```

`EntryRow` 直接读 `entry.attribution` 决定 amber 标记，不跨层级拼接。`DayFile` 保持不变（不新增字段）。

### `get_commitment_progress` 返回值

从 `Vec<CommitmentProgress>` 改为包装结构，提供 warning bar 所需的月级汇总：

```rust
struct CommitmentProgressResult {
    roles: Vec<CommitmentProgress>,
    unattributed_count: u32,         // warning bar 用
    unattributed_total_minutes: u32,
    mismatch_count: u32,
}

struct CommitmentProgress {
    role: String,
    allocation_minutes: u32,
    goal_spent_minutes: u32,     // 新增：有匹配 goal 的部分（dark 段）
    general_spent_minutes: u32,  // 新增：无匹配 goal 的部分（light 段）
    goals: Vec<GoalProgress>,    // 不变
}
// 前端 spent = goal_spent_minutes + general_spent_minutes
```

## 前端 UI

### CommitmentsPanel

- 进度条拆两段：dark 段（`--color-brand-gradient-*`）= goal 耗时，light 段（brand 浅色变体 `#c4b5fd → #ddd6fe`）= general 耗时
- bar 下方加图例（Goal / General 各一小色块）
- 底部 warning bar（仅当存在未归属或 mismatch 时显示）：
  ```
  ⚠ 未归属耗时：1.5h / role/goal 不匹配：2 条
  ```
- bar hover tooltip 显示 goal/general 各自具体值

### EntryRow

未归属 entry 和 mismatch entry 共用同一套视觉标记：
- 行首 amber 圆点（`●`，`color: #d97706`）
- 行背景微黄（`background: #fffbeb`，hover 时 `#fef3c7`）
- duration 数字用 amber 色（`color: #d97706`）
- 不阻塞交互：仍可双击编辑、删除

### DimensionPopover

- dim 阶段：维度列表中出现 `Role`（仅当有 commitments 声明时）
- val 阶段：展示当前月 commitments 中声明的 role 名，单选，键盘导航
- **交叉过滤**：
  - 已选 `role: Dev` 后再选 goal：val 阶段只展示 Dev role 下声明的 goals
  - 已选 `goal: Ship X` 后再选 role：role 列表只展示包含该 goal 的 role
  - 杜绝运行时产生 role-goal 不匹配组合

## 数据校验

`init` 阶段扫描当月 entry，检测 role-goal mismatch（防御旧数据残留、外部编辑、边缘 bug）。

- 扫描到 mismatch → 计入 warning bar（「role/goal 不匹配：N 条」）
- mismatch entry 使用 amber ● 标记（和未归属 entry 同一套视觉语言）
- **不自动纠正**：用户看到标记后手动编辑修复（popover 交叉过滤已生效）

## 边界情况

- **旧 entry 兼容**：无 `role` 维度的旧 entry 继续通过 `goal → role` 映射计入 spent
- **role 改名**：`set_commitments` 中扫描当月 entry 的 `dimensions.role`，匹配旧名则替换为新名（复用现有 goal 改名的批量更新逻辑）
- **role 删除**：清除对应 entry 的 `role` 维度，entry 退回规则 3（可能变成未归属，amber 标记提醒用户处理）
- **外部编辑**：file watcher 重新读取时走新归属规则，`init` 扫描捕捉 mismatch
- **Phase 3 `get_stats`**：`CommitmentStats` 同理新增 `goal_spent_minutes` / `general_spent_minutes`，`MonthStats` 增加 `unattributed_*` / `mismatch_*` 字段。本 spec 不阻塞 Phase 3，但为它预留了扩展点。
