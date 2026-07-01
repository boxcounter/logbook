# 维度自动配色 — 设计

**日期：** 2026-07-01
**状态：** design（待实现计划）

## 问题

维度颜色目前硬编码到 4 个固定 key（`goal`/`category`/`business-line`/`importance-urgency`），分三套 CSS 变量：

| 系统 | token | 用途 | 组件 |
|------|-------|------|------|
| `--dim-bar-*` | 1 色 | 左侧色条 | DimensionPopover、DimensionEditorModal |
| `--color-chip-*-{bg,text}` | 2 色 | entry chip 展示态（柔和） | EntryRow |
| `--color-token-*-{bg,text}` | 2 色 | entry chip 编辑态（饱和） | EntryRowEdit |

维度本质是用户自定义的。任何非默认 key 的维度都落到 fallback，全部撞成同一个灰紫色，色条和 chip 都无法区分。

## 范围

为**任意**维度自动分配可区分的颜色，零配置。不引入用户手选颜色的能力（明确不做）。

## 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 控制权 | 完全自动 | pre-PMF 单用户、低认知负担；不让用户管颜色 |
| 赋色依据 | 维度在当前集合中的位置（均分色轮），非 key 哈希 | 用户要求"当前维度一眼可分"优先于"颜色绝对稳定"；均分保证最大区分 |
| 排序 | 按 key 字母序 | 拖拽重排显示顺序时颜色不变；仅增删维度才重新铺色 |
| 软删除维度 | 不算进 N，固定中性灰 | 软删除维度不参与色轮；历史 entry 用到已删除维度的 chip 显示为灰 |
| 颜色定义 | HSL 运行时公式（非 tokens.css 静态变量） | 颜色是算法生成而非手挑品牌色；公式紧凑、无限扩展、易调 |
| 应用方式 | inline `:style`（非 Tailwind class） | Tailwind 只扫描字面量，`bg-[var(--x-${i})]` 拼接扫不到 |

### 明确不做

- 用户手选/覆盖颜色
- dark-mode 变体（与现状一致，现有 dim 色也无 dark 变体）
- 历史 entry 颜色连续性（单用户、不在意；位置驱动下"原色"本就会漂移）
- 调色板 / 撞色概念（均分色轮后不存在）

## 核心算法：均分色轮

```
活跃维度 = store.dimensions 中 deleted === false 的维度
按 key 字母序排序，得到 N 个维度
第 i 个（0-indexed）分到色相：
  hue(i) = (BASE + i × 360 / N) mod 360      // BASE = 210
```

- N 个维度均匀铺满 360° 色轮 → 永远最大区分。
- 顺序按 key 字母序 → 拖拽重排显示顺序时 hue 不变；仅增/删活跃维度才重新铺色。
- 软删除维度不进 N，取固定灰。

### 五个角色从色相派生

| 角色 | 公式 |
|------|------|
| 色条 | `hsl(H 58% 70%)` |
| chip 展示态 底 | `hsl(H 42% 96%)` |
| chip 展示态 字 | `hsl(H 40% 42%)` |
| chip 编辑态 底 | `hsl(H 66% 95%)` |
| chip 编辑态 字 | `hsl(H 60% 37%)` |

### 软删除维度（灰）

| 角色 | 值 |
|------|-----|
| 色条 | `hsl(0 0% 75%)` |
| chip 展示态 底 / 字 | `hsl(0 0% 96%)` / `hsl(0 0% 45%)` |
| chip 编辑态 底 / 字 | `hsl(0 0% 95%)` / `hsl(0 0% 40%)` |

## 边界情况

| 场景 | 处理 |
|------|------|
| 0 个活跃维度 | 无渲染 |
| 1 个活跃维度 | hue = BASE（210，宜人的蓝） |
| 历史 entry 引用的 key 已不在 store.dimensions | chip 本就不渲染（现有 `filledDims` 已过滤），无影响 |
| 历史 entry 引用的 key 已软删除 | chip 显示为灰 |
| 维度数很大（>20） | 色相间隔变小，区分度下降，但不报错；单用户场景 3-6 个，可接受 |

## 实现

### 重写 `src/utils/dimensionColor.ts`

单一数据源。输入当前维度集合 + 目标 key，输出该 key 的各角色颜色。

- `dimensionHues(dimensions: Dimension[]): Map<string, number | null>`
  按 key 字母序对活跃维度铺色，返回 key → hue；软删除的 key → `null`（表示灰）。
- `dimBar(hue: number | null): string`
- `dimChipStyle(hue: number | null): { background: string; color: string }`
- `dimTokenChipStyle(hue: number | null): { background: string; color: string }`

组件先用 `dimensionHues(store.dimensions)` 拿到映射，再对每个 key 查 hue、算样式。

### 组件接线（4 处，全改 inline style）

- `DimensionEditorModal.vue`：色条 `:style="{ background: dimBar(hue) }"`（draft 维度集合铺色，编辑时增删维度实时重铺，作为即时反馈）
- `DimensionPopover.vue`：色条 `:class="barClass(...)"` → `:style`；删除本地 `barClass`
- `EntryRow.vue`：chip `:class="chipClass(...)"` → `:style="dimChipStyle(hue)"`；删除本地 `chipClass`
- `EntryRowEdit.vue`：chip `:class="chipClass(...)"` → `:style="dimTokenChipStyle(hue)"`；删除本地 `chipClass`

### tokens.css 清理

删除现已无引用的 `--dim-bar-*`、`--color-chip-*`、`--color-token-*`（先 grep 确认无其他引用；heatmap、token-dur 等无关 token 保留）。

## 测试

重写 `src/__tests__/dimensionColor.test.ts`：

- 给定维度集合 → 每个活跃 key 的 hue 正确（`BASE + i×360/N`）
- 色相均匀间隔
- 按 key 字母序稳定：打乱输入数组顺序，hue 映射不变
- 增/删活跃维度触发重新铺色（N 变化 → hue 变化）
- 软删除 key → null（灰）
- 1 个维度 → BASE；0 个 → 空映射
- `dimChipStyle(null)` / `dimTokenChipStyle(null)` 返回灰

EntryRow / EntryRowEdit 现有测试不查颜色，应仍通过。

验证：`pnpm build` + `vitest run` + `vue-tsc` 全绿。

## 取舍

- **位置驱动 vs key 哈希**：选位置驱动，保证当前维度最大区分。代价：增删维度会让所有活跃维度重新铺色（拖拽重排不变）。用户明确接受。
- **HSL 运行时 vs tokens.css 静态变量**：选运行时公式，紧凑、无限扩展、易调 S/L。代价：这几个颜色不进 tokens.css，偏离"颜色入 token"惯例；无 dark 变体（与现状一致）。
- **软删除用灰 vs 保留原色**：选灰。位置驱动下"原色"本会漂移，灰更自洽。代价：与旧维度编辑器设计文档"历史保留原色"相反——本设计取代该条。
