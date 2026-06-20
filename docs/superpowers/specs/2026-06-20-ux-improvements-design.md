# UX 优化设计（导航 / note / 列表分隔 / 跨午夜刷新）

日期：2026-06-20
状态：待实现

## 背景与问题

实际使用中暴露 4 处 UX 问题：

1. **导航快捷键位置与缺失**：输入框（`TwoLineInput`）下方的提示栏除 `@ dim` / `# time` 外，还列了 `⌘[ 上个月` / `⌘] 下个月`。但月导航是整窗行为，挂在输入框提示栏里语义错位。更关键的是最常用的「上一天 / 下一天」**根本没有快捷键**——目前只能点侧边栏日历格子切换。
2. **Day note 位置错位**：note 是「某一天」的备注（`DayFile.note`），却夹在 entry 列表与新增输入框之间，紧贴输入区，读起来像输入框的一部分。
3. **Entry 列表无分隔**：行默认 `border-transparent`，只有 hover 才显边框（`EntryRow.vue:56`）；加上 item 最多 2 行截断，静止时分不清条目边界。
4. **切窗回来被拽回今天**：`App.vue:51` 的聚焦处理是「只要当前日期 ≠ 今天就重置为今天并重载」。注释意图是「跨午夜刷新」，但实现把「跨午夜」与「没在看今天」混为一谈——选了别的日期切走再切回，就被强制拽回今天。

## 目标

- 日导航成为一等操作：DayHeader 可点箭头 + 快捷键。
- 快捷键映射符合使用频率：日 > 月。
- note 视觉上归属「这一天」，与输入区脱钩。
- 列表条目边界一眼可分。
- 聚焦行为符合直觉：停在上次选中的日期，仅真正跨午夜时才按需跟随。

**不在范围**：commitments 编辑器、esc 行为（均在其他 worktree 改动）；后端 Rust 命令不变（导航/note 复用既有 `get_entries` / `set_day_note`）。

---

## ① 日期导航

### 控件：DayHeader 加日箭头

文件：`src/components/DayHeader.vue`、`src/components/MonthView.vue`

- DayHeader 标题 `Friday, June 13` 左右各加一个 `‹` / `›` 按钮，emit `prev-day` / `next-day`。
- 沿用现有设计 token（`--color-border-form`、`--radius-form-lg` 等），样式参照 mockup 方案 A：小方块按钮，`20px`，描边 + 悬停变色。
- **未来日守卫**：当选中日 = 今天时，`›`（下一天）置灰、点击 no-op。判定复用 `MonthView` 既有 `isSelectedToday`，以 prop 传入 DayHeader（如 `can-go-next`）。

### 快捷键重映射

文件：`src/components/MonthView.vue`（`onGlobalKeydown`，`:251`）

| 快捷键 | 动作 | 现状 |
| --- | --- | --- |
| `⌘[` / `⌘]` | 上一天 / 下一天 | 新（原为月） |
| `⌘⇧[` / `⌘⇧]` | 上个月 / 下个月 | 原 `⌘[` / `⌘]` |

- `onGlobalKeydown` 内按 `e.shiftKey` 分流：有 shift → `shiftMonth(±1)`；无 shift → `shiftDay(±1)`。`[` / `]` 在按住 shift 时 `e.key` 仍为 `[` / `]`（shift 不改这两个键的 `key` 值），可靠。
- 下一天 / `⌘]`：选中日 = 今天时 no-op（与控件守卫一致）。

### 日切换逻辑

文件：`src/components/MonthView.vue`、`src/utils/dates.ts`

- `dates.ts` 新增 `addDays(dateStr: string, n: number): string`（基于 `parseDate` + `formatDate`，避开时区漂移）。
- `MonthView` 新增 `shiftDay(delta)`：
  - 算出 `next = addDays(store.currentDate, delta)`。
  - 若 `delta > 0` 且 `store.currentDate` 已是今天 → 直接返回（守卫）。
  - 若 `next` 仍在当前已加载月份（`next in store.monthEntries`）→ 走 `handleSelectDay(next)`。
  - 若跨月 → `loadMonth(yearOf(next), monthOf(next), dayOf(next))`（`loadMonth` 已支持 `defaultDay`）。

### 输入框提示栏精简

文件：`src/components/TwoLineInput.vue`（`:195-201`）

- 移除 `⌘[ prev month` / `⌘] next month` 两条 `<span>`，只留 `@ dim` / `# time`。
- 导航快捷键不在输入框提示，靠 DayHeader 箭头 + 肌肉记忆。

### 侧边栏 tooltip 更新

文件：`src/components/HeatmapCalendar.vue`（`:107`、`:112`）

- `← →` 月箭头保留；`title` 由 `Previous month (⌘[)` / `Next month (⌘])` 改为 `(⌘⇧[)` / `(⌘⇧])`。

---

## ② Day note 位置

文件：`src/components/MonthView.vue`（模板）

- 把 note 的 `contenteditable` 块（`:315-327`）从「EntryList 与 TwoLineInput 之间」移到「DayHeader 与 EntryList 之间」。
- 仅模板位置调整；`noteRef`、`saveNote`、`onNoteFocus`/`onNoteEsc`、`onNotePaste`、`onNoteInput` 及 `watch(store.today?.note)` 全部不动。
- note 对所有选中日期都显示（现状即如此），位置上移后语义为「这一天的备注」，与「仅今天显示」的输入框脱钩。
- 间距微调：移到 DayHeader 下后，原 `mt-[16px]` 改为贴合 DayHeader 底边的间距（实现时按视觉调，作为列表前的轻量附注，不喧宾夺主）。

---

## ③ Entry 列表分隔

文件：`src/components/EntryList.vue`、`src/components/composite/EntryRow.vue`

- **容器**（`EntryList.vue:16`）：去掉 `gap-[2px]`，保留滚动与 padding。
- **行间发丝线**（`EntryRow.vue` 只读态根 div，`:56-57`）：
  - 移除 hover 才显的 `border border-transparent ... hover:border-[var(--color-divider)]`。
  - 改为相邻行之间常驻发丝分隔线：行根 div 加 `border-t border-[var(--color-divider)]`，首行不显（用 `index === 0` 判断或 `first:border-t-0`）。
  - hover 背景 `hover:bg-[var(--color-surface-muted)]` 保留。
  - 圆角与分隔线并存观感不佳，去掉只读态的 `rounded-[var(--radius-form-lg)]`（hover 背景做满宽，与分隔线对齐）。
- `.just-added` 高亮动画（`:87-94`）保留；其 `border-color` 关键帧改为不与发丝线冲突（动画期临时覆盖，结束回到发丝线状态即可，实现时确认视觉无跳变）。
- 空态提示（`EntryList.vue:17-19`）不变。

---

## ④ 跨午夜刷新

文件：`src/App.vue`（`onFocusChanged`，`:45-56`）

引入 `lastKnownToday`（`ref`，`onMounted` 时初始化为当天字符串）。聚焦回调改为：

```
focusRequestId++                 // 自动 focus 输入框，保留
const newToday = todayStr()
if (newToday === lastKnownToday) return          // 同一天：什么都不做
// 跨午夜：
if (store.currentDate === lastKnownToday && store.screen === 'ready') {
  store.currentDate = newToday   // 原本在看「旧的今天」→ 跟随到新今天
  initApp()                      // 刷新数据（initApp 会把 store.today 设为新今天，currentDate 已对齐）
}
// 否则（在看别的日期）：留在原处，不重载
lastKnownToday = newToday
```

行为矩阵：

| 聚焦时 | 原本在看 | 结果 |
| --- | --- | --- |
| 同一天 | 任意 | 不动（修复主诉求） |
| 跨午夜 | 旧的今天 | 跟随到新今天 + 刷新 |
| 跨午夜 | 别的日期 | 留在原处，仅更新 `lastKnownToday` |

- 「跟随」分支仍调 `initApp()`：此时 `currentDate` 已设为新今天，`initApp` 把 `store.today` 设为后端今天数据，二者对齐，无错位。
- 「留在原处」分支不调 `initApp()`，避免 `store.today` 被今天数据覆盖而与 `currentDate`（别的日期）错位。

---

## 边界情况

- **日导航跨月**：`shiftDay` 跨月走 `loadMonth(..., defaultDay)`，复用既有月加载，commitment 进度同步刷新。
- **日导航跨年**：`addDays` 基于 `Date` 运算，自动处理年/月/闰年边界。
- **下一天到今天即止**：控件置灰 + `shiftDay`/快捷键 no-op 双重守卫，不会翻到未来日。
- **shift 键判定**：`⌘⇧[` / `⌘⇧]` 的 `e.key` 在主流布局下仍为 `[` / `]`；若个别布局 shift 改变 `key`，月导航失效但不报错（可接受，实现时若发现再补 `e.code` 判定）。
- **跨午夜「留在原处」分支**：日历「Today」环（`HeatmapCalendar` 实时 `todayStr()`）在下次渲染时自然更新；不主动重载，数据新鲜度按既有 file-watcher 处理。
- **note 上移**：对非今天日期同样显示（现状行为不变），只是位置变了。

## 测试计划

Vitest + Vue Test Utils，沿用 `src/__tests__/components/` 既有风格。

**dates.ts**
- `addDays` 跨月 / 跨年 / 闰年 / 负向。

**DayHeader / MonthView 日导航**
- 点 `‹` → 选中日 -1，`›` → +1。
- 同月内切换走 `handleSelectDay`，跨月走 `loadMonth`。
- 选中日 = 今天时 `›` 置灰且点击无效。
- `⌘[` / `⌘]` → 切日；`⌘⇧[` / `⌘⇧]` → 切月。
- `⌘]` 在今天 → no-op。

**TwoLineInput**
- 提示栏只剩 `@` / `#` 两项，不含月份提示。

**EntryList / EntryRow**
- 多条目时相邻行有分隔线，首行无上边线。
- hover 背景仍生效。

**App 跨午夜**
- 同一天聚焦 → `currentDate` 不变（不 reset）。
- 模拟跨午夜 + 原本在今天 → `currentDate` 跟随到新今天。
- 模拟跨午夜 + 原本在别的日期 → `currentDate` 不变。

## 不做的事（YAGNI）

- 不改后端 Rust 命令。
- 不动 commitments 编辑器、esc 行为（其他 worktree）。
- 输入框提示栏不再补任何导航快捷键提示。
- 「留在原处」分支不做数据自动重载（不引入 entries file-watcher）。
- note 不加折叠 / 标题等额外结构，仅移位。
