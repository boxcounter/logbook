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
    let w = window.clone();
    w.clone().on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            save_window_state(&w, &app_data_dir);
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
        // If we can't enumerate monitors, assume position is valid
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
        // window filling the entire monitor
        assert!(is_position_valid(&monitors, 0, 0, 1920, 1080));
    }
}
