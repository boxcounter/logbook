# 命名审查与改进 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 5 个不清晰的命名改成按职责自解释的名字，并固化一份命名约定文档防复发。

**Architecture:** 纯重命名重构，行为不变。无新功能、无新测试——**现有测试套件 + 类型检查就是验证网**。分 3 个批次按风险递增执行（前端内部 → 跨层 IPC 标识符 → 文档），每批末尾以 typecheck/test 全绿作为 gate。

**Tech Stack:** Vue 3 + TypeScript（前端，vitest + vue-tsc）；Tauri 2.x / Rust（后端，cargo check + cargo test）。

**设计依据：** `docs/superpowers/specs/2026-06-21-naming-audit-design.md`

**TDD 说明：** 本计划是行为保持的重命名，不写新测试。每个 task 的「验证」步骤是跑既有套件确认**仍然全绿**（重命名前后测试结果不变）；对测试文件的改动仅限于「指向被改名符号的标识符/字符串」。

**全局禁忌（每个 task 都适用）：**
- 不动落盘字面量（op log 的 `"append"`/`"update"`/… 字符串、frontmatter 字段名）。
- 不改 IPC 命令名（`append_entry`/`update_entry` 等）与 serde 字段名。
- 重命名一律用**全词匹配**（whole word / `\b` 边界），严防误伤子串（如 `handleUpdateEntry`、`activeDim`、`capture-phase`、`SetupScreen`、`new_entry`）。

---

## 批 1 — 前端内部标识符（零契约）

### Task 1: `TwoLineInput` → `EntryComposer`

**Files:**
- Rename: `src/components/TwoLineInput.vue` → `src/components/EntryComposer.vue`
- Rename: `src/__tests__/components/TwoLineInput.test.ts` → `src/__tests__/components/EntryComposer.test.ts`
- Modify: `src/components/MonthView.vue`（4 处：import、ref 类型、注释、模板标签）
- Modify: `src/__tests__/components/MonthView.test.ts`（5 处：describe 文案、注释、`findComponent({ name })` 字符串）

`TwoLineInput` 是全项目唯一 token（不与任何其他标识符冲突），可整词全替。`./TwoLineInput.vue` 与 `../../components/TwoLineInput.vue` 这两个 import 路径串里也含该 token，同一次替换即覆盖。Vue 从 import 绑定名推断组件名，故 `findComponent({ name: "TwoLineInput" })` 替换后自动对应新组件名。

- [ ] **Step 1: 用 git 改名两个文件（保留历史）**

```bash
cd /Users/boxcounter/Code/Boxcounter/logbook/.claude/worktrees/naming-audit
git mv src/components/TwoLineInput.vue src/components/EntryComposer.vue
git mv src/__tests__/components/TwoLineInput.test.ts src/__tests__/components/EntryComposer.test.ts
```

- [ ] **Step 2: 全词替换 `TwoLineInput` → `EntryComposer`**

在这 4 个文件内，把所有整词 `TwoLineInput` 替换为 `EntryComposer`（含 import 路径串、组件标签 `<TwoLineInput>` 与 `</TwoLineInput>`、`name: "TwoLineInput"` 字符串、describe 文案、注释、文件顶部注释）：
- `src/components/EntryComposer.vue`（第 1 行注释 `<!-- src/components/TwoLineInput.vue -->`）
- `src/components/MonthView.vue`
- `src/__tests__/components/EntryComposer.test.ts`
- `src/__tests__/components/MonthView.test.ts`

```bash
# 仅这 4 个文件，整词替换；macOS sed 需 -i ''
sed -i '' 's/\bTwoLineInput\b/EntryComposer/g' \
  src/components/EntryComposer.vue \
  src/components/MonthView.vue \
  src/__tests__/components/EntryComposer.test.ts \
  src/__tests__/components/MonthView.test.ts
```

- [ ] **Step 3: 确认无残留引用**

Run: `grep -rn "TwoLineInput" src` 
Expected: 无输出（exit 1）。

- [ ] **Step 4: 类型检查 + 测试全绿**

Run:
```bash
pnpm vue-tsc --noEmit
pnpm test -- --run
```
Expected: vue-tsc 无错误；vitest 全部通过（条数与改名前一致）。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(naming): TwoLineInput → EntryComposer（按职责命名核心录入器）"
```

---

### Task 2: DimensionPopover 内部状态变量改名

**Files:**
- Modify: `src/components/DimensionPopover.vue`（仅此文件；其测试只在英文注释/描述里出现 "phase" 字样，不访问这些内部变量，**不改测试**）

三个改名（均限 `DimensionPopover.vue`）：`phase` → `stage`、`activeDimKey` → `selectedDimKey`、`activeIndex` → `highlightedIndex`。

**陷阱：**
- 不要碰注释里的 `capture-phase`（第 99-102 行）、`Dim phase` / `Val phase`（141/184 行注释）——它们含 "phase" 但不是变量。
- 不要碰 computed `activeDim`（46 行）与 `activeValues`（48 行）——它们是不同标识符；`activeDimKey` 用全词匹配不会误伤 `activeDim`。

- [ ] **Step 1: 替换 `phase` 变量（只替变量用法，不碰注释）**

逐处把变量 `phase` 改为 `stage`：
- 第 17 行：`const phase = ref<"dim" | "val">("dim");` → `const stage = ref<"dim" | "val">("dim");`
- `phase.value` 全部 6 处（31、73、87、94、108、119 行）→ `stage.value`
- 模板第 142 行：`<template v-if="phase === 'dim'">` → `<template v-if="stage === 'dim'">`

```bash
# .value 用法 + 声明，安全（注释里无 "phase.value" / "const phase ="）
sed -i '' 's/phase\.value/stage.value/g; s/const phase = ref/const stage = ref/' \
  src/components/DimensionPopover.vue
# 模板里的 phase === 'dim'
sed -i '' "s/v-if=\"phase === 'dim'\"/v-if=\"stage === 'dim'\"/" \
  src/components/DimensionPopover.vue
```

- [ ] **Step 2: 全词替换 `activeDimKey` → `selectedDimKey`、`activeIndex` → `highlightedIndex`**

```bash
sed -i '' 's/\bactiveDimKey\b/selectedDimKey/g; s/\bactiveIndex\b/highlightedIndex/g' \
  src/components/DimensionPopover.vue
```

- [ ] **Step 3: 人工核对没有误伤**

Run: `grep -n "capture-phase\|activeDim\b\|activeValues" src/components/DimensionPopover.vue`
Expected: `capture-phase` 注释仍在、`activeDim`（computed）与 `activeValues` 原样未改。

Run: `grep -n "\bphase\b\|\bactiveDimKey\b\|\bactiveIndex\b" src/components/DimensionPopover.vue | grep -v "capture-phase\|Dim phase\|Val phase"`
Expected: 无输出（变量已全部改名，剩下的只有注释里的 phase 字样）。

- [ ] **Step 4: 类型检查 + 测试全绿**

Run:
```bash
pnpm vue-tsc --noEmit
pnpm test -- --run src/__tests__/components/DimensionPopover.test.ts src/__tests__/components/composite/EntryRowEdit.test.ts
```
Expected: 无类型错误；两个相关测试文件全部通过（键盘导航/高亮断言走 `data-active`，与变量名无关）。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(naming): DimensionPopover 状态变量 phase/activeDimKey/activeIndex → stage/selectedDimKey/highlightedIndex"
```

---

### Task 3: `lastDimensions` → `lastUsedDimensions`

**Files:**
- Modify: `src/stores/useStore.ts`（声明 17 行、初值 37 行）
- Modify: `src/components/MonthView.vue`（写入 123 行、模板 `:initial-values` 374 行）
- Modify: `src/__tests__/useStore.test.ts`（13 行断言）
- Modify: `src/__tests__/components/MonthView.test.ts`（29 行 mock store）

`lastDimensions` 是 AppStore 的前端内部属性（非持久化、非 IPC）。全词替换，token 唯一。

- [ ] **Step 1: 全词替换**

```bash
sed -i '' 's/\blastDimensions\b/lastUsedDimensions/g' \
  src/stores/useStore.ts \
  src/components/MonthView.vue \
  src/__tests__/useStore.test.ts \
  src/__tests__/components/MonthView.test.ts
```

- [ ] **Step 2: 确认无残留**

Run: `grep -rn "lastDimensions" src`
Expected: 无输出（exit 1）。

- [ ] **Step 3: 类型检查 + 测试全绿**

Run:
```bash
pnpm vue-tsc --noEmit
pnpm test -- --run
```
Expected: 无类型错误；vitest 全绿。

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor(naming): store.lastDimensions → lastUsedDimensions（点明是下一条的预填来源）"
```

---

### Task 4: `Screen` 类型 → `AppPhase`

**Files:**
- Modify: `src/types.ts`（99 行类型定义）
- Modify: `src/stores/useStore.ts`（2 行 import、10 行 `screen: Screen`）

只改**类型标识符** `Screen`。**不动**：组件 `SetupScreen.vue`、store 的小写属性 `screen`、字符串字面量 `"loading"|"setup"|"error"|"ready"`（App.vue 比较的是这些值，与类型名无关，故 App.vue 与测试无需改）。

- [ ] **Step 1: 改类型定义**

`src/types.ts` 第 99 行：
```ts
export type Screen = "loading" | "setup" | "error" | "ready";
```
改为：
```ts
export type AppPhase = "loading" | "setup" | "error" | "ready";
```

- [ ] **Step 2: 改 useStore 的两处引用**

`src/stores/useStore.ts` 第 2 行 import 列表里 `Screen` → `AppPhase`：
```ts
import type { Config, DayFile, Commitment, CommitmentProgress, ConfigErrorDetail, AppPhase, Entry } from "../types";
```
第 10 行字段类型（注意 `screen` 属性名不变，只改类型）：
```ts
  screen: AppPhase;
```

- [ ] **Step 3: 确认没有误伤 SetupScreen / 残留 Screen 类型**

Run: `grep -rn "\bScreen\b" src`
Expected: 无输出（exit 1）。`SetupScreen` 仍在（它是 `SetupScreen` 整词，不匹配 `\bScreen\b`）。

- [ ] **Step 4: 类型检查 + 测试全绿**

Run:
```bash
pnpm vue-tsc --noEmit
pnpm test -- --run
```
Expected: 无类型错误；vitest 全绿。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(naming): Screen 类型 → AppPhase（生命周期态非屏幕）"
```

---

### 批 1 Gate（checkpoint）

Run（完整套件，确认批 1 整体绿）：
```bash
pnpm vue-tsc --noEmit && pnpm test -- --run
```
Expected: 全绿。停下确认后再进批 2。

---

## 批 2 — 跨层 IPC 标识符（前后端同步）

### Task 5: `NewEntry`/`UpdateEntry` → `CreateEntryInput`/`UpdateEntryInput`

**Files:**
- Modify: `src/types.ts`（62、68 行——仅声明，前端无 import 引用）
- Modify: `src-tauri/src/models.rs`（77、85 行 struct 定义）
- Modify: `src-tauri/src/commands.rs`（336、381 行参数类型）
- Modify: `src-tauri/src/files.rs`（89 注释、93、112、373 行类型引用）
- Modify: `src-tauri/src/operation_log.rs`（226、670 行类型引用）

只改 Rust struct / TS interface 的**标识符**；serde 字段名（`item`/`duration`/`dimensions`）与命令签名不变 → IPC payload 字节形状不变 → 零落盘/契约迁移。

**陷阱：**
- MonthView.vue 的本地函数 `handleUpdateEntry`（139/153/364 行）含子串 `UpdateEntry`，**不得改名**。全词 `\bUpdateEntry\b` 不会匹配它（前置字符 `e` 非边界），但替换时务必用全词。
- files.rs 的参数名 `new_entry`（snake_case）与函数 `append_new_entry` **保留**；只改类型 `crate::models::NewEntry`。`\bNewEntry\b` 不匹配 `new_entry`。

- [ ] **Step 1: 前端 types.ts 改两个 interface 名**

`src/types.ts`：
```ts
export interface CreateEntryInput {
  item: string;
  duration: string;
  dimensions: Record<string, string>;
}

export interface UpdateEntryInput {
  item?: string;
  duration?: string;
  dimensions?: Record<string, string>;
}
```

- [ ] **Step 2: 后端全词替换（4 个 .rs 文件）**

```bash
sed -i '' 's/\bNewEntry\b/CreateEntryInput/g; s/\bUpdateEntry\b/UpdateEntryInput/g' \
  src-tauri/src/models.rs \
  src-tauri/src/commands.rs \
  src-tauri/src/files.rs \
  src-tauri/src/operation_log.rs
```

- [ ] **Step 3: 确认前端 handleUpdateEntry 未被误伤、无残留旧类型名**

Run: `grep -rn "\bNewEntry\b\|\bUpdateEntry\b" src src-tauri/src`
Expected: 无输出（exit 1）。

Run: `grep -n "handleUpdateEntry" src/components/MonthView.vue`
Expected: 3 处仍为 `handleUpdateEntry`（未变）。

- [ ] **Step 4: 前端类型检查 + 后端编译 + 两端测试全绿**

Run:
```bash
pnpm vue-tsc --noEmit
( cd src-tauri && cargo check && cargo test )
pnpm test -- --run
```
Expected: vue-tsc 无错误；cargo check 通过；cargo test 全绿（含 `models.rs` 的 `init_result_*_json_roundtrip` 等 serde 往返测试，证明 payload 形状未变）；vitest 全绿。

- [ ] **Step 5: 手动冒烟 IPC（确认前后端 invoke 仍通）**

Run: `pnpm tauri dev`（在仓库根目录）
手动验证：新增一条 entry（走 `append_entry` / `CreateEntryInput`）、编辑一条 entry（走 `update_entry` / `UpdateEntryInput`），均成功、无控制台报错。关闭窗口。

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(naming): NewEntry/UpdateEntry → CreateEntryInput/UpdateEntryInput（DTO 用 *Input 后缀，零契约变更）"
```

---

### 批 2 Gate（checkpoint）

确认前后端整体绿 + 冒烟通过后停下确认，再进批 3。

---

## 批 3 — 文档与防复发

### Task 6: 命名约定文档 + #6 结论

**Files:**
- Create: `docs/naming-conventions.md`
- Modify: `src/stores/useStore.ts`（给 `AvailableMonth` 补一行语义注释，记录 #6 复查结论）

- [ ] **Step 1: 写 `docs/naming-conventions.md`**

完整内容如下：

```markdown
# 命名约定

治理本项目（Tauri + Vue + TS）的命名。新增/修改代码、code review 时按此核对。

## 1. 组件按职责命名，不按外观/结构

名字要回答「它负责什么」，不是「它长什么样」。

- ✗ `TwoLineInput`（描述"两行"布局）
- ✓ `EntryComposer`（描述"组合一条 Entry"）

UI 后缀（`Popover` / `Modal` / `Panel` / `Card` / `Row`）允许且鼓励——它表达组件在界面里的角色，不算"按外观命名"。如 `DimensionPopover`、`CommitmentsModal`、`RoleCard` 均合规。

## 2. 目录分层：base / composite

- `src/components/base/`：无领域逻辑的 UI 原语（`AppButton`、`ProgressBar`、`Toast`）。
- `src/components/composite/`：含领域逻辑的组合组件。
- 根目录 `src/components/`：当前仍有历史遗留的领域组件（`MonthView`、`EntryList`、`DimensionPopover` 等）。新增组件按上述二分归位；不为单个组件做孤立迁移（会制造新的不一致）。

## 3. 输入 DTO 用 `*Input` 后缀

跨前后端传递的输入参数类型，用 `*Input` 后缀与持久化实体区分：

- `CreateEntryInput` / `UpdateEntryInput`（前端→后端的命令参数）vs `Entry`（持久化实体）。
- 这同时点明跨层差异，例如 `CreateEntryInput.duration` 是前端预解析的字符串，而 `Entry.duration` 是分钟整数。

## 4. 前后端同一概念用同一词

同一领域概念在 Rust 与 TS 两侧用同一个词。

**serde 字段名即 IPC 契约**：改 `models.rs` 里 struct 的字段名，等于改前后端通信的 JSON 键，必须两端同步，按破坏性变更对待。只改 struct/interface 的**标识符**（类型名）则不影响 payload。

## 5. 落盘格式与代码标识符解耦

磁盘契约独立于代码标识符，互不绑定：

- `operation_log.rs` 落盘的 op 字符串（`"append"`/`"update"`/`"delete"`/`"set_day_note"`）是 `match` 里的硬编码字面量，与 `Operation` 枚举的 Rust 名无关——可自由重命名枚举，但**勿动字面量**。
- frontmatter / `_monthly.md` / `config.yaml` 的字段名是磁盘契约，同理。
- 要改落盘字面量/字段名，必须走显式迁移（读旧、写新），不能裸改。

## 常见误判

- `AvailableMonth`（`{ year, month }`）命名正确：每个元素代表**一个**月份，`availableMonths: AvailableMonth[]` 是其数组。单数类型名 + 复数数组变量名是正确组合，不要"理顺"成复数类型名。
```

- [ ] **Step 2: 给 AvailableMonth 补注释，记录 #6 结论**

`src/stores/useStore.ts` 第 4-7 行的 `AvailableMonth` 接口上方加一行注释：
```ts
// 单个有记录的月份；数组见 AppStore.availableMonths。命名审查 2026-06-21 确认无需改名。
export interface AvailableMonth {
  year: number;
  month: number;
}
```

- [ ] **Step 3: 类型检查（确认注释未破坏文件）**

Run: `pnpm vue-tsc --noEmit`
Expected: 无错误。

- [ ] **Step 4: 一致性检查**

Run: 调用 `/check-consistency` skill，交叉比对 `CLAUDE.md` / `SPEC.md` / 新文档与代码现状。若 `CLAUDE.md` 的「前端交互/项目级规则」宜指向新约定文档，按检查结果补一处链接。
Expected: 无未解决的不一致。

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "docs(naming): 新增命名约定文档；记录 AvailableMonth 复查结论（#6 跳过）"
```

---

### 批 3 Gate（收尾）

Run（全套最终回归）：
```bash
pnpm vue-tsc --noEmit && pnpm test -- --run && ( cd src-tauri && cargo check && cargo test )
```
Expected: 全绿。改名审查完成。

---

## 自检对照（Self-Review）

- **Spec 覆盖：** #1（Task 1）、#2（Task 2）、#3（Task 3）、#5（Task 4）、#4（Task 5）、约定文档 + #6 结论（Task 6）——spec 纳入项全部有对应 task。#7/#8 为 spec 明确剔除项，无需 task。
- **占位符：** 无 TBD/TODO；每个改动步骤给出确切文件、行、命令或完整代码块。
- **类型一致性：** 新名在跨 task 间一致——`EntryComposer`、`stage`/`selectedDimKey`/`highlightedIndex`、`lastUsedDimensions`、`AppPhase`、`CreateEntryInput`/`UpdateEntryInput` 全程同名。
- **风险护栏：** 子串误伤（`handleUpdateEntry`/`activeDim`/`capture-phase`/`SetupScreen`/`new_entry`）已在对应 task 用全词匹配 + grep 复核拦截。
```
