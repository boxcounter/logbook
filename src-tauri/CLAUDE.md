# CLAUDE.md

Logbook — Tauri 2.x (Rust) 后端。前端 Vue 3 在同一仓库上层。

## 开发命令

```bash
cd src-tauri && cargo check                    # 类型检查
cd src-tauri && cargo test                    # 全部测试
cd src-tauri && cargo test -p tauri_app_lib   # 仅 lib tests
pnpm tauri dev                                # 启动 GUI（在仓库根目录跑）
```

Stop hook（自动）：`pnpm vue-tsc --noEmit && cd src-tauri && cargo check && cargo test`

## 模块结构

```
src/
├── main.rs      // fn main()
├── lib.rs       // plugins, setup hook, file watcher, command registration
├── models.rs    // all structs/enums — Config, Dimension, MonthFile, Entry, etc.
├── files.rs     // path helpers, atomic I/O, root_path persistence, frontmatter parse
├── config.rs    // validate_config, validate_monthly, watch_files (notify crate)
└── commands.rs  // 9 Tauri commands + parse_duration + validate_date_format
```

## 测试约定

- **单元测试**：`src/` 内 `#[cfg(test)] mod tests`。纯函数、无 IO、无文件系统、milliseconds
- **集成测试**：`tests/` 目录。可读写 fixture 目录、调 Tauri commands、访问 `crate::files` 等
- 判断标准：`use crate::files` 或碰文件系统？→ 集成测试
- Fixture 目录：`~/Downloads/logbook-test/`（config.yaml + 2026/06/_monthly.md）
- 集成测试中写临时文件用 `std::env::temp_dir()`，事后清理

## 关键约定

- YAML 序列化用 `yaml_serde`（0.10），不是 `serde_yml`
- Entry duration 存储为分钟整数（u32），录入时前端预扫描求和后传字符串，Rust `parse_duration` 做最终转换
- 文件写入：先写 `.tmp` 再 rename（原子写入）
- Frontmatter 解析：手动定位 `---` 边界 + `yaml_serde::from_str`
- Goal 维度 `source: "monthly"`，值列表来自 `_monthly.md`，不在 config.yaml 的 values 里
- Commitments 不在 Rust 端写入——用户直接编辑 `_monthly.md`，由 `notify` watcher 重新读取
- `root_path` 由前端持有，每次 command 调用时传入；Rust 端通过 `root_path.txt` 持久化选择
- **Phase checkpoint**：每完成一个独立 phase 停下来确认，不要连续推进多个 phase 不征求同意
- **Spec-code alignment**：写 HANDOFF.md 之前，dispatch subagent 对比以下文档 vs 实际代码，报告不一致项：
  - `SPEC.md` 命令清单 vs `#[tauri::command]` 实际注解
  - `SPEC.md` 组件树 vs `src/components/` 实际文件
  - Vault `1_Projects/Logbook/README.md` 数据结构 vs `src-tauri/src/models.rs`
  - Subagent 会读取文档和代码，做语义对比（不是 shell grep），发现差异直接列在 handoff 里
