# CLI — entries add + 动词命名约定

Date: 2026-07-04

## 背景

`cli-feature` 分支已有基础 CLI（`entries list`、`commitments list/progress/set`、`dimensions get/set`）。本次增加 `entries add` 命令，并统一 CLI 动词命名。

## CLI 动词约定

记录于 `docs/naming-conventions.md` CLI 动词章节：

| 动词 | 操作 | 适用 |
|------|------|------|
| `list` | 取集合 | `entries list`, `commitments list`, `dimensions list` |
| `get` | 取单体 | 暂无，未来 `entries get --id <uuid>` |
| `add` | 集合内新增 | `entries add` |
| `update` | 修改单体 | 未来 `entries update --id <uuid>` |
| `delete` | 删除单体 | 未来 `entries delete --id <uuid>` |
| `set` | 整体替换 | `commitments set`, `dimensions set` |
| `progress` | 衍生计算视图 | `commitments progress`（领域专用名，非 CRUD） |

- `list` vs `get`：`list` 用于集合资源，`get` 用于单体资源。不互为别名。
- `set` vs `update`：`set` 整体替换资源本身，`update` 部分修改集合中的单个成员。`update` 需要标识符（`--id`、`--key`）定位。
- stdin 约定：所有写入命令（`add`、`set`）统一从 stdin 读 JSON/YAML，不做 CLI flag 分散输入。

## 变更清单

### 1. `entries add`（新增）

```
logbook-cli entries add --date 2026-07-04
# stdin: {"item": "Code review", "duration": "30m", "dimensions": {"role": "Dev"}}
```

- 输入：stdin JSON，格式对齐 `CreateEntryInput { item, duration, dimensions }`
- 复用 `commands::append_entry` —— 日期校验、duration 解析（`parse_duration`）、维度校验、操作日志、attribution 注入全部走现有路径
- 输出：`--json` 返回创建的 `Entry`；human 打印单行确认

**human 输出示例**：
```
Added: "Code review" | 30m | role=Dev
```

### 2. `entries` 语法重构（破坏性变更）

当前 `entries` 是扁平 variant：
```
logbook-cli entries --date 2026-07-04   # list only
```

改为子命令结构以容纳 `add`：
```
logbook-cli entries list --date 2026-07-04
logbook-cli entries add --date 2026-07-04
```

`cli/mod.rs` 中 `Commands::Entries { date }` 改为 `Commands::Entries { subcommand: EntryAction }`，其中 `EntryAction` 为 `List { date } | Add { date }`。

cli-feature 分支未发布，无下游用户。

### 3. `dimensions get` → `dimensions list`（重命名）

`DimensionsCommands::Get` variant 改为 `List`。dimensions 在数据模型中是 `Vec<Dimension>` 集合，`list` 动词更准确。

无逻辑变更。

### 4. `docs/naming-conventions.md` 新增 CLI 章节

在现有章节后追加，记录动词矩阵和 stdin 约定。

## 不做

- 不接受 CLI flags 输入（`--item`、`--duration`、`--dim`）
- 不支持批量添加（JSON array）
- 不给 `commitments list` 加 `get` 别名
- 不加当前不使用的外键约束校验（`dim` 侧的 integrity check 由 `validate_required_dimensions` 覆盖，`attribution` 由 `append_entry` 自动注入）

## 错误处理

全量复用 `append_entry` 现有校验链：

| 场景 | 来源 |
|------|------|
| stdin 为空或格式错误 | `serde_json::from_str` 失败 |
| `--date` 格式错误 | `validate_date_format` |
| duration 不合法 | `parse_duration` |
| 缺 required dimension | `validate_required_dimensions` |
| GUI 运行中 | `InstanceLock` 拒绝 |

CLI 层只需 `match`：`Ok → output` / `Err → stderr + exit 1`。

## 测试

`src-tauri/tests/cli_integration.rs` 新增：

- `entries add --date <valid>` stdin 合法 JSON → 成功，stdout 含 entry id
- `entries add --date <valid>` stdin 空 → parse 错误，exit 1
- `entries add --date <invalid>` → validate_date_format 错误，exit 1
- `entries add --date <valid>` stdin 缺 required dimension → 校验错误，exit 1
- `entries list --date <valid>` → 语法不退化
- `dimensions list --year <y> --month <m>` → 重命名后正常输出

## 对现有 CLI spec 的影响

`2026-06-15-cli-design.md` 中 `entries` 命令清单更新：
- `entries --date ...` → `entries list --date ...`
- 追加 `entries add --date ...`
- dimensions 命令改为 `list`，与 spec 中 `dimensions get` 示例对齐更新
