# Logbook UI Redesign — Design Spec

> Status: approved | Date: 2026-06-18
> Demo: `.superpowers/brainstorm/52591-1781790963/content/demo.html`

## 1. 设计目标

将 Logbook 从原生 HTML + 默认 Tailwind 风格升级为现代、一致的桌面应用。两个维度：

- **视觉风格**：Colorful/Playful（indigo-violet 渐变主色调，pill 形控件，微交互丰富）
- **组件质量**：从裸 HTML 元素迁移到自定义 SFC + Radix Vue（无障碍/键盘导航）

## 2. 实现策略

**Systematic — Mini Design System（B 路径）**

定义 ~10 个基础组件（AppButton、AppInput、AppSelect、Chip、ProgressBar、Popover 等），统一 props/slots/events。Radix Vue 负责行为层（focus trap、键盘导航、ARIA），组件 SFC 负责视觉层。

不使用全功能组件库（如 PrimeVue），原因：Logbook 交互复杂度高（@mention 两阶段菜单、inline edit、动态表单），第三方库的主题覆盖成本大于自建。

## 3. 视觉设计

### 3.1 方向

Colorful / Playful。indigo-violet 渐变作为品牌色，pill 形控件（`rounded-full`），维度 chip 各配一色，popover 有入场动画。

### 3.2 Design Tokens

所有值定义为 CSS custom properties，Tailwind 通过 `var()` 引用。无论证过的裸值不进入代码。

#### 3.2.1 色彩

**Surface & Text（Light mode）**

| Token | 值 | 用途 |
|-------|-----|------|
| Page bg | `#f8fafc` + SVG noise grain | App shell |
| Surface | `#ffffff` + `box-shadow: 0 1px 3px rgba(0,0,0,0.06)` | 卡片（无边框，用投影区分） |
| Text primary | `#1e293b` (slate-800, 14.63:1) | 正文、entry 标题 |
| Text secondary | `#64748b` (slate-500, 4.76:1) | Duration、标签、辅助文字 |
| Placeholder | `#64748b` (4.76:1) | 输入框占位符 |
| Border (form controls) | `#64748b` (4.76:1) | 输入框 2px 边框 |
| Border (decorative) | `#cbd5e1` (slate-300) | 导航按钮、popover 边框 |
| Divider | `#f1f5f9` (slate-100) | 行间分割线 |

**Brand**

| Token | 值 | 用途 |
|-------|-----|------|
| Primary gradient | `#6366f1` → `#8b5cf6` | 主按钮、选中日期、进度条 |
| Primary solid | `#6366f1` (indigo-500) | Focus ring、选中态 |
| Primary link | `#4f46e5` (indigo-600, 6.29:1) | 链接文字、合计数字 |
| Primary soft | `#eef2ff` (indigo-50) | Chip bg、popover 选中项 |

**Dimension Chips（12px text, 4.5:1+）**

| Dimension | Background | Border | Text |
|-----------|-----------|--------|------|
| Category | `#eef2ff` | `#c7d2fe` | `#4338ca` (7.07:1) |
| Business Line | `#f5f3ff` | `#ddd6fe` | `#6d28d9` (6.48:1) |
| Importance/Urgency | `#f0fdfa` | `#99f6e4` | `#0f766e` (5.25:1) |
| Goal | `#f0fdf4` | `#bbf7d0` | `#15803d` (4.79:1) |
| Missing required | `#f8fafc` | `#cbd5e1` dashed | `#64748b` |

**Semantic**

| Token | 值 | 用途 |
|-------|-----|------|
| Success | `#059669` (emerald-600, 3.77:1) | 填充标记、进度点 |
| Danger | `#ef4444` (red-500) | Delete hover ONLY |

**Dark Mode 映射**

| Token | Light | Dark |
|-------|-------|------|
| Page bg | `#f8fafc` | `#0f172a` (slate-950) |
| Surface | `#ffffff` | `#1e293b` (slate-800) |
| Text primary | `#1e293b` | `#e2e8f0` (slate-200) |
| Border | `#64748b` | `#64748b` (保持不变) |

Gradient 和 chip 颜色在 dark mode 中保持不变。卡片投影在 dark mode 中移除（深色背景已有足够对比度）。使用 Tailwind `dark:` prefix。

#### 3.2.2 排版

系统字体栈：`-apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif`。无 web font。

| 角色 | Size | Weight | Tailwind |
|------|------|--------|----------|
| Month title | 16px | 700 | `text-base font-bold` |
| Role name | 15px | 700 | `text-[15px] font-bold` |
| Body / Entry / Input / Form | 14px | 400 | `text-sm` |
| Labels / Goal rows / Links | 13px | 400-500 | `text-[13px]` |
| Chips / Row numbers | 12px | 500 | `text-xs` |
| Section headers | 12px | 700 | `text-xs font-bold uppercase tracking-wide` |
| File path | 11px | 400 | `text-[11px]` |

全局 `-webkit-font-smoothing: antialiased`。Line-height: body 1.5, 编辑控件 1.4。

#### 3.2.3 圆角

| 用途 | 值 | Tailwind |
|------|-----|----------|
| Pill (buttons, inputs, chips, day cells) | 999px | `rounded-full` |
| Card (cards, popovers, day strip, day note) | 12px | `rounded-[12px]` |
| Popover (enhanced) | 14px | `rounded-[14px]` |
| Form control (edit inputs, inline selects) | 8px | `rounded-lg` |
| Nav buttons | 50% | `rounded-full` |

#### 3.2.4 边框

| 用途 | Width | Color |
|------|-------|-------|
| 输入框 (idle) | 2px | `#64748b` |
| 输入框 (focus) | 2px | `#6366f1` |
| 卡片 | none | —（用投影代替） |
| 导航按钮 | 1px | `#cbd5e1` |
| Popover | 1px | `#cbd5e1` |
| 分割线 | 1-2px | `#f1f5f9` |

#### 3.2.5 投影

| 用途 | 值 |
|------|-----|
| Card / Day strip / Day note | `0 1px 3px rgba(0,0,0,0.06)` |
| Popover | `0 16px 48px rgba(0,0,0,0.08), 0 0 0 1px rgba(0,0,0,0.03)` |
| Toast | `0 8px 32px rgba(0,0,0,0.15)` |
| Button primary | `0 2px 12px rgba(99,102,241,0.25)` |
| Button primary hover | `0 4px 20px rgba(99,102,241,0.35)` |
| Focus ring | `0 0 0 4px rgba(99,102,241,0.12), 0 0 20px rgba(99,102,241,0.06)` |

#### 3.2.6 间距

| 用途 | 值 | Tailwind |
|------|-----|----------|
| Layout columns gap | 20px | `gap-5` |
| Card internal padding | 16px | `p-4` |
| Between cards / sections | 12px | `gap-3` |
| Between related items | 8-12px | `gap-2` to `gap-3` |
| Chip gap | 6px | `gap-1.5` |

#### 3.2.7 动效

| 用途 | Duration | Easing | 属性 |
|------|----------|--------|------|
| Hover / focus 变化 | 200ms | ease | background, border-color, box-shadow, color, transform |
| Button hover lift | 200ms | ease | transform, box-shadow |
| Button press | 100ms | ease | transform: scale(0.97) |
| Popover 入场 | 200ms | cubic-bezier(0.16, 1, 0.3, 1) | opacity, transform (scale+fade) |
| Entry row hover slide | 200ms | ease | transform: translateX(2px) |

#### 3.2.8 微交互

- **背景纹理**：SVG feTurbulence noise，opacity 0.03，全局应用（light mode only）
- **按钮辉光**：主按钮默认有 `box-shadow` 辉光，hover 时投影扩散 + 按钮上浮 1px
- **Entry 行交互**：hover 时右移 2px + 背景微变；编辑态左侧 3px indigo 竖线 (`box-shadow: inset 3px 0 0 #6366f1`)
- **Popover 入场**：scale(0.97) + translateY(-4px) → 原位，opacity 0 → 1
- **Focus 状态**：4px 辉光环 + 背景从 `white` 转为 `#fafaff`（紫调微染）

## 4. 组件目录

按 Strategy B，以下组件封装为独立 SFC：

| 组件 | 文件 | 依赖 Radix | 说明 |
|------|------|-----------|------|
| AppButton | `AppButton.vue` | — | `variant`: primary / outline / secondary / danger |
| AppInput | `AppInput.vue` | — | 带 focus ring、placeholder 样式 |
| AppSelect | `AppSelect.vue` | Listbox | 自定义 popover 下拉，支持搜索过滤、键盘导航 |
| AppChip | `AppChip.vue` | — | `color`: category / biz / importance / goal / missing |
| ProgressBar | `ProgressBar.vue` | — | 带 gradient fill |
| Popover | `Popover.vue` | Popover | 入场动画、focus trap |
| MentionMenu | `MentionMenu.vue` | Popover + Listbox | 两阶段 @mention 菜单（dim → val） |
| EntryRow | `EntryRow.vue` | — | 展示态、编辑态（item/duration/dimensions） |
| Toast | `Toast.vue` | — | Undo toast |
| CommitmentsEditor | `CommitmentsEditor.vue` | — | 动态表单（role + goal 增删） |

现有组件（MonthNavigator、DayStrip、CommitmentsPanel、SetupScreen、ConfigErrorBanner）保留结构，内部控件替换为上述基础组件。

## 5. 组件树（不变）

```
App.vue
├── SetupScreen.vue
├── ConfigErrorBanner.vue
└── MonthView.vue
    ├── MonthNavigator.vue
    ├── CommitmentsPanel.vue
    ├── DayStrip.vue
    ├── QuickEntry.vue
    │   ├── AppInput
    │   ├── AppChip × n
    │   └── MentionMenu
    ├── EntryList.vue
    │   └── EntryRow.vue × n
    │       ├── AppSelect (inline dimensions)
    │       └── AppChip (dimension displays)
    └── CommitmentsEditor.vue
```

## 6. 数据流（不变）

Reactive store + `provide/inject` 保持现有架构。Tauri `invoke()` 调用不变。改动仅限前端视图层。

## 7. WCAG 2.2 AA 合规

所有颜色组合满足：
- 普通文字（<18px）：4.5:1 最低对比度
- UI 组件边界：3:1 最低对比度（输入框边框满足）
- Focus indicator：2px 边框 + 4px ring，面积和对比度满足 2.4.11

Radix Vue 提供：
- 键盘导航（Arrow keys、Enter、Escape、Tab）
- ARIA attributes（role、aria-label、aria-expanded 等）
- Focus trap（popover 内部）

`prefers-reduced-motion` 时禁用入场动画和 hover 位移。

## 8. 不在范围内

- 功能变更：所有 Tauri command、数据结构、文件格式不变
- Phase 3 图表（StatsView、Chart.js）：不碰
- 键盘快捷键：Phase 4 单独立项
- 响应式布局 / 移动端：桌面应用无需适配
- 国际化：当前仅 English

## 9. 验收标准

- [ ] 所有 10 个基础组件 SFC 已创建，props/slots 文档化
- [ ] 14px body text，12px section headers
- [ ] 全部颜色组合 WCAG 2.2 AA 达标
- [ ] Light + Dark mode 均有纹理/投影正确处理
- [ ] @mention menu 键盘可操作（↑↓ 导航、Enter 选择、Escape 关闭）
- [ ] Commitments 编辑面板表单控件一致
- [ ] Entry row hover lift + 编辑态竖线
- [ ] Popover 入场动画
- [ ] Focus ring 在所有输入控件上可见
- [ ] `prefers-reduced-motion` 禁用动画
- [ ] 现有集成测试通过
