# 设计 — 行高标准化（line-height standardization）「1b」

**日期**：2026-06-21
**状态**：已评审（mockup + 终端对话），待写实施计划。
**前置**：字号/间距 consolidation 已完成（spec `2026-06-21-design-system-consolidation-design.md`，截至 `20d1833`）。本轮承接 handoff `2026-06-21-line-height-standardization-handoff.md`。

## 结论先行

给四个字号档各挂一个默认行高；删除 7 处数值 `leading-*` 让其继承档默认；保留 8 处 `leading-none` 图标覆盖并加注释；扩展护栏禁止散装数值行高。迁移后全仓不再有「靠浏览器默认」的隐式行高，且只剩 `leading-none` 一种合法显式覆盖。

本轮**以视觉效果为先**定值与定去留，不是为「消除隐式默认」的治理目标硬来——后者是顺带达成的副产物。

## 已核实的关键事实（决定方案可行性）

Tailwind v4 中 `--text-<tier>--line-height` 会让 `text-<tier>` 工具类**同时**输出 `font-size` 与 `line-height`；但 line-height 工具类在生成顺序上排在 font-size 之后，故元素上显式的 `leading-*` 仍然胜出（同特异度、靠源序决胜）。

来源：Tailwind 官方文档 font-size / line-height 页（context7 `/websites/tailwindcss`，2026-06-21 核实）。可靠度：官方文档。

**推论**：加档默认后，已有显式 `leading-*` 的元素视觉不变；真正改变的只有「有 `text-*` 但无 `leading-*`」、当前吃浏览器 `normal`（≈1.2）的元素。落地时再用一次 `npm run build` 实测坐实此排序（build 为事实源）。

## 选定值（已 mockup 验证）

| 档 | 字号 | 行高 |
|---|---|---|
| title | 20px | 1.2 |
| body | 14px | 1.4 |
| secondary | 12px | 1.5 |
| micro | 10px | 1.6 |

验证过程：day view 真实密度对比（标题/徽章/时长/commitments）、多行散文对比（error banner / 空状态 / popover 提示）。用户判定方向正确、多行散文行距「正好」。

## 改动

### 1. `@theme`（`src/assets/main.css`）

四档各补一行：

```css
--text-title: 20px;     --text-title--line-height: 1.2;
--text-body: 14px;      --text-body--line-height: 1.4;
--text-secondary: 12px; --text-secondary--line-height: 1.5;
--text-micro: 10px;     --text-micro--line-height: 1.6;
```

同时更新 `@theme` 内第 14 行附近的注释（原文「Font-size only ... line-height stays with each element's leading-* class」）——该承诺本轮不再成立，改为说明「每档自带默认行高；需紧排时用 `leading-none` 覆盖」。

### 2. 覆盖处置（15 处 → 留 8）

**保留（8 处 `leading-none`）**：均为图标 / 单字形 / 单行导航键，折叠会撑高行盒、错位图标。每处补一行注释说明紧排原因。

- `DimensionPopover.vue:202` 返回键 `←`
- `EntryComposer.vue:156` `+` 字形 / `:180` `×` 删除
- `DayHeader.vue:33` `←` / `:40` `→`
- `composite/EntryRowEdit.vue:166` `×` 删除
- `composite/EntryRow.vue:80` `⋯` 编辑
- `base/Toast.vue:34` 关闭

**删除并入档默认（7 处）**：

| 文件 | 现值 | 并入 | 视觉差 |
|---|---|---|---|
| `composite/EntryRow.vue:63` 正文 | `leading-[1.4]` | body 1.4 | 0（逐像素相同）|
| `EntryComposer.vue:176` token | `leading-[1.6]` | micro 1.6 | 0 |
| `EntryComposer.vue:186` dur-token | `leading-[1.6]` | micro 1.6 | 0 |
| `MonthView.vue:348` 备注 | `leading-[1.5]` | secondary 1.5 | 0 |
| `EntryComposer.vue:163` 单行输入 | `leading-[1.5]` | body 1.4 | ~1px |
| `composite/EntryRow.vue:69` chip | `leading-[1.7]` | micro 1.6 | ~1px，且统一显示态/编辑态 |
| `composite/CommitmentsModal.vue:155` 摘要 | `leading-[1.8]` | secondary 1.5 | 两行收紧（已确认接受）|

（行号为 `20d1833` 时点快照，实施时以实际为准。）

### 3. 护栏（`src/__tests__/tailwind-token-usage.test.ts`）

新增一条 line-height 检查，与现有 spacing/font 检查同构（正则扫 `.vue` 源码）：

- 禁止数值型任意行高 `leading-[...]`（如 `leading-[1.4]`、`leading-[1.8]`）。
- 禁止 Tailwind 数字档 `leading-<number>`（如 `leading-6`）。
- **仅放行 `leading-none`**（唯一合法显式覆盖）。
- 报错信息提示替代：「行高跟随字号档；需紧排用 `leading-none`，破例需注释 + 显式豁免」。

不变量：迁移后除 8 处 `leading-none` 外，不应再有任何 `leading-*`；该 guard 防止日后回归。

### 4. CLAUDE.md「设计 token」段

现该段只约束间距与字号。补一句行高规则：「行高跟随字号档（`--text-<tier>--line-height`）；需紧排显式 `leading-none` 覆盖并注释；禁止散装 `leading-[...]` / `leading-<number>`。」

## 验证

- `npm run verify`（vue-tsc + vitest + guard，含新增行高检查）绿。
- `npm run build` 实测确认 `leading-none` 仍覆盖档默认（坐实 Tailwind 源序假设）。
- 关键面人工比对（day view / commitments / popover / banner）——与本轮 mockup 预演一致。

## 验收

- 四档各有明确默认行高；元素要么吃默认、要么 `leading-none` 覆盖且带注释，无「靠浏览器默认」的隐式状态。
- 全仓显式 `leading-*` 仅余 8 处 `leading-none`。
- guard 含行高检查并通过。
- `npm run verify` 绿。

## 风险与缓解

- **源序假设错误**（显式 leading 未胜出）：用 build 实测坐实；若不成立，需在 `@theme` 之外调整或对 8 处 `leading-none` 改用更高特异度——概率低但已列为 build 验证项。
- **CommitmentsModal 摘要收紧**：1.8→1.5 两行变近，右块变矮、可能与左侧标题块略不等高——已 mockup 确认接受。
- **固定高度容器**（热力图格子等 flex 居中）：行高基本无影响，比对时确认不溢出。

## 落地范围

单 commit：1 个 CSS + 6 个组件文件 + 1 个测试 + CLAUDE.md。改动集中、低风险。
