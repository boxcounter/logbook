# CLI 只读命令跳过实例锁

**日期**: 2026-07-11
**状态**: Draft

## 问题

CLI 在启动时无条件获取与 GUI 相同的 PID 实例锁（`src-tauri/src/cli/mod.rs:124-148`）。GUI 运行时持有 `instance.pid`，CLI 拿不到锁就 `exit(1)`——包括 `entries list`、`commitments progress`、`dimensions list` 这些**纯读命令**也被一并挡掉。

代码注释写的是 "Prevent concurrent writes"，但实际实现阻止了**所有** CLI 命令（读和写）。

## 目标

GUI 运行时，CLI 的只读命令可以正常执行。写命令维持现状（被锁挡住），因为跨进程的 read-modify-write 竞态会静默丢数据。

## 背景：为什么需要锁

`append_entry`、`set_commitments` 等写命令执行 read-modify-write 序列：先读文件、修改内存数据、再原子写回（tmp + rename）。两个进程同时执行这个序列会产生 lost update。原子 rename 只防崩溃损坏，不防并发写。

纯读命令面对 atomic tmp+rename 写策略时是安全的——要么读到旧版本、要么读到新版本，不会读到半写损坏的数据。

## 设计

### 核心机制：`is_read_only()` + 条件获取锁

在 `Commands` enum 上加一个方法，作为读写分类的**唯一真相来源**：

```rust
impl Commands {
    fn is_read_only(&self) -> bool {
        match self {
            Self::Commitments { action } => matches!(
                action,
                CommitmentAction::List { .. } | CommitmentAction::Progress { .. }
            ),
            Self::Entries { action } => matches!(action, EntryAction::List { .. }),
            Self::Dimensions(cmd) => matches!(cmd, DimensionsCommands::List { .. }),
            Self::Migrate => false,
        }
    }
}
```

`run()` 里的锁获取改为条件判断——只读命令直接跳过：

```rust
let _lock = if cli.command.is_read_only() {
    None // 读命令不获取锁，可与 GUI 共存
} else if let Some(lock_dir) = lock_dir() {
    // 现有锁获取逻辑，原封不动
    match InstanceLock::try_acquire(&lock_dir) { ... }
} else {
    None
};
```

### 命令分类表

| 命令 | `is_read_only` | 跳过锁? |
|---|---|---|
| `commitments list` | `true` | 是 |
| `commitments progress` | `true` | 是 |
| `commitments set` | `false` | 否 |
| `entries list` | `true` | 是 |
| `entries add` | `false` | 否 |
| `dimensions list` | `true` | 是 |
| `dimensions set` | `false` | 否 |
| `migrate` | `false` | 否 |

分类依据：已逐个审计所有 7 个 leaf subcommand 的代码路径（见下方"审计结果"），确认名字暗示读的命令（`list`、`progress`）在数据层完全只读，名字暗示写的命令（`set`、`add`、`migrate`）确实写数据。

### 附带修正：注释和错误消息现在变准确了

改动前，注释 `"Prevent concurrent writes: if the GUI is running, refuse CLI writes"` 和错误消息 `"Close the GUI before using CLI write commands"` 不准确——它们说的是"writes"，但代码挡了所有命令。

改动后，只有写命令走到锁获取路径，文字和行为终于一致。**无需改文字。**

### 审计结果（保证分类正确）

逐个追踪了每个 subcommand 的完整代码路径，确认：

- `commitments list` — 纯 `read_commitments_file`
- `commitments progress` — `get_commitment_progress`，纯读（`read_commitments_file`、`read_day_file`、`read_dimensions_file`）
- `entries list` — 纯 `read_day_file`
- `dimensions list` — `resolve_month_dimensions`，纯读（内存回退到 template，不落盘）
- `commitments set` — 写 commitments/day/dimensions
- `entries add` — 写 day file + operations log + 可能实例化 dimensions
- `dimensions set` — 直接 `fs::write`/`rename`
- `migrate` — 写新 `.yaml`、删 `.md`、更新 `version.txt`

**关键区别**：读路径用 `resolve_month_dimensions`（内存回退，不落盘）；写路径用 `create_dimensions_if_missing`（从 template 物化 `dimensions.yaml`）。这两个函数容易混，但正是这个区别保证了读命令只读。

**结论：不存在"语义只读、实际写数据"的命令。**

### 非数据的副作用

所有 CLI 调用都会写 `logbook.log`（日志）。这是基础设施层写，不碰数据文件，不影响安全性。锁逻辑已位于日志初始化之后，维持现状。

## 测试

新增单元测试验证 `is_read_only()` 的分类正确性。这是个纯函数，适合单测。覆盖所有 8 个 leaf subcommand（`List`/`Progress`/`Set` × 各自参数、`List`/`Add`、`List`/`Set`、`Migrate`）。

写命令路径的锁行为已被现有测试覆盖，不需改动。

## 改动范围

仅 `src-tauri/src/cli/mod.rs`：
1. 加 `impl Commands { fn is_read_only(&self) -> bool }`
2. `run()` 中锁获取逻辑加 `is_read_only()` 前置判断
3. 新增 `is_read_only()` 的单元测试

## 不做的事

- 不动 `single_instance.rs`——GUI 侧单实例逻辑不变
- 不动 `files.rs`——不加跨进程文件锁
- 不动 `config.rs` 的 watcher——CLI 只读时不触发 watcher
- 不为 CLI 写命令做跨进程文件锁——用户确认不需要 GUI 运行时写
