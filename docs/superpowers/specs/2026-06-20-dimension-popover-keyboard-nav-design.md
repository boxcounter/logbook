# DimensionPopover 键盘导航设计

日期：2026-06-20
状态：待实现

## 背景与问题

`@` 唤出的维度/值选择菜单（`DimensionPopover`）目前**只支持鼠标点击 + Esc**，没有任何键盘导航。footer 里写的 `↵ select` 是个假提示：popover 开着时按 Enter 实际走的是父组件逻辑——`TwoLineInput` 关闭 popover 并直接提交整条 entry（spec §5.2「Enter 永不被拦」），`EntryRowEdit` 则保存当前编辑。菜单内既无高亮项、也无回车选中。

目标：给 `DimensionPopover` 加键盘导航（上下移动高亮 + 回车选中），让全键盘录入维度成为可能。

`DimensionPopover` 在两处复用：`TwoLineInput`（常驻新增栏）与 `EntryRowEdit`（行内编辑），改动对两处同时生效。

## 关键决策

### Enter 行为：选中高亮项（方案一）

popover 开着时，Enter **永远响应菜单**——选中当前高亮项（dim 阶段进入该维度的值子菜单；val 阶段填入该值）。

代价：popover 开着时 Enter 不再提交 entry / 保存编辑。要提交先按 Esc 关 popover，再 Enter。这是所有下拉/自动补全菜单的通用行为，也让 footer 的 `↵ select` 名副其实。

**这会改变 spec §5.2 的「Enter 永不被拦」**——仅限 popover 开启期间。SPEC.md 相应处需同步（见「文档同步」）。

### 导航键：`CTRL+N/P` + `↑↓` 并存

emacs 习惯用 `CTRL+N/P`，也支持箭头 `↑↓`。两套等价。

### 实现位置：收在 `DimensionPopover` 内部

键盘导航逻辑全部放在 popover 内，扩展它**现有的 window 级 capture-phase keydown 监听**（当前只处理 Esc）。两个父组件无需改动即同时获得能力。

> 备选（否决）：在 `TwoLineInput`、`EntryRowEdit` 各自处理、把高亮 index 当 prop 传下去——逻辑重复两份，props/events 膨胀。
>
> 与 `DimensionPopover` 既有 Esc 处理（window capture-phase）一脉相承：菜单项可能不可聚焦、焦点也可能仍在父级 input 上，window 级监听是焦点无关的，是这里唯一可靠的方式。

## 设计

文件：`src/components/DimensionPopover.vue`

### 状态

- 新增 `activeIndex: ref<number>`，指向**当前阶段列表**的高亮项下标。
- 当前列表：dim 阶段 = `props.dimensions`；val 阶段 = `activeValues`（computed，已存在）。

### 默认高亮

- **dim 阶段**（popover 弹出 / 从 val 阶段返回）：落在**第一个还没填 value 的维度**（`props.dimValues[d.key]` 为空的第一个 `dimensions` 项）；若全部已填，落 `index 0`。
  - 「从 val 返回时高亮下一个未填维度」是显式需求：为维度 X 选完值后 X 已有 value，返回 dim 菜单时「第一个未填」自然跳过 X，落到下一个未填项。
- **val 阶段**（`selectDim` 进入某维度）：若该维度已有值（`dimValues[activeDimKey]` 命中 `activeValues` 中某项），高亮那个值；否则 `index 0`。

`selectDim` / `goBack` / `selectVal`(返回 dim 时) 均按上述规则重算 `activeIndex`。

### 导航键

扩展现有 `onWindowKeydown`（capture 阶段，window 级）：

| 键 | 行为 |
|---|---|
| `CTRL+N` / `↓` | 下一项，到底循环回顶 |
| `CTRL+P` / `↑` | 上一项，到顶循环回底 |
| `Enter` | 选中 `activeIndex` 项：dim 阶段调 `selectDim`、val 阶段调 `selectVal` |
| `Esc` | 维持现状：val 阶段 → 退回 dim；dim 阶段 → `emit('close')` |

所有这些键一律 `preventDefault()` + `stopPropagation()`：

- `stopPropagation` 让父组件的 Enter（`TwoLineInput` 提交 / `EntryRowEdit` 保存）在 popover 开着时收不到事件——**这就是方案一的落地机制**。
- `preventDefault` 必须：`CTRL+P` 在 webview 默认是打印对话框；`↑↓` 会移动父级 input 的光标。
- 仅当 `e.ctrlKey` 时才拦 `N`/`P`（避免吃掉正常输入，虽然 popover 开时焦点通常不在文本 input，但保险起见精确匹配）。

### 鼠标 / 键盘统一

列表项 `@mouseenter` 时把 `activeIndex` 同步到该项下标。鼠标悬停与键盘高亮共用一个状态，不互相打架。

### 视觉

高亮项加内描边 `ring-1 ring-inset ring-[var(--color-brand-solid)]`，独立于现有 hover 底色（`bg-[var(--color-divider)]`）与「已选」底色（`bg-[var(--color-popover-item-selected-bg)]`）——三种状态可叠加、可区分。具体 token 实现时可微调。

### footer 提示

两个阶段的提示栏补 `⌃N/⌃P move`（与现有 `↵ select`、`esc …` 并列）。`↵ select` 此时名副其实，保留。

## 影响面

- `TwoLineInput`、`EntryRowEdit` 复用同一 popover，自动获得导航；无需改动其代码。
- 两处「popover 开着时 Enter 不再提交/保存」是方案一的预期代价，已确认接受。
- `EntryRowEdit` 中 popover 关闭后不自动 refocus（既有行为），不在本次改动范围。

## 边界情况

- 列表为空（理论上 dim 非空；val 阶段若某维度 values 为空）→ `activeIndex` 保持 0，导航/Enter 无可选项时 no-op。
- dim 阶段全部维度已填 → 默认高亮 `index 0`。
- val 阶段选完值后若仍有 required 未填，`selectVal` 返回 dim 阶段，按「第一个未填」重算高亮。
- 阶段切换（dim↔val）时 `activeIndex` 必须重算，不能沿用上一阶段的下标。

## 测试计划

Vitest + Vue Test Utils，沿用 `src/__tests__/components/` 既有风格，新增 `DimensionPopover` 测试：

- `CTRL+N` / `↓` 下移高亮；`CTRL+P` / `↑` 上移；两套键等价。
- 到列表两端循环（底部再下 → 回顶；顶部再上 → 回底）。
- dim 阶段弹出默认高亮**第一个未填维度**；全部已填时高亮 index 0。
- val 阶段进入时，若维度已有值则高亮该值，否则 index 0。
- 从 val 选完值返回 dim 阶段，高亮落到**下一个未填维度**。
- dim 阶段 Enter → 进入对应维度的 val 阶段（`activeDimKey` 正确）。
- val 阶段 Enter → emit `select`（dimKey/value 正确）。
- Enter 触发选中，且不冒泡到父级（验证 `stopPropagation`：父级 submit/save 未触发）。
- Esc 维持既有 val→dim / dim→close。

## 文档同步

- `SPEC.md`：§5.2「Enter 永不被拦」需补注「popover 开启时 Enter 改为选中高亮项」。
- 实现完成后按项目规则跑 `/check-consistency`。

## 不做的事（YAGNI）

- 不引入共享键盘导航 composable（项目现有键盘逻辑均内联，保持一致）。
- 不改动 Esc 现有行为。
- 不做首字母快速跳转 / 模糊搜索过滤——本次只做上下移动 + 回车选中。
- 不触碰其他键盘操作需求（本次范围仅此一条）。
