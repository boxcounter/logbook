# Commitments Editor UX 重设计 — Design Spec

> 日期：2026-06-20 | 状态：design-review
> 关联 mockup：`2026-06-20-commitments-editor-ux-mockup.html`（定稿，七维度已对齐 tokens）
> 取代 `2026-06-15-commitment-editor-design.md` 的「整面板内联编辑」承载方式；后端 `set_commitments` 复用并扩展。

## 背景与目标

现有 commitments 编辑器（`CommitmentsEditor.vue`）内联在 220px 左侧栏，是很原始的表单：裸输入框、数字 spinner、"Delete Role" 纯文字、整面板编辑。

用户痛点优先级（已确认）：**空间局促（4）> 视觉粗糙（1）> 能力缺失（3）**。批量编辑 + 显式保存的模型本身 OK（痛点不在交互模型）。

目标：把编辑搬到更大的承载面、控件精致化、补齐几项能力，且**严格不引入设计漂移**——所有尺寸/颜色/间距/圆角/阴影/动效都映射到 `src/assets/tokens.css` 现有 token。

## 范围

- 编辑承载：从内联面板改为**居中 modal**
- 视觉：role 卡片化、招牌渐变进度条（随 allocation 实时变化）、per-goal logged 时长
- 能力补充：
  - 拖拽排序（role 之间；goal 在所属 role 内）
  - allocation 步进控件（步进 5h、最小 5h、可直接输入）
  - 删除二次确认（role；有 logged 时长的 goal）
  - 顶部总额汇总（Committed 合计 / Logged 实际）
  - 超额软警告（amber，不拦截）
  - 空状态引导
- 校验扩展：role 名唯一、goal 名跨 role 全局唯一、空 goal 静默丢弃

## 不在范围

- 从上一月复制 commitments（未来迭代）
- 跨 role 移动 goal（删了重加）
- i18n（文案先用英文，编辑框内容允许中/英；后续单独做）

## 承载方式

居中 **modal**（方案 A，已选）。宽度 **660px**（针对 MacBook Pro 14"，约占屏宽 44%），遮罩聚焦。低频「偶尔编辑」操作，临时脱离上下文可接受。背景遮罩点击 = 取消。

## 视觉设计

设计理念：**编辑器 = 侧栏 Commitments 面板的可编辑放大版**，而非另起的表单。复用侧栏的 role 组件视觉身份（role 名 + allocation + 渐变进度条 + goal 行），给足宽度并使字段可编辑。

所有视觉值映射到 token（无新增 token、无漂移）：

| 维度 | 取值 |
|---|---|
| **字号** | 标题 `xl 20`/700（= day-title）· role 名 `base 14`/600 主色 · alloc 数字 `base 14`/600 mono · goal 名 `sm 13`/400 次色 · 单位/汇总/logged/错误 `xs-alt 11` · subtitle/Add Goal `xs 12` · Add Role/按钮 `sm 13`。role↔goal 区分靠**字重 + 颜色**（梯度无 15/16） |
| **颜色** | surface/border `#e2e8f0`/divider `#f1f5f9`；brand solid `#6366f1` / link `#4f46e5` / gradient `#6366f1→#8b5cf6`；text 三级 primary `#1e293b` / secondary `#64748b` / muted `#94a3b8`；disabled `#cbd5e1`；danger `#ef4444`；warning `#f59e0b`；拖起边 `#c7d2fe` |
| **间距** | 布局级落在 `4/8/12/16/24/28`：卡 padding `16`、卡间距 `12`、行内 gap `8`、head `24/28/16`、body `16/28/4`。控件内部 padding 保留弹性（demo 自身亦用 `2/7/9/10px`） |
| **圆角** | modal `14`(lg)、role 卡 `8`(form-lg)、输入框/stepper/按钮 `5`(form)、Discard 弹窗 `12`(card)、进度条 `2px`（沿用 demo） |
| **阴影** | modal `--shadow-popover`；拖起 `.lifted` 与 Discard 弹窗 `--shadow-toast`(`0 8px 32px rgba(0,0,0,.15)`)；focus 仅变紫边（= `.edit-dur-input`），无自创光环 |
| **动效** | 微交互统一 `150ms`（hover/opacity/color/border）；进度条 `transition:width .15s`（= 真实 `CommitmentsPanel` 的 `transition-all`）；`prefers-reduced-motion` 由 tokens.css 全局 media query 接管，组件不重复处理 |

两种语义色各司其职：**danger 红** = 硬错误/破坏性（校验拦截、删除确认）；**warning amber** = 软提示（超额，不拦截）。

## 编辑器结构

```
┌─ Edit Commitments ──────────── Committed 60h / Logged 23.4h ─┐  (header)
│                                                              │
│ ┌ role 卡片 (#f8fafc, radius 8) ───────────────────────────┐ │
│ │ ⠿  [Developer        ]   [− 40 +] h        Delete        │ │  role-top
│ │ ▓▓▓▓▓▓▓░░░░░░░░░  14h 30m logged                          │ │  渐变进度条 + spent
│ │   ⠿ [Ship onboarding v2  ]            14h 25m   ×         │ │  goal 行 + logged + ×
│ │   ⠿ [Review auth PR      ]                 5m   ×         │ │
│ │   + Add Goal                                             │ │
│ └──────────────────────────────────────────────────────────┘ │
│ ┌ role 卡片 … Director …                                    ┐ │
│ + Add Role                                                   │
│                                          Cancel    Save      │  (footer)
└──────────────────────────────────────────────────────────────┘
```

- 拖拽手柄 `⠿` hover 浮现
- 进度条宽度 = `spent / 当前 allocation 输入值`，随 stepper **实时**伸缩
- per-goal logged 来自 `CommitmentProgress`（0 时显示灰色 `0`）

## 组件设计

- **`CommitmentsModal.vue`**（新）：modal 容器（遮罩、focus trap、Esc/遮罩关闭、discard 确认），内含编辑逻辑。取代 `CommitmentsEditor.vue` 的内联渲染（`CommitmentsEditor.vue` 可重构为 modal 内容或直接由新组件承载）。
- **`CommitmentsPanel.vue`**（改）：展示态不变；触发改为打开 modal。空状态时显示「Set up commitments」入口（见下）。
- 数据依赖：modal 接收 `commitments`（编辑对象）+ `progress: CommitmentProgress[]`（渲染进度条、per-goal logged、超额判断）。`progress` 已存在于 `store.commitmentProgress`。
- 进度/logged 数字为**保存态快照**，编辑中改名/重排不实时重算（entry 未变更，无法重算），保存并 reload 后刷新。

## 交互规格

### 键盘
| 键 | 行为 |
|---|---|
| `Enter` 在 goal 框 | 下方新增空 goal 并聚焦（连续录入）；尾部空 goal 上按不新增 |
| `Enter` 在 role name | 聚焦该 role 的 allocation |
| `↑`/`↓` 在 allocation | ±5h |
| `Tab`/`Shift+Tab` | 按视觉顺序遍历字段 |
| `⌘/Ctrl+Enter` | 保存 |
| `Esc` / 点遮罩 | 取消（有改动则弹 discard 确认） |

### Allocation 步进
- 步进 **5h**；**最小 5h**（到 5h 时 `−` 置灰）；无上限
- 数字框可**直接点击输入**任意正整数（不强制 5 的倍数）；stepper 仅 ±5

### 拖拽排序
- 手柄 hover 浮现。拖起 role 卡片抬起（`--shadow-toast` + 紫边 `#c7d2fe`），插入位显示紫色 drop-line
- goal 仅在所属 role 内重排，不跨 role
- 依赖决策见「依赖」一节

### 删除确认
- **删 role** → 行内二次确认（`Delete role?  Delete / Cancel`）
- **删 goal**：无 logged 时长 → `×` 直接删；**有 logged 时长** → 行内确认并提示时长（`Remove? 1h 40m logged  Remove / Cancel`）
- role 最少保留 1 个

### 校验（保存时拦截，红框 + 字段级提示）
| 条件 | 处理 |
|---|---|
| role 名为空 | 拦截 |
| **role 名重复**（新增规则） | 拦截，标红重复项 |
| **goal 名跨 role 重复**（由「同 role 内唯一」改为全局唯一） | 拦截，标红 |
| 空 goal 行（如 + Add Goal 未填） | **不报错，保存时静默丢弃**（前端 trim+filter 后再提交） |
| commitments 为空 | 拦截 |

### 超额软警告
- 当 `当前 allocation × 60 < spent`（已 logged 超出计划）→ 进度条转 **amber 满格** + amber 小字 `over by Xh Ym`
- **软提示，不拦截保存**（amber gradient↔solid 切换不 tween，仅宽度动画）

### 空状态
- 本月无 commitments 时，侧栏显示「Set up commitments」入口（取代当前「无 commitments 即隐藏 Edit」）
- 打开 modal 显示引导空态：`No commitments yet for {Month Year}` + `+ Add Role`

### 取消 / 放弃
- 无改动 → 直接关闭；有改动 → 弹 discard 确认（`Discard changes?  Keep editing / Discard`）

## 后端改动（`set_commitments` / `validate_commitments`）

⚠ 以下为**需确认的后端语义变更**，列入「开放决策」。

1. **role 名唯一**：`validate_commitments` 新增——重复 role 名拒绝。
2. **goal 全局唯一**：现有逻辑仅查同 role 内唯一（`goal_set` per role）。改为**跨所有 role 全局唯一**。这同时是正确性修正——`get_commitment_progress` 的 `goal_to_role` 映射要求 goal 名全局唯一，SPEC.md §153 亦载明此约束。
3. **空 goal**：前端提交前已 trim + 过滤空 goal，后端「goal 非空」校验保留作为兜底（正常不触发）。
4. **删除有 entry 的 goal**（**重大变更，需拍板**）：现状是 `count_entries_with_goal > 0` 即**硬拒绝**。新 UX 是「确认后允许移除」。
   - 推荐方案：放宽硬拒绝，确认删除后 goal 从计划移除，引用它的 entry **保留 `dimensions.goal` 文本但变为未归属**（不计入任何 role）。
   - 待验证：orphaned goal 值是否会被 `validate_monthly` 标记、是否影响 goal picker / required-dimension 逻辑。若有副作用，备选：(a) 仍拒绝（放弃此 UX）；(b) 删除时一并剥离引用 entry 的 goal tag（有损）。
5. **goal 改名 vs 重排**：`detect_goal_changes` 的 deleted 用全局 set-diff（顺序无关，安全）；renames 是 per-role 位置启发式。需验证拖拽重排（role 内 goal 顺序变、role 顺序变）不会触发误判 rename。

## 依赖

拖拽排序需要一个 sortable 能力。当前无相关依赖（仅 `@tauri-apps/plugin-dialog`）。决策：
- **方案 A（推荐）**：引入轻量库（如 `vuedraggable@next` / Sortable.js），DnD + 触摸 + 动画成熟，省去自研 a11y/FLIP 成本。
- **方案 B**：基于 Pointer Events 手写。无新依赖，但键盘可达性、触摸、重排动画需自理。

## 测试策略

### Rust
- `validate_commitments`：新增 role 名重复、goal 跨 role 重复用例；保留空值/空数组用例
- 删除有 entry 的 goal：按拍板结果调整现有「拒绝」测试（→ 允许 / 剥离 / 仍拒绝）
- `detect_goal_changes`：重排不误判 rename 的回归用例

### 前端（Vitest + vue-test-utils）
- 展示态 → 打开 modal；空状态入口
- 增删 role/goal、stepper（±5 / 最小 5 / 直接输入）、role/goal 重排
- 校验拦截：空 role、role 重复、goal 重复；空 goal 静默丢弃
- 删除确认：role；有/无 logged 的 goal
- 超额 amber 切换（alloc < spent）
- 进度条随 allocation 实时变化
- 键盘映射、discard 确认、保存 loading/disabled、后端错误展示

## 文档更新

- `SPEC.md`：修正「不提供 `set_commitments` 命令、commitments 通过直接编辑 `_monthly.md` 写入」的过时描述（命令已存在且为主要写入路径）。同步校验规则（role/goal 唯一性）。
- `src-tauri/CLAUDE.md`：同步「Commitments 不在 Rust 端写入」这条过时约定。

## 开放决策清单（需你拍板）

1. **删除有 logged 的 goal**：采用推荐方案（允许 + 未归属）还是备选 (a)/(b)？
2. **拖拽依赖**：引入 `vuedraggable` 还是手写？
3. 其余（modal 660px、step 5h、role/goal 全局唯一、空 goal 丢弃、空状态入口文案）已在 brainstorm 确认，如无异议即采纳。
