# 初始窗口尺寸调整

## 问题

启动 App 后窗口 800x600 太小，Logbook 作为时间追踪工具，内容空间明显不足。

## 方案

**首次启动**（无保存状态）：窗口按主显示器尺寸的 90% 打开。

**后续启动**：恢复上次关闭时的窗口位置和大小。关闭时保存窗口状态到文件，启动时读取并恢复。

## 数据存储

窗口状态保存为 JSON 文件：`<app_local_data_dir>/window_state.json`

```json
{
  "x": 100,
  "y": 80,
  "width": 1512,
  "height": 864
}
```

## 行为细节

### 恢复（`setup` 钩子中）

1. 尝试读 `window_state.json`
2. 如果存在且位置有效（至少部分在当前连接的某个显示器区域内），恢复该尺寸和位置
3. 如果不存在或无效（如上次用外接显示器、现已断开），fallback 到 90% 主显示器尺寸并居中

### 保存（关闭时）

1. 监听窗口关闭事件（`destroyed`）
2. 记录当前尺寸和位置写入 `window_state.json`
3. 如果窗口处于最大化状态，不覆盖保存（避免下次启动直接最大化——还是恢复之前的窗口模式尺寸）

## 改动

### 1. `src-tauri/src/lib.rs` — setup 钩子 + 窗口事件

- 在 `setup` 中调用 `restore_window_state(app_handle)` 恢复窗口
- 注册窗口关闭事件，保存状态

### 2. 新增 `src-tauri/src/window_state.rs` — 窗口状态管理模块

- `restore_window_state(app_handle)` — 读取文件、验证位置有效性、恢复或 fallback
- `save_window_state(window)` — 获取当前尺寸/位置，写入文件
- `is_position_valid(x, y, width, height)` — 检查窗口是否至少部分可见

### 3. `src-tauri/tauri.conf.json`

- 删除固定 `width` / `height`
- 保留 `"center": true` 作为未保存状态时的后备

## 涉及文件

| 文件 | 改动 |
|------|------|
| `src-tauri/src/lib.rs` | setup 中调用 restore，注册关闭事件 |
| `src-tauri/src/window_state.rs` | 新增，保存/恢复/验证逻辑 |
| `src-tauri/tauri.conf.json` | 删除固定 width/height |

## 不做的

- 不监听 resize/move 做实时保存（仅关闭时保存一次）
- 不监听多显示器动态适配
