use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Persisted window geometry — stored in logical (DPI-independent) coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// In-memory cache updated on every move/resize. Written to disk on app exit.
static CACHED_STATE: Mutex<Option<WindowState>> = Mutex::new(None);

/// A monitor's bounds in logical coordinates. Used for position validation.
#[derive(Debug, PartialEq)]
struct MonitorRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// Check whether a window rect overlaps at least one monitor.
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

fn current_monitors(window: &tauri::WebviewWindow) -> Vec<MonitorRect> {
    let scale = window.scale_factor().unwrap_or(1.0);
    window
        .available_monitors()
        .map(|ms| {
            ms.iter()
                .map(|m| {
                    let pos = m.position();
                    let size = m.size();
                    MonitorRect {
                        x: (pos.x as f64 / scale) as i32,
                        y: (pos.y as f64 / scale) as i32,
                        width: (size.width as f64 / scale) as u32,
                        height: (size.height as f64 / scale) as u32,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Restore saved window state from disk, or fall back to 90% of primary monitor.
pub fn restore_window_state(window: &tauri::WebviewWindow, app_data_dir: &std::path::Path) {
    let state_path = app_data_dir.join("window_state.json");
    let saved_state = std::fs::read_to_string(&state_path)
        .ok()
        .and_then(|s| serde_json::from_str::<WindowState>(&s).ok());

    let monitors = current_monitors(window);

    match saved_state {
        Some(state)
            if is_position_valid(&monitors, state.x, state.y, state.width, state.height) =>
        {
            let _ = window.set_size(tauri::LogicalSize::new(
                state.width as f64,
                state.height as f64,
            ));
            let _ = window.set_position(tauri::LogicalPosition::new(
                state.x as f64,
                state.y as f64,
            ));
        }
        _ => {
            if let Ok(Some(monitor)) = window.primary_monitor() {
                let scale = window.scale_factor().unwrap_or(1.0);
                let size = monitor.size();
                let logical_w = size.width as f64 / scale;
                let logical_h = size.height as f64 / scale;
                let mon_pos = monitor.position();
                let logical_mx = mon_pos.x as f64 / scale;
                let logical_my = mon_pos.y as f64 / scale;

                let new_w = logical_w * 0.9;
                let new_h = logical_h * 0.9;
                let x = logical_mx + (logical_w - new_w) / 2.0;
                let y = logical_my + (logical_h - new_h) / 2.0;
                let _ = window.set_size(tauri::LogicalSize::new(new_w, new_h));
                let _ = window.set_position(tauri::LogicalPosition::new(x, y));
            }
        }
    }
}

/// Start tracking window geometry changes. Updates the in-memory cache
/// on every move/resize so the latest state is available at exit time.
pub fn register_state_tracking(window: &tauri::WebviewWindow) {
    let w = window.clone();
    w.clone().on_window_event(move |event| {
        match event {
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
                update_cache(&w);
            }
            _ => {}
        }
    });
}

/// Read current window geometry (physical) and convert to logical for the cache.
fn update_cache(window: &tauri::WebviewWindow) {
    if let Ok(true) = window.is_maximized() {
        return; // don't cache maximized state
    }
    let scale = window.scale_factor().unwrap_or(1.0);
    if let (Ok(size), Ok(position)) = (window.outer_size(), window.outer_position()) {
        let state = WindowState {
            x: (position.x as f64 / scale) as i32,
            y: (position.y as f64 / scale) as i32,
            width: (size.width as f64 / scale) as u32,
            height: (size.height as f64 / scale) as u32,
        };
        if let Ok(mut cache) = CACHED_STATE.lock() {
            *cache = Some(state);
        }
    }
}

/// Write the cached window state to disk. Called on app exit.
pub fn flush_to_disk(app_data_dir: &std::path::Path) {
    let state = CACHED_STATE
        .lock()
        .ok()
        .and_then(|c| c.clone());
    if let Some(state) = state {
        if let Ok(json) = serde_json::to_string(&state) {
            let path = app_data_dir.join("window_state.json");
            let _ = std::fs::write(&path, json);
        }
    }
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
        assert!(is_position_valid(&monitors, 2000, 100, 800, 600));
    }

    #[test]
    fn test_position_valid_no_monitors() {
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
        assert!(is_position_valid(&monitors, 0, 0, 1920, 1080));
    }
}
