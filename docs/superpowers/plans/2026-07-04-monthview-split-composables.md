# MonthView 拆分为 Composable 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 MonthView.vue（524 行）拆分为 4 个 composable，MonthView 降为 ~150 行编排层。不改行为，只挪代码。

**Architecture:** 按职责提取 4 个 composable：`useDayNote`（note 编辑）、`useFileActions`（路径显示/复制）、`useMonthData`（数据加载）、`useEntryActions`（entry CRUD）。每个 composable 接收 `store` 和相关依赖，返回 ref/函数供 MonthView template 绑定。timer 清理由各 composable 的 `onUnmounted` 独立处理。

**Tech Stack:** Vue 3 + TypeScript + Tauri invoke

---

### Task 1: useDayNote — 提取 day note 编辑逻辑

**Files:**
- Create: `src/composables/useDayNote.ts`
- Modify: `src/components/MonthView.vue`

- [ ] **Step 1: 创建 `src/composables/useDayNote.ts`**

```typescript
import { ref, watch, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppStore } from "../stores/useStore";
import { logError } from "../utils/errorLog";

export function useDayNote(store: AppStore) {
  const noteRef = ref<HTMLDivElement>();

  watch(
    () => store.today?.note,
    (n) => {
      if (noteRef.value && noteRef.value.textContent !== (n || "")) {
        noteRef.value.textContent = n || "";
      }
    },
    { immediate: true },
  );

  function onNotePaste(e: ClipboardEvent) {
    e.preventDefault();
    const text = e.clipboardData?.getData("text/plain") || "";
    const sel = window.getSelection();
    if (sel && sel.rangeCount > 0) {
      const range = sel.getRangeAt(0);
      range.deleteContents();
      range.insertNode(document.createTextNode(text));
      range.collapse(false);
    }
  }

  function onNoteInput() {
    if (noteRef.value && noteRef.value.innerHTML !== noteRef.value.textContent) {
      noteRef.value.textContent = noteRef.value.textContent || "";
    }
  }

  async function saveNote() {
    const text = noteRef.value?.textContent || "";
    try {
      await invoke("set_day_note", { rootPath: store.rootPath, date: store.currentDate, note: text });
    } catch (e) {
      logError("useDayNote.saveNote", e);
    }
  }

  let noteSnapshot = "";

  function onNoteFocus() {
    noteSnapshot = noteRef.value?.textContent || "";
  }

  function onNoteEsc(e: KeyboardEvent) {
    e.preventDefault();
    if (noteRef.value) noteRef.value.textContent = noteSnapshot;
    noteRef.value?.blur();
  }

  function onNoteEnter(e: KeyboardEvent) {
    if (e.isComposing) return;
    e.preventDefault();
    noteRef.value?.blur();
  }

  return { noteRef, saveNote, onNotePaste, onNoteInput, onNoteFocus, onNoteEsc, onNoteEnter };
}
```

- [ ] **Step 2: 修改 `src/components/MonthView.vue` — 添加 import 和 composable 调用，移除迁移的代码**

添加 import（在现有 `import { useStore }` 之后）：

```typescript
import { useDayNote } from "../composables/useDayNote";
```

在 `const triggerSavedToast = inject(SAVED_TOAST_KEY, () => {});` 之后添加 composable 调用：

```typescript
const { noteRef, saveNote, onNotePaste, onNoteInput, onNoteFocus, onNoteEsc, onNoteEnter } = useDayNote(store);
```

删除 MonthView.vue 中原有的以下代码块（行 268-310）：

```typescript
// ---- Day note (inline) ----
const noteRef = ref<HTMLDivElement>();
watch(() => store.today?.note, (n) => {
  if (noteRef.value && noteRef.value.textContent !== (n || "")) noteRef.value.textContent = n || "";
}, { immediate: true });

function onNotePaste(e: ClipboardEvent) {
  e.preventDefault();
  const text = e.clipboardData?.getData("text/plain") || "";
  const sel = window.getSelection();
  if (sel && sel.rangeCount > 0) {
    const range = sel.getRangeAt(0);
    range.deleteContents();
    range.insertNode(document.createTextNode(text));
    range.collapse(false);
  }
}

function onNoteInput() {
  if (noteRef.value && noteRef.value.innerHTML !== noteRef.value.textContent) {
    noteRef.value.textContent = noteRef.value.textContent || "";
  }
}

async function saveNote() {
  const text = noteRef.value?.textContent || "";
  try { await invoke("set_day_note", { rootPath: store.rootPath, date: store.currentDate, note: text }); }
  catch (e) { logError("MonthView.saveNote", e); }
}

let noteSnapshot = "";
function onNoteFocus() {
  noteSnapshot = noteRef.value?.textContent || "";
}
function onNoteEsc(e: KeyboardEvent) {
  e.preventDefault();
  if (noteRef.value) noteRef.value.textContent = noteSnapshot;
  noteRef.value?.blur(); // triggers saveNote with unchanged content (no-op write)
}
function onNoteEnter(e: KeyboardEvent) {
  if (e.isComposing) return; // let an IME Enter confirm its candidate, don't commit the note
  e.preventDefault(); // the note is single-line; don't insert a newline
  noteRef.value?.blur(); // commit via the existing blur → saveNote, clearing the caret
}
```

同时清理不再需要的 import：如果 `watch` 和 `ref` 不再被 MonthView 自身使用，从 import 中移除。当前 MonthView 自身的 `ref` 使用：
- `inputRef` (line 22) — 保留 `ref`
- `showDimEditor` (line 25) — 保留 `ref`
- `justAddedId` (line 36) — 保留 `ref`
- `copiedFeedback` (line 326) — 保留 `ref`

MonthView 自身的 `watch` 使用：无（仅 note 的 watch 被移走）。

所以从 import 移除 `watch`：将 `import { inject, computed, watch, ref, onMounted, onUnmounted, nextTick } from "vue";` 改为 `import { inject, computed, ref, onMounted, onUnmounted, nextTick } from "vue";`。

- [ ] **Step 3: 运行测试验证**

```bash
pnpm test
```
Expected: 所有测试通过（30 files, ~426 tests）。

- [ ] **Step 4: 运行类型检查**

```bash
pnpm vue-tsc --noEmit
```
Expected: 无类型错误。

- [ ] **Step 5: Commit**

```bash
git add src/composables/useDayNote.ts src/components/MonthView.vue
git commit -m "refactor: extract useDayNote composable from MonthView"
```

### Task 2: useFileActions — 提取文件路径显示和右键复制

**Files:**
- Create: `src/composables/useFileActions.ts`
- Modify: `src/components/MonthView.vue`

- [ ] **Step 1: 创建 `src/composables/useFileActions.ts`**

```typescript
import { computed, ref, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppStore } from "../stores/useStore";
import { logError } from "../utils/errorLog";
import { HIGHLIGHT_DURATION } from "../utils/constants";

export function useFileActions(store: AppStore) {
  const dayFilePath = computed(() => {
    if (!store.rootPath) return "";
    const d = store.currentDate;
    return `${d.slice(0, 4)}/${d.slice(5, 7)}/${d}.md`;
  });

  const displayPath = computed(() => (store.rootPath ? `…/${dayFilePath.value}` : ""));

  async function revealDayFile() {
    if (!store.rootPath) return;
    try {
      await invoke("reveal_day_file", { rootPath: store.rootPath, date: store.currentDate });
    } catch (e) {
      logError("useFileActions.revealDayFile", e);
    }
  }

  const copiedFeedback = ref(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;

  async function copyFilePath(e: MouseEvent) {
    e.preventDefault();
    if (!store.rootPath) return;
    await navigator.clipboard.writeText(store.rootPath + "/" + dayFilePath.value);
    copiedFeedback.value = true;
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => { copiedFeedback.value = false; }, HIGHLIGHT_DURATION);
  }

  onUnmounted(() => {
    if (copyTimer) clearTimeout(copyTimer);
  });

  return { dayFilePath, displayPath, revealDayFile, copyFilePath, copiedFeedback };
}
```

- [ ] **Step 2: 修改 `src/components/MonthView.vue` — 添加 import 和 composable 调用，移除迁移的代码**

添加 import：

```typescript
import { useFileActions } from "../composables/useFileActions";
```

添加 composable 调用（在 `useDayNote` 调用之后）：

```typescript
const { dayFilePath, displayPath, revealDayFile, copyFilePath, copiedFeedback } = useFileActions(store);
```

删除 MonthView.vue 中原有的以下代码块（行 312-335）：

```typescript
// ---- File path ----
const dayFilePath = computed(() => {
  if (!store.rootPath) return "";
  const d = store.currentDate;
  return `${d.slice(0, 4)}/${d.slice(5, 7)}/${d}.md`;
});
const displayPath = computed(() => (store.rootPath ? `…/${dayFilePath.value}` : ""));
async function revealDayFile() {
  if (!store.rootPath) return;
  try { await invoke("reveal_day_file", { rootPath: store.rootPath, date: store.currentDate }); }
  catch (e) { logError("MonthView.revealDayFile", e); }
}

// ---- File path right-click copy ----
const copiedFeedback = ref(false);
let copyTimer: ReturnType<typeof setTimeout> | null = null;
async function copyFilePath(e: MouseEvent) {
  e.preventDefault();
  if (!store.rootPath) return;
  await navigator.clipboard.writeText(store.rootPath + "/" + dayFilePath.value);
  copiedFeedback.value = true;
  if (copyTimer) clearTimeout(copyTimer);
  copyTimer = setTimeout(() => { copiedFeedback.value = false; }, HIGHLIGHT_DURATION);
}
```

检查 MonthView.vue 的 `onUnmounted`：如果 `copyTimer` 的清理被移到了 composable 内，需要从 MonthView 的 `onUnmounted` 中移除 `if (copyTimer) clearTimeout(copyTimer);` 这一行。

检查 MonthView.vue 的 import：`computed` 仍然在 MonthView 自身使用（`selectedYear`, `selectedMonth`, `isSelectedToday`, `dayEntries`, `dayTotalMinutes`, `dayTitle`），不删除。`HIGHLIGHT_DURATION` 不再被 MonthView 直接使用（仅 composable 用），从 import 中移除 `HIGHLIGHT_DURATION`。

将 `import { HIGHLIGHT_DURATION, UNDO_DELETE_DELAY } from "../utils/constants";` 改为 `import { UNDO_DELETE_DELAY } from "../utils/constants";`。

- [ ] **Step 3: 运行测试验证**

```bash
pnpm test
```
Expected: 所有测试通过。

- [ ] **Step 4: 运行类型检查**

```bash
pnpm vue-tsc --noEmit
```
Expected: 无类型错误。

- [ ] **Step 5: Commit**

```bash
git add src/composables/useFileActions.ts src/components/MonthView.vue
git commit -m "refactor: extract useFileActions composable from MonthView"
```

### Task 3: useMonthData — 提取月数据加载逻辑

**Files:**
- Create: `src/composables/useMonthData.ts`
- Modify: `src/components/MonthView.vue`

- [ ] **Step 1: 创建 `src/composables/useMonthData.ts`**

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { AppStore } from "../stores/useStore";
import type { Entry, DayFile, Commitment, CommitmentProgressResult, MonthDimensions } from "../types";
import { logError } from "../utils/errorLog";
import { yearMonthFromDate } from "../utils/dates";

export function useMonthData(store: AppStore) {
  async function loadMonth(year: number, month: number, defaultDay?: number) {
    store.configErrors = [];
    store.commitments = [];
    store.commitmentProgress = [];
    store.commitmentProgressResult = null;
    const now = new Date();
    const isCurrentMonth = year === now.getFullYear() && month === now.getMonth() + 1;
    let day: number;
    if (defaultDay !== undefined) day = defaultDay;
    else if (isCurrentMonth) day = now.getDate();
    else day = new Date(year, month, 0).getDate();

    const dateStr = `${year}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
    store.currentDate = dateStr;

    try {
      store.monthEntries = await invoke<Record<string, Entry[]>>("get_month_entries", { rootPath: store.rootPath, year, month });
    } catch (e) {
      logError("useMonthData.loadMonth", e);
      store.monthEntries = {};
      const msg = String(e);
      if (msg.includes("dimensions")) {
        store.configErrors = [{ kind: "ConfigError", message: msg }];
        store.configCategory = "in_place";
        return;
      }
    }
    await loadCommitmentProgress(year, month);
    await loadCommitments(year, month);
    await loadMonthDimensions(year, month);
    if (store.currentDate in store.monthEntries) {
      store.today = { note: null, entries: store.monthEntries[store.currentDate] };
      loadDayNote(store.currentDate);
    }
  }

  async function loadCommitmentProgress(year: number, month: number) {
    try {
      const result = await invoke<CommitmentProgressResult>("get_commitment_progress", { rootPath: store.rootPath, year, month });
      store.commitmentProgress = result.roles;
      store.commitmentProgressResult = result;
    } catch (e) {
      logError("useMonthData.loadCommitmentProgress", e);
      store.commitmentProgress = [];
      store.commitmentProgressResult = null;
      const msg = String(e);
      if (msg.includes("dimensions")) {
        store.configErrors = [{ kind: "ConfigError", message: msg }];
        store.configCategory = "in_place";
      }
    }
  }

  async function loadCommitments(year: number, month: number) {
    try {
      store.commitments = await invoke<Commitment[]>("get_commitments", { rootPath: store.rootPath, year, month });
    } catch (e) {
      logError("useMonthData.loadCommitments", e);
      store.commitments = [];
    }
  }

  async function loadMonthDimensions(year: number, month: number) {
    try {
      const md = await invoke<MonthDimensions>("get_month_dimensions", { rootPath: store.rootPath, year, month });
      if (md && Array.isArray(md.dimensions)) {
        store.dimensions = md.dimensions;
        store.usingDefaultDimensions = md.usingDefaultDimensions;
      }
    } catch (e) {
      logError("useMonthData.loadMonthDimensions", e);
      const msg = String(e);
      if (msg.includes("dimensions")) {
        store.configErrors = [{ kind: "ConfigError", message: msg }];
        store.configCategory = "in_place";
      }
    }
  }

  async function onCommitmentsSaved(commitments: Commitment[]) {
    store.commitments = commitments;
    await loadCommitmentProgress(
      yearMonthFromDate(store.currentDate).year,
      yearMonthFromDate(store.currentDate).month,
    );
  }

  async function loadDayNote(dateStr: string) {
    try {
      const df = await invoke<DayFile>("get_entries", { rootPath: store.rootPath, date: dateStr });
      if (store.today) store.today.note = df.note;
    } catch (e) {
      logError("useMonthData.loadDayNote", e);
    }
  }

  async function handleSelectDay(dateStr: string) {
    store.currentDate = dateStr;
    if (dateStr in store.monthEntries) {
      store.today = { note: null, entries: store.monthEntries[dateStr] };
      await loadDayNote(dateStr);
    }
  }

  async function handleNavigate({ year, month }: { year: number; month: number }) {
    await loadMonth(year, month);
  }

  async function handleRequestMonths() {
    if (store.availableMonths !== null) return;
    try {
      store.availableMonths = (await invoke("get_available_months", { rootPath: store.rootPath })) as { year: number; month: number }[];
    } catch (e) {
      logError("useMonthData.handleRequestMonths", e);
      store.availableMonths = [];
    }
  }

  return {
    loadMonth,
    loadCommitmentProgress,
    loadCommitments,
    loadMonthDimensions,
    onCommitmentsSaved,
    loadDayNote,
    handleSelectDay,
    handleNavigate,
    handleRequestMonths,
  };
}
```

- [ ] **Step 2: 修改 `src/components/MonthView.vue` — 添加 import，用 composable 调用替换原函数**

添加 import：

```typescript
import { useMonthData } from "../composables/useMonthData";
```

添加 composable 调用并解构（在 `useDayNote` 和 `useFileActions` 调用之后）：

```typescript
const {
  loadMonth,
  loadCommitmentProgress,
  loadCommitments,
  loadMonthDimensions,
  onCommitmentsSaved,
  loadDayNote,
  handleSelectDay,
  handleNavigate,
  handleRequestMonths,
} = useMonthData(store);
```

删除 MonthView.vue 中原有的以下代码块（行 56-170，即从 `// ---- Month loading ----` 注释开始到 `}` 结束的整个 handleRequestMonths 函数）：

这些行包括：
```typescript
// ---- Month loading ----
async function loadMonth(year: number, month: number, defaultDay?: number) { ... }
async function loadCommitmentProgress(year: number, month: number) { ... }
async function loadCommitments(year: number, month: number) { ... }
async function loadMonthDimensions(year: number, month: number) { ... }
async function onCommitmentsSaved(commitments: Commitment[]) { ... }
async function loadDayNote(dateStr: string) { ... }
async function handleSelectDay(dateStr: string) { ... }
async function handleNavigate({ year, month }: { year: number; month: number }) { ... }
async function handleRequestMonths() { ... }
```

具体删除范围为：
1. 删除注释行 `// ---- Month loading ----` (line 55)
2. 删除 `async function loadMonth` 到其结束 `}` (lines 56-90)
3. 删除 `async function loadCommitmentProgress` 到其结束 `}` (lines 92-107)
4. 删除 `async function loadCommitments` 到其结束 `}` (lines 109-113)
5. 删除 `async function loadMonthDimensions` 到其结束 `}` (lines 117-133)
6. 删除 `async function onCommitmentsSaved` 到其结束 `}` (lines 139-142)
7. 删除 `async function loadDayNote` 到其结束 `}` (lines 144-149)
8. 删除 `async function handleSelectDay` 到其结束 `}` (lines 151-158)
9. 删除 `async function handleNavigate` 到其结束 `}` (lines 160-163)
10. 删除 `async function handleRequestMonths` 到其结束 `}` (lines 165-170)

同时清理 MonthView.vue 中不再需要的 import：`parseDate`, `addDays`, `formatDate` 仍然被 MonthView 自身使用（`dayTitle` computed 用 `parseDate`，`shiftDay` 用 `addDays`，`isSelectedToday` + `goToToday` 用 `formatDate`）。`yearMonthFromDate` 被 `shiftDay` 使用。所有 date utils import 保留。

`Entry`, `Commitment`, `CommitmentProgressResult`, `MonthDimensions`, `Dimension` 等类型中，`Commitment`, `CommitmentProgressResult`, `MonthDimensions` 不再被 MonthView 直接使用，但 `Dimension` 仍用于 `onDimensionsSaved`。检查后决定保留所有类型 import 避免遗漏。

`invoke` 仍然被 MonthView 自身使用吗？检查后：
- `handleSubmit` (line 180) 调用 invoke — 将被移到 useEntryActions
- `handleUpdateEntry` (line 204) 调用 invoke — 将被移到 useEntryActions
- `handleUpdateDimensions` (line 222) 调用 invoke — 将被移到 useEntryActions
- `handleDeleteEntry` (line 233) 调用 invoke — 将被移到 useEntryActions

Task 3 阶段 `invoke` 仍然被 `handleSubmit` 等使用（还未迁移），所以保留 `invoke` import。Task 4 后再移除。

- [ ] **Step 3: 运行测试验证**

```bash
pnpm test
```
Expected: 所有测试通过。

- [ ] **Step 4: 运行类型检查**

```bash
pnpm vue-tsc --noEmit
```
Expected: 无类型错误。

- [ ] **Step 5: Commit**

```bash
git add src/composables/useMonthData.ts src/components/MonthView.vue
git commit -m "refactor: extract useMonthData composable from MonthView"
```

### Task 4: useEntryActions — 提取 entry CRUD 逻辑

**Files:**
- Create: `src/composables/useEntryActions.ts`
- Modify: `src/components/MonthView.vue`

- [ ] **Step 1: 创建 `src/composables/useEntryActions.ts`**

```typescript
import { ref, onUnmounted, inject, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppStore } from "../stores/useStore";
import type { Entry, DayFile, CommitmentProgressResult } from "../types";
import { UNDO_TOAST_KEY, SAVED_TOAST_KEY } from "../types";
import { logError } from "../utils/errorLog";
import { yearMonthFromDate } from "../utils/dates";
import { HIGHLIGHT_DURATION, UNDO_DELETE_DELAY } from "../utils/constants";

interface ComposerRef {
  clearInput(): void;
}

export function useEntryActions(store: AppStore, inputRef: Ref<ComposerRef | null>) {
  const triggerUndoToast = inject(UNDO_TOAST_KEY, () => {});
  const triggerSavedToast = inject(SAVED_TOAST_KEY, () => {});

  const justAddedId = ref<string | null>(null);
  let highlightTimer: ReturnType<typeof setTimeout> | null = null;
  let pendingDeleteTimer: ReturnType<typeof setTimeout> | null = null;

  function sanitizeValues(vals: Record<string, string>): Record<string, string> {
    const validKeys = new Set(store.dimensions.map((d) => d.key));
    const cleaned: Record<string, string> = {};
    for (const [k, v] of Object.entries(vals)) if (validKeys.has(k) && v) cleaned[k] = v;
    return cleaned;
  }

  async function refreshProgress() {
    const ym = yearMonthFromDate(store.currentDate);
    try {
      const result = await invoke<CommitmentProgressResult>("get_commitment_progress", {
        rootPath: store.rootPath,
        year: ym.year,
        month: ym.month,
      });
      store.commitmentProgress = result.roles;
      store.commitmentProgressResult = result;
    } catch (e) {
      logError("useEntryActions.refreshProgress", e);
      store.commitmentProgress = [];
      store.commitmentProgressResult = null;
      const msg = String(e);
      if (msg.includes("dimensions")) {
        store.configErrors = [{ kind: "ConfigError", message: msg }];
        store.configCategory = "in_place";
      }
    }
  }

  async function handleSubmit(item: string, durationMinutes: number, dimensions: Record<string, string>) {
    const finalDimensions = sanitizeValues(dimensions);
    const newEntry = { item, duration: String(durationMinutes) + "m", dimensions: finalDimensions };
    try {
      const result = await invoke("append_entry", {
        rootPath: store.rootPath,
        date: store.currentDate,
        entry: newEntry,
      });
      const added = result as Entry;
      if (store.today) {
        const entries = [...store.today.entries, added];
        store.today = { ...store.today, entries };
        store.monthEntries[store.currentDate] = entries;
      }
      justAddedId.value = added.id;
      if (highlightTimer) clearTimeout(highlightTimer);
      highlightTimer = setTimeout(() => {
        justAddedId.value = null;
      }, HIGHLIGHT_DURATION);
      await refreshProgress();
      inputRef.value?.clearInput();
    } catch (e) {
      logError("useEntryActions.handleSubmit", e);
      triggerSavedToast("Failed to save entry");
    }
  }

  async function handleUpdateEntry(entryId: string, item: string, durationMinutes: number) {
    const entries = store.today?.entries;
    if (!entries) return;
    const entry = entries.find((e) => e.id === entryId);
    if (!entry) return;
    const update: Record<string, unknown> = {};
    if (item !== entry.item) update.item = item;
    if (durationMinutes !== entry.duration) update.duration = String(durationMinutes) + "m";
    if (Object.keys(update).length === 0) return;
    try {
      const df = await invoke<DayFile>("update_entry", {
        rootPath: store.rootPath,
        date: store.currentDate,
        entryId,
        update,
      });
      store.today = df;
      store.monthEntries[store.currentDate] = df.entries;
      await refreshProgress();
      triggerSavedToast("Saved");
    } catch (e) {
      logError("useEntryActions.handleUpdateEntry", e);
    }
  }

  async function handleUpdateDimensions(entryId: string, dimensions: Record<string, string>) {
    try {
      const df = await invoke<DayFile>("update_entry", {
        rootPath: store.rootPath,
        date: store.currentDate,
        entryId,
        update: { dimensions },
      });
      store.today = df;
      store.monthEntries[store.currentDate] = df.entries;
      await refreshProgress();
      triggerSavedToast("Saved");
    } catch (e) {
      logError("useEntryActions.handleUpdateDimensions", e);
    }
  }

  async function handleDeleteEntry(entryId: string) {
    const entries = store.today?.entries;
    if (!entries) return;
    const idx = entries.findIndex((e) => e.id === entryId);
    if (idx === -1) return;
    const date = store.currentDate;
    const { year, month } = yearMonthFromDate(date);
    const [removed] = entries.splice(idx, 1);
    let cancelled = false;
    pendingDeleteTimer = setTimeout(async () => {
      if (cancelled) return;
      try {
        await invoke("delete_entry", { rootPath: store.rootPath, date, entryId });
        store.monthEntries[date] = [...entries];
        await refreshProgress();
      } catch (e) {
        logError("useEntryActions.handleDeleteEntry", e);
        if (entries.findIndex((e) => e.id === entryId) === -1) entries.splice(idx, 0, removed);
      }
    }, UNDO_DELETE_DELAY);
    triggerUndoToast(() => {
      cancelled = true;
      if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
      if (entries.findIndex((e) => e.id === entryId) === -1) {
        entries.splice(idx, 0, removed);
        store.monthEntries[date] = [...entries];
      }
    });
  }

  onUnmounted(() => {
    if (highlightTimer) clearTimeout(highlightTimer);
    if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
  });

  return { handleSubmit, handleUpdateEntry, handleUpdateDimensions, handleDeleteEntry, justAddedId };
}
```

- [ ] **Step 2: 修改 `src/components/MonthView.vue` — 添加 import，用 composable 调用替换原函数**

添加 import：

```typescript
import { useEntryActions } from "../composables/useEntryActions";
```

添加 composable 调用并解构（在已有 composable 调用之后）：

```typescript
const { handleSubmit, handleUpdateEntry, handleUpdateDimensions, handleDeleteEntry, justAddedId } = useEntryActions(store, inputRef);
```

删除 MonthView.vue 中原有的以下代码块：

1. 删除注释 `// Newly-added entry highlight...` 及其后的 `justAddedId` 和 `highlightTimer` 声明（lines 35-37）：

```typescript
// Newly-added entry highlight (spec §5.2 step 7): mark the id, clear after 1.5s.
const justAddedId = ref<string | null>(null);
let highlightTimer: ReturnType<typeof setTimeout> | null = null;
```

2. 删除 `sanitizeValues` (lines 173-178)
3. 删除 `handleSubmit` (lines 180-201)
4. 删除注释 `// ---- Entry mutations ----` (line 203)
5. 删除 `handleUpdateEntry` (lines 204-219)
6. 删除 `handleUpdateDimensions` (lines 222-230)
7. 删除 `pendingDeleteTimer` (line 232)
8. 删除 `handleDeleteEntry` (lines 233-265)

同时清理 MonthView.vue 的 import：
- `ref` — MonthView 仍使用 `inputRef`, `showDimEditor`，保留
- `inject` — MonthView 不再直接使用 inject（toast inject 仅用于传给 composable，当前 MonthView 已不再持有），但 `triggerUndoToast` 和 `triggerSavedToast` 变量从 import 移除：
  - 删除 `const triggerUndoToast = inject(UNDO_TOAST_KEY, () => {});` (line 52)
  - 删除 `const triggerSavedToast = inject(SAVED_TOAST_KEY, () => {});` (line 53)
  - 从 import 移除 `inject`
  - 从 types import 移除 `UNDO_TOAST_KEY`, `SAVED_TOAST_KEY`
- `UNDO_DELETE_DELAY` — 不再被 MonthView 直接使用，从 constants import 移除
- `invoke` — 不再被 MonthView 直接使用，从 Tauri import 移除
- `Entry`, `Commitment`, `CommitmentProgressResult`, `MonthDimensions`, `Dimension` — 检查剩余使用：
  - `Dimension` 仍用于 `onDimensionsSaved(dims: Dimension[])`，保留
  - `DayFile` 不再直接使用（handleUpdateEntry 等已迁移），保留以防 template 使用
  - 其他类型在 template 中可能通过 props 传递，保守保留

同时删除 MonthView.vue 中 `onUnmounted` 里的 timer 清理逻辑：
```typescript
  if (pendingDeleteTimer) clearTimeout(pendingDeleteTimer);
  if (highlightTimer) clearTimeout(highlightTimer);
```
（`copyTimer` 清理已在 Task 2 移除）

- [ ] **Step 3: 运行测试验证**

```bash
pnpm test
```
Expected: 所有测试通过。

- [ ] **Step 4: 运行类型检查**

```bash
pnpm vue-tsc --noEmit
```
Expected: 无类型错误。

- [ ] **Step 5: Commit**

```bash
git add src/composables/useEntryActions.ts src/components/MonthView.vue
git commit -m "refactor: extract useEntryActions composable from MonthView"
```

### Task 5: 最终清理与验证

**Files:**
- Modify: `src/components/MonthView.vue`

- [ ] **Step 1: 清理 MonthView.vue 中不再需要的 import**

检查并清理 import：

```typescript
// 清理前：
import { inject, computed, watch, ref, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useStore } from "../stores/useStore";
import HeatmapCalendar from "./HeatmapCalendar.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import DayHeader from "./DayHeader.vue";
import EntryList from "./EntryList.vue";
import EntryComposer from "./EntryComposer.vue";
import DimensionEditorModal from "./composite/DimensionEditorModal.vue";
import type { DayFile, Entry, Commitment, CommitmentProgressResult, MonthDimensions, Dimension } from "../types";
import { UNDO_TOAST_KEY, SAVED_TOAST_KEY } from "../types";
import { logError, logInfo } from "../utils/errorLog";
import { yearMonthFromDate, parseDate, addDays, formatDate } from "../utils/dates";
import { HIGHLIGHT_DURATION, UNDO_DELETE_DELAY } from "../utils/constants";
import ConfigErrorBanner from "./ConfigErrorBanner.vue";
import { useDayNote } from "../composables/useDayNote";
import { useFileActions } from "../composables/useFileActions";
import { useMonthData } from "../composables/useMonthData";
import { useEntryActions } from "../composables/useEntryActions";

// 清理后：
import { computed, ref, onMounted, onUnmounted, nextTick } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useStore } from "../stores/useStore";
import HeatmapCalendar from "./HeatmapCalendar.vue";
import CommitmentsPanel from "./CommitmentsPanel.vue";
import DayHeader from "./DayHeader.vue";
import EntryList from "./EntryList.vue";
import EntryComposer from "./EntryComposer.vue";
import DimensionEditorModal from "./composite/DimensionEditorModal.vue";
import type { Dimension } from "../types";
import { logInfo } from "../utils/errorLog";
import { parseDate, addDays, formatDate } from "../utils/dates";
import ConfigErrorBanner from "./ConfigErrorBanner.vue";
import { useDayNote } from "../composables/useDayNote";
import { useFileActions } from "../composables/useFileActions";
import { useMonthData } from "../composables/useMonthData";
import { useEntryActions } from "../composables/useEntryActions";
```

移除说明：
- `inject`, `watch` — 不再被 MonthView 直接使用
- `invoke` — 不再被 MonthView 直接使用
- `DayFile`, `Entry`, `Commitment`, `CommitmentProgressResult`, `MonthDimensions` — 仅在 composable 内部使用
- `UNDO_TOAST_KEY`, `SAVED_TOAST_KEY` — 仅在 composable 内部使用
- `logError` — 不再被 MonthView 直接使用
- `yearMonthFromDate` — 从 `dates` import 移除
- `HIGHLIGHT_DURATION`, `UNDO_DELETE_DELAY` — 仅在 composable 内部使用

- [ ] **Step 2: 检查 MonthView.vue 行数**

```bash
wc -l src/components/MonthView.vue
```
Expected: ≤ 200 行。

- [ ] **Step 3: 运行全部测试**

```bash
pnpm test
```
Expected: 所有测试通过（30 files, ~426 tests）。

- [ ] **Step 4: 运行类型检查**

```bash
pnpm vue-tsc --noEmit
```
Expected: 无类型错误。

- [ ] **Step 5: Commit**

```bash
git add src/components/MonthView.vue
git commit -m "chore: clean up unused imports in MonthView after composable extraction"
```
