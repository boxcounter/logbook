# 文档一致性检查

对项目文档做全量交叉比对，检查文档之间、文档与代码之间的一致性。在以下场景调用：

- HANDOFF.md 撰写前
- Phase 结束时
- 用户说「检查一致性」「文档同步」「check consistency」等关键词

## 检查范围

### 文档集

| 文档 | 路径 |
|------|------|
| 根 CLAUDE.md | `CLAUDE.md` |
| 后端 CLAUDE.md | `src-tauri/CLAUDE.md` |
| 技术规格 | `SPEC.md` |
| 交接文档 | `HANDOFF.md` |
| 设计文档 | Obsidian vault `1_Projects/Logbook/README.md` |

### 代码集

| 范围 | 路径 |
|------|------|
| Rust 源码 | `src-tauri/src/` 所有 `.rs` 文件 |
| Rust 集成测试 | `src-tauri/tests/` |
| Cargo 清单 | `src-tauri/Cargo.toml` |
| Tauri 配置 | `src-tauri/tauri.conf.json` |
| Vue 组件 | `src/components/` 所有 `.vue` 文件 |
| 前端入口 | `src/App.vue`、`src/main.ts` |
| 前端工具/store | `src/utils/`、`src/stores/` 所有 `.ts` 文件 |
| 前端测试 | `src/__tests__/`、`vitest.config.ts` |
| 前端清单 | `package.json` |

## 执行协议

单 Agent 执行。一致性检查是结构化交叉比对任务，不需要多维度并行审查。Dispatch 一个 subagent，一次性通读所有文档和代码，产出结构化报告。

## Agent Prompt

```
你是文档一致性检查员。对 Logbook 项目做全量交叉比对。

## 你的任务

阅读以下所有文档和代码，逐项比对，报告每一项不一致。

### 必读文件

文档：
- CLAUDE.md（根目录）
- src-tauri/CLAUDE.md
- SPEC.md
- HANDOFF.md（如果不存在，标注后跳过引用它的检查项）

代码（用 ls / find 动态发现实际文件，以下为预期路径）：
- src-tauri/src/ 下所有 .rs 文件
- src-tauri/tests/ 下所有 .rs 文件
- src-tauri/Cargo.toml
- src-tauri/tauri.conf.json
- src/components/ 下所有 .vue 文件
- src/App.vue、src/main.ts
- src/utils/、src/stores/ 下所有 .ts 文件
- src/__tests__/、vitest.config.ts
- package.json

设计文档 `1_Projects/Logbook/README.md` 用 Obsidian CLI 读取。如果 vault 名不确定，先 `obsidian-cli vault list` 找到包含 Logbook 项目的 vault。无法访问时在报告中注明。

## 检查项目

所有「文档声称的 X」均指动态读取该文档后提取的信息——不要依赖本 prompt 中的任何示例或暗示，以实际文件内容为准。

### A. 文档 ↔ 文档

#### A1. 命令数量
- src-tauri/CLAUDE.md 声称的命令数 vs SPEC.md 列出的命令数 vs HANDOFF.md 声称的命令数（如 HANDOFF.md 存在）
- 不一致时报告：哪个文档说几个，实际应该几个

#### A2. Phase 进度
- HANDOFF.md 声称的「已完成 Phase」与 SPEC.md Phase 表格对比（如 HANDOFF.md 存在）
- CLAUDE.md 和 HANDOFF.md 的 Phase 描述是否一致（如 HANDOFF.md 存在）

#### A3. 模块结构
- src-tauri/CLAUDE.md「模块结构」列出的文件 vs 实际 `src-tauri/src/` 下的 `.rs` 文件（用 ls 动态列出）
- 报告多余或遗漏的文件

#### A4. 组件树
- SPEC.md「组件树」列出的组件 vs 实际 `src/components/` 下的 `.vue` 文件（用 ls 动态列出）
- 注意区分「已实现」和「planned/Phase 3」的组件

#### A5. 数据结构
- SPEC.md「数据结构」中的 struct 定义 vs src-tauri/CLAUDE.md「关键约定」中的描述
- 两个文档是否对同一概念有矛盾的说法

#### A6. 技术栈
- SPEC.md「技术栈」表格 vs Cargo.toml、package.json 和 tauri.conf.json 的实际依赖
- 特别检查：yaml_serde（不是 serde_yml）、Chart.js 引入方式、Tauri 版本、tauri.conf.json 中的 identifier/window 配置是否与 SPEC.md 一致

#### A7. 测试覆盖
- HANDOFF.md 声称的测试数（如存在）vs 实际运行测试的输出
- Rust：`cd src-tauri && cargo test -- -q`
- 前端：检查 `src/__tests__/` 下文件数、`vitest.config.ts` 是否存在、文档是否提及前端测试

### B. 文档 ↔ 代码

#### B1. 命令签名
- SPEC.md 命令清单中每个命令的签名（函数名、参数、返回类型）
- vs 实际 `commands.rs` 中 `#[tauri::command]` 的函数签名
- 检查：命令名是否匹配、参数是否匹配、是否有多余或缺失的命令

#### B2. 数据结构
- SPEC.md 中的 struct 定义（字段名、类型）
- vs `models.rs` 中的实际 struct 定义
- 字段名、类型、Option 包裹是否一致

#### B3. 关键约定
- 读取 src-tauri/CLAUDE.md「关键约定」章节的每一条
- 逐条验证代码实现（而非验证本 prompt 中列出的项）：
  - 读取约定 → 找到对应代码文件 → 确认实现匹配
- 示例（以 src-tauri/CLAUDE.md 实际内容为准，这些只是常见检查点）：
  - YAML 序列化库 → 检查 Cargo.toml 依赖 + files.rs 中的 import
  - Entry duration 类型 → 检查 models.rs
  - 文件写入策略 → 检查 files.rs 实现
  - Frontmatter 解析 → 检查 files.rs
  - Goal 维度配置 → 检查 config.rs
  - root_path 传递方式 → 检查 commands.rs
  - Phase checkpoint 规则 → 检查两个 CLAUDE.md 是否一致

#### B4. 前端组件
- SPEC.md 组件树中的「已实现」组件
- vs `src/components/` 下实际存在的 `.vue` 文件
- 组件是否在 `App.vue` 或父组件中被引用

#### B5. 数据流
- SPEC.md「数据流」描述的事件（`config-changed`、`commitments-changed` 等）
- vs `lib.rs` 中实际 emit 的事件

#### B6. 前端模块遗漏
- 列出 `src/` 下的所有子目录和关键文件（components/、stores/、utils/、__tests__/ 等）
- 检查 SPEC.md 或 HANDOFF.md 是否遗漏了这些模块的说明

## 输出格式

按以下结构输出报告。每个不一致项包含：
- **严重度**：🔴 阻断（数据丢失/编译失败）| 🟡 误导（文档会让人写出错误代码）| ⚪ 过时（文档未更新但无功能影响）
- **位置**：涉及的文档和代码路径
- **说明**：不一致的具体内容

```markdown
# 文档一致性检查报告

## 摘要
- 检查时间：{timestamp}
- 文档 ↔ 文档：X 项不一致
- 文档 ↔ 代码：Y 项不一致

## 🔴 阻断级
（列出所有阻断级不一致。没有则写「无」）

## 🟡 误导级
（列出所有误导级不一致）

## ⚪ 过时
（列出所有过时不一致）

## 已确认一致
（列出已验证的关键项目，证明确实检查过——防止漏检）
- [x] 命令签名匹配
- [x] 数据结构匹配
- ...
```

## 注意事项

- 不要裁剪——每个检查项都必须执行
- 每个结论必须有文件引用（file:line）
- 不确定的地方标注「未验证：...」
- 如果 HANDOFF.md 不存在，在报告中标注，并跳过 A1/A2/A7 中引用 HANDOFF.md 的子项
```

## 使用方式

用户输入 `/check-consistency` 或说「检查一致性」时，按上述 prompt dispatch 一个 subagent。Agent 返回报告后，**直接呈现给用户**——不要二次总结或截断。

如发现 🔴 或 🟡 级不一致，在报告末尾追加一句：「是否立即修复以上不一致？」等待用户决定。

## 与 `/review-project` 的区别

| | `/check-consistency` | `/review-project` |
|---|---|---|
| 关注点 | 文档与代码的**一致性** | 代码**质量**（bug、设计、安全） |
| 执行方式 | 单 Agent 结构化比对 | 6 Agent 并行 + 验证 + 汇总 |
| 输出 | 不一致清单 | 按严重度排名的 findings |
| 触发 | HANDOFF 前、Phase 结束 | 重大变更后、里程碑前 |
