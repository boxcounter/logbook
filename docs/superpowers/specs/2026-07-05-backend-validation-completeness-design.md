# Backend Validation Completeness

**日期**：2026-07-05
**状态**：draft
**原则**：`src-tauri/AGENTS.md` 关键约定 — "后端是数据合法性检查的唯一权威源"

## 动机

Logbook 有两个前端（GUI + CLI），当前部分数据合法性检查仅在前端执行，CLI 可绕过，导致无效数据落盘。本设计补齐后端缺失的三项校验，并抽取统一的 entry 校验入口以消除 `append_entry` 和 `update_entry` 的校验逻辑重复。

## 盘点结论

后端已有覆盖度较高的校验层（duration 格式、日期格式、必填维度、跨维度约束、配置合法性、完整性诊断等），但存在三个缺口：

| # | 缺口 | 严重度 | 当前状况 |
|---|------|--------|---------|
| 1 | **写入前未知维度 key 不拦截** | 中 | 前端 `sanitizeValues` 丢弃未知 key；后端写后诊断才检测；CLI 可注入任意 key |
| 2 | **空 item 不拒绝** | 低 | 前端 `EntryComposer` 硬阻断空 item；后端不校验 |
| 3 | **静态维度值中的空字符串不过滤** | 低 | 前端 `DimensionEditorModal.save()` 过滤空字符串；后端仅检查 values 数组非空，不检查单个值 |

## 设计

### 1. 统一 entry 校验函数

在 `commands.rs` 新增 `validate_entry_input`，合并 `append_entry` 和 `update_entry` 的 entry 级校验：

```rust
/// Unified pre-write validation for entry input.
/// Returns parsed duration (u32 minutes) on success.
pub fn validate_entry_input(
    item: &str,
    duration_str: &str,
    dimensions: &BTreeMap<String, String>,
    dimension_config: &[Dimension],
    role_key: &str,
    goal_key: &str,
    role_to_goals: &HashMap<String, Vec<String>>,
) -> Result<u32, String>
```

校验链（按序）：

1. `item.trim().is_empty()` -> `Err("Entry item cannot be empty")`
2. `parse_duration(duration_str)` -> 沿用现有错误
3. 遍历 `dimensions` 的 keys，任一不在 `dimension_config` 的 key 集合中（含 `deleted: true` 的维度） -> `Err("Unknown dimension key '{}'")`
4. `validate_required_dimensions(dimension_config, dimensions)` -> 沿用现有（内部跳过 deleted 维度）
5. `validate_cross_dimension_constraints(dimensions, role_key, goal_key, role_to_goals)` -> 沿用现有

### 2. 调用点改动

**`append_entry`**：删除分散的 `parse_duration`、`validate_required_dimensions`、`validate_cross_dimension_constraints` 调用，替换为一次 `validate_entry_input(...)` 调用，直接获得 duration 值。

**`update_entry`**：按字段有条件调用校验（`UpdateEntryInput` 字段全为 Option）：

- `update.item` 为 Some 时检查 `item.trim().is_empty()`
- `update.duration` 为 Some 时调用 `parse_duration`
- `update.dimensions` 为 Some 时检查未知 key + required dims + cross-dimension

`update_entry` 不走 `validate_entry_input`（缺少完整输入），但复用的底函数相同。

### 3. `validate_dimensions` 空白值补齐

在 `config.rs` `validate_dimensions` 现有的 `values.is_empty()` 检查后追加：

```rust
Some(vals) if vals.iter().any(|v| v.trim().is_empty()) => {
    errors.push(ConfigErrorDetail {
        kind: "ValuesEmpty".to_string(),
        message: format!(
            "Dimension '{}' (key: {}): values list contains an empty or whitespace-only entry",
            dim.name, dim.key
        ),
    });
}
```

复用现有 `ValuesEmpty` kind，语义延伸（空列表 / 含空字符串均属值缺失）。

### 4. 错误处理策略

所有三项均为**硬拒绝**（返回 `Err`），与前端静默丢弃行为不同。理由：

- CLI 用户需要明确知道输入了什么错误数据
- 后端作为唯一权威校验层，不应静默丢弃任何字段（见 `src-tauri/AGENTS.md` 关键约定）
- 前端已有自己的预处理层，不会发送无效数据到后端

## 测试

### 单元测试（`commands.rs` `#[cfg(test)]`）

| 测试 | 覆盖 |
|------|------|
| `validate_entry_input_rejects_empty_item` | `""` / `"  "` -> Err |
| `validate_entry_input_rejects_unknown_key` | dimensions 含未在 dimension_config 中定义的 key -> Err |
| `validate_entry_input_ok` | 合法输入全链路通过，返回正确 parsed duration |
| `validate_entry_input_duration_fail` | duration 字符串无效 -> Err（验证第 2 步仍被调用） |

### 单元测试（`config.rs` `#[cfg(test)]`）

| 测试 | 覆盖 |
|------|------|
| `validate_dimensions_rejects_empty_value_string` | `values: Some(vec!["ok".into(), "".into()])` -> error 列表含 `ValuesEmpty` |

### 集成测试（`tests/cli_integration.rs`）

| 测试 | 场景 |
|------|------|
| `cli_entries_add_rejects_empty_item` | stdin `{"item":"","duration":"1h","dimensions":{}}` -> exit code != 0 |
| `cli_entries_add_rejects_unknown_dim_key` | stdin dimensions 含未定义 key -> exit code != 0 |
| `cli_dimensions_set_rejects_empty_value_string` | stdin 静态维度含空字符串 -> exit code != 0 |

## 影响面

- **GUI**：零影响。前端 `sanitizeValues` 和 UI 硬阻断已覆盖这些场景。
- **CLI**：新增三个硬拒绝，正向变更。
- **已有数据**：无 breaking change。`validate_dimensions` 空字符串检查仅写入时触发，已有落盘文件读取不受影响。
- **向后兼容**：之前被静默接受的无效输入（空 item、未知 key、含空字符串的静态值）现在返回错误。
