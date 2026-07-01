# Role 维度 — 设计规格

> 问题：无 goal 的日常耗时不计入 role spent → balance 始终等于 allocation → 虚假剩余时间。

## 数据模型

Entry 的 `dimensions` 新增可选 key `role`。值和 Commitment 中声明的 role 名对应。不新增 Entry 一级字段。

```
dimensions: {
    role: "Developer",       // 可选，新增
    goal: "Ship feature X",  // 可选，现有
}
```

role 的值来源为 `_monthly.md` 的 commitments 声明的 role 列表（动态），不在 `template.yaml` 的 `dimensions` 里声明。语义和 goal 的 `source: "monthly"` 相同。

## 归属规则

```
role_spent[role] = sum(entries with dimensions.role == role)
goal_spent[goal]  = sum(entries with dimensions.goal == goal)  // 不变
```

| entry 状态 | role spent | bar segment | EntryRow 标记 |
|-----------|-----------|-------------|--------------|
| `role: Dev`，goal 匹配 Dev 下声明的 goal | 归入 Dev | dark（goal 耗时） | 无 |
| `role: Dev`，无 goal 或 goal 不匹配 Dev 的声明 | 归入 Dev | light（general 耗时） | mismatch 时 amber |
| 无 role，`goal: Ship X`（已在 Dev 下声明） | 归入 Dev（fallback） | dark | 无 |
| 无 role，无 goal（或 goal 未声明） | 未归属 | — | amber |

## Command API 变更

### `get_commitment_progress` 返回值

从 `Vec<CommitmentProgress>` 改为一个包装结构，同时返回 role 统计和问题条目信息：

```rust
struct CommitmentProgressResult {
    roles: Vec<CommitmentProgress>,
    unattributed_count: u32,       // 未归属 entry 数
    unattributed_total_minutes: u32,
    mismatch_count: u32,           // role-goal 不匹配 entry 数（仅计数，时间已计入对应 role）
    mismatch_entry_ids: Vec<String>,  // 前端据此在 EntryRow 上加 amber 标记
    unattributed_entry_ids: Vec<String>,
}

struct CommitmentProgress {
    role: String,
    allocation_minutes: u32,
    spent_minutes: u32,          // role spent 总计（= goal + general）
    goal_spent_minutes: u32,     // 新增：有匹配 goal 的部分
    general_spent_minutes: u32,  // 新增：无匹配 goal 的部分
    goals: Vec<GoalProgress>,    // 不变
}
```

### `init` 也返回问题 entry ID

`InitResult::Ready` 中增加 `unattributed_entry_ids` 和 `mismatch_entry_ids`（当月范围），和 `ScanWarning` 平级。前端 store 持有这两组 ID，EntryRow 通过 `Set` 查询是否应该显示 amber 标记。

启动后每次 `get_commitment_progress` 调用也会更新这组 ID（应对用户编辑后问题消除或新增的场景）。

如果需要展示历史月份的问题 entry，`get_month_dimensions` 不需要；历史月份只在 `get_commitment_progress` 被调用时（比如用户切换到那个月看 Stats）才检测。

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
