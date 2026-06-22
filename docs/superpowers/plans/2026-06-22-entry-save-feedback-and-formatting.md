# Entry 保存反馈 & 时间格式 & 配额输入宽度 — 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现三项 UI 优化：entry 更新后 toast 反馈、commitments 区域紧凑时间格式、RoleCard 配额输入加宽。

**Architecture:** 三个独立改动，无共享依赖。Task 1 新增 `formatDurationCompact` 工具函数 + 测试；Task 2 替换 4 个组件中的 `formatDuration` → `formatDurationCompact` + 更新测试断言；Task 3 在 App.vue/MonthView.vue 新增 "Saved" toast；Task 4 一处 CSS width 调整。

**Tech Stack:** TypeScript, Vue 3 SFC, Vitest, Tailwind CSS

---

### Task 1: 新增 `formatDurationCompact` 工具函数

**Files:**
- Modify: `src/utils/format.ts`（在 `formatDuration` 下方新增函数）
- Modify: `src/__tests__/format.test.ts`（新增 test suite）

- [ ] **Step 1: 写测试**

在 `src/__tests__/format.test.ts` 的 import 中加入 `formatDurationCompact`，并在文件末尾新增 test suite：

```typescript
import { formatDuration, formatDurationCompact, parseDurationFromText, stripDurations, resolveDelta } from "../utils/format";

// ... 现有测试不变 ...

describe("formatDurationCompact", () => {
  it("zero", () => { expect(formatDurationCompact(0)).toBe("0"); });
  it("30m → 0.5h", () => { expect(formatDurationCompact(30)).toBe("0.5h"); });
  it("45m → 0.8h (rounded)", () => { expect(formatDurationCompact(45)).toBe("0.8h"); });
  it("60m → 1h", () => { expect(formatDurationCompact(60)).toBe("1h"); });
  it("90m → 1.5h", () => { expect(formatDurationCompact(90)).toBe("1.5h"); });
  it("120m → 2h", () => { expect(formatDurationCompact(120)).toBe("2h"); });
  it("150m → 2.5h", () => { expect(formatDurationCompact(150)).toBe("2.5h"); });
  it("5m → 0.1h", () => { expect(formatDurationCompact(5)).toBe("0.1h"); });
  it("865m → 14.4h", () => { expect(formatDurationCompact(865)).toBe("14.4h"); });
  it("870m → 14.5h", () => { expect(formatDurationCompact(870)).toBe("14.5h"); });
});
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
pnpm vitest run src/__tests__/format.test.ts
```

预期：`formatDurationCompact is not exported` 或类似错误。

- [ ] **Step 3: 实现 `formatDurationCompact`**

在 `src/utils/format.ts` 的 `formatDuration` 函数下方添加：

```typescript
/** Format minutes to compact hour display: 90 → "1.5h", 30 → "0.5h", 0 → "0" */
export function formatDurationCompact(minutes: number): string {
  if (minutes === 0) return "0";
  const hours = Math.round(minutes / 6) / 10;
  const display = hours % 1 === 0 ? hours.toFixed(0) : String(hours);
  return `${display}h`;
}
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
pnpm vitest run src/__tests__/format.test.ts
```

预期：全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/utils/format.ts src/__tests__/format.test.ts
git commit -m "feat: add formatDurationCompact for compact hour display"
```

---

### Task 2: 替换 commitments 组件中的 duration 格式

**Files:**
- Modify: `src/components/CommitmentsPanel.vue`（L57, L74）
- Modify: `src/components/composite/CommitmentsModal.vue`（L157）
- Modify: `src/components/composite/RoleCard.vue`（L77, L164）
- Modify: `src/components/composite/GoalRow.vue`（L29, L32）
- Modify: `src/__tests__/components/composite/CommitmentsModal.test.ts`（L162, L168, L170-171, L182）

- [ ] **Step 1: 替换 `CommitmentsPanel.vue`**

L5 import 改为只 import `formatDurationCompact`（原 `formatDuration` 不再使用）：

```typescript
import { formatDurationCompact } from "../utils/format";
```

L57 role spent —— `formatDuration` → `formatDurationCompact`：

```html
<span class="mono">{{ formatDurationCompact(s.spent_minutes) }}</span><span class="mono font-normal text-[var(--color-text-secondary)]"> / {{ (s.allocation_minutes / 60).toFixed(0) }}h</span>
```

L74 goal spent —— `formatDuration` → `formatDurationCompact`：

```html
<span v-if="g.spent_minutes > 0" class="mono font-medium text-[var(--color-text-primary)]">{{ formatDurationCompact(g.spent_minutes) }}</span>
```

- [ ] **Step 2: 替换 `CommitmentsModal.vue`**

L7 import 加入 `formatDurationCompact`：

```typescript
import { formatDuration, formatDurationCompact } from "../../utils/format";
```

L157 logged total —— `formatDuration` → `formatDurationCompact`（注意此处的 `formatDuration` 只剩这一个引用后，`formatDuration` import 可移除，但保留亦无妨——commitments 组件已不再使用它。简洁起见，移除该行对 `formatDuration` 的 import）：

```typescript
// 第7行 import 改为只 import formatDurationCompact：
import { formatDurationCompact } from "../../utils/format";
```

L157：

```html
<div>Logged <span data-test="logged" class="mono font-semibold text-[var(--color-text-primary)]">{{ formatDurationCompact(loggedTotal) }}</span></div>
```

- [ ] **Step 3: 替换 `RoleCard.vue`**

L5 import 改为只 import `formatDurationCompact`（原 `formatDuration` 不再使用）：

```typescript
import { formatDurationCompact } from "../../utils/format";
```

L77 overBy：

```typescript
const overBy = computed(() => formatDurationCompact(roleSpent.value - allocMinutes.value));
```

L164 roleSpent：

```html
<span class="mono" :class="isOver ? '' : 'text-[var(--color-text-primary)] font-semibold'">{{ formatDurationCompact(roleSpent) }}</span>
```

- [ ] **Step 4: 替换 `GoalRow.vue`**

L2 import 改为只 import `formatDurationCompact`：

```typescript
import { formatDurationCompact } from "../../utils/format";
```

L29 logged 展示：

```html
>{{ logged > 0 ? formatDurationCompact(logged) : "0" }}</span>
```

L32 title 属性：

```html
:title="logged > 0 ? `${formatDurationCompact(logged)} logged — rename instead` : 'Remove goal'"
```

- [ ] **Step 5: 更新测试断言**

`CommitmentsModal.test.ts` —— 时间字符串由 `"Xh Ym"` 改为紧凑格式：

L162：
```typescript
expect(w.find("[data-test='logged']").text()).toContain("14.5h");
```

L168：
```typescript
expect(w.find("[data-test='role-spent']").text()).toContain("14.5h");
```

L170-171：
```typescript
expect(logged.some(t => t.includes("14.4h"))).toBe(true);
expect(logged.some(t => t.includes("0.1h"))).toBe(true);
```

L182：
```typescript
expect(logged.some(t => t.includes("14.4h"))).toBe(true);
```

- [ ] **Step 6: 运行相关测试，确认通过**

```bash
pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts src/__tests__/components/composite/RoleCard.test.ts src/__tests__/components/CommitmentsPanel.test.ts src/__tests__/format.test.ts
```

预期：全部 PASS。

- [ ] **Step 7: Commit**

```bash
git add src/components/CommitmentsPanel.vue src/components/composite/CommitmentsModal.vue src/components/composite/RoleCard.vue src/components/composite/GoalRow.vue src/__tests__/components/composite/CommitmentsModal.test.ts
git commit -m "feat: use compact hour format in commitments components"
```

---

### Task 3: Entry 保存后 toast 反馈

**Files:**
- Modify: `src/App.vue`（新增 `triggerSavedToast` + provide）
- Modify: `src/components/MonthView.vue`（inject + 调用）

- [ ] **Step 1: 在 `App.vue` 新增 saved toast 状态和 provide**

在 `<script setup>` 中，`triggerUndoToast` 函数附近新增：

```typescript
// Saved toast (brief confirmation for entry update, no undo)
const showSavedToast = ref(false);
const savedToastMessage = ref("");
let savedToastTimer: ReturnType<typeof setTimeout> | null = null;

function triggerSavedToast(message: string) {
  if (savedToastTimer) clearTimeout(savedToastTimer);
  savedToastMessage.value = message;
  showSavedToast.value = true;
  savedToastTimer = setTimeout(() => {
    showSavedToast.value = false;
  }, 2000);
}

function dismissSavedToast() {
  if (savedToastTimer) clearTimeout(savedToastTimer);
  showSavedToast.value = false;
}
```

在 `provide("triggerUndoToast", triggerUndoToast);` 下方新增：

```typescript
provide("triggerSavedToast", triggerSavedToast);
```

在 `<template>` 中，Undo Toast 下方新增 Saved Toast：

```html
<!-- Saved Toast -->
<Toast
  :show="showSavedToast"
  :message="savedToastMessage"
  @dismiss="dismissSavedToast"
/>
```

在 `onUnmounted` 中，`if (undoTimer) clearTimeout(undoTimer);` 下方新增清理：

```typescript
if (savedToastTimer) clearTimeout(savedToastTimer);
```

- [ ] **Step 2: 在 `MonthView.vue` 中 inject 并调用**

在 `<script setup>` 中，`triggerUndoToast` inject 附近新增：

```typescript
const triggerSavedToast = inject<(msg: string) => void>("triggerSavedToast", () => {});
```

在 `handleUpdateEntry` 的 `try` 块末尾（L173 之前），`try` 块的最后一行加入：

```typescript
triggerSavedToast("Saved");
```

完整 `try` 块：

```typescript
try {
  const df = (await invoke("update_entry", { rootPath: store.rootPath, date: store.currentDate, entryId, update })) as DayFile;
  store.today = df;
  store.monthEntries[store.currentDate] = df.entries;
  await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
  triggerSavedToast("Saved");
} catch (e) { logError("MonthView.handleUpdateEntry", e); }
```

同理，`handleUpdateDimensions` 的 `try` 块末尾也加上：

```typescript
try {
  const df = (await invoke("update_entry", { rootPath: store.rootPath, date: store.currentDate, entryId, update: { dimensions } })) as DayFile;
  store.today = df;
  store.monthEntries[store.currentDate] = df.entries;
  await loadCommitmentProgress(selectedYear.value, selectedMonth.value);
  triggerSavedToast("Saved");
} catch (e) { logError("MonthView.handleUpdateDimensions", e); }
```

- [ ] **Step 3: 运行完整测试套件**

```bash
pnpm vitest run
```

预期：全部 PASS。

- [ ] **Step 4: Commit**

```bash
git add src/App.vue src/components/MonthView.vue
git commit -m "feat: show 'Saved' toast after entry update"
```

---

### Task 4: 配额输入框加宽

**Files:**
- Modify: `src/components/composite/RoleCard.vue`（L118）

- [ ] **Step 1: 修改 width**

`w-[42px]` → `w-[52px]`：

```html
<input
  ref="allocInput"
  :value="role.allocation" type="number" data-test="alloc"
  class="w-[52px] text-center px-xs py-xs border border-[var(--color-border-form)] rounded-[var(--radius-form)]
         text-body font-semibold text-[var(--color-text-primary)] mono
         bg-[var(--color-surface)] outline-none focus:border-[var(--color-brand-solid)]"
  @input="onAllocInput($event)"
  @keydown.up.prevent="stepAlloc(STEP)"
  @keydown.down.prevent="stepAlloc(-STEP)"
/>
```

- [ ] **Step 2: 运行测试**

```bash
pnpm vitest run src/__tests__/components/composite/CommitmentsModal.test.ts
```

预期：全部 PASS。

- [ ] **Step 3: Commit**

```bash
git add src/components/composite/RoleCard.vue
git commit -m "fix: widen allocation input to fit 3-digit values"
```

---

### Task 5: 最终验证

- [ ] **Step 1: 运行完整测试套件**

```bash
pnpm vitest run
```

- [ ] **Step 2: 运行 typecheck + build**

```bash
pnpm run build
```

预期：无类型错误，build 成功。
