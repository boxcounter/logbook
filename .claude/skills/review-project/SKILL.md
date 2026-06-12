# 项目审查

对项目进行多 Agent 综合审查。在重大变更后、里程碑前、或用户要求 review / audit / 审查时调用。

## 总体架构

```
┌─ 第一阶段：审查 ──────────────────────────────────────────┐
│  6 个 teammate 并行执行。各自通读整个项目，产出结构化       │
│  findings。通过 TaskCreate/TaskUpdate 协调。                │
│  主 Agent 在所有 task completed 后才进入下一阶段。          │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ 第二阶段：验证 ─────────▼────────────────────────────────┐
│  每个 HIGH+ 的 finding 派一个 teammate（并行）。对抗性检查：│
│  这个发现是真问题还是误报？                                │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ 第三阶段：汇总 ─────────▼────────────────────────────────┐
│  主 Agent 执行：去重 → 按严重度排名 → P0/P1/P2 分批       │
│  → 结构化报告                                             │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ 第四阶段：反对意见（二层）─▼──────────────────────────────┐
│  1 个 teammate。看到所有 findings。元批评：盲区、回声室、   │
│  严重度误判。                                              │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ 第五阶段：完整性审视 ───▼────────────────────────────────┐
│  1 个 teammate。「哪个子目录 / 风险类型 / 故障模式完全没有  │
│  finding？什么从来没有被关注过？」                          │
└─────────────────────────────────────────────────────────────┘
```

## 同步机制

本 skill 使用 **Agent Team + Task 生命周期 + 双列交付清单** 实现 phase 间同步。核心原则：

> **不进入 Phase N+1 直到 Phase N 的所有 task 都已 completed，且所有 findings 已收到。**

`TaskList` 告诉你 teammate 是否结束了——但它不告诉你数据是否已送达。Task completed 和 Findings received 是两个独立信号，必须分别校验。

### 交付协议

每个 reviewer prompt 文件末尾包含「交付协议」: **先用 SendMessage 发送 findings，后用 TaskUpdate 标记 completed。顺序不能颠倒。** 这确保数据在任务标记完成之前就已送达主 Agent。

### 主 Agent 的交付清单

每个 phase 启动后，主 Agent 维护一个双列表：

```
| Reviewer              | Task Completed | Findings Received |
|-----------------------|----------------|-------------------|
| code-reviewer         | ...            | ...               |
```

两个状态来源不同：
- **Task Completed**: 通过 `TaskList` 检查
- **Findings Received**: 通过 teammate-message 检查（收到包含 JSON findings 的消息）

### 每 phase 执行步骤

1. 主 Agent 为该 phase 创建 task（`TaskCreate`）并 spawn teammate（`Agent` 带 `team_name`）
2. 用 `TaskUpdate` 将 task 指派给 teammate（`owner` 设为 teammate 的 `name`）
3. 主 Agent 初始化交付清单（两列均标记 pending），然后结束当前 turn
4. 收到通知后：
   a. 如果是 teammate-message 包含 findings → 清单中「Findings Received」打勾
   b. 检查 `TaskList` → 更新「Task Completed」列
5. **仅当该 phase 所有行两列全部打勾，才进入下一 phase**
6. 部分完成 → 不做任何事，结束 turn，等下一批通知
7. 如果一个 reviewer 的 task 标记 `completed` 但 5 分钟内没有 findings 消息 → 将该维度视为 `failed`，用剩余 findings 继续

## 执行协议

### 第零步：勘察

通读关键文件以了解项目范围：

- `CLAUDE.md`、`README.md`、`SPEC.md`（或等效文件）
- 顶级目录列表
- 包清单（`Cargo.toml`、`package.json`）

确定 `{project_path}` —— 项目根的绝对路径。
确定 `{review_scope}` —— 用户指定的范围，或默认「全部源文件」。

### 第一步：创建 Team 并启动审查

#### 1a. 创建 Team

```
TeamCreate({ team_name: "review-<project>", description: "Multi-dimension project review" })
```

此后所有 teammate 通过 `Agent({ team_name: "review-<project>", name: "...", ... })` 加入。

#### 1b. 启动 6 个 Reviewer

对每个维度，在同一 turn 内完成以下操作：

1. `Agent` — spawn teammate，`subagent_type: "general-purpose"`，prompt 从对应文件读取并追加项目路径。每个 prompt 文件已包含交付协议，确保 reviewer 知道如何交付结果
2. `TaskCreate` — 建 task 描述审查任务
3. `TaskUpdate` — 将 task 的 `owner` 设为该 teammate 的 `name`

全部启动后，主 Agent 初始化一张 6 行 × 2 列的交付清单（Task Completed + Findings Received），然后结束 turn。

| 名称 | 维度 | Prompt 文件 | 关注点 |
|------|------|-----------|--------|
| `code-reviewer` | `code-review` | `prompts/code-review.md` | Bug、崩溃、竞态、泄漏、类型错误、错误吞没 |
| `design-reviewer` | `design-review` | `prompts/design-review.md` | 架构、API 设计、状态管理、数据模型、文件 I/O |
| `observability-reviewer` | `observability-review` | `prompts/observability-review.md` | 日志覆盖、错误传播、静默失败、panic hook |
| `practices-reviewer` | `practices-review` | `prompts/practices-review.md` | 框架习惯用法、项目结构、重复代码、测试 |
| `library-reviewer` | `library-review` | `prompts/library-review.md` | 依赖 API 使用正确性、废弃 API、ReDoS、watcher 模式 |
| `devils-advocate-l1` | `devils-advocate-l1` | `prompts/devils-advocate.md` | 挑战假设、最弱环节、过度/不足工程、崩溃假说 |

**全部启动后结束 turn。** 不要提前进入第二阶段。

#### 1c. 等待并收集

主 Agent 在 1b 阶段已初始化交付清单。每次收到通知时：

1. 如果是 teammate-message 包含 findings JSON → 标记该 reviewer 的「Findings Received」列
2. 检查 `TaskList` → 更新「Task Completed」列
3. 两列全勾 → 该 reviewer 的交付完成
4. 全部 6 行两列都打勾 → 进入 1d
5. 部分完成 → 不做任何事，结束 turn，等下一批通知

> ⚠️ **超时规则**: 如果某个 reviewer 的 task 已标记 `completed` 但 5 分钟内没有收到 findings 消息，将该维度视为 `failed`（task 完成了但数据未送达），用剩余 findings 继续。

#### 1d. 处理 Reviewer 输出

从各 reviewer 的 teammate-message 中提取 JSON findings。确认所有已完成的 reviewer 的 findings 均已收到。

**统一的输出 schema** —— 每个 reviewer 必须返回以下结构：

```json
{
  "dimension": "code-review",
  "status": "ok | failed",
  "findings": [
    {
      "file": "src-tauri/src/commands.rs",
      "line": 142,
      "severity": "CRITICAL | HIGH | MEDIUM | LOW",
      "category": "bug | design | observability | practices | library | assumption | security | ux | other",
      "summary": "一句话说明",
      "detail": "完整解释。什么情况下发生、影响是什么、为什么值得关注。",
      "confidence": 0.85
    }
  ]
}
```

- `line` 可以为 null（适用于跨模块或架构级别的发现）
- `confidence` 是 reviewer 对自身判断的把握（0 = 猜测，1 = 确定）
- 如果 reviewer 的 task 失败，视为 `{ "dimension": "...", "status": "failed", "findings": [] }`
- 收集所有 `findings`，为每个 finding 标记 `source_dimension`

### 第二步：验证 —— 对抗性检查

#### 2a. 筛选与去重

取第一阶段中 `severity` 为 `"CRITICAL"` 或 `"HIGH"` 的 findings。

先去重：如果两个 reviewer 报告了同一个 `file:line`，合并为一个 finding：
- 合并 `source_dimensions`（多 reviewer 独立发现 = 更高基准置信度）
- 保留更高的 `confidence`
- 如有分歧，保留更严重的 `severity`

**如果没有 CRITICAL 或 HIGH finding，跳过第二阶段，直接进入第三阶段。**

#### 2b. 启动 Verifier

对每个去重后的 finding，并行启动 teammate 验证：

- **普通 HIGH**：1 个 verifier
- **CRITICAL**：3 个 verifier（并行），≥2 个说 `real: true` 才保留

使用 **`prompts/verify-finding.md`** 作为 prompt。追加 finding 的具体信息（file, line, summary, severity, source_dimensions）。

每个 verifier 的 task 描述应包含 finding 的完整上下文。

**全部启动后结束 turn。**

#### 2c. 等待并收集

使用与 1c 相同的交付清单模式：为每个 verifier 创建清单行，收到 findings 消息 + task completed 两列都打勾后才判定该 verifier 完成。

判定规则：
- `real: false` → 丢弃该 finding
- `real: true` → 保留
- CRITICAL 加强验证：3 个 verifier 中 <2 个 `real: true` → 丢弃
- 如果 verifier 的 task 失败 → 保守处理，保留该 finding（不丢弃未经证伪的 CRITICAL/HIGH）

### 第三步：汇总

主 Agent 自己完成（不 spawn teammate）：

1. **跨维度去重。** 相同的 `file:line` 或相同根因 → 合并。标记 `source_dimensions`。
2. **排名。** 按 severity 排序 → 然后按独立发现的 reviewer 数量排序 → 然后按 confidence 排序。
3. **分批到优先级：**

| 级别 | 标准 | 行动 |
|------|------|------|
| P0 | 会崩溃或丢数据 | 立即修复 |
| P1 | 降低可靠性或可调试性 | 本 session 修复 |
| P2 | 代码质量、重复、缺测试 | 进入 backlog |

4. **输出汇总表格：**

```
| # | 严重度 | 类别 | 简述 | 位置 | 来源维度 | 置信度 |
|---|--------|------|------|------|----------|--------|
| 1 | CRITICAL | bug | ... | foo.rs:42 | code,design | 0.92 |

P0: 2 项
P1: 5 项
P2: 11 项
```

5. **输出 top 5-10 修复建议**，按收益/成本比排序。

### 第四步：反对意见（二层）—— 元批评

启动 1 个 teammate（`name: "devils-advocate-l2"`），使用 **`prompts/devils-advocate-l2.md`** 作为 prompt。

将第三步的汇总表格 + 所有 finding 详情作为 prompt 参数（追加在 prompt 文件内容之后）。

创建 task → 指派 → 初始化交付清单 → 等 task completed + findings 消息两者都收到。

### 第五步：完整性审视

启动 1 个 teammate（`name: "completeness-reviewer"`），使用 **`prompts/completeness-review.md`** 作为 prompt。

输入（追加在 prompt 文件内容之后）：
- 各维度的 findings 数量摘要
- 第四步的元批评反馈
- 项目目录结构

创建 task → 指派 → 初始化交付清单 → 等 task completed + findings 消息两者都收到。

### 第六步：最终报告

主 Agent 将所有阶段合并为最终报告，交付给用户：

1. 汇总表格（第三步）
2. 优先级分布（P0/P1/P2）
3. 按收益/成本比排序的 top 修复建议
4. 元批评发现（第四步）
5. 完整性审视发现（第五步）
6. Reviewer 健康度：哪些维度 task 失败了（如果有）

### 第七步：清理

向所有 teammate 发送 `shutdown_request` → `TeamDelete`。

## 错误处理

| 场景 | 处理方式 |
|------|----------|
| Reviewer task failed | 标记该维度为 `failed`，用剩余 findings 继续 |
| 6 个 reviewer 全部 failed | 终止，报告失败，建议人工审查 |
| Verifier task failed | 保留该 finding（保守处理——不丢弃未经证伪的 CRITICAL/HIGH） |
| 第四步或第五步 task failed | 跳过该阶段，在报告中注明 |
| Teammate 超时未完成（>10 min） | 将该 task 标记 failed，按对应规则处理 |
| Task completed 但 findings 未收到（>5 min） | 将该维度标记 failed，用剩余 findings 继续——数据交付失败 |
| 项目源文件 > 200 个 | 抽样：请用户缩小范围，或仅审查关键路径 |

## Prompt 文件

所有 prompt 存放在本 skill 目录的 `prompts/` 下。每个文件是独立、完整的 prompt，直接传给 teammate（Agent 的 prompt 参数）。

| 文件 | 步骤 | 用途 |
|------|------|------|
| `code-review.md` | 第一步 | Bug、崩溃、竞态、泄漏、类型错误、错误吞没 |
| `design-review.md` | 第一步 | 架构、API 设计、状态管理、数据模型、文件 I/O |
| `observability-review.md` | 第一步 | 日志覆盖、错误传播、静默失败、panic hook |
| `practices-review.md` | 第一步 | 框架习惯用法、项目结构、重复代码、测试 |
| `library-review.md` | 第一步 | 依赖 API 使用正确性、废弃 API、ReDoS、watcher 模式 |
| `devils-advocate.md` | 第一步 | 挑战假设、最弱环节、过度/不足工程、崩溃假说（L1） |
| `verify-finding.md` | 第二步 | 对抗性验证：发现是真问题还是误报 |
| `devils-advocate-l2.md` | 第四步 | 元批评：盲区、回声室、严重度误判、关注点偏移 |
| `completeness-review.md` | 第五步 | 完整性审视：未访问子树、未覆盖风险、P0 预测 |

**Prompt 编写原则：**

- 描述审查视角和方法论，不描述过往发现
- 说明找什么，而不是上次找到了什么
- 要求结构化输出（统一的 schema）
- 保持聚焦——每个 prompt 一个维度
- 当项目的技术栈或架构变更时更新，而非每次 review 后更新
- **每个 prompt 文件必须包含「交付协议」章节**，明确 SendMessage 先于 TaskUpdate 的顺序。这是防止 reviewer 静默完成的关键机制

如果某个维度的 prompt 文件不存在，按照上述原则生成内联 prompt。

## 本 Skill 不做什么

- **不持久化。** 每次 `/review-project` 是一次全新运行。不在调用之间保存状态。
