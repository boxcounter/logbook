# Spec: 维度 source 可配置 + 删除 _monthly.md

日期: 2026-07-03

---

## 1. 背景

- `source: "monthly"` 命名模糊，不表达 "值来自 commitments" 的语义
- Role 维度 key 硬编码为 `"role"`，用户无法自定义
- `_monthly.md` 是历史遗留文件，维度/commitments 已分别迁至 `dimensions.yaml` / `commitments.yaml`，`_monthly.md` 只读不写
- `from_template` 命名宽泛，不精确表达 "维度来自全局默认模板"

---

## 2. 设计目标

1. `source` 值改名：`"monthly"` → `"commitments:goals"`，新增 `"commitments:role"`
2. Role 维度通过 template 的 `source: "commitments:role"` 定位，key 可自定义
3. 删除 `_monthly.md` 及其所有读写逻辑
4. `from_template` → `usingDefaultDimensions`

---

## 3. 架构变更

### 3.1 维度 source 值

`dimensions.template.yaml` 中 `source` 字段合法值：

| 值 | 语义 | 约束 |
|---|---|---|
| `"static"` | 值在 template 中预定义 | 需 `values` 列表 |
| `"commitments:goals"` | 值来自 commitments 的 goals 字段 | 最多一个 |
| `"commitments:role"` | 值来自 commitments 的 role 字段 | 最多一个 |

无存量兼容路径。存量月份 `dimensions.yaml` 中 `source: "monthly"` 需手动迁移。

### 3.2 维度 key 动态解析

新增 `role_dim_key(root, year, month) -> String`，镜像已有的 `goal_dim_key()`（由 `monthly_dim_key()` 重命名）：

- 从 `resolve_month_dimensions` 返回的维度数组中找 `source == "commitments:role"` 的维度
- 返回其 `key`，无此维度时 fallback `"role"`
- 同理 `goal_dim_key()` 找 `source == "commitments:goals"`，fallback `"goal"`

### 3.3 `usingDefaultDimensions` 判定

**旧**：读 `_monthly.md` → 检查 `dimensions` 字段是否为空

**新**：检查 `dimensions.yaml` 是否存在且非空

```rust
let monthly_dims = read_dimensions_file(root, year, month).unwrap_or_default();
let usingDefaultDimensions = monthly_dims.is_empty();
```

语义：`true` = 该月尚未实例化，维度来自 `dimensions.template.yaml`；`false` = 该月有自己的维度快照。

### 3.4 `_monthly.md` 删除

删除 `MonthlyFile` struct。以下函数全部删除：

| 函数 | 文件 |
|------|------|
| `read_monthly_file` | files.rs |
| `write_monthly_file` | files.rs |
| `monthly_path` | files.rs |
| `ensure_month_instantiated` | files.rs |
| `read_monthly_file_safe` | commands.rs |
| `validate_monthly` | config.rs |

`month_from_monthly_path` 重命名为 `extract_year_month`（仅 watcher 内部使用，从路径提取年/月）。

---

## 4. Rust 后端改动

### 4.1 models.rs

- `Dimension.source` 注释更新（合法值列表）
- 删除 `MonthlyFile` struct
- `MonthDimensions.from_template` → `usingDefaultDimensions`
- `InitResult::Ready.from_template` → `usingDefaultDimensions`

### 4.2 config.rs

`validate_dimensions`：
- `monthly_count` → `goal_source_count`；新增 `role_source_count`
- `"monthly"` match arm → `"commitments:goals"`
- 新增 `"commitments:role"` match arm
- Error kind `"MultipleMonthly"` → `"MultipleGoalSource"`
- Invalid source 错误消息中合法值列表更新

删除：
- `validate_monthly` 函数
- `month_from_monthly_path` 重命名为 `extract_year_month`

Watcher `commitments-changed` handler：删除合成 `MonthlyFile` 的代码，`validate_monthly(&monthly)` → `validate_commitments(&commitments)`

### 4.3 commands.rs

新增：
- `role_dim_key(root, year, month) -> String`

重命名：
- `monthly_dim_key()` → `goal_dim_key()`
- 内部查找 `source == "commitments:goals"`

签名变更：
- `compute_attribution(dimensions, role_key, goal_key, goal_to_role, role_to_goals)`
- `annotate_day_file(day_file, role_key, goal_key, goal_to_role, role_to_goals)`

调用点更新（6 处）：
- `load_root_state`（line ~275）
- `get_entries`（line ~398）
- `append_entry`（line ~452）
- `update_entry`（line ~513）
- `delete_entry`（line ~563）
- `get_commitment_progress` 内部调用（line ~789）

硬编码替换（4 处）：
- `set_commitments` role 重命名：`e.dimensions.get("role")` / `insert("role", ...)` → 动态 key
- `set_commitments` role 清除：`e.dimensions.get("role")` / `remove("role")` → 动态 key

`load_root_state` 逻辑：
- 删除 `read_monthly_file_safe` 调用 + `monthly` 变量
- 删除 `validate_monthly(&monthly)` 调用
- `usingDefaultDimensions` = `read_dimensions_file` 返回空
- `dimensions` = `resolve_month_dimensions` 返回值

`get_month_dimensions` 逻辑：
- 删除 `read_monthly_file` 调用
- `usingDefaultDimensions` = 同上

清理：
- 删除 `read_monthly_file_safe` 函数
- .md 循环中 7 处 `file_name == "_monthly.md" ||` 删除
- `usingDefaultDimensions` 赋值语句更新

### 4.4 files.rs

删除：
- `monthly_path`（含测试 `test_monthly_path`）
- `read_monthly_file`
- `write_monthly_file`
- `ensure_month_instantiated`

### 4.5 scan.rs

- 删除 `_monthly.md` 跳过逻辑（70-72 行）
- 删除对应测试

### 4.6 operation_log.rs

- 删除 `file_name == "_monthly.md"` 跳过（441 行附近）

---

## 5. 前端改动

### 5.1 types.ts

```typescript
export interface Dimension {
  source: "static" | "commitments:goals" | "commitments:role";
}

export interface MonthDimensions {
  usingDefaultDimensions: boolean;
}
```

`InitResult::Ready` 类型中 `from_template` → `usingDefaultDimensions`。

### 5.2 stores/useStore.ts

`fromTemplate: boolean` → `usingDefaultDimensions: boolean`

### 5.3 DimensionPopover.vue

新增 `roleKey` computed：
```typescript
const roleKey = computed(() => {
  const role = props.dimensions.find(d => d.source === "commitments:role");
  return role?.key ?? "role";
});
```

`goalKey` 查找条件更新：
```typescript
const goalKey = computed(() => {
  const monthly = props.dimensions.find(d => d.source === "commitments:goals");
  return monthly?.key ?? "goal";
});
```

硬编码 `"role"` 替换为 `roleKey.value`（4 处）。

伪维度注入处理：
- `visibleDims` 包含 Role 维度 → 正常渲染
- `visibleDims` 不包含 Role 维度且 commitments 非空 → 保留旧有伪注入行为（fallback）

### 5.4 DimensionEditorModal.vue

`"monthly"` → `"commitments:goals"`：
- source 重复校验逻辑
- 错误提示文案
- 模板中 source 判断

### 5.5 其他文件

- `App.vue`：`fromTemplate` → `usingDefaultDimensions`
- `MonthView.vue`：`fromTemplate` → `usingDefaultDimensions`
- `applyInitResult.ts`：同上
- `CommitmentsPanel.vue`、`EntryRow.vue`：警告文案中的 "role" 保持不动（表达概念，非维度键名）

---

## 6. 测试

### 6.1 Rust 单元测试

- `config.rs`：更新 `validate_dimensions` 测试中的 source 值；删除 `test_month_from_monthly_path_*`（重命名为 `test_extract_year_month_*`）；删除 `MonthlyFile` 引用
- `commands.rs`：8 处 `compute_attribution` 调用新增 `"role"` 参数；删除 `test_read_monthly_file_safe_corrupt`
- `files.rs`：删除 `test_monthly_path`

### 6.2 Rust 集成测试

约 20 处 inline YAML `source: monthly` → `source: commitments:goals`：

| 文件 | 处数 |
|------|------|
| cli_integration.rs | 1 |
| commitment_editor_integration.rs | 1 |
| scan_integration.rs | 1 |
| entry_crud_integration.rs | 4 |
| op_log_verify_integration.rs | 2 |
| commitment_progress_integration.rs | 2 |
| monthly_dimensions_integration.rs | 1 |
| recovery_category_integration.rs | 1 |
| dimension_editor_integration.rs | 6 |

### 6.3 Rust 测试 fixture

- `tests/fixtures/template.yaml`：source 字段更新 + 新增 Role 维度
- 删除 `tests/fixtures/2026/06/_monthly.md`

### 6.4 前端测试

约 35 处更新：
- `source: "monthly"` → `"commitments:goals"`
- `fromTemplate` → `usingDefaultDimensions`
- DimensionPopover 测试更新（维度数组含 role 维度）

---

## 7. 迁移

存量月份 `dimensions.yaml` 中若存在 `source: monthly` 的维度，需手动改为 `source: commitments:goals`。`validate_dimensions` 会报 `InvalidSource` 拒绝旧值。
