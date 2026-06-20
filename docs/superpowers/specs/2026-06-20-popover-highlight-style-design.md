# DimensionPopover 高亮样式重设计

日期：2026-06-20
状态：待实现

## 背景与问题

DimensionPopover 键盘导航已实现（见 [键盘导航设计](2026-06-20-dimension-popover-keyboard-nav-design.md)），但 active（当前光标/高亮项）的视觉用了 `ring-1 ring-inset ring-[var(--color-brand-solid)]`——这是**全项目唯一**使用 Tailwind `ring-*` 工具类的地方，与项目设计语言不一致，且 1px 细描边偏弱、易与边框混淆。

项目表达「选中/激活」的惯用语言是：
- **填充色块**为主（EntryRow hover `bg-surface-muted`、popover「已填」项 `bg-popover-item-selected-bg`、chips 填充）。
- shadow 模拟的 ring（HeatmapCalendar 用 `shadow-[0_0_0_2px_…]`，非 `ring-*` 工具类）、输入框 `shadow-focus-ring`。

经真实 token mockup 对比，选定**方案 A：品牌色填充背景**——最贴合项目「填充优先」语言，且与「新增条目高亮动画」(`--anim-highlight-bg` #eef2ff) 同色系。

## 需要区分的三种状态

因为 `mouseenter` 会把 `activeIndex` 同步到悬停项，**hover 与键盘高亮是同一状态**（同一时刻只有一个 active 项）。真正要区分：

| 状态 | 含义 | 视觉 |
|---|---|---|
| 普通 | 未填、非光标 | 默认文字，无背景 |
| filled | 该维度/值已有值 | `--color-popover-item-selected-bg` 底 + 品牌色文字 + semibold |
| active | 当前光标（键盘/鼠标） | `--color-popover-item-active-bg` 底 |
| active + filled | 光标落在已填项上 | active 底 + 品牌色文字 + semibold（叠加） |

active 底色比 filled 底色更突出（active 是光标，应最显眼）。

## 设计

### 1. 新增 design token（`src/assets/tokens.css`）

`--color-brand-soft-bg`(#eef2ff) 无深色覆盖，直接用会在深色模式下变成刺眼亮行。仿照 `--color-popover-item-selected-bg`（浅/深两套）新增专用 token：

`:root`（浅色，约在 `--color-popover-item-selected-bg` 一行之后，line 85 附近）新增：

```css
  --color-popover-item-active-bg: #eef2ff;
```

深色 `@media (prefers-color-scheme: dark)` 块（约在 `--color-popover-item-selected-bg: #1e1b3a;` line 165 之后）新增：

```css
    --color-popover-item-active-bg: #2e2a52;
```

深色值 #2e2a52 比「已填」#1e1b3a 更亮一档，保证 active 在深色模式下仍比 filled 更突出。值可在实现时按观感微调。

> 说明：`docs/superpowers/specs/2026-06-19-ux-redesign-tokens.css` 是用于「与 demo 对比防漂移」的快照副本，不在本次同步范围；只改 canonical 的 `src/assets/tokens.css`。

### 2. DimensionPopover 渲染逻辑（`src/components/DimensionPopover.vue`）

active 与 filled 两个背景不能靠 CSS 类先后决定（两个 `bg-[...]` 任意值优先级相同，源码顺序不可控）。改为在 `:class` 绑定里算**单一背景**与文字样式。

**dim 阶段 item**（当前 line ~100-114）`:class` 改为：

```html
        :class="[
          activeIndex === i
            ? 'bg-[var(--color-popover-item-active-bg)]'
            : (dimValues[d.key] ? 'bg-[var(--color-popover-item-selected-bg)]' : ''),
          dimValues[d.key] ? 'text-[var(--color-brand-solid)] font-semibold' : 'text-[var(--color-text-primary)]',
        ]"
```

并**移除** 该元素 class 列表里的 `hover:bg-[var(--color-divider)]`（active 背景已统一接管悬停项）。`data-active`、`@mouseenter`、左侧维度色条 `barClass` 全部保留不变。

**val 阶段 item**（当前 line ~134-142）`:class` 改为：

```html
        :class="[
          activeIndex === i
            ? 'bg-[var(--color-popover-item-active-bg)]'
            : (activeDimKey && dimValues[activeDimKey] === v ? 'bg-[var(--color-popover-item-selected-bg)]' : ''),
          activeDimKey && dimValues[activeDimKey] === v ? 'text-[var(--color-brand-solid)] font-semibold' : 'text-[var(--color-text-primary)]',
        ]"
```

同样**移除** val item 上的 `hover:bg-[var(--color-divider)]`。

> 移除 `ring-1 ring-inset ring-[var(--color-brand-solid)]`：上面的 `:class` 已不含它，等价于移除。全项目 `ring-*` 工具类清零。

### 3. 行为不变

键盘导航逻辑（CTRL+N/P、↑↓、Enter、默认高亮、阶段切换）一律不动。本次纯视觉：把 active 的表现从「ring 描边」换成「填充背景」，并统一 hover。

## 测试计划

`src/__tests__/components/DimensionPopover.test.ts`。现有断言用 `data-active="true"` 定位高亮项——`data-active` 属性**保留不变**，故现有导航/默认高亮/Enter 等测试全部不受影响，无需改动。

新增/调整针对样式的断言：

- dim 阶段：active 项的 class 含 `bg-[var(--color-popover-item-active-bg)]`，且不再含 `ring-1` 或 `hover:bg-[var(--color-divider)]`。
- dim 阶段：filled 但非 active 的项 class 含 `bg-[var(--color-popover-item-selected-bg)]`，不含 active 背景类。
- dim 阶段：active 落在已填项上时，同时含 active 背景类 + `text-[var(--color-brand-solid)]` + `font-semibold`（验证叠加，且未同时出现 selected 背景类）。
- val 阶段：active 值项含 active 背景类；已选值（非 active）含 selected 背景类。

（断言基于 class 字符串，沿用 `wrapper.findAll(...)[i].classes()` 风格。）

## 影响面

- 仅 `src/assets/tokens.css`（+2 行 token）、`src/components/DimensionPopover.vue`（两处 `:class`）、`DimensionPopover.test.ts`（样式断言）。
- `TwoLineInput` / `EntryRowEdit` 复用同一 popover，自动生效，无需改动。

## 不做的事（YAGNI）

- 不改键盘导航行为。
- 不动维度左侧色条（barClass）、不引入左侧品牌 accent 条（方案 C 被否，dim 阶段会与维度色条双条并列）。
- 不用 shadow ring（方案 B 被否，整行列表里套环偏「盒中盒」）。
- 不同步 `2026-06-19-ux-redesign-tokens.css` 快照副本。
