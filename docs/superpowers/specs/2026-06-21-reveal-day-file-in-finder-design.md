# 设计：点击路径指示器 → 在文件管理器中显示

**日期**：2026-06-21
**状态**：已批准，待实现

## 背景与问题

`MonthView` 右下角有一个路径指示器（`MonthView.vue:377-383`），显示当前所选日的相对路径（如 `…/2026/06/2026-06-21.md`）。当前点击它会调用 `open_in_editor` 命令，**用系统默认编辑器打开那个 `.md` 文件**。

用户想要的是「感知数据保存在哪个目录、方便管理」。打开单个文件无助于此——它跳过了目录本身。更有用的行为是：在文件管理器（Finder / Explorer / Linux 文件管理器）里**打开文件所在目录，并选中该文件**，让用户直接看到数据所在的文件夹及其上下文。

本设计只改这一个点击行为，不改显示文本，不引入切换数据目录等更大范围的能力（那些在 2026-06-21 的范围讨论中被显式排除）。

## 目标 / 非目标

**目标**
- 点击路径指示器，在文件管理器中打开当天日文件所在目录，并选中该文件。
- 日文件不存在时（如今天尚无记录、查看空白历史日），退而打开最近的存在目录。

**非目标**
- 不改路径指示器的显示文本与 hover tooltip（仍是 `…/YYYY/MM/YYYY-MM-DD.md` + 完整路径）。
- 不增加在应用内切换 / 选择数据目录的能力。
- 不展开 root 目录的显示（仍折叠为 `…/`）。

## 方案：逻辑放后端，复用已装的 opener 插件

`@tauri-apps/plugin-opener` / `tauri-plugin-opener`（v2.5.4，已是依赖）自带跨平台的 reveal 能力，无需自写 per-OS 的 `open -R` / `explorer /select` 分支：

- JS：`revealItemInDir(path)`、`openPath(path)`（已验证存在于 `node_modules/@tauri-apps/plugin-opener/dist-js/index.d.ts`）。
- Rust：`app.opener().reveal_item_in_dir(p)`、`app.opener().open_path(path, with)`（trait `tauri_plugin_opener::OpenerExt`）。
- 所需权限 `opener:allow-reveal-item-in-dir` 已包含在 `opener:default` 中，而 `opener:default` 已在 `src-tauri/capabilities/default.json` 内。**无需改 capability。**

逻辑放后端的理由：判断「文件 / 目录是否存在」是文件系统操作，前端未装 fs plugin（仅 dialog + opener），为一次 `exists()` 引入 fs plugin 不划算。后端做判断天然且可单测。

### 决策与副作用分离

把可测的决策抽成纯函数，与不可测的 OS 副作用分开：

- `resolve_reveal_target(root, date)` —— 纯逻辑（仅做 `Path::exists` 判断），返回目标路径 + 是否选中。可在毫秒级测试里覆盖全部分支。
- 真正调用文件管理器的那一行 —— 含副作用、无法在 CI 断言，但它不含逻辑。

## 行为规格

`resolve_reveal_target(root: &Path, date: &str) -> RevealTarget`，其中 `date` 已通过 `validate_date_format`（`YYYY-MM-DD`）：

| 情况 | 目标 `path` | `select` | 文件管理器表现 |
|---|---|---|---|
| 日文件 `root/YYYY/MM/YYYY-MM-DD.md` 存在 | 该文件 | `true` | 打开月目录并选中该文件 |
| 文件不存在，但月目录 `root/YYYY/MM/` 存在 | 月目录 | `false` | 打开月目录 |
| 月目录也不存在 | 数据根 `root` | `false` | 打开数据根目录 |

`root` 自身假定存在（`set_root_path` 入库时已校验）；若被外部删除，`open_path` 会报错并由前端 `logError` 记录——不特殊处理。

## 实现细节

### 后端 `src-tauri/src/commands.rs`

- 新增 `use tauri_plugin_opener::OpenerExt;`
- 模块内部结构体（不跨 IPC、不加 serde、不动 `models.rs`）：
  ```rust
  struct RevealTarget { path: std::path::PathBuf, select: bool }
  ```
- 纯函数：
  ```rust
  fn resolve_reveal_target(root: &std::path::Path, date: &str) -> RevealTarget
  ```
  按上表实现：`year = &date[0..4]`、`month = &date[5..7]`（date 已校验格式）；`file = root/year/month/{date}.md`。
- 删除 `open_in_editor`，新增命令：
  ```rust
  #[tauri::command]
  pub fn reveal_day_file(app: AppHandle, root_path: String, date: String) -> Result<(), String>
  ```
  流程：`error_log::log_command_enter` → `validate_date_format(&date)?` → `resolve_reveal_target` → `select` 真则 `app.opener().reveal_item_in_dir(&t.path)`，否则 `app.opener().open_path(t.path.to_string_lossy().into_owned(), None::<String>)` → 错误 `map_err(|e| e.to_string())` → `log_command_exit`。

> `open_path` 第二参必须写 `None::<String>`：其签名为 `Option<impl Into<String>>`，裸 `None` 推不出具体类型，会编译失败。

### 注册 `src-tauri/src/lib.rs`

- `commands::open_in_editor`（约 line 65）→ `commands::reveal_day_file`。

### 前端 `src/components/MonthView.vue`

- 函数 `openInEditor` → `revealDayFile`。
- `invoke("open_in_editor", { rootPath, date })` → `invoke("reveal_day_file", { rootPath, date })`。
- 模板 `@click="openInEditor"` → `@click="revealDayFile"`。
- `displayPath`、`dayFilePath`、按钮 `title`（完整路径 tooltip）保持不变。

### 文档 `SPEC.md`

- line 33 的 `open_in_editor(root_path, date) → Result<(), String>  // 用系统编辑器打开文件`
  改为 `reveal_day_file(root_path, date) → Result<(), String>  // 在文件管理器中打开目录并选中日文件`。

### 不改动

- 历史记录 `docs/superpowers/plans/*` 与 `docs/superpowers/specs/2026-06-15-*` 中的旧引用是时点快照，不编辑。

## 命令改名的同步要求

命令名是 IPC 契约的一部分。`open_in_editor` → `reveal_day_file` 必须在 5 处 live 引用同步，漏改任一处不会编译报错，而是运行时 `Unknown/Unmocked command`：

1. `src-tauri/src/commands.rs`（定义）
2. `src-tauri/src/lib.rs`（`invoke_handler` 注册）
3. `src/components/MonthView.vue`（调用）
4. `src/__tests__/mocks/tauri.ts`（测试 mock，line 43 的 `case "open_in_editor"`）
5. `SPEC.md`（文档）

## 测试

- **单元测试**（`commands.rs` 内 `#[cfg(test)] mod tests`，用 `std::env::temp_dir()` 建临时目录，仿 `test_read_day_file_safe_corrupt`）：
  - 文件存在 → `path` 指向该文件且 `select == true`。
  - 文件不存在、月目录存在 → `path` 指向月目录且 `select == false`。
  - 文件与月目录都不存在 → `path == root` 且 `select == false`。
  - 测试后清理临时目录。
- **前端**：`__tests__/mocks/tauri.ts` 的 case 改名后，现有测试套件应保持绿色（无组件测试断言该按钮的点击）。
- **验证命令**：`pnpm vue-tsc --noEmit && cd src-tauri && cargo check && cargo test`（即 stop hook）。设计系统 token 护栏测试不受影响（本改动不碰间距 / 字号 class）。

## 风险

- `reveal_item_in_dir` 对不存在路径会报错——已由 `resolve_reveal_target` 的存在性兜底消除。
- Linux 上「选中文件」依赖文件管理器对 freedesktop `FileManager1` 接口的支持；不支持时插件行为降级，属插件能力边界，不在本设计处理范围。
