# HANDOFF — 拆分 MonthView

## 背景

MonthView.vue 当前 524 行，职责过重：月数据加载、entry CRUD、键盘导航、day note 编辑、文件路径显示/复制、CommitmentsPanel 集成、维度编辑器、高亮管理。

审查 finding: `practices-review` 标记为 LOW——可以不改，但拆分后每个 composable 可独立测试。


## 目标

将 MonthView.vue 拆分为 4 个 composable，MonthView 自身降为 ~150 行编排层。**不改行为，只挪代码。**


## 拆分清单

### 1. `useMonthData.ts`

提取数据加载逻辑。输入 `store`，输出加载函数。

**移入**：
- `loadMonth(year, month, defaultDay?)` (line 56-90)
- `loadCommitmentProgress(year, month)` (line 92-107)
- `loadCommitments(year, month)` (line 109-113)
- `loadMonthDimensions(year, month)` (line 117-133)
- `onCommitmentsSaved(commitments)` (line 139-142)
- `loadDayNote(dateStr)` (line 144-149)
- `handleSelectDay(dateStr)` (line 151-158)
- `handleNavigate({ year, month })` (line 160-163)
- `handleRequestMonths()` (line 165-170)

**依赖**：`store`, `invoke`, `logError`, Dimension 类型。

**产出**：`useMonthData.ts`，导出 `useMonthData(store)`。

### 2. `useDayNote.ts`

提取 inline note 编辑逻辑。

**移入**：
- `noteRef` ref (line 268)
- `noteSnapshot` 局部变量 (line 297)
- `onNotePaste` (line 273-283)
- `onNoteInput` (line 285-289)
- `saveNote` (line 291-295)
- `onNoteFocus` (line 298-300)
- `onNoteEsc` (line 301-305)
- `onNoteEnter` (line 306-309)
- `watch(() => store.today?.note, ...)` (line 269-271)

**依赖**：`store`, `invoke`, `logError`。

**产出**：`useDayNote.ts`，导出 `useDayNote(store)`，返回 `{ noteRef, saveNote, onNotePaste, onNoteInput, onNoteFocus, onNoteEsc, onNoteEnter }`。

### 3. `useFileActions.ts`

提取文件路径显示和右键复制。

**移入**：
- `dayFilePath` computed (line 313-317)
- `displayPath` computed (line 318)
- `revealDayFile` (line 319-323)
- `copyFilePath` (line 328-335)
- `copiedFeedback` ref (line 326)
- `copyTimer` (line 327)

**依赖**：`store`, `invoke`, `logError`, `HIGHLIGHT_DURATION`。

**注意**：copyTimer 需在 `onUnmounted` 中清理，composable 内部用 `onUnmounted` 处理（不复用外部）。HIGHLIGHT_DURATION 已在 `src/utils/constants.ts` 中，直接 import。

**产出**：`useFileActions.ts`，导出 `useFileActions(store)`，返回 `{ dayFilePath, displayPath, revealDayFile, copyFilePath, copiedFeedback }`。

### 4. `useEntryActions.ts`

提取 entry CRUD。

**移入**：
- `sanitizeValues` (line 173-178)
- `handleSubmit` (line 180-201)
- `handleUpdateEntry` (line 204-219)
- `handleUpdateDimensions` (line 222-230)
- `handleDeleteEntry` (line 233-265)
- `pendingDeleteTimer` (line 232)
- `justAddedId` ref (line 36)
- `highlightTimer` (line 37)

**依赖**：`store`, `invoke`, `logError`, `triggerUndoToast`, `triggerSavedToast`, `inputRef`, `UNDO_DELETE_DELAY`, `HIGHLIGHT_DURATION`。

**注意**：
- `inputRef` 需作为参数传入（composer 的 clearInput 方法）
- `triggerUndoToast` / `triggerSavedToast` 在 composable 内部用 `inject(key)` 获取
- pendingDeleteTimer / highlightTimer 需在 `onUnmounted` 中清理，composable 内部处理
- `handleSubmit` 中的 `justAddedId` 和 highlight 逻辑一并移入

**产出**：`useEntryActions.ts`，导出 `useEntryActions(store, inputRef)`，返回 `{ handleSubmit, handleUpdateEntry, handleUpdateDimensions, handleDeleteEntry, justAddedId }`。


## MonthView 保留内容

保留在 MonthView.vue 中的是纯粹的编排和呈现逻辑：

- 组件导入和 template
- `store` 创建、`inputRef`
- 维度编辑器 (`showDimEditor`, `openDimEditor`, `onDimensionsSaved`)
- 键盘快捷键 (`onGlobalKeydown`, `shiftDay`, `shiftMonth`, `goToToday`)  ← 导航紧密耦合模板
- `isSelectedToday`, `dayEntries`, `dayTotalMinutes`, `dayTitle` computed
- `guardUnsaved`
- `onMounted` / `onUnmounted` 生命周期
- `onBeforeUnload`
- `triggerUndoToast` / `triggerSavedToast` inject ← 仅用于给 composable，自身不再持有

Template 不变：composable 返回的函数/ref 直接绑定到已有 slot。


## 执行步骤

按 `executing-plans` skill 流程：

1. **Phase 1 — useDayNote**（最小、最独立，验证拆分模式）
2. **Phase 2 — useFileActions**（同样独立）
3. **Phase 3 — useMonthData**（数据加载，最大改动）
4. **Phase 4 — useEntryActions**（CRUD，最复杂）
5. **Phase 5 — 清理 MonthView**（移除已迁移代码，整理 import）

每 phase 结束运行 `pnpm test` + `pnpm vue-tsc --noEmit` 确认无回归。

Phase checkpoint：每完成一个 composable 停下确认，不进下一个。


## 验收

- MonthView.vue ≤ 200 行
- 4 个 composable 在 `src/composables/` 下
- `pnpm test` 29 文件 425 测试全绿
- `pnpm vue-tsc --noEmit` 无报错
- 行为不变：键盘导航、day note 编辑、entry CRUD、文件路径复制均与拆分前一致


## 风险

- `handleDeleteEntry` 和 `handleSubmit` 内部有 setTimeout + store 引用闭包，迁移时确保闭包捕获正确
- `inputRef` 传递：`useEntryActions` 需要 `Ref<InstanceType<typeof EntryComposer> | null>`，调用 `clearInput()` 前判空
- `onUnmounted` 的 timer 清理：每个 composable 独立清理自己的 timer，MonthView 不再集中清理

## 不动的部分

- 键盘导航逻辑保留在 MonthView — 与 template keydown 事件绑定紧密，拆出增加间接层无收益
- `guardUnsaved` 保留在 MonthView — 调用 `inputRef.value?.hasUnsavedContent()` 且被键盘/导航多处使用
- `shiftDay`/`shiftMonth`/`goToToday` — 虽可拆为 `useNavigation`，但它们调用 `loadMonth`（来自 useMonthData），跨 composable 调用会增加耦合
