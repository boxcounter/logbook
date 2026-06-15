# Quality Assurance — 功能质量保障设计

## 背景

开发者不熟悉 Tauri 2.x + Vue 3 + Rust 技术栈，主要通过 Vibe Coding（AI 辅助编码）迭代。需要不依赖人工代码审查的质量保障手段。当前关心四类功能质量问题：

- **回归**：改 A，B 功能悄悄坏
- **数据完整性**：entry 丢失、duration 算错、文件写坏
- **边界情况**：空输入、极端值、并发操作崩溃
- **AI 生成代码的逻辑错误**：能跑但行为不符合预期

UI 质量不在当前范围内。

## 现有质量基础设施

| 设施 | 现状 |
|------|------|
| 前端测试 | Vitest + jsdom，15 个文件 ~3000 行，覆盖主要组件 |
| 后端集成测试 | Rust test，4 个文件 ~700 行 CRUD + op log + commitment progress |
| 类型检查 | `vue-tsc --noEmit` + `cargo check` |
| 原子写入 | `.tmp` → `rename`，已实现 |
| Operation log | 每次写操作记录，已实现 |
| SPEC.md | 14 个 command 的完整行为定义 |

## 设计

三个组件，互补：

### 1. 合约测试

每个 Tauri command 对应一份合约文件（YAML），定义：
- 正常路径行为
- 边界条件（空输入、不存在的数据、极端值）
- 错误路径（非法输入、格式错误）

合约文件放在 `src-tauri/tests/contracts/`，Rust 侧编译为参数化测试。执行路径：

```
AI 改代码前 → 读取相关合约文件 → 了解预期行为
AI 改代码后 → cargo test → 合约测试自动验证 → 差异即回归
```

合约文件格式：

```yaml
command: get_entries
description: 读取指定日期的 entry 列表

cases:
  - name: 正常读取有数据的日期
    setup:
      fixture: "2026/06/15.md"
    input:
      root_path: "{FIXTURE_ROOT}"
      date: "2026-06-15"
    expect:
      ok:
        note: "今日总结"
        entries:
          length: 3
          "[0].item": "开会"
          "[0].duration": 30

  - name: 读取不存在的日期（空文件）
    input:
      root_path: "{FIXTURE_ROOT}"
      date: "2025-01-01"
    expect:
      ok:
        entries:
          length: 0

  - name: 非法日期格式
    input:
      root_path: "{FIXTURE_ROOT}"
      date: "invalid"
    expect:
      err: "Invalid date format"
```

覆盖范围：14 个 Tauri command，每个至少覆盖 1 正常 + 1 边界 + 1 错误路径。

为合约测试定义一套 fixture 数据目录（`src-tauri/tests/fixtures/`），与现有 `~/Downloads/logbook-test/` 互不干扰，合约测试完全自包含。

#### 合约文件作为 AI context

Vibe coding 时，AI 读取合约文件获得精确的行为定义。这比 SPEC.md 的自然语言描述更结构化，比测试代码更可读。合约文件扮演双重角色：
- **给人看**：行为的权威定义
- **给 AI 看**：可操作的验收标准
- **给机器看**：自动化测试输入

### 2. 启动时数据自检

`init()` 命令扩展，启动时扫描数据目录，返回扫描结果。

扫描规则：

| 发现 | 处理 |
|------|------|
| 文件名不符合 `YYYY-MM-DD.md` | 跳过，记录 skip_warning |
| Frontmatter 解析失败 | 记录 corrupted，标记文件名 |
| 空文件 | 正常，不报告 |
| 孤儿 `.tmp` 文件 | 记录 orphaned_tmp，提示可恢复 |

返回结构新增字段：

```rust
struct InitResult {
    // ... 现有字段 ...
    scan_warnings: Vec<ScanWarning>,  // 新增
}

struct ScanWarning {
    kind: String,   // "SkippedFile" | "CorruptedFile" | "OrphanedTemp"
    path: String,   // 相对路径
    message: String,
}
```

前端处理：`Ready` 且 `scan_warnings` 非空时，以 non-blocking toast 展示摘要（如"检测到 2 个异常，详见 error.log"），不阻塞正常使用。`ConfigError` 时，扫描结果附加在错误信息中。

### 3. Operation log 回放验证

已有 `operation_log.rs` 模块记录每次写操作。补齐验证函数：

```
verify_op_log(root_path) → Result<(), Vec<OpLogMismatch>>
```

逻辑：
1. 读取 op log
2. 在 temp_dir 中回放所有操作
3. 逐文件对比 temp 结果与当前数据目录
4. 返回不一致的文件列表

触发方式：手动（`cargo test` 中的集成测试），不作为启动时例行检查（避免性能影响）。

## 实现顺序

1. **数据自检**（投入最小、收益直接）
2. **合约测试基础设施**（构建合约 runner + 2-3 个 command 的合约，验证流程可行）
3. **补齐合约覆盖**（剩余 command）
4. **Op log 回放验证**

每步完成后停下确认，不连续推进。

## 非目标

- E2E 测试（Tauri WebDriver 方案成本高，当前不引入）
- UI 快照测试
- CI 集成（本地开发质量保障为主，CI 后续再看）
- 性能测试
