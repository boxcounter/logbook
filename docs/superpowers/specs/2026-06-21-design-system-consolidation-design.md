# 设计规范梳理与一致性治理（Design System Consolidation）

**日期**：2026-06-21
**状态**：设计已敲定，待转 writing-plans
**范围**：前端设计 token 体系——字号、间距为主，动效定最小规范，圆角顺手收；外加一套面向 agentic 开发的治理机制。颜色、阴影维持现状不动。

---

## 1. 背景与诊断

起因：肉眼感到"相似组件字号/间距略有不一致"。对 `src/components` + `src/App.vue` 做了全维度代码扫描，健康度如下：

| 维度 | 状态 | 证据 |
|---|---|---|
| 间距 | 🔴 最烂 | `--space-*` 阶梯已定义却 **0 引用**（死 token）；实际散落 **16 种**字面 px（`1 2 3 4 5 6 7 8 9 10 12 14 16 20 24 28`），混用 Tailwind 默认档与任意值两套 |
| 字号 | 🔴 烂 | 9–14px 挤 **6 档**，命名不按大小排序（`micro`10 < `xs-alt`11 < `xs`12），按元素散点分配、无语义层 |
| 动效 | 🟡 薄+乱 | 无 token 阶梯，时长散用 150/200/500，缓动零星 |
| 圆角 | 🟢 基本健康 | 多数走 token，4 处逃逸 |
| 阴影 | 🟢 健康 | 全走 token |
| 颜色 | 🟢 健康 | 语义命名到位，明暗模式有对应，硬编码 hex 仅 4 处（疑似有意） |

**根因**：缺的不是 token，是**语义层**。颜色已做到语义命名（`text-primary/secondary/muted`），字号/间距没跟上——它们按"大小"或"像素"命名，导致选值靠手挑、跨组件对不齐。

**附带发现**：
- `tokens.css` 的注释把字号映射写成"谁在用我"的实例清单（如 `--app-text-2xs` 标注 "Enter badge, kbd, heatmap weekday…"），且**已与代码不符**——例如注释称 commitment 角色名用 `xs`(12)、goal 名用 `xs-alt`(11)，实际代码是 `base`(14) 与 `sm`(13)。手写的实例清单必然漂移。
- `tokens.css` 顶部声明"extracted from UX-REDESIGN-DEMO.html，compare with demo to verify no drift"——这是**双真相源**，注定漂移。

---

## 2. 目标 token 体系

### 2.1 字号：6 档 → 4 档（语义化）

| 新 token | px | 语义角色 | 合并自 |
|---|---|---|---|
| `text-title` | 20 | 标题 | xl(20)，不变 |
| `text-body` | 14 | 正文 / 输入 | base(14)，不变 |
| `text-secondary` | 12 | 次要文本 / 元数据 | sm(13) + xs(12) + xs-alt(11) |
| `text-micro` | 10 | chip / 标签 / 热力图 / kbd | micro(10) + 2xs(9) |

- 18px 的输入框"+"号是装饰字形，**踢出文字阶梯**，单独保留并改名（如 `--glyph-plus`）。
- 依据：相邻档（9/10、11/12/13）差 1px，肉眼不可分，合并不丢可见信息；界面实际变化全部 ≤1px。
- 层级原则：**1px 字号差撑不起层级，层级交给字重/颜色/结构**。已验证 Role/Goal 区分（结构嵌套 + 字重 600/400 + 颜色 primary/secondary）不依赖字号，合并后 Role 14 vs Goal 12 差距反而从 1px 扩到 2px。

**迁移映射**：9→10；11→12；13→12；10/12/14/20 维持。

### 2.2 间距：16 散值 → 复活的 7 档网格

原 `--space-*` 阶梯基本正确，只是没人用。复活 + 两处修正：

| token | px | 修正 |
|---|---|---|
| `space-2xs` | 2 | **新增**（承接 1/2px 微调） |
| `space-xs` | 4 | — |
| `space-sm` | 8 | — |
| `space-md` | 12 | — |
| `space-lg` | 16 | — |
| `space-xl` | 24 | — |
| `space-2xl` | 32 | 原 28 → 32，回到 4px 网格 |

**迁移映射（就近取整）**：

| 现状 px | → 档 | 影响 |
|---|---|---|
| 1, 2 | 2xs(2) | ≈无 |
| 3, 4, 5 | xs(4) | ≤1px |
| **6** | sm(8) | **+2** |
| 7, 8, 9 | sm(8) | ≤1px |
| **10** | sm(8) | **−2** |
| 12 | md(12) | 不变 |
| **14** | md(12) | **−2** |
| 16 | lg(16) | 不变 |
| **20** | xl(24) | **+4** |
| 24 | xl(24) | 不变 |
| **28** | 2xl(32) | **+4** |

- 与字号不同，间距**有可见变化**：5 处值（6/10/14/20/28）移动 2–4px。已在整页高密度 mockup（4 条 entry + 输入卡 + commitments 卡）确认效果可接受。
- 高频表面的具体影响：entry 行 `px14 py9 → px12 py8`；输入卡 `py10 → py8`；输入 token `py1 → py2`；day header `mb20 → mb24, pb14 → pb12`；commitments 卡基本不动。

### 2.3 动效：3 档时长 + 默认缓动（命名现有值，零行为变化）

| token | 值 | 用途 | 现状来源 |
|---|---|---|---|
| `--motion-fast` | 150ms | hover / 颜色 / 边框 / 透明度 微反馈 | duration-150 ×5 |
| `--motion-base` | 200ms | 按钮、Toast 进出 | duration-200 ×2 + Toast 0.2s |
| `--motion-slow` | 500ms | 进度条填充（要看得见地动） | ProgressBar ×1 |
| `--anim-highlight-duration` | 1.5s | 新增条目高亮淡出 | 已有，保留 |

- 缓动：`--ease-out` 设为默认；`--ease-in` 仅用于离场（Toast leave）。
- 全部照搬现有行为，只把魔法数字换 token；动起来感觉一致。动效面积小，**不强制 guard**，约定用 token 即可。

### 2.4 圆角：收 4 处逃逸（基本零视觉变化）

| 逃逸 | 位置 | 改成 | 影响 |
|---|---|---|---|
| `rounded-[4px]` ×2 | `ProgressBar.vue:10,12` | `rounded-full` | 进度条本为胶囊，0 |
| `rounded-[2px]` ×2 | `RoleCard.vue:152,155` | `rounded-full` | 0；同时统一两个进度条圆角 |
| `rounded-[10px]` | `Toast.vue:22` | `--radius-card`(12) | +2px，无感 |
| `rounded-lg` ×2 | `ConfigErrorBanner.vue:7`、`App.vue:155` | `--radius-form-lg`(8) | 精确相等，0 |

- 圆角阶梯本身不变（sm3/md4/form5/form-lg8/card12/lg14）。

### 2.5 裸 Tailwind 簇（迁移时一并收编）

`ConfigErrorBanner.vue` 与 `App.vue:155`（错误/配置态边缘 UI）使用裸 Tailwind 默认值（`p-4`、`mx-4`、`bg-blue-600`、`text-sm`、`/5` `/20` 透明度），完全在 token 体系外。低频，但属"最不守规矩"，迁移时收进体系；guard test 也会逼出它们。

---

## 3. 治理机制（面向 agentic 开发）

前提认知：人学一次规范即内化；**agent 每个 session 是新的，行为只来自 (a) 当前 context 里有什么、(b) 什么反馈在纠正它**。`--space-*` 的死亡正是"只有散文层、没有可执行层"的后果。因此治理是**双核 + 辅助**，而非人类团队那种四层均衡。

### 3.1 双核（承重）

**核一：可执行 guard，且 agent 在 session 内自跑**
- 扩展现有 `src/__tests__/tailwind-token-usage.test.ts`，新增规则：
  - 禁止裸 px 间距（`p-[*px]`/`gap-[*px]`/`m*-[*px]` 等任意 px 值）。
  - 禁止非语义字号（`text-[length:var(--app-text-*)]` 之外的字号写法 / 已废弃的旧字号 token）。
- 接入 `npm run build` / pre-commit（husky + lint-staged）+ CI。**只挂 CI 不够**——那时 agent 的 session 已结束，看不到反馈；必须进 agent 的 verify 循环，并由 CLAUDE.md 要求"完成前先 verify"。
- **guard 的报错写成"教学信息"**：不只报违规，要给出合法替代，例如
  `gap-[8px] 违规 → 请用 gap-[var(--space-sm)]（=8px）；间距必须走 --space-* 阶梯`。
  对 agent，测试失败信息是主要教学通道，给出替代可让它一轮改对。
- **人工把关一次**：guard 的覆盖范围由人（或一次性人工）过一眼，防止 agent 自己把 guard 写松；过关后 agent 即对着它自律。

**核二：常驻 context 的硬规则**
- 在 CLAUDE.md / 常驻文档写一句：「间距/字号必须用语义 token，禁止裸 px」。对 agent，**没载入 context 的规范等于不存在**——这是 `--space-*` 死亡的 agentic 版本。

### 3.2 辅助

- **`@theme` 单一真相源**：阶梯放进 Tailwind v4 `@theme`，作为 agent 的合法 token 词汇表（配合 guard 报错使用；对 agent，编辑器自动补全价值≈0，但词汇表价值高）。`@theme` 的确切行为在实现阶段对照官方文档核实。
- **逃生口留痕 + 人签字**：确有正当破例时，要求一行注释说明 + 显式 lint ignore，**且需人工签字**（agent 会为偷懒编造合理理由，"留痕区分有意/偷懒"对 agent 不成立）。
- **改进通道**：改阶梯 / 加 token 必须走 PR + 说明理由（沿用 `interaction-principles.md` 既有约定的风格），让规范被刻意演进而非偷偷腐蚀。
- **漂移体检**：把"设计 token 漂移"加进 `/check-consistency` skill，phase 结束时重跑扫描。

### 3.3 文档约定（治本注释漂移）

- **注释写意图，不写清单（document intent, not inventory）**：凡能从代码 grep 出来的（谁用、用在哪），不手写；手写只留代码看不出的（为何存在、为何这么命名）。
  - 语义改名即治本：`--app-text-secondary` 名字自带用途，grab-bag 实例清单变多余 → 删除。
  - 每个 token 留**一行角色注释**（如「次要文本/元数据」），不留实例清单（角色稳定、实例易变）。
  - 保留"理由型"注释（如 Tailwind `--font-*`/`--text-*` 命名空间冲突说明、active/passive 状态区分）——这些是代码看不出的 know-how。
- **单一真相源**：删除 `tokens.css` 顶部"compare with demo"声明，`UX-REDESIGN-DEMO.html` 降为历史存档，`tokens.css` / `@theme` 为唯一真相源。

---

## 4. 命名约定（沿用既有，避免 Tailwind v4 命名空间冲突）

`tokens.css` 已记录：Tailwind v4 保留 `--font-*` / `--text-*` 的 `@theme` 命名空间，裸 `--font-*` / `--text-*` 会冲突，故用 `--app-font-*` / `--app-text-*` 前缀（见 commit `0acd2ee`）。本次新增/改名 token **沿用该前缀约定**，具体命名（语义档放 `:root` 还是 `@theme`、如何同时满足 IntelliSense 与无冲突）在实现阶段对照 Tailwind v4 文档确定。

---

## 5. 范围边界

**做**：字号 4 档语义化 + 全量迁移；间距复活 7 档 + 全量迁移；动效 3 档命名；圆角收 4 处 + 统一进度条；裸 Tailwind 簇收编；guard test 扩展 + CI/pre-commit 接入；CLAUDE.md 规则；tokens.css 注释整改 + 去双源。

**不做**（非目标）：
- 颜色、阴影体系（健康，不动）。
- 重新设计任何组件的视觉/布局——本次只统一 token，不改设计语言。
- 引入 `eslint-plugin-tailwindcss` 全家桶或 Style Dictionary 等 token 流水线（对 12 组件过度设计，自写正则 guard 已足够）。
- 暗色模式重做（仅随 token 改名同步）。

---

## 6. 风险

- **间距迁移有可见变化**（5 处 2–4px 移动）。缓解：已用整页高密度 mockup 验收；迁移后逐组件目检；保留按值回退的余地。
- **guard 误伤合法写法**（如热力图描边 `shadow-[0_0_0_2px_...]` 等内联场景）。缓解：guard 规则精确限定到间距/字号工具类，逃生口 + 人签字兜底。
- **build 严格 typecheck 测试**（已知：`npm run build` 跑 vue-tsc over tests，vitest 绿 ≠ build 绿）。缓解：guard test 写法遵守 `noUnusedLocals` 等约束，迁移后跑 `npm run build` 而非仅 vitest。
- **agent 自写 guard 偏松**。缓解：§3.1 的一次性人工把关。

---

## 7. 验收标准

- 字号：组件仅引用 `text-title/body/secondary/micro`（+ `--glyph-plus`）；旧 6 档 token 删除。
- 间距：组件零裸 px 间距；`--space-*` 7 档全部"活着"（有引用）；guard test 通过。
- 动效/圆角：魔法数字替换为 token；进度条统一 `rounded-full`。
- guard test 在 `npm run build` + CI + pre-commit 三处生效，报错含教学信息。
- `tokens.css` 注释为"角色 + 理由"型，无实例清单；无"compare with demo"声明。
- CLAUDE.md 含"间距/字号必须用语义 token"规则。
- `npm run build` 与全量测试通过。
