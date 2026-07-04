# 移除 attribution，用 required dimension + 跨维度约束取代

> 诊断：attribution 机制（`Unattributed`/`Mismatch` amber 标记）本质是在补维度系统没有 `required` 和跨维度值约束的缺。如今两个能力都已具备，attribution 成为冗余。
>
> 取代 `2026-07-01-role-dimension-design.md` 中的 attribution 部分（`Attribution` 枚举、amber 标记、warning bar）。role 维度、交叉过滤、CommitmentProgress 两段 bar 等其余内容保留不变。

## 核心变更

用两样取代 attribution：

1. **role `required: true`**（`dimensions.template.yaml`）—— 拦截"没填 role"
2. **跨维度约束校验**（新增 `validate_cross_dimension_constraints`）—— 拦截"role + goal 不匹配"（CLI 场景）

## 删除清单

### Rust 后端

| 位置 | 删除内容 |
|------|---------|
| `models.rs` | `Attribution` 枚举（含 `Default` impl） |
| `models.rs` | `Entry.attribution` 字段 |
| `commands.rs` | `compute_attribution` 函数 + 所有单元测试 |
| `commands.rs` | `annotate_day_file` 函数 |
| `commands.rs` | 所有 attribution 注入调用点（`load_root_state`、`get_entries`、`append_entry`、`update_entry`、`delete_entry` 等约 8 处） |
| `commands.rs` | `CommitmentProgressResult` wrapper struct（只剩 `roles` 一个字段，改为直接返回 `Vec<CommitmentProgress>`） |
| `commands.rs` | `get_commitment_progress` 中 `Unattributed` / `Mismatch` 分支 |
| `files.rs` | 所有 `attribution: Attribution::default()` 赋值（约 7 处） |
| `operation_log.rs` | 所有 `attribution: Attribution::default()` 赋值（约 2 处） |
| `tests/operation_log_integration.rs` | `attribution: Attribution::default()` 赋值 |

### 前端

| 位置 | 删除内容 |
|------|---------|
| `types.ts` | `Attribution` 类型定义 |
| `types.ts` | `Entry.attribution` 字段 |
| `types.ts` | `CommitmentProgressResult` 中 `unattributed_count`、`unattributed_total_minutes`、`mismatch_count` |
| `EntryRow.vue` | `isProblemEntry` computed + amber 标记样式（行背景、圆点、duration 颜色） |
| `CommitmentsPanel.vue` | warning bar 整块 UI + `warningUnattributedMinutes` / `warningMismatchCount` computed props + 对应 props 定义 |
| `tokens.css` | `--color-problem-entry-*` 和 `--color-warning-bar-*` token |
| `mocks/fixtures.ts` | `attribution: "ok"` 默认值 |

### 配置

- `dimensions.template.yaml`：`Role` 加 `required: true`

### 配置（测试 fixture）

- `src-tauri/tests/fixtures/dimensions.template.yaml`：同步加上 `required: true`

## 新增：跨维度约束校验

### 函数签名

```rust
fn validate_cross_dimension_constraints(
    dimensions: &BTreeMap<String, String>,
    role_key: &str,
    goal_key: &str,
    role_to_goals: &HashMap<String, Vec<String>>,
) -> Result<(), String>
```

`role_key` / `goal_key` 由调用方从当月 dimensions 中解析（查找 `source: commitments:role` / `source: commitments:goals` 的维度 key），`role_to_goals` 由调用方从 `commitments.yaml` 构建。

### 校验规则

- 仅当 entry **同时有** role 和 goal 时才执行
- role 不在 `role_to_goals` 中（该 role 未声明任何 goal）→ 放行
- goal 不在该 role 的 goals 列表中 → 返回 `Err("Goal '<goal>' is not declared under role '<role>'")`

### 调用点

与 `validate_required_dimensions` 串行，在 `append_entry`、`update_entry`、CLI add 中调用。调用方负责读取 commitments 并构建映射。

### GUI 防护

GUI 中 DimensionPopover 已有交叉过滤（选 role 后 goal 候选集只显示该 role 下的 goals），此校验在 GUI 路径中永远不触发。它是 CLI 的防御线。

## 测试

| 类型 | 内容 |
|------|------|
| Rust 新增 | `validate_cross_dimension_constraints` 单元测试（同 role+goal 放行、不匹配 reject、无 goal 放行、role 不在 role_to_goals 中放行） |
| Rust 删除 | `compute_attribution` 所有单元测试 |
| Rust 修改 | 集成测试中 `Attribution::default()` 赋值移除，`required: true` fixture 下的集成测试 |
| 前端删除 | `EntryComposer` / `EntryRowEdit` 中 attribution/amber 相关测试用例 |
| 前端删除 | `fixtures.ts` 中 `attribution` mock 默认值 |

## CommitmentProgress 计算简化

`get_commitment_progress` 不再需要三态分支：

- 所有 entry 必有 role（`required: true` 保证）→ 全部计入 role progress
- goal 可选，有 goal 的额外按 goal 聚合
- 去除 `unattributed_count`、`unattributed_total_minutes`、`mismatch_count` 字段

`CommitmentsPanel` warning bar 移除。Panel 仅保留 Allocation / Spent / Balance 进度条。

## 数据迁移

不需要。`attribution` 不在 markdown 落盘格式中，是读时计算注入的字段。删除后，旧月份 entry 以无 amber 的方式渲染——`required: true` 只在写入时生效，读旧数据不报错。

## 边界情况

| 情况 | 行为 |
|------|------|
| entry 无 role | `validate_required_dimensions` hard reject，返回 `"Missing required dimension: Role"` |
| entry 有 role，goal 不匹配（CLI） | `validate_cross_dimension_constraints` hard reject |
| 旧月份 entry 无 role | 正常渲染，无 amber（amber 已删），不报错 |
| role 在该月被标记为 `deleted` | `deleted` 维度被 `validate_required_dimensions` 跳过（现有逻辑），不校验 |
| commitments 未声明 role/goal 映射 | `role_to_goals` 为空 → 跨维度校验无约束 → 放行 |
