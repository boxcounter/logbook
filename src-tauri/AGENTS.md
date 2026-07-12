# AGENTS.md

Logbook — Tauri 2.x (Rust) 后端。前端 Vue 3 在同一仓库上层。

## 开发命令

```bash
cd src-tauri && cargo check                    # 类型检查
cd src-tauri && cargo test                    # 全部测试
cd src-tauri && cargo test -p tauri_app_lib   # 仅 lib tests
pnpm tauri dev                                # 启动 GUI（在仓库根目录跑）
```

自动校验（OpenCode `verify-on-idle` plugin，会话 idle 时触发）：`pnpm vue-tsc --noEmit && cd src-tauri && cargo check && cargo test`

## 模块结构

```
src/
├── main.rs          // fn main()
├── lib.rs           // plugins, setup hook, file watcher, command registration
├── models.rs        // all structs/enums — Template, Dimension, MonthlyFile, Entry, RecoveryCategory, etc.
├── files.rs         // path helpers, atomic I/O, root_path persistence, pure-YAML day file read/write, template/month dimensions
├── config.rs        // validate_dimensions, validate_monthly; WatcherState + ensure_watcher (restartable notify watcher)
├── commands.rs      // Tauri commands + load_root_state (error classification) + parse_duration + validate_date_format
├── error_log.rs     // init, log_error, log_frontend_error
├── operation_log.rs // 操作日志（JSONL 写入 + 回放验证）
├── scan.rs          // 数据目录完整性扫描（挂掉的 .tmp、格式异常文件等）
├── integrity.rs     // 运行时数据完整性守卫（全局 compromised 状态 + check_scoped_integrity + day file 校验）
├── single_instance.rs  // InstanceLock 单实例锁
├── cli/             // CLI 子命令（mod.rs, commitments.rs, dimensions.rs, entries.rs, install.rs, migrate.rs, output.rs, root_path.rs）
├── bin/logbook-cli.rs  // CLI 入口（与 cli/ 平级）
└── window_state.rs  // default window geometry (90% primary monitor, centered)
```

## 测试约定

- **单元测试**：`src/` 内 `#[cfg(test)] mod tests`。纯函数、无 IO、无文件系统、milliseconds
- **集成测试**：`tests/` 目录。可读写 fixture 目录、调 Tauri commands、访问 `crate::files` 等
- 判断标准：`use crate::files` 或碰文件系统？→ 集成测试
- Fixture 目录：`~/Downloads/logbook-test/`（dimensions.template.yaml + 2026/06/ 含测试数据）
- 集成测试中写临时文件用 `std::env::temp_dir()`，事后清理

## 关键约定

- YAML 序列化用 `yaml_serde`（0.10），不是 `serde_yml`
- Entry duration 存储为分钟整数（u32），录入时前端预扫描求和后传字符串，Rust `parse_duration` 做最终转换
- 文件写入：先写 `.tmp` 再 rename（原子写入）
- Day file 是纯 YAML（`note` + `entries`，无 frontmatter `---` 分隔符）：`read_day_file` 剥掉 BOM 后整体 `yaml_serde::from_str::<DayFile>`，`write_day_file` 整体 `yaml_serde::to_string`。日期为文件名规范值，不写进文件内容（见 files.rs:56 注释）
- 维度集合按月存放：`dimensions.template.yaml`（全局默认，旧 `config.yaml`、`template.yaml` 已弃用）→ 每月首次写入时 `create_dimensions_if_missing` 快照进该月 `dimensions.yaml`；`resolve_month_dimensions` 取 `dimensions.yaml` 否则 `dimensions.template.yaml`（缺文件容错返回空）。改 template 不回溯影响已实例化的月份；纯读不实例化
- Goal 维度 `source: "commitments:role:goals"`，值列表来自 `commitments.yaml` 的 commitments goals 并集，不在 template/维度块的 values 里
- Commitments 经 `set_commitments` 命令写入（校验 + goal/role 改名批量更新 entry + 原子写 `commitments.yaml`）；外部直接编辑 `commitments.yaml` 仍由 `notify` watcher 重新读取
- `root_path` 由前端持有，每次 command 调用时传入；Rust 端通过 `root_path.txt` 持久化选择
- **禁止硬编码 fallback**：`goal_dim_key` / `role_dim_key` 等配置解析函数失败时，要么用 `?` 传播错误（拒绝执行），要么跳过该操作并写入 `ConfigErrorDetail` 推送前端。不得 fallback 到 `"goal"`、`"role"` 等字面量——与用户实际配置不一致会导致静默语义错误
- **后端是数据合法性检查的唯一权威源**：不得依赖前端或 CLI 的校验来保证数据完整性。所有写入路径（`commands.rs` 的 Tauri 命令、CLI 的各 `set`/`add` 命令）必须在落盘前自行完成完整校验，不假设调用方已做过合法过滤。新增写入命令时按此原则审查：写前校验链是否覆盖了所有无效输入路径。
- **Phase checkpoint**：每完成一个独立 phase 停下来确认，不要连续推进多个 phase 不征求同意
