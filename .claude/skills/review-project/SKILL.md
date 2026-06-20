---
name: review-project
description: 多 Agent 并行项目审查工具。产出按严重度排名的结构化 findings 报告。
disable-model-invocation: true
---

# 项目审查

对项目进行多 Agent 综合审查。在重大变更后、里程碑前、或用户要求 review / audit / 审查时调用。

## 总体架构

```
┌─ 第一步：审查 ────────────────────────────────────────────┐
│  7 个 subagent 并行执行（一条消息一批发出）。各自通读整个  │
│  项目，最终消息返回结构化 findings，自动回到主 Agent。     │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ 第二步：验证 ───────────▼────────────────────────────────┐
│  每个 HIGH+ 的 finding 派一个 subagent（并行）。对抗性检查：│
│  这个发现是真问题还是误报？                                │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ 第三步：汇总 ───────────▼────────────────────────────────┐
│  主 Agent 执行：去重 → 按严重度排名 → P0/P1/P2 分批       │
│  → 结构化报告                                             │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ 第四步：反对意见（二层）───▼──────────────────────────────┐
│  1 个 subagent。看到所有 findings。元批评：盲区、回声室、   │
│  严重度误判。                                              │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ 第五步：完整性审视 ─────▼────────────────────────────────┐
│  1 个 subagent。「哪个子目录 / 风险类型 / 故障模式完全没有  │
│  finding？什么从来没有被关注过？」                          │
└─────────────────────────────────────────────────────────────┘
```

## 执行模型

本 skill 用**前台 subagent** 实现各 phase。核心机制只有一句：

> **每个 phase = 在同一条消息里 spawn 该 phase 的全部 subagent（前台，不带 `run_in_background`）→ 它们并行执行 → 各自的最终消息（纯 JSON）作为 `Agent` 工具的返回值自动回到主 Agent。**

要点：

- **前台 = 自动回传**：subagent 的最终消息直接作为 `Agent` 工具返回值回来。无需 `SendMessage`、无需 Task 协调、无需交付清单。
- **一批发出 = 并行**：把该 phase 的 N 个 `Agent` 调用写在**同一条消息**里，它们并发执行；主 Agent 这一 turn 阻塞到**整批**返回（天然栅栏），总耗时 ≈ 最慢的那个，而非相加。
- **顺序屏障**：一个 phase 的全部返回拿齐后，才进入下一 phase。
- **不持久**：subagent 跑完即返回，无常驻 teammate，无需清理。

### 进度汇报

每完成一个 phase，主 Agent 向用户发送进度摘要后再进入下一 phase。摘要含该 phase 的关键数字和下一步动作。

| Phase | 汇报内容 |
|-------|----------|
| 第一步审查 | "7 个 reviewer 全部返回，共 N 个 findings（C CRITICAL, H HIGH）。进入验证。" |
| 第二步验证 | "验证完成：W 个 finding 确认有效，D 个被证伪丢弃。进入汇总去重。" |
| 第三步汇总 | "汇总完成：P0 X 项，P1 Y 项，P2 Z 项。进入元批评。" |
| 第四步反对意见 | "元批评发现 K 个新观察。进入完整性审视。" |
| 第五步完整性审视 | "完整性审视发现 M 个盲区/未覆盖风险。生成最终报告。" |

## 执行协议

### 第零步：勘察

> **前置条件**：本 skill 依赖 `Agent` 工具 spawn 前台 subagent 的能力（标准特性，无需任何实验开关）。若 spawn 失败，按「错误处理」表如实上报，不要硬撑。

通读关键文件以了解项目范围：

- `CLAUDE.md`、`README.md`、`SPEC.md`（或等效文件）
- 顶级目录列表
- 包清单（`Cargo.toml`、`package.json`）

确定 `{project_path}` —— 项目根的绝对路径。
确定 `{review_scope}` —— 用户指定的范围，或默认「全部源文件」。

### 第一步：启动审查

#### 1a. 启动 7 个 Reviewer

在**同一条消息**里，对下表每个维度发一个 `Agent` 调用（共 7 个，并行）：

- `subagent_type: "general-purpose"`，**前台**（不带 `run_in_background`）
- prompt 从对应文件读取，在末尾追加：`\n\n---\n\nProject path: {project_path}\n\nRead ALL source files in: {列出关键源码目录}`
- 每个 prompt 文件已要求 subagent 把最终消息设为纯 JSON 结果

| 维度 | Prompt 文件 | 关注点 |
|------|-----------|--------|
| `code-review` | `prompts/code-review.md` | Bug、崩溃、竞态、泄漏、类型错误、错误吞没 |
| `design-review` | `prompts/design-review.md` | 架构、API 设计、状态管理、数据模型、文件 I/O |
| `observability-review` | `prompts/observability-review.md` | 日志覆盖、错误传播、静默失败、panic hook |
| `practices-review` | `prompts/practices-review.md` | 框架习惯用法、项目结构、重复代码 |
| `test-quality-review` | `prompts/test-quality-review.md` | 测试覆盖缺口、测试有效性、错误路径覆盖、测试架构 |
| `library-review` | `prompts/library-review.md` | 依赖 API 使用正确性、废弃 API、ReDoS、watcher 模式 |
| `devils-advocate-l1` | `prompts/devils-advocate.md` | 挑战假设、最弱环节、过度/不足工程、崩溃假说 |

主 Agent 这一 turn 阻塞到 7 个全部返回。

#### 1b. 处理 Reviewer 输出

从 7 个 `Agent` 返回值中解析 JSON findings。

**统一的输出 schema** —— 每个 reviewer 返回以下结构：

```json
{
  "dimension": "code-review",
  "status": "ok | failed",
  "findings": [
    {
      "file": "src-tauri/src/commands.rs",
      "line": 142,
      "severity": "CRITICAL | HIGH | MEDIUM | LOW",
      "category": "bug | design | observability | practices | library | assumption | security | ux | test | other",
      "summary": "一句话说明",
      "detail": "完整解释。什么情况下发生、影响是什么、为什么值得关注。",
      "confidence": 0.85
    }
  ]
}
```

- `line` 可以为 null（适用于跨模块或架构级别的发现）
- `confidence` 是 reviewer 对自身判断的把握（0 = 猜测，1 = 确定）
- 如果某个 reviewer 返回错误或无法解析，视为 `{ "dimension": "...", "status": "failed", "findings": [] }`
- 收集所有 `findings`，为每个 finding 标记 `source_dimensions`（数组，初始含该 reviewer 一个维度；第二/三步去重时追加合并的其他维度）

> **schema 例外**：上述统一 `findings[]` schema 适用于第一步的 7 个 reviewer 与第四步的 `devils-advocate-l2`。第二步的 `verify-finding` 与第五步的 `completeness-review` 的产出结构不同（见各自 prompt 文件），是有意的例外，不套用 `findings[]`。

向用户发送 Phase 1 进度摘要（见执行模型 > 进度汇报），然后进入第二步。

### 第二步：验证 —— 对抗性检查

#### 2a. 筛选与去重

取第一步中 `severity` 为 `"CRITICAL"` 或 `"HIGH"` 的 findings。

去重（基于 `file:line`）：如果两个 reviewer 报告了同一个 `file:line`，合并为一个 finding：
- 合并 `source_dimensions`（多 reviewer 独立发现 = 更高基准置信度）
- 保留更高的 `confidence`
- 如有分歧，保留更严重的 `severity`

注意：Phase 2 的去重是粗粒度的（按 `file:line`），用于决定哪些 finding 需要验证。Phase 3 的去重是细粒度的（按根因），用于最终报告——同一个问题可能被不同 reviewer 定位到不同 file:line（如一个指向 error_log.rs:52，另一个指向 error_log.rs:52-78「所有调用点」），Phase 3 会按根因合并。

**如果没有 CRITICAL 或 HIGH finding，跳过第二步，直接进入第三步。**

#### 2b. 启动 Verifier

对每个去重后的 finding，在**同一条消息**里并行 spawn 前台 subagent 验证：

- **普通 HIGH**：1 个 verifier
- **CRITICAL**：3 个 verifier（并行），≥2 个说 `real: true` 才保留

每个 `Agent` 调用：`subagent_type: "general-purpose"`、**前台**、prompt 用 **`prompts/verify-finding.md`**，在末尾追加该 finding 的具体信息（finding_id, file, line, summary, severity, source_dimensions）。`finding_id` 是主 Agent 为该 finding 分配的稳定编号，verifier 在返回的 JSON 里原样回填，用于把验证结论对回原 finding（CRITICAL 的 3 个 verifier 也靠它归组计票）。

主 Agent 这一 turn 阻塞到全部 verifier 返回。

#### 2c. 判定

从 verifier 返回值解析判定：

- `real: false` → 丢弃该 finding
- `real: true` → 保留
- CRITICAL 加强验证：3 个 verifier 中 <2 个 `real: true` → 丢弃
- 如果某个 verifier 返回错误 → 保守处理，保留该 finding（不丢弃未经证伪的 CRITICAL/HIGH）

向用户发送 Phase 2 进度摘要，然后进入第三步。

### 第三步：汇总

主 Agent 自己完成（不 spawn subagent）：

1. **跨维度去重。** 相同的 `file:line` 或相同根因 → 合并。标记 `source_dimensions`。
2. **排名。** 按严重度排序（以 Phase 2 verifier 的 `adjusted_severity` 为准，无调整则用 Phase 1 原始 `severity`）→ 然后按独立发现的 reviewer 数量排序 → 然后按 confidence 排序。
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

向用户发送 Phase 3 进度摘要，然后进入第四步。

### 第四步：反对意见（二层）—— 元批评

spawn 1 个**前台 subagent**，使用 **`prompts/devils-advocate-l2.md`** 作为 prompt，在末尾追加第三步的汇总表格 + 所有 finding 详情。等其返回值。

> **定位**：第四步（L2 元批评）与第五步（完整性审视）是**顾问层**——其产出（含 L2 指出的「严重度误判」）作为最终报告的独立章节呈现，**不回灌、不重排**第三步已定稿的优先级表。这样优先级表保持稳定，元批评单独可见，人来裁量是否据此调整。

向用户发送 Phase 4 进度摘要，然后进入第五步。

### 第五步：完整性审视

spawn 1 个**前台 subagent**，使用 **`prompts/completeness-review.md`** 作为 prompt，在末尾追加：

- 各维度的 findings 数量摘要
- 第四步的元批评反馈
- 项目目录结构

等其返回值。

向用户发送 Phase 5 进度摘要，然后进入第六步生成最终报告。

### 第六步：最终报告

主 Agent 将所有步骤合并为最终报告，交付给用户：

1. 汇总表格（第三步）
2. 优先级分布（P0/P1/P2）
3. 按收益/成本比排序的 top 修复建议
4. 元批评发现（第四步）
5. 完整性审视发现（第五步）
6. Reviewer 健康度：哪些维度失败了（如果有）

## 错误处理

| 场景 | 处理方式 |
|------|----------|
| 某个 reviewer subagent 返回错误或无法解析 | 标记该维度为 `failed`，用剩余 findings 继续 |
| 7 个 reviewer 全部 failed | 终止，报告失败，建议人工审查 |
| 某个 verifier subagent 返回错误 | 保留该 finding（保守处理——不丢弃未经证伪的 CRITICAL/HIGH） |
| 第四步或第五步 subagent 失败 | 跳过该步，在报告中注明 |
| 无法 spawn subagent | 不硬撑：如实告知用户本 skill 需运行在能 spawn subagent 的 session，并建议改用单 Agent 人工审查 |
| 项目源文件 > 200 个 | 抽样：请用户缩小范围，或仅审查关键路径 |

## Prompt 文件

所有 prompt 存放在本 skill 目录的 `prompts/` 下。每个文件是独立、完整的 prompt，直接传给 subagent（`Agent` 的 prompt 参数）。

| 文件 | 步骤 | 用途 |
|------|------|------|
| `code-review.md` | 第一步 | Bug、崩溃、竞态、泄漏、类型错误、错误吞没 |
| `design-review.md` | 第一步 | 架构、API 设计、状态管理、数据模型、文件 I/O |
| `observability-review.md` | 第一步 | 日志覆盖、错误传播、静默失败、panic hook |
| `practices-review.md` | 第一步 | 框架习惯用法、项目结构、重复代码 |
| `test-quality-review.md` | 第一步 | 测试覆盖缺口、测试有效性、错误路径覆盖、测试架构 |
| `library-review.md` | 第一步 | 依赖 API 使用正确性、废弃 API、ReDoS、watcher 模式 |
| `devils-advocate.md` | 第一步 | 挑战假设、最弱环节、过度/不足工程、崩溃假说（L1） |
| `verify-finding.md` | 第二步 | 对抗性验证：发现是真问题还是误报 |
| `devils-advocate-l2.md` | 第四步 | 元批评：盲区、回声室、严重度误判、关注点偏移 |
| `completeness-review.md` | 第五步 | 完整性审视：未访问子树、未覆盖风险、P0 预测 |

**Prompt 编写原则：**

- 描述审查视角和方法论，不描述过往发现
- 说明找什么，而不是上次找到了什么
- 要求结构化输出（第一步 reviewer 与 L2 用统一的 `findings[]` schema；verify、completeness 用各自定义的 schema）
- 保持聚焦——每个 prompt 一个维度
- 当项目的技术栈或架构变更时更新，而非每次 review 后更新
- **每个 prompt 文件必须包含「返回结果」章节**，要求 subagent 把最终消息设为纯 JSON、且不调用 `SendMessage` 或任务工具。这是保证主 Agent 能拿到结果的关键

如果某个维度的 prompt 文件不存在，按照上述原则生成内联 prompt。

## 本 Skill 不做什么

- **不持久化。** 每次 `/review-project` 是一次全新运行。不在调用之间保存状态。
