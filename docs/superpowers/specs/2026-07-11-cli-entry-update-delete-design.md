# CLI 增加 entry update / delete 命令

**日期**: 2026-07-11
**状态**: Draft

## 问题

Rust 后端 `commands::update_entry`（`commands.rs:666-746`）和 `commands::delete_entry`（`commands.rs:748-791`）已完整实现并注册为 Tauri invoke handler（`lib.rs:274-275`），前端 GUI 通过 `EntryRowEdit` 等组件调用它们修改 / 删除条目。

但 CLI 的 `EntryAction`（`cli/mod.rs:93-107`）只有 `List` 和 `Add` 两个变体，**没有 update / delete**。AI Agent 无法通过 CLI 批量修改 entries——例如「7 月新增了一个 dimension，用 Agent 遍历 7 月所有 entries 为每条设置新 dimension 的值」这种场景，Agent 只能读和加，不能改。

## 目标

新增两个 CLI 子命令，封装已有后端能力：

```
logbook-cli entries update --date <YYYY-MM-DD> --entry-id <UUID>   # stdin: UpdateEntryInput JSON
logbook-cli entries delete --date <YYYY-MM-DD> --entry-id <UUID>   # 无 stdin
```

支持 AI Agent 逐条判断、逐条修改的工作流。不做批量接口（YAGNI），Agent 循环调用单条命令即可。

## 设计

### 命令定义

`cli/mod.rs` 的 `EntryAction` enum 新增两个变体：

```rust
#[derive(Subcommand)]
pub enum EntryAction {
    /// List entries for a date
    List {
        #[arg(long)]
        date: String,
    },
    /// Add an entry (read JSON from stdin)
    Add {
        #[arg(long)]
        date: String,
    },
    /// Update an entry (read JSON from stdin)
    Update {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
        /// Entry ID to update
        #[arg(long)]
        entry_id: String,
    },
    /// Delete an entry
    Delete {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
        /// Entry ID to delete
        #[arg(long)]
        entry_id: String,
    },
}
```

与现有 `List` / `Add` 的 `--date` 长选项风格一致。写入命令的输入数据从 stdin 读取（与 `entries add`、`commitments set`、`dimensions set` 统一）。

### 数据流

**update**（`cli/entries.rs` 新增 `update` 函数）：

1. integrity check（`integrity::check()`，与 `add` 一致）
2. 从 stdin 读 JSON，反序列化为 `UpdateEntryInput`
3. 调 `commands::update_entry(root, date, entry_id, update)` → 后端返回 `DayFile`
4. CLI 层从 `DayFile.entries` 按 `entry_id` 提取单条 Entry
5. 输出单条 Entry

提取不会失败：后端 `update_entry` 成功意味着 entry 存在（`commands.rs:716` 找不到会返回 `Err`）。

**delete**（`cli/entries.rs` 新增 `delete` 函数）：

1. integrity check
2. 调 `commands::delete_entry(root, date, entry_id)` → 后端返回 `DayFile`（忽略不输出）
3. 输出操作结果（不输出数据）

### `is_read_only()` 分类

`cli/mod.rs:120` 现有代码：

```rust
Self::Entries { action } => matches!(action, EntryAction::List { .. }),
```

**不用改。** `matches!` 只匹配 `List`，新增的 `Update` / `Delete` 自动落入 false（写命令，需加锁）。与 `entries add` 行为一致——GUI 运行时执行会被拒绝，防止跨进程 read-modify-write 竞态。

### 输出格式

| 命令 | JSON 模式 | Human 模式 |
|------|-----------|------------|
| `update` | 单条 `Entry` pretty JSON | `Updated: "item" | 60m | key=val, key=val` |
| `delete` | `{"ok": true, "date": "2026-07-04", "entry_id": "..."}` | `Deleted: <entry-id> from 2026-07-04` |

三个写命令的输出语义：
- `add`：返回创建的对象（Entry）
- `update`：返回修改后的对象（Entry）——与 `add` 对称
- `delete`：返回操作确认（对象已不存在）——要二次确认，Agent 再 `entries list`

### 错误处理

全部复用后端。`update_entry` / `delete_entry` 已包含：

- integrity check（`integrity::check()`）
- 日期格式校验（`validate_date_format`）
- dimension 校验链（unknown keys / required / cross-dim constraints）——仅 update 且传 dimensions 时
- pre-write integrity guard（`check_day_file_integrity`）
- operation_log 记录（before snapshot，用于回放验证）

CLI 层只做：
- stdin 解析错误提示（JSON 反序列化失败时打印期望格式 + 错误详情，exit 1）——仅 update
- 透传后端 `Err`（`output::print_error` + exit 1）

### 调度

`cli/mod.rs:run()` 的 `Commands::Entries` match 分支新增两条 arm：

```rust
EntryAction::Update { date, entry_id } => {
    entries::update(&root, &date, &entry_id, cli.json);
}
EntryAction::Delete { date, entry_id } => {
    entries::delete(&root, &date, &entry_id, cli.json);
}
```

## 测试

### 单元测试（`cli/mod.rs` 的 `#[cfg(test)] mod tests`）

`test_write_commands` 补两条断言：

```rust
assert!(!Commands::Entries {
    action: EntryAction::Update {
        date: "2026-07-11".to_string(),
        entry_id: "test-id".to_string(),
    },
}
.is_read_only());
assert!(!Commands::Entries {
    action: EntryAction::Delete {
        date: "2026-07-11".to_string(),
        entry_id: "test-id".to_string(),
    },
}
.is_read_only());
```

### 集成测试（`tests/cli_integration.rs`）

端到端用例，复用现有 fixture 隔离模式（`LOGBOOK_LOCK_DIR` + `LOGBOOK_LOG_DIR` + `run_with_stdin`）：

1. `entries add` → 从输出解析 entry-id
2. `entries update`（stdin 传 `UpdateEntryInput`，改 dimensions）→ 验证输出包含新维度值
3. `entries list` → 验证更新已落盘
4. `entries delete` → 验证操作结果
5. `entries list` → 验证 entry 已移除

## 不做

- **批量接口**：Agent 循环调用单条命令即可。批量要处理部分失败、事务语义，超出当前需求。
- **available-months CLI 命令**：Agent 扫文件系统或逐天调用即可（YAGNI）。
- **命令行 flag 指定更新值**（如 `--set-dimension key=val`）：与现有写命令统一用 stdin JSON 的风格不一致，且 `UpdateEntryInput` 结构已适合 stdin 传递。
- **merge 语义**（dimensions 追加而非替换）：与前端 `EntryRowEdit` 行为不一致，会在同一后端 command 上引入两套 dimensions 语义。Agent 可先读旧值、合并新 key、整体发送。
