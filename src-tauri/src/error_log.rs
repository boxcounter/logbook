use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Initialize the error log path. Call once during app setup.
pub fn init(app_data_dir: &std::path::Path) {
    let log_path = app_data_dir.join("error.log");
    if let Ok(mut path) = LOG_PATH.lock() {
        *path = Some(log_path.clone());
    }
    // Write a startup marker
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = append_log(&format!("--- Logbook started at {} ---", timestamp));
}

/// Append a line to the error log. Non-blocking, best-effort.
fn append_log(line: &str) -> Result<(), String> {
    let path = LOG_PATH.lock().map_err(|e| format!("Lock error: {}", e))?;
    let path = path.as_ref().ok_or("Log not initialized")?;
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Failed to open error.log: {}", e))?;
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    writeln!(file, "[{}] {}", timestamp, line)
        .map_err(|e| format!("Failed to write error.log: {}", e))?;
    Ok(())
}

/// Log a Rust error with context.
pub fn log_error(context: &str, error: &str) {
    let _ = append_log(&format!("ERROR [{}] {}", context, error));
}

/// Log a frontend error (called via Tauri command).
pub fn log_frontend_error(message: &str) {
    let _ = append_log(&format!("FRONTEND {}", message));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_append_log_creates_file() {
        let tmp = std::env::temp_dir().join("logbook_error_log_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        init(&tmp);
        append_log("test message").unwrap();
        let log_path = tmp.join("error.log");
        assert!(log_path.exists());
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("test message"));
        let _ = fs::remove_dir_all(&tmp);
    }
}
