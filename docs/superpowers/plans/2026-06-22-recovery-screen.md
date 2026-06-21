# Recovery Screen (按可恢复性分层) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把删除数据目录后的「Configuration Errors + 仅 Retry」死胡同，重构成按可恢复性分三层（in_place / config_missing / root_missing）、每层给出正确恢复操作的恢复屏幕，并修复 watcher 生命周期使「自动检测」在所有路径成立。

**Architecture:** 后端在 `init` / `set_root_path` 之前用一个纯函数 `load_root_state(root)` 对错误分类（不持有 `AppHandle`，便于测试），分类结果经 `InitResult::ConfigError { category, root_path, .. }` 传给前端；前端新增 `RecoveryScreen.vue` 按 `category` 渲染操作集，复用现有 `create_starter_files` 做安全重建、新增 `reveal_config_file` 命令；watcher 改为可重启的 `WatcherState` managed state，`set_root_path` / 恢复成功后启动。

**Tech Stack:** Rust (Tauri 2.x, `notify` crate, `yaml_serde`)、Vue 3 + TypeScript、Vitest + @vue/test-utils、`cargo test`。参考规格：`docs/superpowers/specs/2026-06-22-recovery-screen-by-recoverability-design.md`。

**约定提醒：**
- 测试规范见 `src-tauri/CLAUDE.md`：纯函数/无 IO → `#[cfg(test)] mod tests`（单元）；碰文件系统/调命令 → `tests/`（集成），临时目录用 `std::env::temp_dir()` 并清理。
- 前端 token 规范见根 `CLAUDE.md`：间距/字号用语义 token（`gap-sm`/`p-md`/`text-body` 等），禁裸 px 与 Tailwind 数字档；`leading-none` 是唯一合法行高覆盖。`src/__tests__/tailwind-token-usage.test.ts` 是护栏。
- 交互遵循 `docs/interaction-principles.md`。
- 分支已在 `feat/recovery-screen`。

---

## 文件结构

**后端（`src-tauri/src/`）**
- `models.rs` — 修改：新增 `RecoveryCategory` enum；`InitResult::ConfigError` 加 `category` + `root_path` 字段。
- `commands.rs` — 修改：新增 `pub fn load_root_state`、`pub fn reveal_config_target`、`#[tauri::command] reveal_config_file`；`init` / `set_root_path` 委托 `load_root_state` 并调用 `ensure_watcher`。
- `config.rs` — 修改：新增 `WatcherState`、`pub fn ensure_watcher`、`pub fn needs_restart`（纯）、`fn spawn_watcher`；删除旧 `pub fn watch_files`。
- `lib.rs` — 修改：`app.manage(WatcherState::new())`；启动处改用 `ensure_watcher`；注册 `reveal_config_file`。
- `tests/recovery_category_integration.rs` — 新建：`load_root_state` 五种分类 + `reveal_config_target` 决策。

**前端（`src/`）**
- `types.ts` — 修改：新增 `RecoveryCategory` 类型；`InitResult` 的 ConfigError data 加 `category` + `root_path`。
- `stores/useStore.ts` — 修改：`AppStore` 加 `configCategory`；`createStore` 默认值。
- `utils/applyInitResult.ts` — 新建：`InitResult → store` 映射（App 与 picker 共用），返回 `scan_warnings`。
- `composables/useRootFolderPicker.ts` — 新建：目录选择 + `set_root_path` + store 更新（SetupScreen 与 RecoveryScreen 共用）。
- `components/RecoveryScreen.vue` — 新建：按 `category` 渲染三层。
- `components/ConfigErrorBanner.vue` — 复用为 Tier 1 错误清单子组件（不改）。
- `components/SetupScreen.vue` — 修改：改用 `useRootFolderPicker`，删除 `confirm()`/字符串判断。
- `App.vue` — 修改：`initApp` 用 `applyInitResult`；写 `configCategory`；error 态渲染 `<RecoveryScreen :reload="initApp" />`。
- `__tests__/components/App.test.ts` — 修改：更新受影响的错误态测试。
- `__tests__/components/RecoveryScreen.test.ts` — 新建。

---

## Phase 1 — 后端错误分类

### Task 1: `RecoveryCategory` enum + `ConfigError` 加字段

**Files:**
- Modify: `src-tauri/src/models.rs:96-111`（`InitResult` 定义）+ 在其后新增 enum

- [ ] **Step 1: 写失败的单元测试**

在 `src-tauri/src/models.rs` 的 `#[cfg(test)] mod tests` 内（紧接 `init_result_config_error_with_scan_warnings` 测试之后）新增：

```rust
    #[test]
    fn recovery_category_serializes_snake_case() {
        let json = serde_json::to_string(&RecoveryCategory::RootMissing).expect("serialize");
        assert_eq!(json, "\"root_missing\"");
        let back: RecoveryCategory =
            serde_json::from_str("\"config_missing\"").expect("deserialize");
        assert_eq!(back, RecoveryCategory::ConfigMissing);
    }

    #[test]
    fn config_error_carries_category_and_root_path() {
        let result = InitResult::ConfigError {
            category: RecoveryCategory::ConfigMissing,
            root_path: "/tmp/logbook".to_string(),
            errors: vec![],
            scan_warnings: vec![],
        };
        match result {
            InitResult::ConfigError { category, root_path, .. } => {
                assert_eq!(category, RecoveryCategory::ConfigMissing);
                assert_eq!(root_path, "/tmp/logbook");
            }
            _ => panic!("expected ConfigError"),
        }
    }
```

- [ ] **Step 2: 运行测试确认失败（编译错误）**

Run: `cd src-tauri && cargo test -p tauri_app_lib recovery_category_serializes_snake_case`
Expected: 编译失败 —— `cannot find type RecoveryCategory` / `ConfigError` 缺字段。

- [ ] **Step 3: 实现 enum + 字段**

在 `src-tauri/src/models.rs` 的 `InitResult` enum **之前**新增：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryCategory {
    InPlace,
    ConfigMissing,
    RootMissing,
}
```

把 `InitResult::ConfigError` 变体改为：

```rust
    ConfigError {
        category: RecoveryCategory,
        root_path: String,
        errors: Vec<ConfigErrorDetail>,
        scan_warnings: Vec<ScanWarning>,
    },
```

同时修正 `models.rs` 内既有的两个构造点：`init_result_config_error_with_scan_warnings`（约 `models.rs:255`）增加 `category: RecoveryCategory::ConfigMissing,` 和 `root_path: "/tmp/x".to_string(),` 两个字段（否则该测试编译失败）。

- [ ] **Step 4: 运行测试**

Run: `cd src-tauri && cargo test -p tauri_app_lib config_error`
Expected: 编译可能仍因 `commands.rs` 旧构造点失败 —— 这是预期的，下一 Task 修。先确认 `models.rs` 本身的 `models::tests` 通过：`cargo test -p tauri_app_lib --lib models::tests::recovery_category_serializes_snake_case` 应 PASS（若整 crate 编译被 commands.rs 挡住，则先做 Task 2 再回来跑）。

> 说明：本 Task 与 Task 2 共同构成一次可编译提交。允许 Step 5 推迟到 Task 2 之后一起提交。

- [ ] **Step 5: （与 Task 2 合并提交）**

---

### Task 2: `load_root_state` 分类函数 + `init`/`set_root_path` 委托

**Files:**
- Modify: `src-tauri/src/commands.rs:138-322`（`init` 与 `set_root_path`）
- Create test: `src-tauri/tests/recovery_category_integration.rs`

- [ ] **Step 1: 写失败的集成测试**

新建 `src-tauri/tests/recovery_category_integration.rs`：

```rust
//! Integration tests for load_root_state error classification.

use std::fs;
use std::path::PathBuf;
use tauri_app_lib::commands::load_root_state;
use tauri_app_lib::models::{InitResult, RecoveryCategory};

const VALID_CONFIG: &str = "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n";

fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!("logbook_recovery_{}", uuid::Uuid::new_v4()))
}

fn category_of(result: &InitResult) -> RecoveryCategory {
    match result {
        InitResult::ConfigError { category, .. } => *category,
        other => panic!("expected ConfigError, got {:?}", other),
    }
}

#[test]
fn root_missing_when_dir_absent() {
    let root = temp_root(); // never created
    let result = load_root_state(&root);
    assert_eq!(category_of(&result), RecoveryCategory::RootMissing);
    match result {
        InitResult::ConfigError { root_path, scan_warnings, .. } => {
            assert_eq!(root_path, root.to_string_lossy());
            assert!(scan_warnings.is_empty(), "no scan on a missing root");
        }
        _ => unreachable!(),
    }
}

#[test]
fn config_missing_when_dir_present_but_no_config() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    let result = load_root_state(&root);
    assert_eq!(category_of(&result), RecoveryCategory::ConfigMissing);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn in_place_when_config_present_but_malformed() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("config.yaml"), "this: is: not: valid: yaml: : :").unwrap();
    let result = load_root_state(&root);
    assert_eq!(category_of(&result), RecoveryCategory::InPlace);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn in_place_when_config_valid_but_invalid_values() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    // parses fine, but source is not static/monthly → validate_config error
    fs::write(
        root.join("config.yaml"),
        "dimensions:\n  - name: X\n    key: x\n    source: bogus\n",
    )
    .unwrap();
    let result = load_root_state(&root);
    assert_eq!(category_of(&result), RecoveryCategory::InPlace);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn ready_when_everything_valid() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("config.yaml"), VALID_CONFIG).unwrap();
    let result = load_root_state(&root);
    assert!(matches!(result, InitResult::Ready { .. }), "got {:?}", result);
    fs::remove_dir_all(&root).unwrap();
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test --test recovery_category_integration`
Expected: 编译失败 —— `load_root_state` 不存在。

- [ ] **Step 3: 实现 `load_root_state` 并改写 `init` / `set_root_path`**

在 `src-tauri/src/commands.rs` 中新增函数（放在 `init` 之前）：

```rust
/// Classify the data root and load initial state.
/// No AppHandle → unit/integration testable. init/set_root_path delegate here.
pub fn load_root_state(root: &std::path::Path) -> InitResult {
    if !root.exists() {
        return InitResult::ConfigError {
            category: RecoveryCategory::RootMissing,
            root_path: root.to_string_lossy().into_owned(),
            errors: vec![ConfigErrorDetail {
                kind: "RootMissing".to_string(),
                message: format!("Data folder not found: {}", root.display()),
            }],
            scan_warnings: vec![],
        };
    }

    let scan_warnings = crate::scan::scan_data_dir(root);

    if !files::config_path(root).exists() {
        return InitResult::ConfigError {
            category: RecoveryCategory::ConfigMissing,
            root_path: root.to_string_lossy().into_owned(),
            errors: vec![ConfigErrorDetail {
                kind: "ConfigMissing".to_string(),
                message: format!("config.yaml not found in {}", root.display()),
            }],
            scan_warnings,
        };
    }

    let config = match files::read_config(root) {
        Ok(c) => c,
        Err(e) => {
            return InitResult::ConfigError {
                category: RecoveryCategory::InPlace,
                root_path: root.to_string_lossy().into_owned(),
                errors: vec![ConfigErrorDetail {
                    kind: "ConfigReadError".to_string(),
                    message: e,
                }],
                scan_warnings,
            };
        }
    };

    let mut all_errors = validate_config(&config);

    let now = chrono::Local::now();
    let monthly = match read_monthly_file_safe(root, now.year(), now.month()) {
        Ok(mf) => mf,
        Err(e) => {
            all_errors.push(ConfigErrorDetail {
                kind: "MonthlyFileCorrupt".to_string(),
                message: e,
            });
            MonthlyFile { commitments: vec![] }
        }
    };
    all_errors.extend(validate_monthly(&monthly));

    let today_date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());
    let today = match read_day_file_safe(root, &today_date) {
        Ok(df) => df,
        Err(e) => {
            all_errors.push(ConfigErrorDetail {
                kind: "DayFileCorrupt".to_string(),
                message: e,
            });
            DayFile { note: None, entries: vec![] }
        }
    };

    if !all_errors.is_empty() {
        return InitResult::ConfigError {
            category: RecoveryCategory::InPlace,
            root_path: root.to_string_lossy().into_owned(),
            errors: all_errors,
            scan_warnings,
        };
    }

    InitResult::Ready {
        root_path: root.to_string_lossy().into_owned(),
        config,
        today,
        commitments: monthly.commitments,
        scan_warnings,
    }
}
```

把 `init` 函数体（`commands.rs:138-237`）替换为：

```rust
#[tauri::command]
pub fn init(app: AppHandle) -> InitResult {
    error_log::log_command_enter("init", "");
    let app_data_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let root_path = match read_root_path(&app_data_dir) {
        Some(p) => p,
        None => {
            error_log::log_command_exit("init", true, "NeedsSetup");
            return InitResult::NeedsSetup;
        }
    };

    let result = load_root_state(&root_path);
    if root_path.exists() {
        crate::config::ensure_watcher(&app, root_path.clone());
    }
    error_log::log_command_exit("init", !matches!(&result, InitResult::ConfigError { .. }), "");
    result
}
```

把 `set_root_path` 函数体（`commands.rs:239-322`）替换为：

```rust
#[tauri::command]
pub fn set_root_path(app: AppHandle, path: String) -> Result<InitResult, String> {
    error_log::log_command_enter("set_root_path", &format!("path={}", path));
    let app_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let root_path = std::path::Path::new(&path);
    if !root_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if !root_path.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    save_root_path(&app_data_dir, root_path)?;

    let result = load_root_state(root_path);
    crate::config::ensure_watcher(&app, root_path.to_path_buf());
    error_log::log_command_exit("set_root_path", true, "");
    Ok(result)
}
```

> `ensure_watcher` 在 Task 4 实现。本 Task 先用 `crate::config::ensure_watcher` 引用——若 Task 4 未做会编译失败，因此 **Task 4 与本 Task 属同一编译单元**：可先临时把这两行 `ensure_watcher(...)` 注释掉跑通分类测试，再在 Task 4 解除注释。推荐顺序：完成本 Task 的分类逻辑（注释 ensure_watcher）→ Task 4 → 回填。

确认 `RecoveryCategory` 已在 `commands.rs` 顶部的 `use crate::models::...` 中引入（与 `ConfigErrorDetail`、`InitResult` 同处）。

- [ ] **Step 4: 运行测试**

Run: `cd src-tauri && cargo test --test recovery_category_integration`
Expected: 5 个测试全部 PASS。
再跑 `cargo test -p tauri_app_lib --lib models::tests`：Task 1 的 models 测试 PASS。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands.rs src-tauri/tests/recovery_category_integration.rs
git commit -m "feat(recovery): classify init errors into in_place/config_missing/root_missing"
```

> **Phase 1 checkpoint：停下确认后再进入 Phase 2。**

---

## Phase 2 — `reveal_config_file` 命令

### Task 3: `reveal_config_target` 决策 + `reveal_config_file` 命令

**Files:**
- Modify: `src-tauri/src/commands.rs`（新增函数 + 命令）
- Modify: `src-tauri/src/lib.rs:53-69`（注册命令）
- Modify: `src-tauri/tests/recovery_category_integration.rs`（追加测试）

- [ ] **Step 1: 写失败的集成测试**

在 `src-tauri/tests/recovery_category_integration.rs` 末尾追加：

```rust
use tauri_app_lib::commands::reveal_config_target;

#[test]
fn reveal_target_selects_config_when_present() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("config.yaml"), VALID_CONFIG).unwrap();
    let (path, select) = reveal_config_target(&root);
    assert!(select, "should select the config file when it exists");
    assert!(path.ends_with("config.yaml"));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn reveal_target_opens_root_when_config_absent() {
    let root = temp_root();
    fs::create_dir_all(&root).unwrap();
    let (path, select) = reveal_config_target(&root);
    assert!(!select, "no file to select → open the dir");
    assert_eq!(path, root);
    fs::remove_dir_all(&root).unwrap();
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test --test recovery_category_integration reveal_target`
Expected: 编译失败 —— `reveal_config_target` 不存在。

- [ ] **Step 3: 实现决策函数 + 命令**

在 `src-tauri/src/commands.rs` 中新增（放在既有 `reveal_day_file` 命令附近）：

```rust
/// (path, select) for revealing config: select the file if it exists, else open the root dir.
pub fn reveal_config_target(root: &std::path::Path) -> (std::path::PathBuf, bool) {
    let config = files::config_path(root);
    if config.exists() {
        (config, true)
    } else {
        (root.to_path_buf(), false)
    }
}

#[tauri::command]
pub fn reveal_config_file(app: AppHandle, root_path: String) -> Result<(), String> {
    error_log::log_command_enter("reveal_config_file", &format!("root={}", root_path));
    let root = std::path::Path::new(&root_path);
    let (target, select) = reveal_config_target(root);
    let result = if select {
        app.opener()
            .reveal_item_in_dir(&target)
            .map_err(|e| format!("Failed to reveal {}: {}", target.display(), e))
    } else {
        app.opener()
            .open_path(target.to_string_lossy().into_owned(), None::<String>)
            .map_err(|e| format!("Failed to open {}: {}", target.display(), e))
    };
    error_log::log_command_exit("reveal_config_file", result.is_ok(), "");
    result
}
```

在 `src-tauri/src/lib.rs` 的 `tauri::generate_handler![...]`（`lib.rs:53-69`）列表中，于 `commands::reveal_day_file,` 之后加一行：

```rust
            commands::reveal_config_file,
```

- [ ] **Step 4: 运行测试**

Run: `cd src-tauri && cargo test --test recovery_category_integration reveal_target`
Expected: 2 个 reveal 测试 PASS。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/tests/recovery_category_integration.rs
git commit -m "feat(recovery): add reveal_config_file command"
```

---

## Phase 3 — Watcher 生命周期

### Task 4: 可重启 watcher（`WatcherState` + `ensure_watcher`）

**Files:**
- Modify: `src-tauri/src/config.rs:119-228`（替换 `watch_files`）
- Modify: `src-tauri/src/lib.rs`（manage state + 启动处）

- [ ] **Step 1: 写失败的单元测试（纯决策函数）**

在 `src-tauri/src/config.rs` 的 `#[cfg(test)] mod tests`（约 `config.rs:230`）内追加：

```rust
    #[test]
    fn needs_restart_logic() {
        use std::path::Path;
        assert!(super::needs_restart(None, Path::new("/a")), "no watcher → start");
        assert!(
            !super::needs_restart(Some(Path::new("/a")), Path::new("/a")),
            "same path → no-op"
        );
        assert!(
            super::needs_restart(Some(Path::new("/a")), Path::new("/b")),
            "different path → restart"
        );
    }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test -p tauri_app_lib needs_restart_logic`
Expected: 编译失败 —— `needs_restart` 不存在。

- [ ] **Step 3: 实现 WatcherState / needs_restart / ensure_watcher / spawn_watcher，删除旧 watch_files**

在 `src-tauri/src/config.rs` 顶部 `use` 区补充（若缺）：

```rust
use notify::RecommendedWatcher;
use std::sync::Mutex;
use tauri::Manager;
```

把现有 `pub fn watch_files(app_handle: AppHandle, root_path: PathBuf) { ... }`（`config.rs:119-228`）整体替换为：

```rust
/// Managed state holding the live file watcher. Dropping the inner watcher stops
/// its event stream (the receiver thread exits when the channel closes).
pub struct WatcherState {
    inner: Mutex<Option<WatcherHandle>>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self { inner: Mutex::new(None) }
    }
}

impl Default for WatcherState {
    fn default() -> Self {
        Self::new()
    }
}

struct WatcherHandle {
    path: PathBuf,
    _watcher: RecommendedWatcher, // kept alive; drop = stop watching
}

/// Pure decision: do we need to (re)start the watcher for `requested`?
pub fn needs_restart(current: Option<&std::path::Path>, requested: &std::path::Path) -> bool {
    current != Some(requested)
}

/// Start (or restart) the recursive file watcher for `root_path`.
/// Idempotent for the same path; replaces the watcher when the path changes.
pub fn ensure_watcher(app: &AppHandle, root_path: PathBuf) {
    let state = app.state::<WatcherState>();
    let mut guard = state.inner.lock().expect("WatcherState lock poisoned");
    if !needs_restart(guard.as_ref().map(|h| h.path.as_path()), &root_path) {
        return;
    }
    match spawn_watcher(app.clone(), root_path.clone()) {
        Ok(watcher) => {
            // Assigning Some replaces (and drops) any previous handle → old watcher stops.
            *guard = Some(WatcherHandle { path: root_path, _watcher: watcher });
        }
        Err(e) => {
            crate::error_log::log_error("ensure_watcher", &e);
            *guard = None;
        }
    }
}

/// Build the watcher and spawn its receiver thread. Returns the watcher to be
/// held in WatcherState; the receiver thread exits when the watcher is dropped.
fn spawn_watcher(app_handle: AppHandle, root_path: PathBuf) -> Result<RecommendedWatcher, String> {
    crate::error_log::log_info("file_watcher", &format!("Watching {}", root_path.display()));
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
        Ok(event) => {
            if let Err(e) = tx.send(event) {
                crate::error_log::log_error("file_watcher", &format!("send error: {:?}", e));
            }
        }
        Err(e) => {
            crate::error_log::log_error("file_watcher", &format!("notify error: {}", e));
        }
    })
    .map_err(|e| format!("Failed to create file watcher: {}", e))?;

    watcher
        .configure(NotifyConfig::default())
        .map_err(|e| format!("Failed to configure watcher: {}", e))?;

    watcher
        .watch(&root_path, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch {}: {}", root_path.display(), e))?;

    let watch_root = root_path.clone();
    std::thread::spawn(move || {
        let debounce_ms = Duration::from_millis(300);
        let mut last_event: HashMap<std::path::PathBuf, Instant> = HashMap::new();

        for event in rx {
            let is_modify = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_));
            if !is_modify {
                continue;
            }

            for path in &event.paths {
                let now = Instant::now();
                if let Some(last) = last_event.get(path) {
                    if now.duration_since(*last) < debounce_ms {
                        continue;
                    }
                }
                last_event.insert(path.clone(), now);

                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "config.yaml" {
                    match files::read_config(&watch_root) {
                        Ok(config) => {
                            let errors = validate_config(&config);
                            if let Err(e) = app_handle.emit("config-changed", &errors) {
                                crate::error_log::log_error(
                                    "file_watcher",
                                    &format!("emit config-changed failed: {}", e),
                                );
                            }
                        }
                        Err(e) => {
                            if let Err(e2) = app_handle.emit(
                                "config-changed",
                                &vec![ConfigErrorDetail {
                                    kind: "ParseError".to_string(),
                                    message: e,
                                }],
                            ) {
                                crate::error_log::log_error(
                                    "file_watcher",
                                    &format!("emit config-changed failed: {}", e2),
                                );
                            }
                        }
                    }
                } else if file_name == "_monthly.md" {
                    let now = chrono::Local::now();
                    match files::read_monthly_file(&watch_root, now.year(), now.month()) {
                        Ok(monthly) => {
                            let errors = validate_monthly(&monthly);
                            if let Err(e) = app_handle.emit("commitments-changed", &errors) {
                                crate::error_log::log_error(
                                    "file_watcher",
                                    &format!("emit commitments-changed failed: {}", e),
                                );
                            }
                        }
                        Err(e) => {
                            if let Err(e2) = app_handle.emit(
                                "commitments-changed",
                                &vec![ConfigErrorDetail {
                                    kind: "ParseError".to_string(),
                                    message: e,
                                }],
                            ) {
                                crate::error_log::log_error(
                                    "file_watcher",
                                    &format!("emit commitments-changed failed: {}", e2),
                                );
                            }
                        }
                    }
                }
            }
        }
        crate::error_log::log_info("file_watcher", "receiver thread exited");
    });

    Ok(watcher)
}
```

在 `src-tauri/src/lib.rs`：
1. 删除 `use config::watch_files;`（`lib.rs:11`）。
2. 在 setup 闭包内、创建窗口之前加入：`app.manage(config::WatcherState::new());`
3. 把启动监听处（`lib.rs:45-50`）：

```rust
            if let Some(root_path) = files::read_root_path(&app_data_dir) {
                if root_path.exists() {
                    files::cleanup_tmp_files(&root_path);
                    watch_files(app_handle, root_path);
                }
            }
```

改为：

```rust
            if let Some(root_path) = files::read_root_path(&app_data_dir) {
                if root_path.exists() {
                    files::cleanup_tmp_files(&root_path);
                    config::ensure_watcher(&app_handle, root_path);
                }
            }
```

4. 回到 Task 2：解除 `init` / `set_root_path` 中 `crate::config::ensure_watcher(...)` 的注释（若当时注释过）。

- [ ] **Step 4: 运行测试 + 编译全量**

Run: `cd src-tauri && cargo test -p tauri_app_lib needs_restart_logic`
Expected: PASS。
Run: `cd src-tauri && cargo check`
Expected: 通过，无 `watch_files` 未定义 / 未使用告警。
Run: `cd src-tauri && cargo test`
Expected: 全绿（含 Phase 1/2 测试）。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs src-tauri/src/commands.rs
git commit -m "feat(recovery): restartable file watcher via WatcherState; start after set_root_path"
```

> **Phase 3 checkpoint：后端完成，停下确认后再进入前端。**

---

## Phase 4 — 前端类型与 store

### Task 5: `RecoveryCategory` 类型、`InitResult` 字段、store

**Files:**
- Modify: `src/types.ts:80-97`
- Modify: `src/stores/useStore.ts`
- Modify: `src/__tests__/useStore.test.ts`（若断言了完整 store 形状）

- [ ] **Step 1: 写失败的测试**

在 `src/__tests__/useStore.test.ts` 中新增（与既有 `createStore` 默认值测试同组）：

```ts
it("createStore 默认 configCategory 为 null", () => {
  const store = createStore();
  expect(store.configCategory).toBeNull();
});
```

（确认文件顶部已 `import { createStore } from "../stores/useStore";`，否则补上。）

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run src/__tests__/useStore.test.ts`
Expected: FAIL —— `configCategory` 不存在 / 类型错误。

- [ ] **Step 3: 实现类型与 store 字段**

`src/types.ts`：在 `AppStatus` 定义附近新增类型：

```ts
export type RecoveryCategory = "in_place" | "config_missing" | "root_missing";
```

把 `InitResult` 的 ConfigError 分支（`src/types.ts:82`）改为：

```ts
  | {
      status: "ConfigError";
      data: {
        category: RecoveryCategory;
        root_path: string;
        errors: ConfigErrorDetail[];
        scan_warnings: ScanWarning[];
      };
    }
```

`src/stores/useStore.ts`：
1. 顶部 import 增加 `RecoveryCategory`：
```ts
import type { Config, DayFile, Commitment, CommitmentProgress, ConfigErrorDetail, AppStatus, Entry, RecoveryCategory } from "../types";
```
2. `AppStore` 接口加字段（紧随 `configErrors`）：
```ts
  configCategory: RecoveryCategory | null;
```
3. `createStore` 的 `reactive<AppStore>({...})` 默认值加（紧随 `configErrors: []`）：
```ts
    configCategory: null,
```

- [ ] **Step 4: 运行测试**

Run: `pnpm vitest run src/__tests__/useStore.test.ts`
Expected: PASS。

- [ ] **Step 5: Commit**

```bash
git add src/types.ts src/stores/useStore.ts src/__tests__/useStore.test.ts
git commit -m "feat(recovery): add RecoveryCategory type and store.configCategory"
```

---

## Phase 5 — 共用逻辑：applyInitResult + useRootFolderPicker

### Task 6: `applyInitResult` 工具

**Files:**
- Create: `src/utils/applyInitResult.ts`
- Create: `src/__tests__/applyInitResult.test.ts`

- [ ] **Step 1: 写失败的测试**

新建 `src/__tests__/applyInitResult.test.ts`：

```ts
import { describe, it, expect } from "vitest";
import { applyInitResult } from "../utils/applyInitResult";
import { createStore } from "../stores/useStore";
import type { InitResult } from "../types";

describe("applyInitResult", () => {
  it("NeedsSetup → status setup", () => {
    const store = createStore();
    const warnings = applyInitResult(store, { status: "NeedsSetup" });
    expect(store.status).toBe("setup");
    expect(warnings).toEqual([]);
  });

  it("ConfigError → status error, category + rootPath + errors set", () => {
    const store = createStore();
    const result: InitResult = {
      status: "ConfigError",
      data: {
        category: "root_missing",
        root_path: "/data/logbook",
        errors: [{ kind: "RootMissing", message: "gone" }],
        scan_warnings: [{ kind: "OrphanedTemp", path: "x.tmp", message: "t" }],
      },
    };
    const warnings = applyInitResult(store, result);
    expect(store.status).toBe("error");
    expect(store.configCategory).toBe("root_missing");
    expect(store.rootPath).toBe("/data/logbook");
    expect(store.configErrors).toEqual([{ kind: "RootMissing", message: "gone" }]);
    expect(warnings).toHaveLength(1);
  });

  it("Ready → status ready, category cleared", () => {
    const store = createStore();
    store.configCategory = "in_place";
    const result: InitResult = {
      status: "Ready",
      data: {
        root_path: "/data/logbook",
        config: { dimensions: [] },
        today: { note: null, entries: [] },
        commitments: [],
        scan_warnings: [],
      },
    };
    applyInitResult(store, result);
    expect(store.status).toBe("ready");
    expect(store.rootPath).toBe("/data/logbook");
    expect(store.configCategory).toBeNull();
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run src/__tests__/applyInitResult.test.ts`
Expected: FAIL —— 模块不存在。

- [ ] **Step 3: 实现**

新建 `src/utils/applyInitResult.ts`：

```ts
import type { AppStore } from "../stores/useStore";
import type { InitResult, ScanWarning } from "../types";

/**
 * Map an InitResult onto the store and return scan_warnings for the caller to
 * surface (toast). Shared by App.initApp and useRootFolderPicker so the two
 * entry points stay in sync.
 */
export function applyInitResult(store: AppStore, result: InitResult): ScanWarning[] {
  switch (result.status) {
    case "NeedsSetup":
      store.status = "setup";
      return [];
    case "ConfigError":
      store.configErrors = result.data.errors;
      store.configCategory = result.data.category;
      store.rootPath = result.data.root_path;
      store.status = "error";
      return result.data.scan_warnings;
    case "Ready":
      store.rootPath = result.data.root_path;
      store.config = result.data.config;
      store.today = result.data.today;
      store.configCategory = null;
      store.status = "ready";
      return result.data.scan_warnings;
  }
}
```

- [ ] **Step 4: 运行测试**

Run: `pnpm vitest run src/__tests__/applyInitResult.test.ts`
Expected: 3 个测试 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/utils/applyInitResult.ts src/__tests__/applyInitResult.test.ts
git commit -m "feat(recovery): add applyInitResult store mapper"
```

---

### Task 7: `useRootFolderPicker` composable + SetupScreen 改用

**Files:**
- Create: `src/composables/useRootFolderPicker.ts`
- Modify: `src/components/SetupScreen.vue`
- Create: `src/__tests__/useRootFolderPicker.test.ts`

- [ ] **Step 1: 写失败的测试**

新建 `src/__tests__/useRootFolderPicker.test.ts`：

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";

const { mockInvoke, mockOpen } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockOpen: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mockOpen }));
vi.mock("../utils/errorLog", () => ({ logError: vi.fn() }));

import { useRootFolderPicker } from "../composables/useRootFolderPicker";
import { createStore } from "../stores/useStore";

describe("useRootFolderPicker", () => {
  beforeEach(() => vi.clearAllMocks());

  it("cancel dialog → no invoke, store untouched", async () => {
    mockOpen.mockResolvedValue(null);
    const store = createStore();
    const { pick } = useRootFolderPicker(store);
    await pick();
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(store.status).toBe("loading");
  });

  it("pick → set_root_path Ready maps to store", async () => {
    mockOpen.mockResolvedValue("/data/logbook");
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: {
        root_path: "/data/logbook",
        config: { dimensions: [] },
        today: { note: null, entries: [] },
        commitments: [],
        scan_warnings: [],
      },
    });
    const store = createStore();
    const { pick } = useRootFolderPicker(store);
    await pick();
    expect(mockInvoke).toHaveBeenCalledWith("set_root_path", { path: "/data/logbook" });
    expect(store.status).toBe("ready");
    expect(store.rootPath).toBe("/data/logbook");
  });

  it("pick → set_root_path ConfigError(config_missing) routes to error", async () => {
    mockOpen.mockResolvedValue("/data/logbook");
    mockInvoke.mockResolvedValue({
      status: "ConfigError",
      data: {
        category: "config_missing",
        root_path: "/data/logbook",
        errors: [{ kind: "ConfigMissing", message: "no config" }],
        scan_warnings: [],
      },
    });
    const store = createStore();
    const { pick } = useRootFolderPicker(store);
    await pick();
    expect(store.status).toBe("error");
    expect(store.configCategory).toBe("config_missing");
  });

  it("invoke throws → error state with SetupError", async () => {
    mockOpen.mockResolvedValue("/data/logbook");
    mockInvoke.mockRejectedValue("boom");
    const store = createStore();
    const { pick } = useRootFolderPicker(store);
    await pick();
    expect(store.status).toBe("error");
    expect(store.configErrors[0].kind).toBe("SetupError");
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run src/__tests__/useRootFolderPicker.test.ts`
Expected: FAIL —— 模块不存在。

- [ ] **Step 3: 实现 composable**

新建 `src/composables/useRootFolderPicker.ts`：

```ts
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppStore } from "../stores/useStore";
import type { InitResult } from "../types";
import { applyInitResult } from "../utils/applyInitResult";
import { logError } from "../utils/errorLog";

/**
 * Folder selection + set_root_path + store update. Shared by SetupScreen
 * (first run) and RecoveryScreen ("Choose a different folder").
 */
export function useRootFolderPicker(store: AppStore) {
  async function pick(): Promise<void> {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Logbook data folder",
    });
    if (!selected) return;
    await applyRootPath(selected as string);
  }

  async function applyRootPath(path: string): Promise<void> {
    try {
      const result = (await invoke("set_root_path", { path })) as InitResult;
      applyInitResult(store, result);
    } catch (e) {
      logError("useRootFolderPicker.applyRootPath", e);
      store.configErrors = [{ kind: "SetupError", message: `Failed: ${e}` }];
      store.configCategory = "root_missing";
      store.status = "error";
    }
  }

  return { pick, applyRootPath };
}
```

- [ ] **Step 4: 运行测试**

Run: `pnpm vitest run src/__tests__/useRootFolderPicker.test.ts`
Expected: 4 个测试 PASS。

- [ ] **Step 5: 改写 SetupScreen 使用 composable**

把 `src/components/SetupScreen.vue` 的 `<script setup>` 整段替换为：

```vue
<script setup lang="ts">
import { useStore } from "../stores/useStore";
import { useRootFolderPicker } from "../composables/useRootFolderPicker";

const store = useStore();
const { pick } = useRootFolderPicker(store);
</script>
```

把模板里按钮的 `@click="selectFolder"` 改为 `@click="pick"`。模板其余不动。

> 这删除了原 `selectFolder` / `trySetRootPath` / `confirm("No config.yaml found...")` / 字符串匹配分支——「选了文件夹但无 config」现在由后端返回 `ConfigError(config_missing)` 经 `applyInitResult` 路由到 RecoveryScreen。

- [ ] **Step 6: 验证 SetupScreen 无遗留测试受影响**

Run: `ls src/__tests__/components/ | grep -i setup || echo "no SetupScreen test"`
若存在 SetupScreen 测试，运行并按新接口修正；若输出 "no SetupScreen test" 则跳过。
Run: `pnpm vitest run` 确认无回归（App.test.ts 会在 Task 9 修，此处若它失败属预期，记下）。

- [ ] **Step 7: Commit**

```bash
git add src/composables/useRootFolderPicker.ts src/__tests__/useRootFolderPicker.test.ts src/components/SetupScreen.vue
git commit -m "feat(recovery): extract useRootFolderPicker; SetupScreen drops string-matching"
```

---

## Phase 6 — RecoveryScreen + App 接线

### Task 8: `RecoveryScreen.vue`

**Files:**
- Create: `src/components/RecoveryScreen.vue`
- Create: `src/__tests__/components/RecoveryScreen.test.ts`

- [ ] **Step 1: 写失败的组件测试**

新建 `src/__tests__/components/RecoveryScreen.test.ts`：

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { STORE_KEY } from "../../stores/useStore";
import { createTestStore } from "../mocks/store";
import RecoveryScreen from "../../components/RecoveryScreen.vue";

const { mockInvoke, mockPick } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockPick: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("../../composables/useRootFolderPicker", () => ({
  useRootFolderPicker: () => ({ pick: mockPick, applyRootPath: vi.fn() }),
}));
vi.mock("../../utils/errorLog", () => ({ logError: vi.fn() }));

function mountWith(category: string, reload = vi.fn()) {
  const store = createTestStore({
    status: "error",
    configCategory: category as never,
    rootPath: "/data/logbook",
    configErrors: [{ kind: "ConfigReadError", message: "boom" }],
  });
  const wrapper = mount(RecoveryScreen, {
    props: { reload },
    global: { provide: { [STORE_KEY as symbol]: store }, stubs: { Teleport: true } },
  });
  return { wrapper, store, reload };
}

describe("RecoveryScreen", () => {
  beforeEach(() => vi.clearAllMocks());

  it("in_place: shows error list, NO Retry button", () => {
    const { wrapper } = mountWith("in_place");
    expect(wrapper.findComponent({ name: "ConfigErrorBanner" }).exists()).toBe(true);
    expect(wrapper.text()).not.toContain("Retry");
    expect(wrapper.text()).toContain("Reveal");
  });

  it("in_place: Reveal calls reveal_config_file", async () => {
    const { wrapper } = mountWith("in_place");
    await wrapper.get('[data-testid="reveal-config"]').trigger("click");
    expect(mockInvoke).toHaveBeenCalledWith("reveal_config_file", { rootPath: "/data/logbook" });
  });

  it("config_missing: Recreate calls create_starter_files then reload", async () => {
    mockInvoke.mockResolvedValue(undefined);
    const { wrapper, reload } = mountWith("config_missing");
    await wrapper.get('[data-testid="recreate-config"]').trigger("click");
    await Promise.resolve();
    expect(mockInvoke).toHaveBeenCalledWith("create_starter_files", { path: "/data/logbook" });
    expect(reload).toHaveBeenCalled();
  });

  it("root_missing: shows Retry, Retry calls reload", async () => {
    const { wrapper, reload } = mountWith("root_missing");
    expect(wrapper.text()).toContain("Retry");
    await wrapper.get('[data-testid="retry"]').trigger("click");
    expect(reload).toHaveBeenCalled();
  });

  it("root_missing: Start-fresh requires a second confirm", async () => {
    mockInvoke.mockResolvedValue(undefined);
    const { wrapper } = mountWith("root_missing");
    // first click only reveals the confirm sub-panel, does not call create
    await wrapper.get('[data-testid="start-fresh"]').trigger("click");
    expect(mockInvoke).not.toHaveBeenCalledWith("create_starter_files", expect.anything());
    // confirm
    await wrapper.get('[data-testid="start-fresh-confirm"]').trigger("click");
    await Promise.resolve();
    expect(mockInvoke).toHaveBeenCalledWith("create_starter_files", { path: "/data/logbook" });
  });

  it("both config_missing and root_missing offer Choose-folder", async () => {
    for (const cat of ["config_missing", "root_missing"]) {
      vi.clearAllMocks();
      const { wrapper } = mountWith(cat);
      await wrapper.get('[data-testid="choose-folder"]').trigger("click");
      expect(mockPick).toHaveBeenCalled();
    }
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run src/__tests__/components/RecoveryScreen.test.ts`
Expected: FAIL —— 组件不存在。

- [ ] **Step 3: 实现 RecoveryScreen**

新建 `src/components/RecoveryScreen.vue`（注意：间距/字号用语义 token；按钮样式参考 App.vue 现有 Retry 与 SetupScreen 主按钮）：

```vue
<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../stores/useStore";
import { useRootFolderPicker } from "../composables/useRootFolderPicker";
import { logError } from "../utils/errorLog";
import ConfigErrorBanner from "./ConfigErrorBanner.vue";

const props = defineProps<{ reload: () => Promise<void> | void }>();

const store = useStore();
const { pick } = useRootFolderPicker(store);
const confirmingFresh = ref(false);

async function recreate() {
  try {
    await invoke("create_starter_files", { path: store.rootPath });
    await props.reload();
  } catch (e) {
    logError("RecoveryScreen.recreate", e);
    store.configErrors = [{ kind: "RecreateError", message: `Failed: ${e}` }];
  }
}

async function revealConfig() {
  try {
    await invoke("reveal_config_file", { rootPath: store.rootPath });
  } catch (e) {
    logError("RecoveryScreen.revealConfig", e);
  }
}
</script>

<template>
  <div class="flex flex-col items-center justify-center min-h-screen p-2xl max-w-xl mx-auto text-center">
    <!-- Tier 1: in_place -->
    <template v-if="store.configCategory === 'in_place'">
      <ConfigErrorBanner />
      <button
        data-testid="reveal-config"
        class="mt-lg px-lg py-sm rounded-[var(--radius-form-lg)] bg-blue-600 text-white text-secondary cursor-pointer hover:bg-blue-700"
        @click="revealConfig"
      >
        Reveal config.yaml in Finder
      </button>
    </template>

    <!-- Tier 2: config_missing -->
    <template v-else-if="store.configCategory === 'config_missing'">
      <h1 class="text-title font-bold mb-md text-[var(--color-text-primary)]">Your config.yaml is missing</h1>
      <p class="text-[var(--color-text-secondary)] mb-sm">Your data folder is here, but its config file is gone:</p>
      <code class="text-secondary font-mono bg-[var(--color-danger)]/10 px-sm py-xs rounded mb-md">{{ store.rootPath }}</code>
      <p class="text-[var(--color-text-secondary)] mb-xl">Your records are still in place. Recreate a default config to continue.</p>
      <div class="flex gap-md">
        <button
          data-testid="recreate-config"
          class="px-lg py-sm rounded-[var(--radius-form-lg)] bg-blue-600 text-white text-secondary cursor-pointer hover:bg-blue-700"
          @click="recreate"
        >
          Recreate default config.yaml
        </button>
        <button
          data-testid="choose-folder"
          class="px-lg py-sm rounded-[var(--radius-form-lg)] border border-[var(--color-border)] text-secondary cursor-pointer"
          @click="pick"
        >
          Choose a different folder…
        </button>
      </div>
    </template>

    <!-- Tier 3: root_missing (and fallback) -->
    <template v-else>
      <h1 class="text-title font-bold mb-md text-[var(--color-text-primary)]">Can't find your Logbook folder</h1>
      <p class="text-[var(--color-text-secondary)] mb-sm">Logbook expects your data here, but it isn't available:</p>
      <code class="text-secondary font-mono bg-[var(--color-danger)]/10 px-sm py-xs rounded mb-md">{{ store.rootPath }}</code>
      <p class="text-[var(--color-text-secondary)] mb-xl">
        This can happen if iCloud hasn't finished syncing, the drive isn't mounted, or the folder was
        moved or deleted. Logbook won't create files here automatically, to avoid conflicting with data
        that may still be syncing.
      </p>
      <div class="flex gap-md mb-lg">
        <button
          data-testid="retry"
          class="px-lg py-sm rounded-[var(--radius-form-lg)] bg-blue-600 text-white text-secondary cursor-pointer hover:bg-blue-700"
          @click="props.reload"
        >
          Retry
        </button>
        <button
          data-testid="choose-folder"
          class="px-lg py-sm rounded-[var(--radius-form-lg)] border border-[var(--color-border)] text-secondary cursor-pointer"
          @click="pick"
        >
          Choose a different folder…
        </button>
      </div>

      <div class="text-secondary text-[var(--color-text-secondary)]">
        <button
          v-if="!confirmingFresh"
          data-testid="start-fresh"
          class="underline cursor-pointer"
          @click="confirmingFresh = true"
        >
          Folder was deleted on purpose? Start fresh here
        </button>
        <div v-else class="flex flex-col items-center gap-sm">
          <p>This creates a brand-new empty Logbook at the path above. If your data is only out of sync, do NOT do this.</p>
          <div class="flex gap-md">
            <button
              data-testid="start-fresh-confirm"
              class="px-lg py-sm rounded-[var(--radius-form-lg)] bg-[var(--color-danger)] text-white text-secondary cursor-pointer"
              @click="recreate"
            >
              Yes, create a fresh Logbook
            </button>
            <button
              class="px-lg py-sm rounded-[var(--radius-form-lg)] border border-[var(--color-border)] text-secondary cursor-pointer"
              @click="confirmingFresh = false"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
```

> 若 `--color-border` token 不存在，运行 `pnpm vitest run src/__tests__/tailwind-token-usage.test.ts` 时护栏会报合法替代；按提示改成已存在的边框 token。

- [ ] **Step 4: 运行测试**

Run: `pnpm vitest run src/__tests__/components/RecoveryScreen.test.ts`
Expected: 6 个测试 PASS。
Run: `pnpm vitest run src/__tests__/tailwind-token-usage.test.ts`
Expected: PASS（如失败按报错替换 token，重跑组件测试）。

- [ ] **Step 5: Commit**

```bash
git add src/components/RecoveryScreen.vue src/__tests__/components/RecoveryScreen.test.ts
git commit -m "feat(recovery): RecoveryScreen renders recovery actions by category"
```

---

### Task 9: App.vue 接线 + 修订 App.test.ts

**Files:**
- Modify: `src/App.vue`
- Modify: `src/__tests__/components/App.test.ts`

- [ ] **Step 1: 更新 App.test.ts 受影响测试（先改测试）**

在 `src/__tests__/components/App.test.ts` 中：

1. 把 `"ConfigError: shows ConfigErrorBanner and Retry button"` 测试整体替换为：

```ts
  it("ConfigError in_place: shows RecoveryScreen with error list, no Retry", async () => {
    mockInvoke.mockResolvedValue({
      status: "ConfigError",
      data: {
        category: "in_place",
        root_path: "/test",
        errors: [{ kind: "MissingName", message: "Dimension 0 has an empty name" }],
        scan_warnings: [],
      },
    });
    const { wrapper, store } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    expect(store.status).toBe("error");
    expect(store.configCategory).toBe("in_place");
    expect(wrapper.findComponent({ name: "RecoveryScreen" }).exists()).toBe(true);
    expect(wrapper.text()).toContain("Dimension 0 has an empty name");
    expect(wrapper.text()).not.toContain("Retry");
  });
```

2. 把 `"Retry button re-calls initApp"` 测试整体替换为：

```ts
  it("root_missing: Retry re-calls init", async () => {
    mockInvoke.mockResolvedValue({
      status: "ConfigError",
      data: {
        category: "root_missing",
        root_path: "/test",
        errors: [{ kind: "RootMissing", message: "gone" }],
        scan_warnings: [],
      },
    });
    const { wrapper } = mountApp();
    await vi.runAllTimersAsync();
    await nextTick();

    vi.clearAllMocks();
    mockInvoke.mockResolvedValue({
      status: "Ready",
      data: { root_path: "/test", config: makeConfig(), today: makeDayFile(), commitments: [], scan_warnings: [] },
    });

    await wrapper.get('[data-testid="retry"]').trigger("click");
    await vi.runAllTimersAsync();

    expect(mockInvoke).toHaveBeenCalledWith("init");
  });
```

3. `"config-changed event with errors shows error screen"` 测试，追加 category 断言：

```ts
    expect(store.status).toBe("error");
    expect(store.configCategory).toBe("in_place");
    expect(store.configErrors).toEqual([{ kind: "MissingName", message: "Bad config" }]);
```

4. `"shows scan warning toast when ConfigError has scan_warnings"` 测试，给 mock 的 `data` 补上 `category: "in_place", root_path: "/test",`，并把断言 `findComponent({ name: "ConfigErrorBanner" })` 改为 `findComponent({ name: "RecoveryScreen" })`。

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run src/__tests__/components/App.test.ts`
Expected: 上述用例 FAIL（App.vue 仍渲染旧 banner + 裸 Retry、未写 configCategory、无 data-testid）。

- [ ] **Step 3: 改写 App.vue**

`src/App.vue` `<script setup>`：
1. import 调整：删除 `import ConfigErrorBanner from "./components/ConfigErrorBanner.vue";`，新增：
```ts
import RecoveryScreen from "./components/RecoveryScreen.vue";
import { applyInitResult } from "./utils/applyInitResult";
```
2. 把 `initApp`（`App.vue:87-120`）替换为：

```ts
async function initApp() {
  logInfo("App.initApp", "start");
  try {
    const result = (await invoke("init")) as InitResult;
    const warnings = applyInitResult(store, result);
    if (warnings.length > 0) {
      scanWarnings.value = warnings;
      showScanWarning.value = true;
    }
    logInfo("App.initApp", result.status);
  } catch (e) {
    logError("App.initApp", e);
    store.configErrors = [{ kind: "InitError", message: `Failed: ${e}` }];
    store.configCategory = "root_missing";
    store.status = "error";
  }
}
```

3. `config-changed` 监听（`App.vue:39-46`）的 errors 分支加一行 category：

```ts
    unlistenConfig = await listen<ConfigErrorDetail[]>("config-changed", (event) => {
      if (event.payload.length === 0) {
        initApp();
      } else {
        store.configErrors = event.payload;
        store.configCategory = "in_place"; // watcher live ⇒ root exists ⇒ in-place fix
        store.status = "error";
      }
    });
```

`src/App.vue` `<template>`：把 error 块（`App.vue:161-169`）：

```html
    <template v-else-if="store.status === 'error'">
      <ConfigErrorBanner />
      <button ... @click="initApp">Retry</button>
    </template>
```

替换为：

```html
    <RecoveryScreen v-else-if="store.status === 'error'" :reload="initApp" />
```

- [ ] **Step 4: 运行测试**

Run: `pnpm vitest run src/__tests__/components/App.test.ts`
Expected: 全部 PASS。
Run: `pnpm vitest run`
Expected: 全部前端测试 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/App.vue src/__tests__/components/App.test.ts
git commit -m "feat(recovery): App renders RecoveryScreen; init uses applyInitResult"
```

---

## Phase 7 — 全量校验

### Task 10: 全量构建 + 手动验证

**Files:** 无（仅校验）

- [ ] **Step 1: 全量校验**

Run（仓库根）：`pnpm vue-tsc --noEmit && pnpm vitest run`
Expected: 类型检查通过；所有前端测试 PASS。
Run：`cd src-tauri && cargo check && cargo test`
Expected: 通过 + 全绿。

> 注意（来自项目记忆）：`build` / `vue-tsc` 会对测试文件严格类型检查（`noUnusedLocals`），vitest 绿 ≠ 构建绿。务必跑 `vue-tsc --noEmit`。

- [ ] **Step 2: 手动验证三层（无法自动化的 GUI 路径）**

启动：`pnpm tauri dev`，按下表逐项验证：

| 场景 | 制造方法 | 期望 |
|---|---|---|
| Tier 3 root_missing | 退出 app → 删除整个数据目录 → 启动 | "Can't find your Logbook folder"，有 Retry + Choose folder；Start fresh 需二次确认 |
| Tier 3 恢复（同步回来） | 上一步后把目录放回 → 点 Retry | 进入正常界面；之后外部编辑 config.yaml 能自动检测（验证 watcher 已启动） |
| Tier 2 config_missing | 仅删除 `config.yaml`，保留目录 | "Your config.yaml is missing"；点 Recreate → 恢复正常 |
| Tier 1 in_place（损坏） | 把 config.yaml 写成非法 YAML | 错误清单 + Reveal；无 Retry；改回正确内容后**自动**恢复（不点任何按钮） |
| Tier 1 in_place（校验） | source 写成 bogus | 同上，显示 InvalidSource 类错误 |
| 首次 setup watcher | 全新 root（NeedsSetup）→ 选目录 → 不重启，外部编辑 config.yaml | 自动检测生效（验证 set_root_path 后 watcher 已启动，修掉 latent bug） |

记录每项结果。任一不符 → 回到对应 Task 修复，勿标记完成。

- [ ] **Step 3: 一致性检查提醒**

提醒用户运行 `/check-consistency`（文档↔文档 + 文档↔代码），由用户显式发起。

- [ ] **Step 4: Final commit（如有手动验证引出的微调）**

```bash
git add -A
git commit -m "test(recovery): manual verification pass for 3-tier recovery"
```

---

## 自检：spec 覆盖映射

| spec 要求 | 对应 Task |
|---|---|
| §3 三层模型（含 in_place 含"config 存在但损坏"） | Task 2（`load_root_state` 五分类测试覆盖 malformed→in_place） |
| §4 `RecoveryCategory` + ConfigError 加 category/root_path | Task 1、Task 5 |
| §5.1 init/set_root_path 委托 load_root_state | Task 2 |
| §5.2 复用 create_starter_files | Task 8（Recreate / Start fresh） |
| §5.3 reveal_config_file | Task 3 |
| §5.4 watcher 可重启 + set_root_path 后启动 | Task 4、Task 2（ensure_watcher 调用点） |
| §6.1 RecoveryScreen | Task 8 |
| §6.2 抽取 folder picker + SetupScreen 去字符串匹配 | Task 7 |
| §6.3 store.configCategory + 接线 | Task 5、Task 9 |
| §6.4 文案 | Task 8 |
| §7 测试（Rust 集成 + 前端组件） | Task 2/3/4/6/7/8/9 |
| §8 二次确认（自建轻量） | Task 8（confirmingFresh 两步） |
