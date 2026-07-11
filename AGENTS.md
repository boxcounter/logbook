# AGENTS.md

Logbook — 个人工作时间记录工具。Tauri 2.x + Vue 3 + TypeScript。

**设计文档**：Vault `1_Projects/Logbook/README.md`（产品设计中心，不做代码实现）
**后端约定**：`src-tauri/CLAUDE.md`（Rust 测试规范、模块结构）
**交互原则**：`docs/interaction-principles.md`（不丢输入、消解一致性、快捷键按频率分配、尊重输入上下文——治理所有前端交互）
**命名约定**：`docs/naming-conventions.md`（组件按职责命名、DTO 用 `*Input`、落盘格式与标识符解耦——治理命名）

## 命令字典

用户指令 → 具体命令的直映射。**不猜测，查到即执行。** 新增命令时同步更新此表。

| 用户说 | 执行命令 | 产物 |
|--------|---------|------|
| 打包正式版 / 生产版本 / production build | `pnpm tauri:build` | `Logbook.app`（`com.boxcounter.logbook`），含 CLI |
| 打包开发版 / dev build | `pnpm tauri:build:dev` | `Logbook Dev.app`（`com.boxcounter.logbook.dev`） |
| 启动 / run / dev | `pnpm tauri dev` | 开发模式热重载 |
| 测试 / test | `pnpm test`（前端 vitest）+ `cd src-tauri && cargo test`（后端） | `pnpm test` 仅跑 vitest；后端测试须另跑 cargo test（Stop hook 会两者都跑） |

## 前端架构

### 组件树（Vue 3 SFC + `<script setup>`）

```
App.vue
├── SetupScreen.vue                     // 首次启动，folder picker
├── ConfigErrorBanner.vue               // 启动时 config 错误提示
├── RecoveryScreen.vue                  // ConfigError/root_missing 恢复界面
├── DataVersionScreen.vue               // 数据版本缺失/不匹配（DataVersionNotFound / DataVersionMismatch）恢复界面
└── MonthView.vue                       // 固定月视图
    ├── HeatmapCalendar.vue             // 月历热力图 + 切换月份
    ├── QuickJumpPopover.vue            // 年/月快速跳转双下拉（基于 get_available_months）
    ├── CommitmentsPanel.vue            // Allocation / Spent / Balance 进度条
    ├── IntegrityBanner.vue             // 运行时数据完整性告警（integrity-changed 事件驱动）
    ├── DayHeader.vue                   // 日期头部，显示当天 entry 合计
    ├── EntryComposer.vue               // 快速录入（内嵌 DimensionPopover）
    │   └── DimensionPopover.vue        // 维度选择 popover（dim 阶段 / val 阶段）
    └── EntryList.vue
        └── composite/EntryRow.vue      // 条目行（只读）
            └── composite/EntryRowEdit.vue  // 条目编辑（复用 DimensionPopover）

// base/ 基础组件：
//   Toast.vue
// composite/ 复合组件：
//   CommitmentsModal.vue, DimensionEditorModal.vue
```

### 状态管理

`reactive()` + `provide/inject`。根组件创建 reactive store，`provide()` 注入子组件树。

### 前端模块

| 路径 | 说明 |
|------|------|
| `src/types.ts` | 前端 TypeScript 类型定义（与 Rust models 对应 + UI 专用类型） |
| `src/stores/useStore.ts` | Reactive store（`reactive()` + `provide/inject`） |
| `src/composables/useRootFolderPicker.ts` | 文件夹选择逻辑，SetupScreen / RecoveryScreen 复用 |
| `src/composables/useMonthData.ts` | 月数据加载（entries / commitments / dimensions / day note / 导航） |
| `src/composables/useEntryActions.ts` | Entry CRUD（提交 / 更新 / 删除 + undo / 高亮） |
| `src/composables/useDayNote.ts` | Day note 内联编辑（保存 / esc 还原 / IME 安全） |
| `src/composables/useClickOutside.ts` | 点击别处消解（实现交互原则 §2） |
| `src/utils/dates.ts` | 日期工具函数 |
| `src/utils/format.ts` | 格式化函数 |
| `src/utils/commitments.ts` | Commitments 计算/聚合 |
| `src/utils/errorLog.ts` | 前端错误日志上报 |
| `src/utils/heatmap.ts` | 热力图数据生成 |
| `src/utils/dimensionColor.ts` | 维度颜色辅助 |
| `src/utils/applyInitResult.ts` | init 结果应用到 store 的逻辑 |
| `src/utils/constants.ts` | 时长常量（HIGHLIGHT_DURATION / SAVED_TOAST_DURATION / UNDO_DELETE_DELAY） |
| `src/utils/ime.ts` | IME 组合状态辅助（`isIMEEvent`，回车选词守卫） |
| `src/__tests__/` | 前端单元测试（vitest + jsdom），`mocks/` 下含 store / tauri / fixtures 桩 |

### 特殊处理

- Goal 维度：值列表不从 template/月度维度块取，从 Rust 端 `get_commitments` 返回的 goals 并集构建
- CommitmentsPanel：始终可见（录入框上方）
- DimensionPopover 键盘导航：`CTRL+N`/`CTRL+P` 或 `↑`/`↓` 移动高亮（循环），默认高亮第一个还没填 value 的维度（从 val 阶段返回时高亮下一个未填项）。popover 开启时 `Enter` 改为「选中当前高亮项」（dim 阶段进入值菜单 / val 阶段填值），不再提交 entry / 保存编辑；按 `Esc` 关闭 popover 后 `Enter` 恢复提交。`EntryComposer` 与 `EntryRowEdit` 复用同一 popover，行为一致。

## 数据流

- **启动**: App mount → `invoke('init')`。Rust 端读 `root_path.txt`：
  - 无文件 → 返回 `NeedsSetup` → 前端弹文件夹选择器 → `set_root_path` → 重新 init
  - 数据版本缺失/不匹配 → 返回 `DataVersionNotFound { root_path }` / `DataVersionMismatch { root_path, expected, found }` → 前端渲染 DataVersionScreen
  - dimensions.template.yaml / dimensions.yaml / commitments.yaml 有错 → 返回 `ConfigError { category, root_path, errors, scan_warnings }` → 前端按 `category` 决定 RecoveryScreen 路由
  - 正常 → 返回 `Ready { root_path, dimensions, usingDefaultDimensions, today, commitments, scan_warnings, integrity_issues }`（dimensions = 当前月生效维度）→ 渲染 Today
- **文件监听**: Tauri `setup` hook 中启动 `notify` 线程，watch `dimensions.template.yaml` + 当月 `dimensions.yaml` + `commitments.yaml` + 当月 day yaml。变更时重新校验并复查完整性，emit `dimensions-changed` / `commitments-changed` / `integrity-changed` / `day-file-changed` 事件推前端。（「Copy User Data Path」菜单另 emit `copy-data-path-event`，非文件监听）
- **录入**: 用户输入 → 前端扫描全文 duration（regex 求和） → 去除匹配片段得到 item → 添加继承的维度 → `invoke('append_entry', ...)` → Rust 解析 duration 字符串为 u32，写文件 → 返回 Entry → 前端 refresh 列表 + Commitments
- MonthView 通过逐个调用 `get_entries` 加载月份数据。

## 项目级规则

### 文档一致性检查

何时该做：写 HANDOFF.md 之前、Phase 结束时，应完成一次文档一致性检查。

`/check-consistency` skill 已设为仅手动调用（其 frontmatter 含 `disable-model-invocation: true`），不会被自动触发。因此在上述时机，主动提醒用户运行 `/check-consistency`，由用户显式发起；用户说「检查一致性」/「文档同步」时同样运行。检查项目（文档 ↔ 文档 + 文档 ↔ 代码）已固化在 skill 定义中，不在此重复。

### 前端交互

新增或修改弹层、编辑器、输入控件、焦点/键盘/取消行为时，遵循 `docs/interaction-principles.md`，不逐组件另起一套。评审时按该文件逐条核对。

### 设计 token

间距、字号必须用语义 token：间距走 `--spacing-*` 命名档（`gap-sm`/`p-md`，禁止裸 px 与 Tailwind 数字默认档如 `p-4`）；字号走 `text-title/body/secondary/micro`（默认 `text-sm` 等已用 `--text-*: initial` 清除，不可用）；行高跟随字号档（`@theme` 的 `--text-<tier>--line-height`），元素继承档行高，需紧排时用 `leading-none`（唯一合法显式覆盖），禁止散装 `leading-[...]`/`leading-<number>`/`leading-tight` 等。组件尺寸（`w-`/`h-`/`min-`/`max-`）不在此约束内，可用任意 px。新增或调整阶梯走 PR 说明理由；破例需一行注释 + 显式豁免 + 人工签字。`src/__tests__/tailwind-token-usage.test.ts` 是可执行护栏（报错含合法替代），接入 `npm run verify` + pre-commit + CI。详见 `docs/superpowers/specs/2026-06-21-design-system-consolidation-design.md` §2–3。

### 其他

- 诊断先于计划：handoff 标记了 bug → 先写测试确认 bug 还存在 → 再计划修复
