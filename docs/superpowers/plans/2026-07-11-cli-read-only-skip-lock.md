# CLI 只读命令跳过实例锁 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** GUI 运行时 CLI 的只读命令（`list` / `progress`）可以正常执行，不再被实例锁挡掉。

**Architecture:** 在 `Commands` enum 上加一个 `is_read_only()` 方法作为读写分类的唯一真相来源。`run()` 在获取锁之前先判断：只读命令跳过锁，写命令维持现有行为。

**Tech Stack:** Rust, clap 4.x

## Global Constraints

- 改动仅限 `src-tauri/src/cli/mod.rs`
- 不动 `single_instance.rs`、`files.rs`、`config.rs`
- 写命令（`set`、`add`、`migrate`）的锁行为不变
- 已有测试基线：Rust 141 passed, 1 pre-existing failure（`error_log::tests::test_log_rotation_keeps_appending_below_threshold`，与本改动无关）

**Spec:** `docs/superpowers/specs/2026-07-11-cli-read-only-skip-lock-design.md`

---

### Task 1: `is_read_only()` 方法 + 单元测试

**Files:**
- Modify: `src-tauri/src/cli/mod.rs`（在 `Commands` enum 定义之后，`run()` 之前插入 impl 块 + `#[cfg(test)]` 模块）

**Interfaces:**
- Produces: `Commands::is_read_only(&self) -> bool` — 后续 Task 2 依赖此方法决定是否获取锁

**Enum 形状参考（已从源码确认）：**
- `Commands::Commitments { action: CommitmentAction }` — `CommitmentAction::{List{year,month}, Progress{year,month}, Set{year,month}}`
- `Commands::Entries { action: EntryAction }` — `EntryAction::{List{date}, Add{date}}`
- `Commands::Dimensions(DimensionsCommands)` — `DimensionsCommands::{List{year,month,template,json}, Set{year,month,template,json}}`
- `Commands::Migrate`

- [ ] **Step 1: Write the failing test**

在 `mod.rs` 文件末尾（`run` 函数的闭合 `}` 之后）追加测试模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_commands() {
        // 只读命令 → true
        assert!(Commands::Commitments {
            action: CommitmentAction::List { year: 2026, month: 7 },
        }
        .is_read_only());
        assert!(Commands::Commitments {
            action: CommitmentAction::Progress { year: 2026, month: 7 },
        }
        .is_read_only());
        assert!(Commands::Entries {
            action: EntryAction::List { date: "2026-07-11".to_string() },
        }
        .is_read_only());
        assert!(Commands::Dimensions(DimensionsCommands::List {
            year: Some(2026),
            month: Some(7),
            template: false,
            json: false,
        })
        .is_read_only());
        // dimensions list --template 也只读
        assert!(Commands::Dimensions(DimensionsCommands::List {
            year: None,
            month: None,
            template: true,
            json: false,
        })
        .is_read_only());
    }

    #[test]
    fn test_write_commands() {
        // 写命令 → false
        assert!(!Commands::Commitments {
            action: CommitmentAction::Set { year: 2026, month: 7 },
        }
        .is_read_only());
        assert!(!Commands::Entries {
            action: EntryAction::Add { date: "2026-07-11".to_string() },
        }
        .is_read_only());
        assert!(!Commands::Dimensions(DimensionsCommands::Set {
            year: Some(2026),
            month: Some(7),
            template: false,
            json: false,
        })
        .is_read_only());
        assert!(!Commands::Migrate.is_read_only());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --lib cli::tests`
Expected: 编译失败，`no method named is_read_only found for enum Commands`

- [ ] **Step 3: Write minimal implementation**

在 `Commands` enum 定义之后、`run()` 之前插入：

```rust
impl Commands {
    /// Whether this command only reads data and never writes.
    ///
    /// Read-only commands skip the instance lock so they can run while the
    /// GUI is open. Write commands acquire the lock to prevent cross-process
    /// read-modify-write races that would silently lose data.
    fn is_read_only(&self) -> bool {
        match self {
            Self::Commitments { action } => {
                matches!(action, CommitmentAction::List { .. } | CommitmentAction::Progress { .. })
            }
            Self::Entries { action } => matches!(action, EntryAction::List { .. }),
            Self::Dimensions(cmd) => matches!(cmd, DimensionsCommands::List { .. }),
            Self::Migrate => false,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --lib cli::tests`
Expected: PASS（2 tests）

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cli/mod.rs
git commit -m "feat(cli): add is_read_only() classification for all subcommands"
```

---

### Task 2: `run()` 条件跳过锁

**Files:**
- Modify: `src-tauri/src/cli/mod.rs:122-148`（锁获取逻辑）

**Interfaces:**
- Consumes: `Commands::is_read_only(&self) -> bool`（来自 Task 1）

- [ ] **Step 1: Modify the lock acquisition logic**

将 `run()` 中的锁获取块（当前第 122-148 行）：

```rust
    // Prevent concurrent writes: if the GUI is running, refuse CLI writes to
    // avoid cross-process read-modify-write races that would silently lose data.
    let _lock = if let Some(lock_dir) = lock_dir() {
        match InstanceLock::try_acquire(&lock_dir) {
            Ok(guard) => Some(guard),
            Err(e) => {
                match e {
                    InstanceLockError::AlreadyRunning(pid) => {
                        eprintln!(
                            "Error: Logbook GUI is already running (PID {}).\n\
                             Close the GUI before using CLI write commands.",
                            pid
                        );
                    }
                    InstanceLockError::Io(io_err) => {
                        eprintln!(
                            "Error: Failed to acquire instance lock: {}. Check permissions on {}.",
                            io_err, lock_dir.display()
                        );
                    }
                }
                std::process::exit(1);
            }
        }
    } else {
        None
    };
```

替换为：

```rust
    // Prevent concurrent writes: if the GUI is running, refuse CLI write
    // commands to avoid cross-process read-modify-write races that would
    // silently lose data. Read-only commands skip the lock so they can run
    // alongside the GUI.
    let _lock = if cli.command.is_read_only() {
        None
    } else if let Some(lock_dir) = lock_dir() {
        match InstanceLock::try_acquire(&lock_dir) {
            Ok(guard) => Some(guard),
            Err(e) => {
                match e {
                    InstanceLockError::AlreadyRunning(pid) => {
                        eprintln!(
                            "Error: Logbook GUI is already running (PID {}).\n\
                             Close the GUI before using CLI write commands.",
                            pid
                        );
                    }
                    InstanceLockError::Io(io_err) => {
                        eprintln!(
                            "Error: Failed to acquire instance lock: {}. Check permissions on {}.",
                            io_err, lock_dir.display()
                        );
                    }
                }
                std::process::exit(1);
            }
        }
    } else {
        None
    };
```

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo build`
Expected: 编译成功，无 warning

- [ ] **Step 3: Run full test suite**

Run: `cd src-tauri && cargo test --lib`
Expected: 143 passed（141 原有 + 2 新增），1 pre-existing failure（`error_log::tests::test_log_rotation_keeps_appending_below_threshold`，与本改动无关）

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/cli/mod.rs
git commit -m "fix(cli): skip instance lock for read-only commands so they run alongside GUI"
```
