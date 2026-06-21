# 设计：按可恢复性重构错误/恢复屏幕

- **日期**：2026-06-22
- **状态**：设计已批准，待评审
- **触发**：手动删除数据目录后重启，应用进入「Configuration Errors + 仅 Retry」的死胡同。

## 1. 问题

删除数据目录后重启，应用显示 `ConfigReadError: Failed to read .../config.yaml: No such file or directory`，下方只有一个 Retry 按钮。这个设计有三处缺陷：

1. **错误分类缺失**。`init`（`commands.rs:161-173`）把 `read_config` 的任何失败——目录整个不存在、目录在但 `config.yaml` 被删、文件存在但格式非法——统一包成 `ConfigReadError`。三种失败的恢复方式完全不同，却共用一个死胡同屏幕。
2. **Retry 是错误的 affordance**。Retry 只是重跑 `init`。对「文件真的没了」这种非瞬态失败，两次尝试之间什么都没变，结果必然相同，把用户送进循环。
3. **「Changes are detected automatically」是谎话**。文件 watcher 仅在 `lib.rs:45-48` 启动且要求 `root_path.exists()`。目录被删时 watcher 从不启动；且 `set_root_path`（`commands.rs:240-322`）成功后也从不启动 watcher，导致首次 setup 后到下次重启前自动检测一直失效（既有 latent bug）。

数据目录位于 iCloud 路径（`~/Library/Mobile Documents/com~apple~CloudDocs/Logbook`），放大了风险：「目录不存在」可能是 iCloud 尚未同步 / 未登录 / 盘未挂载，而非用户删除。此时任何「自动重建 config」都可能在真实数据回来前凭空造文件，制造 iCloud 冲突副本或掩盖真正问题。

## 2. 目标与范围

**目标**：按「可恢复性」把错误态分成三层，每层给出正确的恢复操作；破坏性操作默认悲观。

**范围内**：
- 后端 `init` 对错误分类。
- 新的恢复屏幕组件，按分类渲染操作。
- 复用 `create_starter_files` 做安全重建。
- 新增 `reveal_config_file` 命令。
- 修复 watcher 生命周期，使「自动检测」在所有路径上成立。

**范围外**：first-run setup 流程的整体重做、root_path 持久化策略的重新设计。

## 3. 错误三层模型

| Tier | 判定 | 含义 | watcher | 主操作 | 次操作 |
|---|---|---|---|---|---|
| **1 in_place** | `root.exists()` 且 `read_config` 成功，但 validate/monthly/day 文件有错 | 内容非法，可在原地改文件修复 | 有效 | 错误清单 + Reveal in Finder | —（存盘即自动重载，**去掉 Retry**） |
| **2 config_missing** | `root.exists()` 但 `read_config` 失败 | 数据目录在，仅 config 丢失；数据大概率还在 | 有效 | Recreate default config.yaml | Choose a different folder |
| **3 root_missing** | `!root.exists()` | 整个目录不可用；可能在同步/未挂载/被删 | 失效 | Retry + Choose a different folder | ▸ Start fresh here（展开后二次确认） |

设计原则：**affordance 即承诺**。Retry 仅出现在重试可能改变结果的 Tier 3（目录可能同步回来）；Tier 1 用自动检测取代 Retry；破坏性的「重建/Start fresh」在风险最高的 Tier 3 降级为需二次确认的隐藏操作。

## 4. 架构决策

**分类放后端**，前端只渲染。前端不得按 error message 子串猜类型——现有 `SetupScreen.vue:37` 的 `msg.includes("No such file")` 正是要消除的反模式，它把后端语义泄漏进前端。

**表示法（已选 A）**：在 `InitResult::ConfigError` 负载中增加分类字段，而非新增多个 InitResult 变体（enum + 前端 churn 大）或前端字符串匹配（脆弱）。

```rust
// models.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryCategory {
    InPlace,
    ConfigMissing,
    RootMissing,
}

// InitResult::ConfigError 增加字段
ConfigError {
    category: RecoveryCategory,
    errors: Vec<ConfigErrorDetail>,
    scan_warnings: Vec<ScanWarning>,
}
```

## 5. 后端改动

### 5.1 `init` 与 `set_root_path` 分类（`commands.rs`）

`init` 在 `read_config` 之前先判 `root.exists()`：

- `false` → 返回 `ConfigError { category: RootMissing, errors: [一条说明性错误], scan_warnings: [] }`（root 不存在时跳过 scan）。
- `true` 且 `read_config` 失败 → `ConfigError { category: ConfigMissing, ... }`。
- `read_config` 成功但 `validate_config` / monthly / day 文件有错 → `ConfigError { category: InPlace, ... }`。

`set_root_path` 的错误返回同样携带 `category`（当前它对「无 config」返回 `Err(String)`，改为返回 `Ok(ConfigError { category, .. })`，与 `init` 对齐，便于前端统一处理）。

### 5.2 重建：复用 `create_starter_files`

`create_starter_files`（`commands.rs:933`）已具备：路径不存在则 `create_dir_all`，`config.yaml` 不存在才写默认内容。幂等，直接复用：
- Tier 2「Recreate default config.yaml」：`create_starter_files(root)` 后重跑 `init`。
- Tier 3「Start fresh here」：同一命令（会重建目录），但前端二次确认后才调用。

### 5.3 新增 `reveal_config_file`（`commands.rs` + `lib.rs` 注册）

仿 `reveal_day_file`（`commands.rs:912`，用 `app.opener()`）。Tier 1 的「Reveal in Finder」：若 `config.yaml` 存在则 reveal 该文件，否则 reveal root 目录（覆盖 monthly/day 文件损坏的情形）。

### 5.4 修复 watcher 生命周期（`config.rs` + `lib.rs` + `commands.rs`）

现状：`watch_files`（`config.rs:119`）`thread::spawn` 一个脱离线程，watcher 句柄被 move 进闭包，无句柄、不可停止；重复调用产生重复线程与重复事件。

改为**可重启**：

- 引入 Tauri managed state，例如 `WatcherState { watched: Mutex<Option<PathBuf>> , watcher: Mutex<Option<RecommendedWatcher>> }`。把 `RecommendedWatcher` 持有在 state 中（而非 move 进线程），drop 它即停止事件流，接收线程随 channel 关闭而退出。
- 提供 `ensure_watcher(app, root)`：
  - 已在监听同一 path → no-op（幂等）。
  - 在监听不同 path → 先停旧（drop 旧 watcher）再起新。
  - 无 → 起新。
- 调用点：`lib.rs` setup（保留）、`set_root_path` 成功后、重建后重跑 `init` 进入 Ready 后。

效果：Tier 1/2 文案「Changes are detected automatically」在所有路径成立；顺带修掉首次 setup 的 latent bug。

## 6. 前端改动

### 6.1 新增 `RecoveryScreen.vue`

取代 `App.vue:161-168` 当前的「`ConfigErrorBanner` + 裸 Retry」。按 `store.configCategory` 渲染三层（见已批准的 mockup）。`ConfigErrorBanner` 可保留为 Tier 1 错误清单子组件或并入新组件。

### 6.2 抽取共用的「选文件夹」逻辑

`SetupScreen.vue` 的目录选择 + `set_root_path` 调用抽成可复用单元（composable，如 `useRootFolderPicker`），供 SetupScreen 与 RecoveryScreen 共用。删除 `SetupScreen.vue:35-54` 的 `confirm()` + 字符串判断分支——「选了文件夹但无 config」改由后端 `category` 驱动，进入 RecoveryScreen 的 config_missing 态。

### 6.3 store 与接线

- `store` 增加 `configCategory: RecoveryCategory | null`。
- `App.vue` 的 `init` 分支与 `config-changed` 监听写入 `configCategory`。
- 操作接线：
  - Retry → `init`
  - Choose a different folder → dialog + `set_root_path`（复用 composable）
  - Recreate / Start fresh → `create_starter_files` + `init`（Tier 3 前置二次确认）
  - Reveal in Finder → `reveal_config_file`

### 6.4 文案（英文 UI）

- Tier 1：保留「Configuration Errors (N)」+ 错误清单 +「Fix these in your config.yaml or _monthly.md. Changes are detected automatically.」+ Reveal 按钮。
- Tier 2：标题「Your config.yaml is missing」；正文「Your data folder is here, but its config file is gone:」+ 路径 +「Your records are still in place. Recreate a default config to continue.」
- Tier 3：标题「Can't find your Logbook folder」；正文「Logbook expects your data here, but it isn't available:」+ 路径 +「This can happen if iCloud hasn't finished syncing, the drive isn't mounted, or the folder was moved or deleted. Logbook won't create files here automatically, to avoid conflicting with data that may still be syncing.」+ 折叠项「Folder was deleted on purpose? Start fresh here」。

遵循 `docs/interaction-principles.md`（不丢输入、取消行为一致）与设计 token 规范（语义间距/字号 token，禁裸 px / Tailwind 数字档）。

## 7. 测试

**Rust 集成测试**（`tests/`，依约定碰文件系统走集成测试）：
- `init` 在三种 fixture 下返回正确 `category`：root 不存在 / root 在但无 config / config 非法。
- `set_root_path` 对无 config 的目录返回 `ConfigError { category: ConfigMissing }`。
- `create_starter_files` 幂等：已有 config.yaml 时不覆盖。
- `reveal_config_file` smoke（不验证 GUI，只验证命令不 panic、路径解析正确）。
- watcher 可重启：同 path 幂等、换 path 替换（以「不重复 emit」为断言目标，或退化为对 `ensure_watcher` 状态机的单元测试）。

**前端组件测试**：
- `RecoveryScreen` 按三种 `category` 渲染对应操作集合（Tier 1 无 Retry、Tier 3 有 Retry 且 Start fresh 需确认）。

## 8. 风险与取舍

- **watcher 可重启的线程模型**是本设计技术风险最高处：需把 `RecommendedWatcher` 移出脱离线程、改由 managed state 持有，接收循环随 channel 关闭退出。若实现成本超预期，退路是「session 内只跟随首个有效 root，换文件夹需重启才重新指向」并相应弱化文案，但这会让 folder-change 路径上「自动检测」再次失真，非首选。
- **二次确认用原生 `confirm` 还是自建对话框**：原生 `confirm` 最省事但样式不可控；本仓库已有 Toast 等基础组件，倾向自建轻量确认。留待实现期定，不影响架构。
