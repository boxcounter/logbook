# 初始窗口尺寸调整

## 问题

启动 App 后窗口 800x600 太小，Logbook 作为时间追踪工具，内容空间明显不足。

## 方案

启动时窗口按主显示器尺寸的 90% 自动调整，之后用户手动 resize 不做动态适配。

## 改动

### 1. `src-tauri/src/lib.rs` — setup 钩子中设置窗口尺寸

```rust
let window = app.get_webview_window("main").unwrap();
if let Ok(Some(monitor)) = window.primary_monitor() {
    let size = monitor.size();
    let _ = window.set_size(tauri::LogicalSize::new(
        size.width as f64 * 0.9,
        size.height as f64 * 0.9,
    ));
}
```

### 2. `src-tauri/tauri.conf.json` — 删除固定 width/height

移除 `app.windows[0]` 中的 `"width": 800` 和 `"height": 600`。窗口先以默认尺寸短暂闪现，setup 中立即调整为 90%。加 `"center": true` 确保居中。

## 涉及文件

| 文件 | 改动 |
|------|------|
| `src-tauri/src/lib.rs` | 在 `setup` 闭包内添加窗口尺寸设置逻辑 |
| `src-tauri/tauri.conf.json` | 删除固定 width/height，添加 center: true |

## 不做的

- 不监听 `tauri::window` 事件做多显示器动态适配
- 不保存/恢复上次窗口尺寸
