# Window Size Adjustment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace hardcoded 800x600 window with 90%-of-monitor default on first launch, and restore last-saved position/size on subsequent launches.

**Architecture:** New `window_state` module handles save/restore/validation of window geometry. It reads/writes `window_state.json` in the app's local data directory. `lib.rs` calls restore in `setup` and registers a close handler to persist state on exit. `tauri.conf.json` drops the fixed dimensions.

**Tech Stack:** Tauri 2.x (Rust), `serde_json`, `tauri::window` APIs

---

## File Structure

| File | Role |
|------|------|
| `src-tauri/src/window_state.rs` | **New.** `WindowState` struct, `is_position_valid` (pure, testable), `restore_window_state`, `save_window_state`, `register_save_on_close` |
| `src-tauri/src/lib.rs` | **Modify.** Import module, call restore + register save in `setup` |
| `src-tauri/tauri.conf.json` | **Modify.** Remove hardcoded `width`/`height` |

---

### Task 1: Create `window_state.rs` module

**Files:**
- Create: `src-tauri/src/window_state.rs`

- [ ] **Step 1: Write the module with all functions and unit tests**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persisted window geometry.
#[derive(Debug, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A monitor's bounds in physical coordinates. Used for position validation.
/// Exists for testability — separates geometry math from Tauri types.
#[derive(Debug, PartialEq)]
struct MonitorRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// Check whether a window rect overlaps at least one monitor.
/// Pure function — unit-testable.
fn is_position_valid(monitors: &[MonitorRect], x: i32, y: i32, width: u32, height: u32) -> bool {
    let window_left = x;
    let window_right = x + width as i32;
    let window_top = y;
    let window_bottom = y + height as i32;

    for m in monitors {
        let ml = m.x;
        let mr = m.x + m.width as i32;
        let mt = m.y;
        let mb = m.y + m.height as i32;

        if window_left < mr && window_right > ml && window_top < mb && window_bottom > mt {
            return true;
        }
    }

    false
}

/// Restore saved window state, or fall back to 90% of primary monitor.
pub fn restore_window_state(window: &tauri::WebviewWindow, app_data_dir: &std::path::Path) {
    let state_path = app_data_dir.join("window_state.json");
    let saved_state = std::fs::read_to_string(&state_path)
        .ok()
        .and_then(|s| serde_json::from_str::<WindowState>(&s).ok());

    let monitors: Vec<MonitorRect> = window
        .available_monitors()
        .map(|ms| {
            ms.iter()
                .map(|m| {
                    let pos = m.position();
                    let size = m.size();
                    MonitorRect {
                        x: pos.x,
                        y: pos.y,
                        width: size.width,
                        height: size.height,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    match saved_state {
        Some(state)
            if is_position_valid(&monitors, state.x, state.y, state.width, state.height) =>
        {
            let _ = window.set_size(tauri::PhysicalSize::new(state.width, state.height));
            let _ = window.set_position(tauri::PhysicalPosition::new(state.x, state.y));
        }
        _ => {
            if let Ok(Some(monitor)) = window.primary_monitor() {
                let size = monitor.size();
                let _ = window.set_size(tauri::PhysicalSize::new(
                    (size.width as f64 * 0.9) as u32,
                    (size.height as f64 * 0.9) as u32,
                ));
                // window is centered by default via tauri.conf.json "center": true
            }
        }
    }
}

/// Save current window geometry to disk. No-op if window is maximized
/// (to preserve the pre-maximized state).
pub fn save_window_state(window: &tauri::WebviewWindow, app_data_dir: &std::path::Path) {
    if window.is_maximized().unwrap_or(false) {
        return;
    }

    let (Ok(size), Ok(position)) = (window.outer_size(), window.outer_position()) else {
        return;
    };

    let state = WindowState {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    };

    if let Ok(json) = serde_json::to_string(&state) {
        let path = app_data_dir.join("window_state.json");
        let _ = std::fs::write(&path, json);
    }
}

/// Register a close handler that persists window state on destroy.
pub fn register_save_on_close(window: &tauri::WebviewWindow, app_data_dir: PathBuf) {
    let window = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            save_window_state(&window, &app_data_dir);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_valid_overlapping() {
        let monitors = vec![MonitorRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];
        assert!(is_position_valid(&monitors, 100, 100, 800, 600));
    }

    #[test]
    fn test_position_valid_offscreen() {
        let monitors = vec![MonitorRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];
        // window entirely to the right of the monitor
        assert!(!is_position_valid(&monitors, 2000, 100, 800, 600));
    }

    #[test]
    fn test_position_valid_partial_overlap() {
        let monitors = vec![MonitorRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];
        // window partially off right edge
        assert!(is_position_valid(&monitors, 1500, 100, 800, 600));
    }

    #[test]
    fn test_position_valid_multi_monitor() {
        let monitors = vec![
            MonitorRect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            MonitorRect {
                x: 1920,
                y: 0,
                width: 1440,
                height: 900,
            },
        ];
        // window on second monitor
        assert!(is_position_valid(&monitors, 2000, 100, 800, 600));
    }

    #[test]
    fn test_position_valid_no_monitors() {
        // If we can't enumerate monitors, we assume position is valid
        // (handled by caller — is_position_valid with empty vec returns false)
        assert!(!is_position_valid(&[], 100, 100, 800, 600));
    }

    #[test]
    fn test_position_valid_edge_exact_overlap() {
        let monitors = vec![MonitorRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];
        // window touching the edge — still considered valid (overlap check works)
        assert!(is_position_valid(&monitors, 0, 0, 1920, 1080));
    }
}
```

- [ ] **Step 2: Add `mod window_state;` to `lib.rs`**

In `src-tauri/src/lib.rs`, add the module declaration at the top with the others:

```rust
pub mod commands;
pub mod config;
mod error_log;
pub mod files;
pub mod models;
mod window_state;  // <-- add this line
```

- [ ] **Step 3: Build check**

Run: `cd src-tauri && cargo check`
Expected: Compiles without errors. May have unused import warnings for `tauri` types in the new module — that's fine.

- [ ] **Step 4: Run unit tests**

Run: `cd src-tauri && cargo test -p tauri_app_lib -- window_state`
Expected: 6 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/window_state.rs src-tauri/src/lib.rs
git commit -m "feat: add window_state module with save/restore/validation"
```

---

### Task 2: Integrate into setup hook

**Files:**
- Modify: `src-tauri/src/lib.rs:16-29` (the `setup` closure)

- [ ] **Step 1: Add restore and save-registration calls**

In `src-tauri/src/lib.rs`, update the `setup` closure. Current:

```rust
.setup(|app| {
    error_log::install_panic_hook();
    let app_handle = app.handle().clone();
    let app_data_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    error_log::init(&app_data_dir);
    if let Some(root_path) = files::read_root_path(&app_data_dir) {
        if root_path.exists() {
            files::cleanup_tmp_files(&root_path);
            watch_files(app_handle, root_path);
        }
    }
    Ok(())
})
```

Replace with:

```rust
.setup(|app| {
    error_log::install_panic_hook();
    let app_handle = app.handle().clone();
    let app_data_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    error_log::init(&app_data_dir);

    // Restore window geometry (or 90% default) and persist on close
    if let Some(window) = app.get_webview_window("main") {
        window_state::restore_window_state(&window, &app_data_dir);
        window_state::register_save_on_close(&window, app_data_dir.clone());
    }

    if let Some(root_path) = files::read_root_path(&app_data_dir) {
        if root_path.exists() {
            files::cleanup_tmp_files(&root_path);
            watch_files(app_handle, root_path);
        }
    }
    Ok(())
})
```

- [ ] **Step 2: Build check**

Run: `cd src-tauri && cargo check`
Expected: Compiles without errors.

- [ ] **Step 3: Run all tests to verify no regressions**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS (unit + integration).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: integrate window state restore/save into setup hook"
```

---

### Task 3: Update tauri.conf.json

**Files:**
- Modify: `src-tauri/tauri.conf.json:13-18` (the `windows` array)

- [ ] **Step 1: Remove fixed dimensions, ensure center is true**

Current:

```json
"windows": [
  {
    "title": "Logbook",
    "width": 800,
    "height": 600
  }
],
```

Replace with:

```json
"windows": [
  {
    "title": "Logbook",
    "center": true
  }
],
```

- [ ] **Step 2: Build check**

Run: `cd src-tauri && cargo check`
Expected: Compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat: remove hardcoded window size, let runtime manage geometry"
```

---

### Task 4: Manual verification

- [ ] **Step 1: First launch — verify 90% sizing**

Run: `pnpm tauri dev`
Expected: Window opens at ~90% of primary monitor, centered.

- [ ] **Step 2: Resize and restart — verify restore**

1. Drag and resize the window to an arbitrary position and size.
2. Close the app.
3. Check that `<app_local_data_dir>/window_state.json` exists and contains plausible values.
4. Re-launch with `pnpm tauri dev`.
Expected: Window restores to the same position and size.

- [ ] **Step 3: Maximized close — verify pre-maximized state preserved**

1. Resize window to a specific non-maximized size/position.
2. Maximize the window.
3. Close the app.
4. Check `window_state.json` — it should NOT have the maximized dimensions (should still hold the pre-maximized state).
5. Re-launch.
Expected: Window restores to the pre-maximized size/position, not fullscreen.

- [ ] **Step 4: Delete state — verify fallback**

1. Delete `<app_local_data_dir>/window_state.json`.
2. Re-launch.
Expected: Falls back to 90% of primary monitor, centered.
