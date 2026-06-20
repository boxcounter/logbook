# Esc 行为改进设计

日期：2026-06-20
状态：待实现

## 背景与问题

编辑类组件缺少统一的 esc（退出/取消）处理。`EntryRowEdit` 进入编辑模式后只能点 Cancel 按钮退出，按 esc 无反应。多个弹层与编辑器也各自为政，esc 行为不一致或缺失。

唯一已正确实现 esc 的是 `DimensionPopover`：window 级 capture-phase 监听，phase 感知（值选择阶段 → 退回维度选择；维度阶段 → 关闭），`preventDefault` + `stopPropagation`。它作为参考标杆，本次不改动。

**排除范围**：`CommitmentsEditor` / `CommitmentsPanel` 的编辑态——在另一 worktree 改动，本次绕开以避免冲突。

## 目标

给以下组件补齐 esc 处理，并确立一套作用域明确的交互模型：

- `EntryRowEdit`（行内编辑器）
- `TwoLineInput`（常驻新增栏）
- Day note（`MonthView` 内的 contenteditable 笔记）
- `QuickJumpPopover` / `HeatmapCalendar`（月份跳转弹层）

## 架构决策：作用域绑定，不加全局监听

新增的 esc 处理一律挂在**组件自身根元素的 `@keydown`** 上，靠事件冒泡捕获内部聚焦子元素（input / button / select）的按键，从而把作用域天然限定在该组件内。

**为什么不照搬 `DimensionPopover` 的 window 级监听**：新增栏常驻、行编辑可与新增栏同时存在。若都挂 window 监听，按一次 esc 会同时触发多个 handler——例如编辑某行时 esc 会顺手清空下方新增栏。元素级 `@keydown` 靠焦点天然隔离，避开这个 bug。

**不引入共享 composable**（如 `useEscape`）：各组件 esc 语义不同（退出/清空/还原/关闭），强行抽象收益低。

**`DimensionPopover` 维持现状**：它需要焦点无关行为（焦点可能落在不可聚焦的菜单项上），且作为最内层，capture-phase + `stopPropagation` 保证它优先吃掉 esc。

**精度规则**：父编辑器的 esc handler 一律先判 `if (popoverOpen) return;`——popover 开着时 esc 归 popover，关掉后才轮到父级。当 popover 开启时，其 capture-phase 监听的 `stopPropagation` 也会阻止事件冒泡到父级 DOM，二者共同保证精度。

## 各组件行为

### EntryRowEdit —— dirty 感知 + 就地确认条（C 方案）

文件：`src/components/composite/EntryRowEdit.vue`

**dirty 判定**（对比进入编辑时的快照，已有 `item` / `durText` / `dimValues` ref）：

- `item.value !== props.entry.item`
- `resolveDelta(durText.value, props.entry.duration) !== props.entry.duration`（按解析后分钟比，兼容 `+30` 等增量写法）
- `JSON.stringify(dimValues.value) !== JSON.stringify(props.entry.dimensions)`（复用 `EntryRow.onSave` 既有比较方式）

**esc 行为**（根 div `@keydown`）：

- popover 开 → 忽略（归 popover）
- 未 dirty → `emit('cancel')`，回只读
- dirty → 进入 `confirming` 态：底部操作按钮行**就地**替换为 `放弃修改？ [放弃] [继续编辑]`

**confirming 态**：

- 再按 esc / 点「放弃」→ `emit('cancel')`
- 按 Enter / 点「继续编辑」/ 在任意输入框继续打字（input 事件）→ 退出 confirming，回到编辑
- 「删除」按钮不受确认条拦截，仍直接 `emit('delete')`（删除是独立动作）

确认条完全套用现有设计 token，不引入全局 modal。

### TwoLineInput —— 有内容直接清空，不弹确认

文件：`src/components/TwoLineInput.vue`

在现有 `onKeydown`（input 聚焦时）中处理 esc；popover 开时事件被 popover 的 capture-phase 监听吃掉，天然不触发。

- 有内容（`text` 非空 **或** `dimValues` 偏离 `initialValues`）→ 清空：`text = ""`、`dimValues` 重置回 `{ ...initialValues }`、`submitAttempted = false`，保持 input 聚焦
- 空（无文本且 `dimValues` 等于 `initialValues`）→ 无操作

重置目标是 `initialValues`（继承的默认维度），不是空对象，保持与首次进入一致。

### Day note —— 静默还原

文件：`src/components/MonthView.vue`（contenteditable 笔记区）

- 进入编辑（`@focus`）时快照 `noteSnapshot = noteRef.textContent`
- esc（contenteditable 的 `@keydown`）→ `preventDefault`，还原 `textContent = noteSnapshot`，然后 `blur()`
- `blur` 触发既有 `saveNote`，此时内容等于快照 → 等效空保存，无副作用

不弹确认（自动保存的笔记弹确认偏重）。

### QuickJumpPopover / HeatmapCalendar —— 纯关闭

文件：`src/components/QuickJumpPopover.vue`、`src/components/HeatmapCalendar.vue`

- `QuickJumpPopover`：新增 `close` emit；在 popover 根 div 上 `@keydown.esc` → `emit('close')`
- `HeatmapCalendar`：`<QuickJumpPopover @close="showJump = false" />`

弹层无 dirty 概念，esc 直接关闭。

## 边界情况

- `EntryRowEdit` 的 `durText` 为空或非法 → 用 `resolveDelta` 解析后分钟比较，回退值参与比较，不会误判 dirty。
- confirming 态下「删除」直接生效，不被拦截。
- `TwoLineInput` 重置用 `initialValues` 而非空对象。
- Day note：esc 后 `blur()` 触发 `saveNote`，内容等于快照即无写入副作用，不破坏既有「blur 保存」。
- 多个 `DimensionPopover` 同时开（新增栏 + 某行同时开）属既有行为，不在本次范围。

## 测试计划

Vitest + Vue Test Utils，沿用 `src/__tests__/components/` 既有风格。

**EntryRowEdit**
- 无改动按 esc → 触发 `cancel`
- 改动后按 esc → 出现确认条，且**未** `cancel`
- 确认条下再按 esc / 点「放弃」→ `cancel`
- 确认条下点「继续编辑」→ 回编辑态（确认条消失）
- popover 开时按 esc → 不退出编辑

**TwoLineInput**
- 有文本时 esc → 清空且不 `submit`
- 空时 esc → 无操作
- popover 开时 esc → 归 popover（不清空）

**QuickJumpPopover**
- esc → emit `close`

**Day note**
- 聚焦后修改内容，再 esc → `textContent` 还原为快照

## 不做的事（YAGNI）

- 不引入共享 esc composable
- 不改动 `DimensionPopover` 与 `CommitmentsEditor`
- TwoLineInput / Day note 不做「弹确认」（仅 EntryRowEdit 需要）
