# Design: logbook-cli Agent Skill

**Date**: 2026-07-11
**Status**: Draft (pending user review)

## Context

logbook-cli 已经实现了完整的命令行操作能力（entries / commitments / dimensions / migrate），但 Agent（ZCode、Claude Code 等）不知道怎么正确使用它——命令从 stdin 读 JSON、duration 是字符串、两个不同的 `--json`、写命令会被 GUI 占锁失败……这些陷阱足以让 Agent 频繁出错。

slax-reader 已有一个成熟的 Agent Skill 作为参照模式。本设计为 logbook-cli 创建同构的配套 Skill。

## Goal

Agent 能通过 Skill 正确操作用户的工时数据：录时间、查时间、查进度、管理维度配置。**定位是「个人助理操作工时数据」，不是「开发调试 logbook 本身」。**

## Non-goals

- 不覆盖 `migrate`（一次性内部数据迁移，与日常工时操作无关）
- 不覆盖 build/install/bundle ID 隔离等开发陷阱
- 不教 Agent 从源码编译 CLI

## Decisions

### D1: 开发位置——本体 repo `skill/logbook-cli/`

不放独立仓库，也不放 `.agents/skills/`。

- **不放独立仓库**：Skill 是 CLI 的衍生文档，必须与 CLI 同步演化。独立 repo 会导致跨 repo drift（CLI 改 flag、Skill 没跟上）。
- **不放 `.agents/skills/`**：那是 project-scoped auto-discovery 路径，放进去意味着「服务 logbook repo 开发」，与本 Skill 的意图（操作用户工时数据）相反。
- **放 `skill/logbook-cli/`**：纯分发物，在 repo 里版本控制、跟 CLI 同步演化；安装时 `cp -r skill/logbook-cli ~/.agents/skills/`。

### D2: 安装说明的职责划分

| 文档 | 位置 | 内容 |
|------|------|------|
| repo README | `README.md` | 极简：一句话提及 CLI 存在 + 一行链接指向 Skill |
| Skill README | `skill/logbook-cli/README.md` | 自包含：定位、前置依赖、安装步骤 |

**理由**：CLI 安装由 app 菜单承担（已有「Install Command Line Tool…」），README 读者（开发者）不需要 CLI 安装步骤。Skill 用户就是 README 读者，其安装说明跟着 Skill 文件走，copy 时一起带走，自包含。

### D3: Skill 结构——复刻 slax-reader 骨架

单文件 `SKILL.md`，正文区段：

```
SKILL.md
├── frontmatter (name + description)
├── # Logbook CLI                        — 一句话定位
├── ## Before using commands             — 前置门（2 个）
├── ## Commands                          — Task → Command 表
├── ## Entry rules                       — 写 entry 的行为约束
├── ## Commitments rules                 — 写 commitments 的行为约束
├── ## Dimensions rules                  — 写 dimensions 的行为约束
├── ## Listing and querying              — 读语义
└── ## Security rules                    — 硬约束
```

### D4: 覆盖范围

| 命令域 | 覆盖 | 说明 |
|--------|------|------|
| entries list/add/update/delete | ✅ | 全部 |
| commitments list/progress/set | ✅ | 全部 |
| dimensions list/set | ✅ | 全部（set 用于创建/修改维度配置） |
| migrate | ❌ | 一次性内部工具，排除 |
| `--root-path` override | ❌ | 个人助理场景默认从 root_path.txt 读 |

## Skill 内容详述

### Frontmatter

```yaml
---
name: logbook-cli
description: "Use logbook-cli to read and write work time tracking data — list, add, update, delete entries, check commitments progress, view and edit dimensions. Trigger when the user asks to log time, check hours worked, see time breakdown by role/goal, or manage their Logbook dimensions."
---
```

### Before using commands（前置门）

两个检查，按执行顺序：

1. **Binary 可用性**：`which logbook-cli` 确认在 PATH。失败时引导用户：Logbook app → 菜单栏 → Install Command Line Tool… → 确认 `~/.local/bin` 在 PATH。
2. **GUI 占锁预检**（仅写命令需要）：执行写命令前检查 GUI 是否在运行。检测方式：`pgrep -fi "Logbook.app"`（或等价的 System Events 查询）。如果 GUI 开着，提醒用户关闭再继续——不要直接执行然后等报错。

检测脚本候选（skill 里给 agent 一个可直接用的命令）：

```bash
pgrep -fi "Logbook.app/Contents/MacOS" > /dev/null && echo "GUI_RUNNING" || echo "OK"
```

> **实现期需验证**：上述进程匹配模式是推测，实际进程名取决于 Tauri 打包后的 binary 名（可能是 `Logbook`、`logbook` 等）。实现时需启动 GUI 实测 `pgrep` 输出，确认匹配模式可靠，再写入 SKILL.md。误判（以为 GUI 没开）会导致写命令直接失败。

### Commands 表

| Task | Command |
|------|---------|
| List entries for a date | `logbook-cli entries list --date 2026-07-11` |
| List entries as JSON | `logbook-cli --json entries list --date 2026-07-11` |
| List today's entries | `logbook-cli entries list --date $(date +%Y-%m-%d)` |
| Add an entry | `echo '<json>' \| logbook-cli entries add --date 2026-07-11` |
| Update an entry | `echo '<json>' \| logbook-cli entries update --date 2026-07-11 --entry-id <uuid>` |
| Delete an entry | `logbook-cli entries delete --date 2026-07-11 --entry-id <uuid>` |
| Check commitments progress | `logbook-cli commitments progress --year 2026 --month 7` |
| List commitments | `logbook-cli commitments list --year 2026 --month 7` |
| Set commitments | `echo '<yaml/json>' \| logbook-cli commitments set --year 2026 --month 7` |
| List dimensions (month) | `logbook-cli dimensions list --year 2026 --month 7` |
| List template dimensions | `logbook-cli dimensions list --template` |
| Set dimensions (month, YAML) | `echo '<yaml>' \| logbook-cli dimensions set --year 2026 --month 7` |
| Set template dimensions | `echo '<yaml>' \| logbook-cli dimensions set --template` |

### Entry rules

- `duration` 是**字符串**传给服务端 `parse_duration`（如 `"90"`、`"1h30m"`），不是数字——直接传数字会被拒
- `entries add` stdin JSON 形状：`{"item": "...", "duration": "60", "dimensions": {"key": "value"}}`，`dimensions` 可选默认 `{}`
- `entries update` stdin JSON：所有字段可选，`{"item?": "...", "duration?": "...", "dimensions?": {...}}`
- `entries add/update/delete` 从 **stdin** 读输入，不是 CLI 参数——忘记 pipe 会 hang
- 写命令会触发 `integrity::check()`，数据目录损坏时直接失败——这是后端的权威校验，不要绕过

### Commitments rules

- `commitments set` 接受 JSON 或 YAML（stdin）
- **set 会传播 goal/role 重命名到已有 entries**，且保护被 entries 引用的 goals——这是后端行为，set 不是「只改配置」，它会改历史数据
- `allocation` 单位是**小时**（不是分钟）
- commitments JSON 形状：`[{"role": "Dev", "allocation": 40, "goals": ["v2 launch"]}]`

### Dimensions rules

- `dimensions set` 默认吃 YAML；加 `--json` 标志表示输入是 JSON（注意：这是 dimensions 子命令的**局部** `--json`，与全局 `--json` 含义不同）
- `dimensions list` 的人格式输出是 **YAML**（不是其他命令的文本格式），月度版首行有 `# source:` 注释
- `dimensions list` 也有一个局部 `--json`（输出 JSON 而非 YAML）
- Dimension 形状：`{name, key, source?, values?, required?, deleted?}`，`source` 默认 `"static"`，goal 维度用 `"commitments:role:goals"`
- 两个 `--json` 的区别要写清（这是最易踩的坑）：
  - 全局 `--json`（放在子命令前）：输出 JSON 而非人格式
  - `dimensions list/set` 的局部 `--json`（放在子命令后）：list 的输出格式 / set 的输入格式

### Listing and querying

- `entries list` 只接受单个 `--date`，没有范围查询——要查多天就跑多次
- `commitments progress` 人格式 `Role: X (NN% — Y.Yh / Zh)`；加全局 `--json` 得结构化数据
- 诊断信息（`Using data root:`、warning）走 **stderr**，数据走 **stdout**——可安全 pipe
- root path 解析：默认从 app-data dir 的 `root_path.txt` 读；个人助理场景不需要 `--root-path` override

### Security rules

- **不猜 `entry-id`**：先 `entries list` 拿到真实 id 再 update/delete
- **写操作前向用户确认内容**（新增/修改的具体内容）
- **`commitments set` 和 `dimensions set` 需明确确认**——前者会传播重命名到历史 entries，后者改变数据结构，影响面都大于录一条 entry
- **不编造 duration**：用户没说时长就问
- **删除需额外确认**：delete 是不可逆操作

## 产物清单

| 文件 | 作用 |
|------|------|
| `skill/logbook-cli/SKILL.md` | Skill 本体 |
| `skill/logbook-cli/README.md` | Skill 安装说明（自包含） |
| `README.md`（修改） | 追加 `## CLI` 节 |
| `docs/superpowers/specs/2026-07-11-logbook-cli-skill-design.md` | 本设计文档 |

## README.md 变更

在 Development 节后、Documents 节前追加：

```markdown
## CLI

Logbook ships with `logbook-cli`, a command-line tool for reading and
writing time data outside the GUI. Install it via the app menu:
**Logbook → Install Command Line Tool…**

An [Agent Skill](./skill/logbook-cli/) is available for AI agents
(ZCode, Claude Code, etc.) to operate your time data correctly.
```

## skill/logbook-cli/README.md 内容

```markdown
# logbook-cli Agent Skill

Companion skill for AI agents to operate Logbook time data via `logbook-cli`.

## Prerequisites

- logbook-cli installed (Logbook app → Install Command Line Tool…)
- `~/.local/bin` on your PATH

## Install

Copy to your agent's skill discovery path:

cp -r skill/logbook-cli ~/.agents/skills/
```
