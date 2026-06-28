# 维度编辑器 — GUI + CLI

**日期：** 2026-06-28
**状态：** design（待实现计划）

## 问题

维度及其值目前只能通过手改 YAML 文件（`template.yaml`、`_monthly.md`）来管理。用户必须知道文件位置、格式和校验规则。这阻碍了新用户上手。

## 范围

为维度和其静态值提供 GUI 和 CLI 的创建、编辑、排序、删除能力。编辑始终面向 MonthView 当前查看的月份；另有单独操作可将编辑器内存态推广到模板。

## 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 入口 | Composer 输入行右侧的 ⚙ 图标 | 稳定可见、不与 sidebar 竞争、维度使用场景就近 |
| 编辑目标 | 当前月（写入 `_monthly.md`）；另有「保存为模板」操作 | 先试用再推广；避免月/模板切换的认知负担 |
| Key | 创建时指定，创建后锁定 | 避免改名时的数据迁移 |
| Source（`static` / `monthly`） | 创建时选定，创建后锁定 | 防止多月度源冲突 |
| 删除维度 | 软删除：标记 `deleted: true`；历史 entry 保留原始颜色和标签 | 可撤销、历史可读、无数据丢失 |
| 排序 | 左栏维度列表拖拽排序 | 控制 popover 显示顺序 |
| CLI | `dimensions get/set` | 对齐 `commitments set` 模式 |
| 种子数据 | 默认 template 包含 Goal 维度（`source: monthly`） | 新用户无需文档即可自然发现维度概念 |

## GUI

### 入口

在 `EntryComposer.vue` 的输入行内，Enter 徽章右侧：

```
[+] [What did you work on?         ] [Enter] [⚙]
```

⚙ 图标样式：`text-[var(--color-text-muted)]`（与 placeholder 同色），hover 变为 brand 色。始终可见，不随输入状态变化。

### Modal 布局

点击 ⚙ 打开。Teleport 到 `<body>`，居中 overlay（`fixed inset-0 z-50 bg-black/30`）。

对齐 `CommitmentsModal` 骨架。对话框：`w-[660px]`，`rounded-[var(--radius-lg)]`（14px），`shadow-[var(--shadow-popover)]`。

```
┌──────────────────────────────────────────────────────────────────┐
│ Edit Dimensions                                             [×]  │
│ Editing June 2026              [Save as template]                │
├───────────────┬──────────────────────────────────────────────────┤
│ (surface-     │                                                  │
│  muted bg)    │  Name: [Biz_____________]                        │
│               │  key: biz (locked)  Source: STATIC (locked)      │
│  ▎ Goal       │  ☑ Required                                     │
│  ▎ Biz     ←  │                                                  │
│  ▎ Importance │  VALUES                                          │
│               │  ⠿ [Product_____________] [×]                   │
│               │  ⠿ [Marketing___________] [×]                   │
│               │  ⠿ [Engineering_________] [×]                   │
│               │  [New value____________] [+]                     │
│               │                                                  │
│               │                                   [Delete dim]   │
│  ──────────── │                                                  │
│  + Add dim    │                                                  │
│               │                                                  │
├───────────────┴──────────────────────────────────────────────────┤
│                                          [Cancel]  [Save]        │
└──────────────────────────────────────────────────────────────────┘
```

#### 选中 monthly 维度时右栏变体

选中 Goal（`source: monthly`）时，右栏不显示 Values 列表，替换为提示信息：

```
│  Name: [Goal______________]                     │
│  key: goal (locked)  Source: MONTHLY (locked)   │
│  ☐ Required                                     │
│                                                  │
│  Values are derived from commitment goals.       │
│  Edit commitments to change available values.    │
│                                                  │
│                                   [Delete dim]   │
```

#### Header

- 标题：「Edit Dimensions」，`text-title font-bold tracking-[-0.3px]`（20px）
- 副标题：「Editing `<月份年份>`」+ 右侧「Save as template」链接（`text-[var(--color-brand-link)]`、`font-medium`）
- 「Save as template」点击立即将编辑器内存态写入 `template.yaml`。无需确认——模板只是默认值，不回溯影响已实例化的月份
- 关闭按钮：×，`text-[var(--color-text-muted)]`
- 内边距：`px-2xl pt-xl pb-lg`（32px / 24px / 16px），底部 `border-b border-[var(--color-divider)]`

#### 左栏（210px）

- 背景：`var(--color-surface-muted)`
- 维度列表，每行：色条（3px × 16px，颜色引用 `--dim-bar-*`）、名称（`text-body`、14px）、source 徽章（`text-micro`）
- 选中行：`bg-[var(--color-brand-soft-bg)]`、`rounded-[var(--radius-form-lg)]`（8px）
- 拖拽排序：每行有拖拽手柄（⠿ 字符），使用 `vue-draggable-plus`（与 CommitmentsModal 一致）
- 底部：「+ Add dimension」按钮，样式对齐 CommitmentsModal 的「+ Add Role」——无背景、无边框、`text-secondary font-semibold text-[var(--color-brand-link)]`
- 「Show deleted」开关（仅当存在已删除维度时显示）：`text-micro text-[var(--color-text-muted)]`，默认关闭。开启后已删除维度以 `opacity-40` 显示在列表末尾（不参与拖拽），选中后右栏显示只读详情 +「Restore」按钮

#### 右栏（flex-1）

- 内边距：`px-2xl py-xl`（32px / 24px）
- **Name 输入**：`text-title font-bold`（20px），底部 `border-bottom` 分隔线，可编辑
- **Key 显示**：只读、等宽字体、`text-secondary`，附带 `(locked)` 标记
- **Source 显示**：只读徽章，`(locked)` 标记
- **Required 复选框**：`accent-color: var(--color-brand-solid)`
- **Values 区域**（仅 `source: static` 时显示）：
  - 每行：拖拽手柄 + 文本输入框（`text-body`、`rounded-[var(--radius-form)]`、`border-[var(--color-border-form)]`）+ × 删除按钮
  - 底部：虚线边框的「New value」占位输入 + 「+」按钮
  - 值之间支持拖拽排序
- **monthly 维度说明**（仅 `source: monthly` 时显示，替代 Values 区域）：
  - 信息卡：`bg-[var(--color-page-bg)]`、`rounded-[var(--radius-form-lg)]`，内容为「Values are derived from commitment goals. Edit commitments to change available values.」
- **删除维度按钮**：底部、左侧，危险样式（`text-[var(--color-danger)]`、`border-[#fecaca]`、`rounded-[var(--radius-form)]`）
  - 点击后将维度标记为 `deleted: true`，按钮变为「Undo delete」
  - 已删除维度在左栏列表中降低透明度（`opacity-40`），不在 DimensionPopover 的可选维度中显示
  - 若该维度已被 entry 使用：Toast 提示「Biz deleted — N entries keep their values」（复用已有 Toast + Undo 模式）
  - Undo 后恢复为正常状态，无需重新填写任何字段

#### Footer

- 右对齐，`flex justify-end gap-sm`
- 内边距：`px-2xl py-lg`（32px / 16px），顶部 `border-t border-[var(--color-divider)]`
- **Cancel**：`text-secondary font-semibold text-[var(--color-text-muted)]`、`rounded-[var(--radius-form)]`、`px-md py-sm`、无背景、hover 变深
- **Save**：`text-secondary font-semibold text-white bg-[var(--color-brand-solid)]`、`rounded-[var(--radius-form)]`、`px-md py-sm`、hover 变 `var(--color-brand-link)`、disabled 时 `opacity-50`

### 新增维度流程

点击「+ Add dimension」在左栏底部插入内联表单：

```
┌─────────────────────┐
│ Name: [_________]   │
│ Key:  [_________]   │  ← 用户输入；失焦时校验
│ Source: [static ▾]  │  ← 下拉选择（static / monthly）
│                     │
│ [Cancel]  [Create]  │
└─────────────────────┘
```

- 表单样式：`border border-[var(--color-brand-solid)]`、`rounded-[var(--radius-form-lg)]`、`bg-[var(--color-brand-soft-bg)]`
- Key 输入：失焦时校验——仅允许字母数字 + 连字符 + 下划线、不可重复、不可为空
- Source 默认为 `static`
- 若选择 `monthly` 且已有另一个 monthly 维度：内联错误提示「Only one monthly-source dimension is allowed.」
- 创建后：维度出现在列表中、自动选中、右栏显示其详情编辑器
- Cancel / Create 按钮：small、font-semibold；Cancel = `text-muted` 无背景，Create = `bg-brand-solid white`

### 保存

- 校验所有维度（复用 `validate_dimensions` 规则）
- 校验通过：写入当前月份 `_monthly.md`。若月份未实例化，先调用 `ensure_month_instantiated`（保留已有 commitments）
- 文件监听器自动感知变更；前端 store 通过已有 `config-changed` / `commitments-changed` 事件更新
- 校验失败：高亮左侧列表中出错的维度；右栏底部或 footer 上方显示错误信息（`text-secondary text-[var(--color-danger)]`，对齐 CommitmentsModal 模式）

### 「保存为模板」

- 位于 modal header 右侧，维度有效时始终可用
- 写入编辑器内存态（非持久化的文件态）到 `template.yaml`。不要求先 Save——可以编辑、保存为模板、然后 Cancel 不影响当月
- Toast 确认：「Dimensions saved to template」（复用已有 Toast 组件）
- 不回溯影响已实例化的月份

### 丢弃确认

若用户有关闭操作（×、Cancel、Escape、点击 overlay 背景）且有未保存改动：

- 显示内联确认条：「Discard changes?」+「Keep editing」（brand solid）+「Discard」（danger text）
- 复用 CommitmentsModal 的 discard overlay 模式：`bg-black/10` 遮罩 + 居中白色卡片 + `shadow-[var(--shadow-toast)]`
- 无改动时：直接关闭，不打扰

### 键盘

- `Escape`：关闭 modal（有改动时先弹丢弃确认）
- `⌘Enter` / `Ctrl+Enter`：保存
- 打开时焦点移入 modal（`tabindex="-1"` + `el.focus()`），确保 Escape/Enter 能被 dialog 级 `@keydown` 捕获（对齐 CommitmentsModal 第 42-48 行）

### 消解（Dismissal）

遵循 `docs/interaction-principles.md` 原则 2：

- 点击 overlay 背景：document 级 `mousedown`（capture 阶段）监听，仅在打开时挂、关闭/卸载时移除；内部点击不自关
- Escape：window capture 级 `keydown` 监听 + dialog 元素 `@keydown.esc` 双保险
- 失焦：document `focusin`，目标落在 modal 之外即消解

### 拖拽排序

- 使用 `vue-draggable-plus`（与 CommitmentsModal 一致）
- 左栏：`.drag-grip-dim` handle 类
- Values 列表：`.drag-grip-val` handle 类
- 拖拽中 ghost 元素 `opacity-40`（复用已有 `.sortable-ghost` 样式）

## CLI

### `logbook-cli dimensions get`

```
logbook-cli dimensions get [--year Y] [--month M] [--template]
```

- 无参数：返回当前月有效维度（已实例化则取月快照，未实例化则取模板）
- `--template`：直接返回模板维度
- 输出：默认 YAML，`--json` 输出 JSON
- 成功 exit 0，错误 exit 1（文件缺失、解析错误等）

### `logbook-cli dimensions set`

```
logbook-cli dimensions set [--year Y] [--month M] [--template]
```

- 从 stdin 读取 YAML 或 JSON
- `--template`：写入 `template.yaml`。无标记：写入 `_monthly.md`
- 写入前校验（规则同 GUI）
- 成功：原子写入（tmp + rename），无输出，exit 0
- 校验失败：错误信息输出到 stderr，exit 1
- 整量替换维度数组（非部分更新——对齐 `commitments set`）

## 数据流

```
GUI 打开编辑器 ──→ resolve_month_dimensions() ──→ 读取 _monthly.md 或 template.yaml
GUI Save ──→ _monthly.md（当前月）
GUI "Save as template" ──→ template.yaml
CLI set ──→ _monthly.md 或 template.yaml
CLI get ←── resolve_month_dimensions() ←── _monthly.md 或 template.yaml

文件监听器（已有）：
  template.yaml 变更 ──→ 重校验、emit dimensions-changed（现名 config-changed，实现时改名）
  _monthly.md 变更 ──→ 重校验、emit commitments-changed
  → 前端 store 响应式更新维度数据
```

### 月份实例化

若保存时月份尚未实例化（`_monthly.md` 无 dimensions 块）：
- 调用 `ensure_month_instantiated`（已有函数）
- 保存的维度成为该月的快照
- 后续模板变更不影响该月

### 软删除的维度

`Dimension` 结构体新增 `#[serde(default)] deleted: bool` 字段。已有文件中缺失该字段的维度自动视为 `deleted: false`（向后兼容）。

当维度被标记为 `deleted: true`：
- **新 entry 不可选**：`DimensionPopover` 过滤掉已删除维度
- **历史 entry 保持原样**：`EntryRow.vue` 正常渲染 chip，颜色和标签均保留（因为该维度 key 仍在配置中，只是 `deleted: true`）
- **左栏可见可恢复**：编辑器左栏底部「Show deleted」开关（默认关闭）。开启后已删除维度以 `opacity-40` 显示，选中后右侧显示只读详情 +「Restore」按钮
- **排序**：已删除维度排在列表末尾，不参与拖拽排序
- **模板推广**：软删除状态随「Save as template」写入 `template.yaml`

## 校验规则（复用 `config.rs::validate_dimensions`）

- `name`：非空
- `key`：非空、仅 `[a-zA-Z0-9_-]`、所有维度间唯一
- `source`：`"static"` 或 `"monthly"`
- 若 `source: "static"`：`values` 必须存在且非空
- 最多一个维度 `source: "monthly"`

GUI 层额外校验：
- key 在**非已删除**维度中唯一（软删除的 key 可被新维度复用）
- 同一维度内无重复 value 名称

## 边界情况

| 场景 | 处理 |
|------|------|
| 软删除 source=monthly 的维度 | 软删除后 popover 不显示该维度，`monthly_dim_key` 仍能解析到它（deleted 不影响 resolution），commitment progress 正常。Restore 后恢复 |
| 重命名某个已被 entry 使用的 value | 旧 entry 保留旧值字符串，不迁移。用户可通过 CLI 或手改文件进行查找替换 |
| 保存为模板时模板文件缺失 | 创建 `template.yaml` 写入维度（「Save as template」为幂等创建或更新） |
| 月份未实例化时保存维度 | 先实例化月份 + 保存维度 |
| 编辑器打开期间其他进程修改 `_monthly.md` | 文件监听器触发 `commitments-changed`。Modal 显示「文件已在外部变更」提示并禁用保存直到重新加载。复杂度高则 v1 跳过——文件监听器为亚秒级，单用户桌面场景竞争窗口极小 |
| 创建时 key 与已删除维度重复 | 允许创建——软删除的 key 可被新维度复用。新维度创建后，历史 entry 中该 key 的值会关联到新维度（key 相同），颜色和标签采用新维度定义 |
| 创建时 key 与活跃维度重复 | 失焦时内联校验错误：「Key 'biz' already exists.」 |
| static 维度的 values 列表为空 | 保存时校验错误：「Dimension 'Biz' has no values.」 |

## 不纳入范围

- Value 重命名并迁移 entry 数据
- 新用户 onboarding 流程（等多用户时再做）
- 编辑已实例化的历史月份维度（仅支持编辑当前查看的月份；历史月份可通过 CLI 编辑）
- 超出 CLI `get/set` 的维度配置导入/导出
