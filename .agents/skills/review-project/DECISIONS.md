# review-project 设计决策记录

> 单一事实源。新决策往下追加,不覆盖旧条目。

---

## ADR-001：底座应为 subagents,而非 agent teams

- **日期**：2026-06-20
- **状态**：**已实施(2026-06-20)** —— 迁移到裸 subagents(选项 2)。Workflow(B)经评估后放弃,理由见下「最终决定」。
- **结论**：这个 skill 的正确底座是 **subagents**(实施时采用此底座);其产品化形态是 **Workflow**(因可维护性约束未采用)。原 **agent teams** 底座是结构性错配。

### 背景

review-project 用 agent teams 实现多 Agent 审查:lead 派 7 个 reviewer 并行通读项目 → 验证 → 汇总 → 元批评 → 完整性审视。为在 teams 上跑通,skill 手搓了一整套同步机制:交付协议(SendMessage 先于 TaskUpdate)、双列交付清单、超时重试、「结束 turn 等通知」。

### 决定性证据:通信拓扑里零条 teammate↔teammate 边

官方文档给 teams 的存在理由是「teammates 互相挑战 / 分享 / 自行协调」。但本 skill 的每一条通信边都是 teammate→lead 的星型:

| 阶段 | 通信 | 横向通信 |
|------|------|---------|
| 7 reviewer | 各自读项目 → 发 lead | 无 |
| verifier | 各自验一条 → 报 lead | 无 |
| L2 元批评 | 从 lead 拿汇总 → 报 lead | 无 |
| completeness | 从 lead 拿汇总 → 报 lead | 无 |

skill 是**对抗性的,但不是协作性的**:对抗价值来自「独立视角 + 验证层」,全部由 lead 居中调度。对抗 ≠ 对话。这种「扇出独立工作 → 汇总归约」是 subagents 的定义场景。连 shared task list 也是退化用法(lead 建并指派,teammate 只更新自己那条,无横向 self-claim)。

### 用 teams 付的代价

1. **手搓结果回传**:teams 不自动回传 payload(实测 + 文档一致:只有 idle/状态自动,结果须 teammate 主动 SendMessage)。整套交付协议 + 双列清单 + 超时重试,**100% 是在补偿这一个洞**。subagents 原生「results return to the caller」,这套全部蒸发。
2. **实验性、默认关闭**:没有 `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` 的 session 跑 `/review-project`,第一步 spawn 直接失败。
3. **更贵**:每个 teammate 是独立 Claude 实例;subagents 结果汇总回主上下文,成本更低。

### 给 teams 的无罪辩护(并说明为何不足以翻案)

- **官方有先例**:Anthropic「Run a parallel code review」示例用 teams 干同构的事。→ 但那是一次性演示,非硬化 skill;按其自身表格判据(是否横向通信)仍指向 subagents。
- **运行中可观测/可干预**:teams lead 全程在场,可戳卡住的 teammate、派替补。→ 但 review-project 价值是「跑完出报告」,中途人工操控不在目标内;其超时启发式本就是该原生能力的劣质替代。

去掉这两条,其余全部倒向 subagents。这两条好处都不属于 skill 的既定价值,代价却每次都付。

### 三个底座对比

| 底座 | 结果回传 | 协调 | 默认可用 | 成本 | 契合度 |
|------|---------|------|---------|------|--------|
| A：teams(现状) | 手搓交付协议 | 手搓双列清单+超时 | ❌ 需实验 flag | 高 | 错配 |
| 裸 subagents | 自动回传 | lead 跨 turn 手工编排 | ✅ | 低 | 对,但编排仍模型驱动、略脆 |
| **B：Workflow** | 自动 + schema 校验 | 确定性 JS(loop/dedup/fan-out) | ✅ | 低 | **最契合** |

关键衔接:**Workflow 的 `agent()` 就是 subagent 模型**(一次性、有返回值、可 schema 校验)+ 确定性编排。skill 的形状(扇出 7 → 归约 → 按 HIGH+ 再扇出 verifier → 归约 → 综合)是典型确定性 map-reduce,编排逻辑(去重、按严重度分批、CRITICAL 三验票)本就该是代码,不该让模型跨七八个 turn 手工盯。「改用 subagents」与「B」是同一条路的两个精细度,B 是终点。

### 推荐

- **最小纠偏**:把 reviewer/verifier 从「后台 teammate + 交付协议」改为 subagents(结果自动回传),删掉整套同步机械。比现状简单,且摆脱实验 flag 依赖。
- **做到位(推荐)**:迁移到 Workflow,一并获得确定性编排 + schema 校验。

### 最终决定(2026-06-20):裸 subagents,不上 Workflow

B 的产品化形态本是技术最优,但被一条更高优先级的约束否决:**owner 不熟 JS,需保留对 skill 的可读 / 可评估 / 可维护权**。

- Workflow 把编排逻辑搬进 JS,owner 无法审查 AI 对它的改动 —— 这是治理问题,不只是技能缺口。
- 裸 subagents 全程 Markdown,owner 能逐句审;且已修了底座(对的原语)、删了整套同步机械、摆脱实验 flag 依赖。
- B 的确定性收益对本 skill 不兑现:综合步骤(去重 / 排名 / 分批)在 teams 与 subagents 下本来都是模型驱动,选 subagents 不是退步,只是不升级成代码。

代价(已接受):不强制 schema 校验(lead 仍自行解析 JSON);前台批次期间无中途干预。

### 来源与可靠度

一手抓取自 `https://code.claude.com/docs/en/agent-teams.md`(2026-06-20),官方文档,高可靠度:

- 「Agent teams are experimental and disabled by default. Enable them by adding `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` ... Without that variable, no team is set up at session start ... Claude does not spawn or propose teammates.」
- 「Before v2.1.178, ... Claude used the `TeamCreate` and `TeamDelete` tools ... Both tools no longer exist. The `team_name` input on the Agent tool is accepted but ignored.」
- 表格:Subagents「results return to the caller」「Report results back to the main agent only」「Token cost: Lower」;Agent teams「Teammates message each other directly」「Token cost: Higher」。
- 「Use subagents when you need quick, focused workers that report back. Use agent teams when teammates need to share findings, challenge each other, and coordinate on their own.」
- 「Idle notifications: when a teammate finishes and stops, they automatically notify the lead.」+「Automatic message delivery: when teammates send messages, they're delivered.」(状态自动;结果须主动发)

**实测**(本 session,高可靠度):派一个不调用 SendMessage 的后台 teammate,完成后 lead 只收到 `idle_notification`,**收不到其产出正文** —— 证实 teams 不自动回传结果。

**实测**(本 session,高可靠度):同一条消息发出 3 个前台 subagent,各自最终消息作为 `Agent` 返回值自动回到 lead,`tool_uses: 0`(未用 SendMessage)—— 证实「前台 subagent 并行 + 自动回传」。

**仍属推断、未证实**(中可靠度):「前台并行时一个 subagent 卡住会拖垮整批」—— 官方文档未涵盖,本 session 也未实测。属已接受的低风险。

---

## 改动历程(2026-06-20,同一 session 内分两轮)

**第一轮:在 teams 底座上修复(底座后被第二轮取代,但 F1–F5 内容保留)**
1. 修 dead API:移除 `TeamCreate`/`TeamDelete`/`team_name`,改隐式单 team + `run_in_background: true`。
2. 防御性:第零步前置条件、错误处理表加行。
3. 一致性清理 F1–F5:`source_dimensions` 统一、schema 例外注明、「阶段/步」术语统一、verifier `finding_id` 补全、L2/completeness 定位为顾问层。

**第二轮:底座迁移到裸 subagents(实施 ADR-001)**
- SKILL.md:删「同步机制」整节、1a「关于 Team」、1c「等待并收集」、第七步「清理」;reviewer/verifier/L2/completeness 改为**前台 subagent 一批发出、结果自动回传**;新增「执行模型」节;前置条件改为只依赖 subagent spawn(去掉实验 flag 依赖);错误处理表去掉 teams 专有失败模式。
- 10 个 prompt 文件:「交付协议」节(SendMessage→TaskUpdate)替换为「返回结果」节(最终消息即纯 JSON,不调用 SendMessage/任务工具)。
- 第一轮的 `run_in_background: true` 被反转(改回前台);F1–F5 的内容修复予以保留。
