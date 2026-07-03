use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Initialize the log path. Call once during app setup.
pub fn init(app_data_dir: &std::path::Path) {
    let log_path = app_data_dir.join("logbook.log");
    {
        let mut path = LOG_PATH.lock().unwrap_or_else(|e| e.into_inner());
        *path = Some(log_path.clone());
    }
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if let Err(e) = append_log("START", "", &format!("Logbook started at {}", timestamp)) {
        eprintln!("[logbook:START] log write failed: {e}");
    }
}

/// Append a line to the log. Non-blocking, best-effort.
fn append_log(level: &str, context: &str, message: &str) -> Result<(), String> {
    let path = LOG_PATH
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = path.as_ref().ok_or("Log not initialized")?;
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("[logbook] failed to create log dir: {e}");
        }
    }
    const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > MAX_LOG_BYTES {
            let old_path = path.with_extension("log.old");
            let _ = std::fs::rename(path, &old_path);
        }
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let context_str = if context.is_empty() {
        String::new()
    } else {
        format!("[{}] ", context)
    };
    writeln!(
        file,
        "[{}] [{:<5}] {}{}",
        timestamp, level, context_str, message
    )
    .map_err(|e| format!("Failed to write log: {}", e))?;
    Ok(())
}

// ---- Public API ----

/// Log an informational message.
pub fn log_info(context: &str, message: &str) {
    if let Err(e) = append_log("INFO", context, message) {
        eprintln!("[logbook:INFO] [{context}] log write failed: {e}");
    }
}

/// Log an error with context.
pub fn log_error(context: &str, error: &str) {
    if let Err(e) = append_log("ERROR", context, error) {
        eprintln!("[logbook:ERROR] [{context}] log write failed: {e}");
    }
}

/// Log a frontend error (called via Tauri command).
pub fn log_frontend_error(message: &str) {
    if let Err(e) = append_log("ERROR", "FRONTEND", message) {
        eprintln!("[logbook:ERROR] [FRONTEND] log write failed: {e}");
    }
}

/// Log a frontend info message (called via Tauri command).
pub fn log_frontend_info(message: &str) {
    if let Err(e) = append_log("INFO", "FRONTEND", message) {
        eprintln!("[logbook:INFO] [FRONTEND] log write failed: {e}");
    }
}

/// Log command entry with serialized args.
pub fn log_command_enter(name: &str, args: &str) {
    if let Err(e) = append_log("CMD>", name, args) {
        eprintln!("[logbook:CMD>] [{name}] log write failed: {e}");
    }
}

/// Log command exit with result summary.
pub fn log_command_exit(name: &str, ok: bool, detail: &str) {
    let status = if ok { "OK" } else { "ERR" };
    if let Err(e) = append_log("CMD<", name, &format!("{} {}", status, detail)) {
        eprintln!("[logbook:CMD<] [{name}] log write failed: {e}");
    }
}

// ---- Panic hook ----

/// Install a panic hook that logs panics before the process aborts.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture();
        let msg = format!("{info}\nBacktrace:\n{bt}");
        eprintln!("FATAL PANIC: {msg}");
        // Bypass the mutex to avoid deadlock if the panic happened while holding it
        let path = LOG_PATH.try_lock().ok().and_then(|p| p.clone());
        if let Some(ref log_path) = path {
            if let Err(e) = (|| -> Result<(), String> {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_path)
                    .map_err(|e| format!("{e}"))?;
                let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                writeln!(file, "[{ts}] [PANIC] {msg}").map_err(|e| format!("{e}"))
            })() {
                eprintln!("[logbook:PANIC] failed to write panic log: {e}");
            }
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_append_log_creates_file() {
        let tmp = std::env::temp_dir().join("logbook_log_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        init(&tmp);
        log_info("test", "info message");
        log_error("test", "error message");
        let log_path = tmp.join("logbook.log");
        assert!(log_path.exists());
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("INFO"));
        assert!(content.contains("ERROR"));
        assert!(content.contains("info message"));
        assert!(content.contains("error message"));
        let _ = fs::remove_dir_all(&tmp);
    }
}
