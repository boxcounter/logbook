# 命名审查与改进 设计

- 日期：2026-06-21
- 状态：已批准设计，待写实施计划
- 范围：前端 + 后端命名清晰度改进，零落盘迁移

## 背景

起点是一个具体观感：`TwoLineInput.vue` 这个名字看了不知道它负责什么——它按**外观/结构**（"两行输入"）命名，而非按**职责**。由此发起一次全项目命名审查。

一次 very thorough 扫描（前端组件/类型/store/utils + 后端 Rust 模块/struct/函数）产出 19 处候选，本设计据证据收窄到值得动手的子集。

## 两个经核实的结论（先于方案）

### 结论 1：本任务无需任何磁盘格式迁移

`src-tauri/src/operation_log.rs:66-106` 中，落盘的 op 字符串（`"append"` / `"update"` / `"delete"` / `"set_day_note"`）是 `match` 分支里**硬编码的字面量**，不是 serde 从 `Operation` 枚举派生；replay 端（`:196-259`）同样按这些字面量匹配。**Rust 枚举标识符从不落盘。** 因此即便重命名 `Operation::*` 也是零迁移（本设计未纳入该改名）。

真正落盘的 struct（`Entry` / `DayFile` / `Commitment` / `Config`，见 `models.rs`）字段名（`item` / `duration` / `dimensions` / `role` / `allocation` / `goals` …）本就清晰，**不在改名清单内**。后端无任何 `serde(rename)`（已 grep 确认），即「serde 字段名 == Rust 字段名」，但本设计不改动任何落盘字段名。

→ 整个任务不触碰磁盘格式，不需要兼容/迁移逻辑。

### 结论 2：项目命名并非系统性糟糕，TwoLineInput 是唯一明确的「按外观命名」

证据不支持「这种坏命名普遍存在」的假设：

- 明确「按外观/结构命名」的只有 1 处：`TwoLineInput`。
- 被初判 HIGH 的 `DimensionPopover` / `CommitmentsModal` / `CommitmentsPanel` / `RoleCard` / `GoalRow`，均为「领域名 + UI 后缀（Popover/Modal/Panel/Card/Row）」的常规约定，能自解释、不误导——**不改**。
- 其余为少数 DTO 标识符与内部变量的清晰度改进。

→ 这是「一个明确改名 + 一小撮澄清 + 固化一条规则防复发」，不是大重构。

## 范围

### 纳入（改名清单）

| # | 当前 | 目标 | 层 | 波及文件 |
|---|------|------|----|---------|
| 1 | `TwoLineInput.vue` | `EntryComposer.vue` | 前端 | `TwoLineInput.vue`、`MonthView.vue`、`__tests__/components/TwoLineInput.test.ts`（连带改名）、`__tests__/components/MonthView.test.ts` |
| 2 | DimensionPopover 内部变量 `phase` / `activeIndex` / `activeDimKey` | `stage` / `highlightedIndex` / `selectedDimKey` | 前端 | `DimensionPopover.vue` 内部（及其 test 中对这些状态的断言，如有） |
| 3 | `lastDimensions`（useStore） | `lastUsedDimensions` | 前端 | `stores/useStore.ts` 及引用处、`__tests__/useStore.test.ts` |
| 4 | `NewEntry` / `UpdateEntry`（类型标识符） | `CreateEntryInput` / `UpdateEntryInput` | 前端+后端 | `types.ts`、`MonthView.vue`、`models.rs`、`commands.rs`、`files.rs`、`operation_log.rs` |
| 5 | `Screen`（类型标识符） | `AppPhase` | 前端 | `types.ts`、`App.vue`、`stores/useStore.ts`、`__tests__/components/App.test.ts`、`__tests__/components/SetupScreen.test.ts` |

附：**轻量命名约定文档** `docs/naming-conventions.md`（防复发）。

### 经复查后剔除

- **#6 `AvailableMonth`**：初判为「单数名存数组」。复查证伪——`AvailableMonth` 表示**单个**月份，`availableMonths: AvailableMonth[]` 是其数组，命名本就正确。**不改名**；如需可仅补一行 doc 注释说明语义。（此项原被选入，复查后建议跳过，请在 spec 评审时确认。）
- **#7**：`DimensionPopover` / `CommitmentsModal` / `CommitmentsPanel` / `RoleCard` / `GoalRow`——「领域名 + UI 后缀」常规约定，不误导，**不改**。
- **#8**：`filledDims` / `missingRequired` 等内部变量——小文件内有注释，属吹毛求疵，**不改**。

### 明确的非目标

- **不重命名落盘字面量**（op log 的 `"append"` 等、frontmatter 字段名）。
- **不改 IPC 命令名**（`add_entry` / `update_entry` 等）与 **serde 字段名**——#4 只改 Rust struct / TS interface 的**标识符**，payload 形状不变。
- **不重定位文件**：`EntryComposer.vue` 留在 `src/components/`，不移入 `composite/`（属正交重构，避免范围蔓延）。
- **不改 #5 的字符串值**（`"loading"|"setup"|"error"|"ready"` 保留），只改类型标识符；**不触碰 `SetupScreen.vue` 组件**（grep 子串误匹配）。

## 实施方式：分层小批（方案 B）

按风险分批，每批跑完验证再推进；符合项目 Phase checkpoint 规则、可 bisect。

### 批 1 — 纯前端、零契约

含 #1、#2、#3、#5。均为前端内部标识符/变量/类型，不触任何前后端契约。

- #1：`git mv` 组件与测试文件 → `EntryComposer.vue` / `EntryComposer.test.ts`；改组件内自引用注释、`MonthView.vue` 的 import 与标签、相关测试的 import 与 describe。
- #2：在 `DimensionPopover.vue` 内重命名三个 ref/状态；同步其 test 中相关断言。
- #3：`lastDimensions` → `lastUsedDimensions`，改 store 定义与所有引用、测试。
- #5：`Screen` → `AppPhase`（仅类型标识符），改 `types.ts` 导出、`App.vue` / `useStore.ts` 的类型标注、测试中的类型引用。

**验证**：`pnpm vue-tsc --noEmit` + `pnpm test`（vitest）全绿。

### 批 2 — 跨层 IPC 标识符（前后端同步）

含 #4。

- 后端：`models.rs` 重命名 struct；`commands.rs` / `files.rs` / `operation_log.rs` 中的类型引用同步。serde 字段名与命令签名的**字段顺序/名称不变**。
- 前端：`types.ts` 重命名 interface；`MonthView.vue` 调 `invoke` 时的类型标注同步。
- 因无 `serde(rename)` 且只改标识符，IPC payload 字节形状不变。

**验证**：`pnpm vue-tsc --noEmit` + `cd src-tauri && cargo check && cargo test` + `pnpm test` 全绿；手动 `pnpm tauri dev` 冒烟一次增/改条目，确认前后端 invoke 仍通。

### 批 3 — 文档与防复发

- 创建 `docs/naming-conventions.md`（内容见下）。
- 为 #6 决策落一行结论（如确认跳过，可在约定文档的「常见误判」小节记一笔，或仅补 `AvailableMonth` 注释）。
- 跑 `/check-consistency`，与 `CLAUDE.md` / `SPEC.md` 对齐；若 `CLAUDE.md` 项目规则需要指向新约定文档，则补链接。

**验证**：一致性检查通过。

## 交付物：`docs/naming-conventions.md`（约 1 屏）

固化项目**已有的**隐含约定 + 本次教训：

1. **组件按职责命名，不按外观/结构**。✗ `TwoLineInput`（描述"两行"布局）→ ✓ `EntryComposer`（描述"组合一条 Entry"）。UI 后缀（`Popover` / `Modal` / `Panel` / `Card` / `Row`）允许且鼓励，因为它表达组件角色。
2. **目录分层**：`base/` 放无领域逻辑的 UI 原语；`composite/` 放含领域逻辑的组合组件。
3. **输入 DTO 用 `*Input` 后缀** 区别于持久化实体（如 `CreateEntryInput` vs `Entry`），点明「时长是前端预解析的字符串」这类跨层差异。
4. **前后端同一概念用同一词**；**serde 字段名即 IPC 契约**，改字段名需两端同步并视作破坏性变更。
5. **落盘格式与代码标识符解耦**：op log 的 op 字面量、frontmatter 字段名是磁盘契约，可自由重命名 Rust/TS 标识符，但**勿动落盘字面量**，除非走显式迁移。

## 风险与缓解

- **遗漏引用导致编译/测试红**：靠 `vue-tsc` + `cargo check` + 两套测试在每批末尾兜底；分批使红区可定位。
- **#4 误改 serde 字段名**：明确只改 struct 标识符；验证含 `cargo test`（已有 serde roundtrip 测试，如 `models.rs` 的 `init_result_*_json_roundtrip`）+ `tauri dev` 冒烟。
- **测试文件改名遗漏**：`git mv` 测试文件并同步 import；构建会因 `noUnusedLocals` 对测试严格类型检查而暴露遗漏（见项目记忆：vitest 绿 ≠ build 绿）。

## 测试与验证总览

- 每批：`pnpm vue-tsc --noEmit`、相关 `pnpm test` / `cargo test`。
- 批 2 额外：`pnpm tauri dev` 手动冒烟（新增 + 编辑条目走通 IPC）。
- 收尾：`/check-consistency`。
- 不新增功能测试——本任务是纯重命名，行为不变；现有测试即回归网。

## 实施调整（2026-06-21 整合进 main 时）

合并时 main 已前移并合入 `worktree-entry-dim-ux`，与本设计重叠，故两处调整：

- **#3 `lastDimensions` 改名作废**：main 的 `e9be865` 已「不再预填上次 dimension」并**删除** `lastDimensions`（且加了断言其不存在）。要改名的符号已不存在，本项丢弃。
- **#5 `Screen` 的最终名改为 `AppStatus`（而非 `AppPhase`）**：评审中为避免 "phase" 与弹层两步导航的 `stage` 多义，改用业界通用的 `status`；同时把属性 `screen` 一并改为 `status`（`store.status`），使生命周期概念在类型与属性两侧一致。
- 其余改名（EntryComposer、popover `stage`/`selectedDimKey`/`highlightedIndex`、`CreateEntryInput`/`UpdateEntryInput`）按设计重新套用到 main 的当前代码，以更新后的测试套件为准验证全绿。
