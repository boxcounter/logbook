/// Compute default window geometry: 90% of primary monitor, centered, in logical coordinates.
/// Returns (width, height, x, y).
pub fn default_window_geometry(app_handle: &tauri::AppHandle) -> (u32, u32, i32, i32) {
    if let Ok(Some(monitor)) = app_handle.primary_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let pos = monitor.position();
        let logical_w = size.width as f64 / scale;
        let logical_h = size.height as f64 / scale;
        let logical_x = pos.x as f64 / scale;
        let logical_y = pos.y as f64 / scale;

        let w = (logical_w * 0.9) as u32;
        let h = (logical_h * 0.9) as u32;
        let x = (logical_x + (logical_w - w as f64) / 2.0) as i32;
        let y = (logical_y + (logical_h - h as f64) / 2.0) as i32;
        return (w, h, x, y);
    }
    (800, 600, 0, 0)
}
