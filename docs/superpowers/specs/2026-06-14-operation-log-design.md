# Operation Log — 设计规格

> 状态：设计确认，待实施

## 目的

防御代码 bug 导致的数据损坏。每次 mutation 操作记录到 append-only JSONL 日志。数据异常时，人工翻阅日志定位问题、手动恢复。

不覆盖：文件系统损坏、磁盘故障、云同步冲突。

## 日志文件

### 路径

```
{root_path}/.logbook/operations/{year}/{month:02}/{date}.jsonl
```

与 day file 的目录结构平行。目录首次操作时懒创建（`create_dir_all`）。

root_path 切换时，日志属于旧目录，新目录从零开始记。

### 格式

每行一条 JSON（compact，无换行），四个操作类型：

```
append      → {"ts":"...","op":"append","date":"...","entry_id":"...","params":{...}}
update      → {"ts":"...","op":"update","date":"...","entry_id":"...","before":{...},"params":{...}}
delete      → {"ts":"...","op":"delete","date":"...","entry_id":"...","before":{...}}
set_day_note → {"ts":"...","op":"set_day_note","date":"...","before":"...","params":{"note":"..."}}
```

字段：

| 字段 | 出现于 | 说明 |
|------|--------|------|
| `ts` | 全部 | ISO 8601 with timezone，操作时间 |
| `op` | 全部 | `append` / `update` / `delete` / `set_day_note` |
| `date` | 全部 | 目标 day file 的日期（冗余，方便 grep） |
| `entry_id` | append / update / delete | 操作对象的 Entry UUID |
| `before` | update / delete / set_day_note | 改之前的完整快照（Entry JSON object 或 note 字符串）。append 没有 |
| `params` | append / update / set_day_note | 操作参数。delete 没有 |

### 保留策略

手动清理。系统不做自动删除。

## Rust 实现

### 新模块

`src-tauri/src/operation_log.rs`，暴露：

```rust
pub fn append(root_path: &str, op: Operation) -> Result<(), String>
```

`Operation` enum：

```rust
enum Operation {
    Append {
        date: String,
        entry_id: String,
        params: serde_json::Value,  // NewEntry 序列化
    },
    Update {
        date: String,
        entry_id: String,
        before: Entry,              // 改之前的完整快照
        params: serde_json::Value,  // UpdateEntry 序列化
    },
    Delete {
        date: String,
        entry_id: String,
        before: Entry,
    },
    SetDayNote {
        date: String,
        before: Option<String>,     // 改之前的 note
        params: String,             // 新 note 内容
    },
}
```

在 `lib.rs` 中 `mod operation_log;`。

### 改动点

四个 mutation command 各加调用：

1. **append_entry**：`Operation::Append { date, entry_id, params }` — 无 before
2. **update_entry**：先读取 DayFile 找到目标 Entry 作为 `before`，`Operation::Update { date, entry_id, before, params }`
3. **delete_entry**：同上，`Operation::Delete { date, entry_id, before }`
4. **set_day_note**：先读当前 note 作为 `before`，`Operation::SetDayNote { date, before, params }`

### 调用顺序：先记日志，后改数据

```rust
// 以 update_entry 为例
let day_file = read_day_file(&root_path, &date)?;
let before = day_file.entries.iter().find(|e| e.id == entry_id).cloned()
    .ok_or("Entry not found")?;

// 1. 先写日志
operation_log::append(&root_path, Operation::Update { ... })?;
// 失败 → 返回 error，数据未被修改

// 2. 再执行实际写入（现有原子 write-temp-rename）
let result = do_update_entry(...)?;
```

此顺序保证：
- 日志写入失败 → 操作整体失败，数据不变。**不会漏记。**
- 操作写入失败 → 日志多了一条"未生效"记录（误报）。**不会丢数据。**
- 偏好多记而非漏记。

严格原子性（日志和数据在同一个 atomic rename 内）留待后续按需实现。

### 边界情况

| 场景 | 行为 |
|------|------|
| `.logbook/` 目录不存在 | `create_dir_all` 递归创建 |
| JSON 序列化 | 标准类型不会失败，不做 fallback |
| 磁盘满 | `write` 返回 error → command 返回 error → 前端提示 |
| `.logbook/` 被人删除 | 下次操作自动重建（已丢失的日志不可恢复） |

## 不实现

- 恢复工具/UI — 本阶段只建日志机制
- 日志校验/完整性检查
- 日志压缩/清理
- 读取操作日志（纯只读操作不记录）
- 多文件原子提交

## 实施影响范围

| 文件 | 改动量 |
|------|--------|
| `src-tauri/src/operation_log.rs` | 新增 ~80 行 |
| `src-tauri/src/lib.rs` | +1 行 `mod operation_log` |
| `src-tauri/src/commands.rs`（或当前 mutation 所在文件） | 四处各 +5 行 |
| 前端 | 无改动 |
| 测试 | 新增 operation_log 单元测试 + 集成测试调 verify |
