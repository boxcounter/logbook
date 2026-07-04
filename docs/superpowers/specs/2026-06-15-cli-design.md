# CLI Design — Logbook

Date: 2026-06-15

## 目标

提供独立 CLI binary `logbook-cli`，方便用户通过 Agent（Claude Code）与 Logbook 数据交互。

核心场景：月末 Agent 回顾当月数据、和用户讨论下月计划、Agent 写 commitment 到 `_monthly.md`。

## Binary 结构

`src-tauri/Cargo.toml` 新增 `[[bin]]` target，编译为独立 binary `logbook-cli`：

```toml
[[bin]]
name = "logbook-cli"
path = "src/bin/logbook-cli.rs"
```

新增依赖 `clap`（derive 模式）：

```toml
clap = { version = "4", features = ["derive"] }
```

目录结构：

```
src-tauri/src/
├── bin/
│   └── logbook-cli.rs        # 入口：parse args
├── cli/
│   ├── mod.rs                # 路由子命令
│   ├── root_path.rs          # 解析 root_path
│   ├── commitments.rs        # list, progress, set
│   ├── entries.rs            # list
│   └── output.rs             # 统一输出：human / JSON
```

CLI binary 依赖 `tauri_app_lib` 的 `models`、`files`、`commands` 模块，不 link Tauri runtime。

## root_path 解析

优先级：

1. `--root-path` / `-r` 全局 flag
2. 读 GUI 持久化的 `root_path.txt`（macOS: `~/Library/Application Support/com.logbook/root_path.txt`）
3. 都没有 → stderr 报错，提示设置 `--root-path` 或启动一次 GUI

所有子命令共享此逻辑，收敛在 `cli/root_path.rs`。

## 全局 flag

- `-r, --root-path <PATH>`：数据目录（可选，有默认值）
- `--json`：JSON 输出模式。Global flag，对 Agent 使用必需，人类默认 human-readable

## 命令清单

```
logbook-cli commitments list     --year 2026 --month 6
logbook-cli commitments progress --year 2026 --month 6
logbook-cli commitments set      --year 2026 --month 7    # 从 stdin 读 YAML/JSON

logbook-cli entries list         --date 2026-06-15
logbook-cli entries add          --date 2026-06-15    # 从 stdin 读 JSON

logbook-cli dimensions list      --year 2026 --month 6
logbook-cli dimensions set       --year 2026 --month 7    # 从 stdin 读 YAML/JSON
```

### commitments list

输出当月所有 commitments（调用 `files::read_monthly_file`）。

```
$ logbook-cli --json commitments list --year 2026 --month 6
{"commitments":[{"role":"Dev","allocation":40,"goals":["Ship feature X"]}]}
```

### commitments progress

输出当月 commitment 进度（调用 `commands::get_commitment_progress`）。

```
$ logbook-cli --json commitments progress --year 2026 --month 6
[{"role":"Dev","allocation_minutes":2400,"spent_minutes":1800,"goals":[...]}]
```

### commitments set

从 stdin 读 MonthlyFile 格式的 YAML 或 JSON，校验后原子写入 `_monthly.md`。

```
$ echo 'commitments:
  - role: Dev
    allocation: 40
    goals:
      - Ship feature X' | logbook-cli commitments set --year 2026 --month 7
```

校验分两层：

1. **结构校验**：serde 解析时自动验证（字段类型、必填项）
2. **业务校验**：调用现有 `validate_monthly()` 检测 MissingRole、ZeroAllocation、DuplicateGoal 等

校验失败时 stderr 输出错误原因，exit code 1，不写文件。

Agent self-discoverable 设计：`commitments list --json` 的输出格式就是 `commitments set` 接受的输入格式。

### entries list

输出当天 entries。

```
$ logbook-cli --json entries list --date 2026-06-15
{"note":null,"entries":[{"id":"...","item":"Code","duration":60,"dimensions":{"goal":"Ship it"}}]}
```

### entries add

从 stdin 读 `CreateEntryInput` JSON，复用 `append_entry` 创建条目。

```
$ echo '{"item":"Code review","duration":"30m","dimensions":{"role":"Dev"}}' | logbook-cli entries add --date 2026-06-15
Added: "Code review" | 30m | role=Dev
```

`duration` 支持 `parse_duration` 的所有格式（`30m`、`1h30m`、`120` 等）。dimensions 可选，省略时为 `{}`。

---

**注意**：`commitments set` 需要在 `files.rs` 新增 `write_monthly_file()`——和现有 `write_day_file()` 同样模式：YAML 序列化 → 包 frontmatter `---` → 写 `.tmp` → rename 原子写入，~10 行。这是本次变更中唯一新增的 Rust 业务代码。

## 错误处理

| 层 | 处理方式 |
|---|---|
| 参数解析错误 | clap 自动报错，exit code 2 |
| 业务错误（文件不存在、解析失败、校验失败） | stderr 输出错误消息，exit code 1 |
| 成功 | stdout 输出结果，exit code 0 |

root_path 找不到时，错误消息提示 `--root-path` 用法。

## 测试

集成测试 `src-tauri/tests/cli_integration.rs`，用 `std::process::Command` 调编译好的 binary，对临时 fixture 目录操作。

覆盖：
- 参数缺失 → clap 报错
- `list` 空月 / 有数据
- `progress` 计算正确
- `set` 写入成功 + 写入后 `list` 可读回 + 校验错误不写文件
- `--json` vs human 输出格式
- `--root-path` 解析优先级

## 不在 scope

- GUI 功能不做改动
- `entries add` 已实现（2026-07-04）
- 不支持 entry update/delete——按需后续加
- 不发布到 crates.io / Homebrew——`cargo install --path src-tauri` 安装
- 不修改 `_monthly.md` 的文件监听逻辑——CLI 写入后 GUI 通过 `notify` watcher 自动检测
