# Menu Data Directory Actions

**Date**: 2026-07-05
**Status**: Design

## Motivation

在 Logbook 菜单中增加两个操作：复制用户数据路径到剪贴板、在 Finder 中打开用户数据目录，同时移除窗口底部当前日期文件路径的显示组件。

## Design

### 1. Menu Structure

文件：`src-tauri/src/lib.rs`，`setup` hook 菜单构建块。

两个新 `MenuItem` 放置在 Logbook 菜单的 "Install Command Line Tool…" 和 "Services" 之间，上下各用分隔线形成一个"数据目录"分组：

```
Logbook
├── About Logbook
├── ──────────────
├── Install Command Line Tool…
├── ──────────────
├── Copy User Data Path
├── Open User Data Directory
├── ──────────────
├── Services
├── ──────────────
├── Hide Logbook
├── Hide Others
├── Show All
├── ──────────────
├── Quit Logbook
```

菜单项 id：`copy-data-path`、`open-data-dir`。无快捷键。Edit 和 Window 菜单不变。

### 2. Behavior (Rust)

两个菜单项的事件处理在 `lib.rs` 的 `on_menu_event` 闭包中实现，不需要 IPC 或前端参与。`MenuItem` 实例 clone 进闭包（改为 `move` 闭包）。

#### Copy User Data Path

1. 通过 `files::read_root_path(&app_data_dir)` 读取 `root_path.txt`
2. 若 `None`：弹出 error dialog（`MessageDialogKind::Error`），文案 `"No data directory configured."`，无文本翻转
3. 若 `Some(path)`：
   - `std::process::Command::new("pbcopy").stdin(Stdio::piped())` 写入路径字符串
   - 若 `pbcopy` 成功：`copy_item.set_text("Copied!")`
   - 若 `pbcopy` 失败：`copy_item.set_text("Copy failed")`，记录 `error_log`
   - `std::thread::spawn` 一个线程 sleep 1.5 秒后 `set_text("Copy User Data Path")` 还原

#### Open User Data Directory

1. 同上读取 `root_path.txt`
2. 若 `None`：与 Copy 相同的 error dialog
3. 若 `Some(path)`：`std::process::Command::new("open").arg(path).spawn()` 打开 Finder
4. 无文本翻转；若 `open` 失败，记录 `error_log`（不弹 dialog——macOS 会自行提示 "The folder can't be found"）

### 3. Front-end Cleanup

| 文件 | 变更 |
|------|------|
| `src/components/MonthView.vue` | 删除路径栏模板（L226–233）；删除 `useFileActions` import（L8）和解构（L58） |
| `src/composables/useFileActions.ts` | 整个文件删除 |

`useFileActions.ts` 仅被 `MonthView.vue` 使用。Rust command `reveal_day_file` 保留不动——command 无副作用，保留供未来复用无额外成本。

### 4. Edge Cases & Error Handling

- **未配置 root_path**（`root_path.txt` 不存在，Setup/Recovery 状态）：两个菜单项仍可点击，`read_root_path` 返回 `None` → error dialog，菜单文本不翻转
- **root_path 目录已被删除**：`open` 失败由 macOS Finder 自行提示；`pbcopy` 复制路径字符串不受目录存在影响
- **`pbcopy` 系统调用失败**（极端情况）：`set_text("Copy failed")` → 1.5s 还原，记录 error_log
- **多线程安全**：Tauri 2.x `MenuItem` 为 `Send + Sync`，`set_text` 可安全跨线程调用

### 5. Testing

- **Rust 单元测试**：`pbcopy`/`open` 为系统调用不在此覆盖。`read_root_path` + `None` 分支逻辑本身已在 `files.rs` 中通过集成测试覆盖
- **手动验证清单**：
  1. 正常流程：Copy → 粘贴到文本编辑器确认路径正确 → 1.5s 后菜单文本还原为 "Copy User Data Path"
  2. Open 菜单项 → Finder 打开 root_path 目录
  3. 未设置 root_path 时 → 两个菜单项弹出 error dialog（"No data directory configured."）
  4. 底部路径栏已移除，`reveal_day_file` command 不受影响
