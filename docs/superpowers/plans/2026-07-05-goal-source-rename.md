# commitments:goals → commitments:role:goals 重命名 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将所有 `commitments:goals` 字符串字面量替换为 `commitments:role:goals`，纯重命名，无行为变更。

**Architecture:** 跨 Rust 后端（models/config/commands + 12 个集成测试 + 1 fixture）+ 前端 Vue/TS（types/DimensionPopover/DimensionEditorModal + 6 个测试文件）的全局 search-and-replace。

**Tech Stack:** Rust, Vue 3 + TypeScript, YAML

## Global Constraints

- 纯字符串替换，不改任何逻辑行为
- `commitments:role` 不动
- `MultipleGoalSource` error kind 不动
- 不向后兼容旧值

---

### Task 1: Rust 后端源文件

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/config.rs:114,120,142`
- Modify: `src-tauri/src/commands.rs:785,797,803,1755,1974,2032,2065,2120,2151,2172`
- Modify: `src-tauri/AGENTS.md:50`

**Interfaces:**
- Produces: 所有 Rust 侧 `"commitments:goals"` 字面量已替换

**Pattern A — string literals in match/if conditions:**

```rust
// BEFORE
"commitments:goals" => {
// AFTER
"commitments:role:goals" => {
```

**Pattern B — string literals in error messages:**

```rust
// BEFORE
"Dimension '{}': only one dimension may have source: commitments:goals",
// AFTER
"Dimension '{}': only one dimension may have source: commitments:role:goals",
```

```rust
// BEFORE
"Dimension '{}': invalid source '{}' (expected 'static', 'commitments:goals', or 'commitments:role')",
// AFTER
"Dimension '{}': invalid source '{}' (expected 'static', 'commitments:role:goals', or 'commitments:role')",
```

**Pattern C — inline YAML in Rust strings:**

```rust
// BEFORE
"    source: commitments:goals",
// AFTER
"    source: commitments:role:goals",
```

**Pattern D — doc comment:**

```rust
// BEFORE — models.rs comment or commands.rs doc comment
/// dimension with source=="commitments:goals".
// AFTER
/// dimension with source=="commitments:role:goals".
```

**Pattern E — Markdown doc in AGENTS.md:**

```markdown
<!-- BEFORE -->
- Goal 维度 `source: "commitments:goals"`
<!-- AFTER -->
- Goal 维度 `source: "commitments:role:goals"`
```

- [ ] **Step 1: 替换 config.rs 中 3 处**

`src-tauri/src/config.rs`:

1. Line 114: `"commitments:goals" =>` → `"commitments:role:goals" =>`
2. Line 120: `"Dimension '{}': only one dimension may have source: commitments:goals"` → `"Dimension '{}': only one dimension may have source: commitments:role:goals"`
3. Line 142: invalid source error message 中 `'commitments:goals'` → `'commitments:role:goals'`

- [ ] **Step 2: 替换 commands.rs 中所有 10 处**

`src-tauri/src/commands.rs`:

1. Line 785: doc comment `source=="commitments:goals"` → `source=="commitments:role:goals"`
2. Line 797: `.find(|d| d.source == "commitments:goals")` → `.find(|d| d.source == "commitments:role:goals")`
3. Line 803: inline YAML `source: commitments:goals` → `source: commitments:role:goals`
4. Lines 1755, 1974, 2032, 2065, 2120, 2151, 2172: 所有内联 YAML 中的 `source: commitments:goals` → `source: commitments:role:goals`

- [ ] **Step 3: 替换 models.rs 注释（如有）和 AGENTS.md**

`src-tauri/src/models.rs` — 确认 `source` 字段注释中是否含 `commitments:goals`，若有则替换。
`src-tauri/AGENTS.md` line 50: `"commitments:goals"` → `"commitments:role:goals"`

- [ ] **Step 4: 运行 Rust 检查**

```bash
cd src-tauri && cargo check
```
Expected: 无编译错误。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/config.rs src-tauri/src/commands.rs src-tauri/AGENTS.md
git commit -m "refactor: rename commitments:goals to commitments:role:goals in Rust source"
```

---

### Task 2: Rust 集成测试 + fixtures

**Files:**
- Modify: `src-tauri/tests/fixtures/dimensions.template.yaml:4`
- Modify: `src-tauri/tests/cli_integration.rs:24,300`
- Modify: `src-tauri/tests/commitment_editor_integration.rs:14`
- Modify: `src-tauri/tests/commitment_progress_integration.rs:15,131,159`
- Modify: `src-tauri/tests/cross_dimension_validation_integration.rs:27,231`
- Modify: `src-tauri/tests/data_version_integration.rs:21`
- Modify: `src-tauri/tests/dimension_editor_integration.rs:32,46,77,122,157,189,224`
- Modify: `src-tauri/tests/entry_crud_integration.rs:18,165,199,235`
- Modify: `src-tauri/tests/integrity_guard_integration.rs:22,54`
- Modify: `src-tauri/tests/monthly_dimensions_integration.rs:19,75`
- Modify: `src-tauri/tests/op_log_verify_integration.rs:21,87`
- Modify: `src-tauri/tests/recovery_category_integration.rs:8`
- Modify: `src-tauri/tests/scan_integration.rs:15`

**Interfaces:**
- Consumes: 无
- Produces: 所有测试 fixture 中 `commitments:goals` 已替换

**Replacement pattern — 3 variants:**

**Variant 1: YAML fixture 文件：**

```yaml
# BEFORE
    source: commitments:goals
# AFTER
    source: commitments:role:goals
```

**Variant 2: Rust inline YAML string 中：**

```rust
// BEFORE
"dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:goals\n"
// AFTER
"dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n"
```

**Variant 3: dimension_editor_integration.rs 中单引号形式：**

```rust
// BEFORE
make_dimension("Goal", "goal", "commitments:goals", None, false),
// AFTER
make_dimension("Goal", "goal", "commitments:role:goals", None, false),
```

- [ ] **Step 1: 替换 fixtures/dimensions.template.yaml**

文件 `src-tauri/tests/fixtures/dimensions.template.yaml` line 4: `source: commitments:goals` → `source: commitments:role:goals`

- [ ] **Step 2: 替换所有集成测试文件中的字面量**

每个文件将 `"commitments:goals"` 替换为 `"commitments:role:goals"`（Rust 字符串字面量）或 `commitments:goals` 替换为 `commitments:role:goals`（内联 YAML 内部）。

文件清单见上。建议用全局 sed 完成：

使用 sed（macOS 需 `-i ''`）全局替换：

```bash
# Rust string literals
rg -l '"commitments:goals"' src-tauri/tests/ | xargs sed -i '' 's/"commitments:goals"/"commitments:role:goals"/g'
# YAML inline (within Rust strings, unquoted)
rg -l 'source: commitments:goals' src-tauri/tests/ | xargs sed -i '' 's/source: commitments:goals/source: commitments:role:goals/g'
```

- [ ] **Step 3: 运行全部 Rust 测试**

```bash
cd src-tauri && cargo test
```
Expected: 所有测试通过。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/
git commit -m "test: rename commitments:goals to commitments:role:goals in test fixtures"
```

---

### Task 3: 前端源文件

**Files:**
- Modify: `src/types.ts:7`
- Modify: `src/components/DimensionPopover.vue:53,82,97`
- Modify: `src/components/composite/DimensionEditorModal.vue:33,133-134,316,462`

**Interfaces:**
- Produces: 前端所有 `commitments:goals` 引用已替换

**Pattern A — TypeScript union type:**

```typescript
// BEFORE
source: "static" | "commitments:goals" | "commitments:role";
// AFTER
source: "static" | "commitments:role:goals" | "commitments:role";
```

**Pattern B — Vue computed property:**

```typescript
// BEFORE
const monthly = props.dimensions.find(d => d.source === "commitments:goals");
// AFTER
const goal = props.dimensions.find(d => d.source === "commitments:role:goals");
```

**Pattern C — Vue template info card:**

```html
<!-- BEFORE -->
<template v-if="selectedDimension.source === 'commitments:goals'">
<!-- AFTER -->
<template v-if="selectedDimension.source === 'commitments:role:goals'">
```

**Pattern D — Vue validation:**

```typescript
// BEFORE
if (newDimSource.value === "commitments:goals" && draft.value.some(d => d.source === "commitments:goals" && !d.deleted)) {
  return "Only one commitments:goals source dimension allowed";
// AFTER
if (newDimSource.value === "commitments:role:goals" && draft.value.some(d => d.source === "commitments:role:goals" && !d.deleted)) {
  return "Only one commitments:role:goals source dimension allowed";
```

**Pattern E — Vue select option:**

```html
<!-- BEFORE -->
<option value="commitments:goals">Commitments: Goals</option>
<!-- AFTER -->
<option value="commitments:role:goals">Commitments: Role Goals</option>
```

**Pattern F — Ref type:**

```typescript
// BEFORE
const newDimSource = ref<"static" | "commitments:goals" | "commitments:role">("static");
// AFTER
const newDimSource = ref<"static" | "commitments:role:goals" | "commitments:role">("static");
```

**Pattern G — Comment:**

```vue
<!-- BEFORE (DimensionPopover.vue line 82) -->
// Goal dimension: values from commitments:goals source
<!-- AFTER -->
// Goal dimension: values from commitments:role:goals source
```

- [ ] **Step 1: 替换 types.ts**

`src/types.ts` line 7: `"commitments:goals"` → `"commitments:role:goals"`

- [ ] **Step 2: 替换 DimensionPopover.vue 中 3 处**

`src/components/DimensionPopover.vue`:
- Line 53: `d.source === "commitments:goals"` → `"commitments:role:goals"`
- Line 82: comment `commitments:goals source` → `commitments:role:goals source`
- Line 97: `d.source === "commitments:goals"` → `"commitments:role:goals"`

- [ ] **Step 3: 替换 DimensionEditorModal.vue 中全部 5 处 + 1 error message**

`src/components/composite/DimensionEditorModal.vue`:
- Line 33: ref type `"commitments:goals"` → `"commitments:role:goals"`
- Lines 133-134: validation 中 3 处 `"commitments:goals"` → `"commitments:role:goals"`；error message 文本同上
- Line 316: `<option value="commitments:goals">Commitments: Goals</option>` → `<option value="commitments:role:goals">Commitments: Role Goals</option>`
- Line 462: `'commitments:goals'` → `'commitments:role:goals'`

- [ ] **Step 4: 运行前端类型检查**

```bash
pnpm vue-tsc --noEmit
```
Expected: 无类型错误。

- [ ] **Step 5: Commit**

```bash
git add src/types.ts src/components/DimensionPopover.vue src/components/composite/DimensionEditorModal.vue
git commit -m "refactor: rename commitments:goals to commitments:role:goals in frontend source"
```

---

### Task 4: 前端测试文件

**Files:**
- Modify: `src/__tests__/mocks/fixtures.ts:41,50`
- Modify: `src/__tests__/components/DimensionPopover.test.ts:12,37`
- Modify: `src/__tests__/components/composite/DimensionEditorModal.test.ts:14,66,83,85,89,91,103,105,419,430,431,435,437`
- Modify: `src/__tests__/components/composite/EntryRowEdit.test.ts:13`
- Modify: `src/__tests__/components/EntryComposer.test.ts:18`
- Modify: `src/__tests__/applyInitResult.test.ts:40`
- Modify: `src/__tests__/useRootFolderPicker.test.ts:32`

**Interfaces:**
- Consumes: 无
- Produces: 前端测试全部通过

**Replacement pattern:**

所有 `.ts` 测试文件中的 `"commitments:goals"` → `"commitments:role:goals"`。

特别注意 DimensionEditorModal.test.ts 中有大量中文描述文案（"commitments:goals" 出现在测试名称和 expect 断言中），一并替换。

- [ ] **Step 1: 全局替换所有前端测试文件**

```bash
rg -l '"commitments:goals"' src/__tests__/ | xargs sed -i '' 's/"commitments:goals"/"commitments:role:goals"/g'
```

- [ ] **Step 2: 运行前端测试**

```bash
pnpm test -- src/
```
Expected: 所有前端测试通过。

- [ ] **Step 3: Commit**

```bash
git add src/__tests__/
git commit -m "test: rename commitments:goals to commitments:role:goals in frontend tests"
```

---

### Task 5: 最终验证

- [ ] **Step 1: 运行全部测试（Rust + 前端）**

```bash
pnpm test
```
Expected: 所有测试通过。

- [ ] **Step 2: 确认无遗漏**

```bash
rg "commitments:goals" --glob '!docs/superpowers/specs/*' --glob '!.git/*'
```
Expected: 无输出（设计文档中的历史引用除外）。

- [ ] **Step 3: Commit（如无新变更则跳过）**

---

**注意：** 设计文档 (`docs/superpowers/specs/2026-07-03-*.md`, `2026-07-04-*.md`) 中的 `commitments:goals` 是历史记录，不做修改。
