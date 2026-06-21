# 设计：切月 commitments 跟随选中月（修复「假继承 + 写错月」bug）

- 日期：2026-06-21
- 范围：**只修 bug**（继承上月作为草稿模板是独立增强，不在本设计内）
- 状态：已通过 brainstorming 评审，待实现
- 关联：HANDOFF.md（2026-06-21，根因诊断）

## 1. 问题

创建当前月（如 6 月）的 commitments 后，切到一个没有 commitments 的月（如 7 月）打开 Edit commitments，弹层显示 6 月的 role/goal（看似「继承」）。在该弹层保存，会把 6 月数据真实写进 `2026/07/_monthly.md`——**数据正确性 bug，非显示问题**。

### 根因

`store.commitments` 是全局单槽，语义为「应用启动时的真实日历月」，**切月时从不刷新**。

- `MonthView.loadMonth`（切月主路径）刷新了 `monthEntries`、`commitmentProgress`、`today`，**唯独漏了 `store.commitments`**。
- `CommitmentsModal.buildDraft()` 从 `props.commitments`（一路接到 `store.commitments`）种出草稿；保存用 `set_commitments({ year: selectedYear, month: selectedMonth, commitments: toCommitments(draft) })`——种子是错月数据，写入目标是选中月，于是写错月。

后端无责：`get_commitments(root_path, year, month)` 经 `files::read_monthly_file` 按月读取，月文件缺失返回 `vec![]`，**无任何继承逻辑**。bug 100% 在前端加载时机。

## 2. 决策：方案 A —— `store.commitments` 跟随选中月

`store.commitments` 的语义从「应用启动时的真实日历月」改为「当前选中月」，与 `store.commitmentProgress` **同生命周期**：同处加载、同处刷新、永远指向同一个月。

**核心不变量（invariant）**：凡是按选中月刷新 `commitmentProgress` 的地方，都必须并排刷新 `commitments`。

### 为什么是 A，而非「隔离独立状态」

`store.commitments` 有三个消费方，**全部应跟随选中月**：

| 消费方 | 入口 | 期望行为 |
|---|---|---|
| CommitmentsModal 草稿种子 | `CommitmentsModal.buildDraft` ← Panel prop ← `MonthView` `:commitments="store.commitments"` | 编辑 7 月就显示 7 月（空） |
| CommitmentsPanel 的 Set up/Edit 门控 | `hasCommitments` ← prop | 与进度条一致（进度条已按选中月刷新） |
| EntryRow 编辑态的 goal 候选 | `EntryRow.vue` 直接读 `store.commitments`，对任意选中日生效 | 回到 6 月某天改 entry，候选是 6 月的 goal |

EntryComposer 仅在 `isSelectedToday` 渲染，selected 永远等于 current，不受影响。

考察过的替代方案：

- **方案 B（保留 `store.commitments`=当前月 + 另开 `selectedCommitments`）**：EntryRow 直接读 `store.commitments`，要修它就得改读新字段；改完后 `store.commitments`=当前月只剩 EntryComposer 一个消费方（而它 selected==current）——该字段沦为冗余。B 实质塌缩成 A，只多养一个状态和一处同步。否决。
- **方案 C（懒加载，Modal 打开时现拉）**：Modal 爆炸半径最小，但 EntryRow 仍坏、Panel 门控仍 stale，且重复一份拉取逻辑。只修了三个症状里的一个。否决。

判据：`EntryRow` 直接耦合到全局 `store.commitments`，任何不统一这个单一数据源的方案都得在多处打补丁。修数据源 > 修每个消费方。

## 3. 改动点

后端零改动。前端三处写入路径：

| # | 位置 | 现状 | 改为 |
|---|---|---|---|
| 1 | `MonthView.loadMonth`（切月主路径） | 只刷 `commitmentProgress` | 并排 `invoke("get_commitments", { rootPath, year, month })` → `store.commitments`（核心改动） |
| 2 | `App.initApp`（`App.vue` `Ready` 分支） | 强写**当前月** commitments 到 `store.commitments` | **不再写 `store.commitments`**；启动加载交给 `MonthView.onMounted` 的 `loadMonth` 统一负责 |
| 3 | `commitments-changed` 文件事件（`App.vue` 监听器） | 触发 `initApp` → 强读当前月 → 冲掉选中月 | 改为重读**选中月**的 commitments + progress（轻量 reload，不走整个 `initApp`） |
| 4 | `SetupScreen.trySetRootPath`（`SetupScreen.vue` `Ready` 分支） | 写**当前月** commitments 到 `store.commitments` | **不再写 `store.commitments`**；setup 后 `MonthView.onMounted` 的 `loadMonth` 统一加载（与改动 2 同理） |

> 改动 4 是范围扩展（2026-06-21 评审后追加）。它本身不触发本 bug——setup 时选中月恒等于当前月，故 SetupScreen 写的数据与 `loadMonth` 一致。删除它纯为收紧不变量（见下），代价仅是 setup 后到 `loadMonth` 完成的几帧里 commitments 短暂为空（与 `monthEntries`/`commitmentProgress` 既有的加载行为一致）。

`onCommitmentsSaved`（`MonthView` 乐观回填）保持不变：它写的就是刚保存的选中月数据，语义已正确。

### 不变量（修复后）

`store.commitments` 只由三处写入，且全部指向**选中月**：`MonthView.loadMonth`（→`loadCommitments`）、`MonthView.onCommitmentsSaved`、`App.vue` 的 `commitments-changed` 监听器。`initApp` 与 `SetupScreen` 都不再写它——「选中月级」状态的所有权集中在 `loadMonth` 一侧。

### 改动 2/3 的职责边界

`initApp` 负责「应用级」状态（`rootPath`/`config`/`today`）；「选中月」状态（`monthEntries`/`commitmentProgress`/`commitments`）统一归 `loadMonth` 管。当前 bug 的本质是 commitments 错误地挂在了 app 级初始化上，而它其实是月级状态。

### 改动 3 的跨层细节

`commitments-changed` 监听器位于 `App.vue`，而「选中月」是 `MonthView` 的 computed。`App.vue` 不直接持有它，但 `store.currentDate` 是全局的——监听器用 `yearMonthFromDate(store.currentDate)` 取选中月即可，无需把状态上提。

`commitments-changed` 仅在 `_monthly.md` 变化时触发，无需刷新 `config`/`today`；narrowing 到「选中月 commitments + progress」既正确又更轻。`set_commitments` 保存后后端会发该事件，因此这是热路径——不修等于没修。

> 实现期可选的工厂化（plan 决定）：把「按选中月重读 commitments + progress」抽成一个可复用函数，供 `loadMonth` 与 `commitments-changed` 监听器共用，避免重复 ~4 行。本设计只规定行为，不强制具体抽法。

## 4. 错误处理

沿用 `loadCommitmentProgress` 既有范式：`try/catch` + `logError(...)`，失败时 `store.commitments = []`（与后端「月文件缺失返回 `vec![]`」语义一致，不抛、不卡 UI）。

状态守卫：`commitments-changed` 的轻量 reload 仅在 `store.status === "ready"` 时执行（镜像现有 focus 处理器的守卫），避免 setup/error 态下的无效加载。

## 5. 测试与验收

项目规则：先写测试复现 bug（红），再改（绿）。

### 新增前端测试（vitest）

1. **种子按选中月**（核心复现）：mock `get_commitments` 按月返回不同数据（当前月=有 role/goal，目标月=`[]`）；挂载 → 切到目标月 → 打开 Modal → 断言 `draft` 种子为空，而非当前月数据。**改动前此测试应失败。**
2. **`commitments-changed` 不冲掉选中月**：停在非当前月 → 触发 `commitments-changed` 事件 → 断言 `store.commitments` 仍是选中月，未被冲回当前月。（覆盖改动 3 这条热路径，防回归）

测试要覆盖的是「每条写入路径」，而非单一症状——测试 2 专门针对易漏的改动 3（保存后必经的文件事件回路）。

### 回归基线（HANDOFF 实测，改完须仍全绿）

| 检查项 | 基线 |
|---|---|
| `pnpm test`（vitest） | 26 files / 296 tests pass |
| `cargo test`（src-tauri） | 153 pass（后端不动，应零变化） |
| `pnpm run build` | 绿（vue-tsc 严格类型检查测试文件，noUnusedLocals；vitest 绿 ≠ build 绿） |

## 6. 非目标（明确排除）

- **真·继承上月**：用户原始诉求是「继承上月作为新月草稿模板」。当前是「假继承 + 写错月」，本设计修复后各月正确显示各自数据（新月=空）。继承是独立增强，单独评估，不与本 bug 混做。
- **后端改动**：`get_commitments` / `read_monthly_file` 行为已正确，不动。
- **HANDOFF 记录的文档漂移**（SPEC 前端组件树过时等）：与本 bug 正交，独立任务。
