use crate::config::{validate_config, validate_monthly};
use crate::error_log;
use crate::files::{self, save_root_path, read_root_path};
use crate::models::*;
use chrono::Datelike;
use regex::Regex;
use std::sync::LazyLock;
use tauri::{AppHandle, Manager};

static DURATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d+(?:\.\d+)?)\s*(h|m|H|M)?").unwrap()
});

/// Read a day file, distinguishing "file not found" from "file found but corrupt".
fn read_day_file_safe(root: &std::path::Path, date: &str) -> Result<DayFile, String> {
    let path = files::day_path(root, date)?;
    match files::read_day_file(root, date) {
        Ok(day_file) => Ok(day_file),
        Err(e) => {
            if path.exists() {
                Err(format!(
                    "Day file exists but cannot be parsed: {}. File: {}. Manual recovery needed.",
                    e,
                    path.display()
                ))
            } else {
                Ok(DayFile { note: None, entries: vec![] })
            }
        }
    }
}

/// Read a monthly file, distinguishing "file not found" from "file found but corrupt".
fn read_monthly_file_safe(root: &std::path::Path, year: i32, month: u32) -> Result<MonthlyFile, String> {
    let path = files::monthly_path(root, year, month);
    match files::read_monthly_file(root, year, month) {
        Ok(mf) => Ok(mf),
        Err(e) => {
            if path.exists() {
                Err(format!(
                    "Monthly file exists but cannot be parsed: {}. File: {}. Manual recovery needed.",
                    e,
                    path.display()
                ))
            } else {
                Ok(MonthlyFile { commitments: vec![] })
            }
        }
    }
}

/// Parse a duration string to minutes.
/// Handles: "90", "1.5h", "30m", "1h 30m", "准备会议（15m），面聊（45m）"
pub fn parse_duration(input: &str) -> Result<u32, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Duration is empty".to_string());
    }

    // Try plain number first
    if let Ok(n) = input.parse::<u32>() {
        if n > 0 { return Ok(n); }
        return Err("Duration must be positive".to_string());
    }

    // Try float (e.g. "1.5")
    if let Ok(n) = input.parse::<f64>() {
        if n > 0.0 { return Ok(n.round() as u32); }
        return Err("Duration must be positive".to_string());
    }

    // Scan for all duration patterns (compiled once via LazyLock)
    let re = &*DURATION_RE;
    let mut total: f64 = 0.0;
    let mut matched = false;

    for cap in re.captures_iter(input) {
        let value: f64 = cap[1].parse().unwrap_or(0.0);
        let unit = cap.get(2).map(|m| m.as_str().to_lowercase()).unwrap_or_else(|| "m".to_string());
        match unit.as_str() {
            "h" => { total += value * 60.0; matched = true; }
            "m" | "" => { total += value; matched = true; }
            _ => {}
        }
    }

    if !matched {
        return Err(format!("Could not parse duration from '{}'. Examples: 1.5h, 30m, 45", input));
    }

    let total = total.round() as u32;
    if total == 0 { return Err("Parsed duration is zero".to_string()); }
    Ok(total)
}

/// Validate that all required dimensions have values in the entry.
/// Returns Ok(()) or Err with a human-readable message naming the first missing required dimension.
pub fn validate_required_dimensions(
    config: &Config,
    dimensions: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    for dim in &config.dimensions {
        if dim.required && !dimensions.contains_key(&dim.key) {
            return Err(format!("Missing required dimension: {}", dim.name));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn init(app: AppHandle) -> InitResult {
    error_log::log_command_enter("init", "");
    let app_data_dir = app.path().app_local_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let root_path = match read_root_path(&app_data_dir) {
        Some(p) => p,
        None => {
            error_log::log_command_exit("init", true, "NeedsSetup");
            return InitResult::NeedsSetup;
        }
    };

    let root = std::path::Path::new(&root_path);

    let config = match files::read_config(root) {
        Ok(c) => c,
        Err(e) => {
            error_log::log_error("init: read_config", &e);
            error_log::log_command_exit("init", false, "ConfigReadError");
            return InitResult::ConfigError(vec![ConfigErrorDetail {
                kind: "ConfigReadError".to_string(), message: e,
            }]);
        }
    };

    let mut all_errors = validate_config(&config);

    let now = chrono::Local::now();
    let monthly = match read_monthly_file_safe(root, now.year(), now.month()) {
        Ok(mf) => mf,
        Err(e) => {
            error_log::log_error("init: read_monthly_file", &e);
            all_errors.push(ConfigErrorDetail {
                kind: "MonthlyFileCorrupt".to_string(),
                message: e,
            });
            MonthlyFile { commitments: vec![] }
        }
    };
    all_errors.extend(validate_monthly(&monthly));

    let today_date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());
    let today = match read_day_file_safe(root, &today_date) {
        Ok(df) => df,
        Err(e) => {
            error_log::log_error("init: read_day_file", &e);
            all_errors.push(ConfigErrorDetail {
                kind: "DayFileCorrupt".to_string(),
                message: e,
            });
            DayFile { note: None, entries: vec![] }
        }
    };

    if !all_errors.is_empty() {
        error_log::log_command_exit("init", false, &format!("{} config errors", all_errors.len()));
        return InitResult::ConfigError(all_errors);
    }

    error_log::log_command_exit("init", true, &format!("Ready, {} entries today", today.entries.len()));
    InitResult::Ready { root_path: root_path.to_string_lossy().into_owned(), config, today, commitments: monthly.commitments }
}

#[tauri::command]
pub fn set_root_path(app: AppHandle, path: String) -> Result<InitResult, String> {
    error_log::log_command_enter("set_root_path", &format!("path={}", path));
    let app_data_dir = app.path().app_local_data_dir().map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let root_path = std::path::Path::new(&path);
    if !root_path.exists() { return Err(format!("Path does not exist: {}", path)); }
    if !root_path.is_dir() { return Err(format!("Path is not a directory: {}", path)); }

    save_root_path(&app_data_dir, root_path)?;

    let config = files::read_config(root_path).map_err(|e| {
        error_log::log_error("set_root_path: read_config", &e);
        format!("Failed to read config: {}", e)
    })?;
    let mut all_errors = validate_config(&config);

    let now = chrono::Local::now();
    let monthly = match read_monthly_file_safe(root_path, now.year(), now.month()) {
        Ok(mf) => mf,
        Err(e) => {
            error_log::log_error("set_root_path: read_monthly_file", &e);
            all_errors.push(ConfigErrorDetail {
                kind: "MonthlyFileCorrupt".to_string(),
                message: e,
            });
            MonthlyFile { commitments: vec![] }
        }
    };
    all_errors.extend(validate_monthly(&monthly));

    let today_date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());
    let today = match read_day_file_safe(root_path, &today_date) {
        Ok(df) => df,
        Err(e) => {
            error_log::log_error("set_root_path: read_day_file", &e);
            all_errors.push(ConfigErrorDetail {
                kind: "DayFileCorrupt".to_string(),
                message: e,
            });
            DayFile { note: None, entries: vec![] }
        }
    };

    if !all_errors.is_empty() {
        error_log::log_command_exit("set_root_path", true, &format!("{} config errors", all_errors.len()));
        return Ok(InitResult::ConfigError(all_errors));
    }

    error_log::log_command_exit("set_root_path", true, "Ready");
    Ok(InitResult::Ready { root_path: path.clone(), config, today, commitments: monthly.commitments })
}

#[tauri::command]
pub fn get_entries(root_path: String, date: String) -> Result<DayFile, String> {
    error_log::log_command_enter("get_entries", &format!("date={}", date));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let result = files::read_day_file(root, &date);
    let ok = result.is_ok();
    let entry_count = result.as_ref().map(|d| d.entries.len()).unwrap_or(0);
    error_log::log_command_exit("get_entries", ok, &format!("{} entries", entry_count));
    result
}

#[tauri::command]
pub fn append_entry(root_path: String, date: String, entry: NewEntry) -> Result<Entry, String> {
    error_log::log_command_enter("append_entry", &format!("date={} item={} dur={}", date, entry.item, entry.duration));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let duration = parse_duration(&entry.duration)?;
    let config = files::read_config(root)?;
    validate_required_dimensions(&config, &entry.dimensions)?;
    let entry = Entry {
        id: uuid::Uuid::new_v4().to_string(),
        item: entry.item,
        duration,
        dimensions: entry.dimensions,
    };
    let result = files::append_to_day_file(root, &date, &entry);
    let ok = result.is_ok();
    error_log::log_command_exit("append_entry", ok, &format!("id={}", entry.id));
    result
}

#[tauri::command]
pub fn update_entry(root_path: String, date: String, entry_id: String, update: UpdateEntry) -> Result<DayFile, String> {
    error_log::log_command_enter("update_entry", &format!("date={} id={}", date, entry_id));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    if let Some(ref dur_str) = update.duration {
        parse_duration(dur_str)?;
    }
    if let Some(ref dims) = update.dimensions {
        let config = files::read_config(root)?;
        validate_required_dimensions(&config, dims)?;
    }
    let result = files::update_entry_in_file(root, &date, &entry_id, &update);
    let ok = result.is_ok();
    error_log::log_command_exit("update_entry", ok, &format!("{} entries", result.as_ref().map(|d| d.entries.len()).unwrap_or(0)));
    result
}

#[tauri::command]
pub fn delete_entry(root_path: String, date: String, entry_id: String) -> Result<DayFile, String> {
    error_log::log_command_enter("delete_entry", &format!("date={} id={}", date, entry_id));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let result = files::delete_entry_from_file(root, &date, &entry_id);
    let ok = result.is_ok();
    error_log::log_command_exit("delete_entry", ok, "");
    result
}

#[tauri::command]
pub fn set_day_note(root_path: String, date: String, note: String) -> Result<DayFile, String> {
    error_log::log_command_enter("set_day_note", &format!("date={}", date));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let result = files::set_day_note_in_file(root, &date, &note);
    let ok = result.is_ok();
    error_log::log_command_exit("set_day_note", ok, "");
    result
}

#[tauri::command]
pub fn get_commitments(root_path: String, year: i32, month: u32) -> Result<Vec<Commitment>, String> {
    error_log::log_command_enter("get_commitments", &format!("{}-{:02}", year, month));
    let root = std::path::Path::new(&root_path);
    let result = files::read_monthly_file(root, year, month).map(|m| m.commitments);
    let ok = result.is_ok();
    let count = result.as_ref().map(|c| c.len()).unwrap_or(0);
    error_log::log_command_exit("get_commitments", ok, &format!("{} commitments", count));
    result
}

fn validate_date_format(date: &str) -> Result<chrono::NaiveDate, String> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}. Expected YYYY-MM-DD", date, e))
}

#[tauri::command]
pub fn open_in_editor(root_path: String, date: String) -> Result<(), String> {
    use std::process::Command;
    error_log::log_command_enter("open_in_editor", &format!("date={}", date));
    let parsed = validate_date_format(&date)?;
    let root = std::path::Path::new(&root_path);
    let year = format!("{}", parsed.year());
    let month = format!("{:02}", parsed.month());
    let file_path = root.join(&year).join(&month).join(format!("{}.md", date));
    if !file_path.exists() {
        error_log::log_command_exit("open_in_editor", false, "file not found");
        return Err(format!("File not found: {}", file_path.display()));
    }
    #[cfg(target_os = "macos")]
    { Command::new("open").arg(&file_path).spawn().map_err(|e| format!("Failed to open: {}", e))?; }
    #[cfg(target_os = "linux")]
    { Command::new("xdg-open").arg(&file_path).spawn().map_err(|e| format!("Failed to open: {}", e))?; }
    #[cfg(target_os = "windows")]
    { Command::new("cmd").arg("/c").arg("start").arg("").arg(&file_path).spawn().map_err(|e| format!("Failed to open: {}", e))?; }
    error_log::log_command_exit("open_in_editor", true, "");
    Ok(())
}

#[tauri::command]
pub fn create_starter_files(path: String) -> Result<(), String> {
    let root = std::path::Path::new(&path);
    if !root.exists() {
        std::fs::create_dir_all(root)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let config_path = root.join("config.yaml");
    if !config_path.exists() {
        std::fs::write(&config_path, "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n")
            .map_err(|e| format!("Failed to write config.yaml: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn log_error(message: String) {
    crate::error_log::log_frontend_error(&message);
}

#[tauri::command]
pub fn log_info(message: String) {
    crate::error_log::log_frontend_info(&message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_plain_number() { assert_eq!(parse_duration("90").unwrap(), 90); }

    #[test]
    fn test_parse_duration_float() { assert_eq!(parse_duration("1.5").unwrap(), 2); }

    #[test]
    fn test_parse_duration_hours() { assert_eq!(parse_duration("1.5h").unwrap(), 90); }

    #[test]
    fn test_parse_duration_minutes() { assert_eq!(parse_duration("30m").unwrap(), 30); }

    #[test]
    fn test_parse_duration_compound() { assert_eq!(parse_duration("1h 30m").unwrap(), 90); }

    #[test]
    fn test_parse_duration_embedded_chinese() {
        assert_eq!(parse_duration("准备会议（15m），面聊（45m）").unwrap(), 60);
    }

    #[test]
    fn test_parse_duration_zero() { assert!(parse_duration("0").is_err()); }

    #[test]
    fn test_parse_duration_empty() { assert!(parse_duration("").is_err()); }

    #[test]
    fn test_parse_duration_invalid() { assert!(parse_duration("no duration").is_err()); }

    #[test]
    fn test_validate_date_format_valid() { assert!(validate_date_format("2026-06-12").is_ok()); }

    #[test]
    fn test_validate_date_format_invalid() { assert!(validate_date_format("bad").is_err()); }

    #[test]
    fn test_validate_date_format_month_99() { assert!(validate_date_format("2026-99-12").is_err()); }

    #[test]
    fn test_read_day_file_safe_corrupt() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_test_corrupt_day");
        let _ = fs::remove_dir_all(&tmp);
        let date = "2026-06-12";
        let path = files::day_path(&tmp, date).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "---\nentries: [\n---\n").unwrap(); // broken YAML
        let result = read_day_file_safe(&tmp, date);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Manual recovery"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_monthly_file_safe_corrupt() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_test_corrupt_monthly");
        let _ = fs::remove_dir_all(&tmp);
        let path = files::monthly_path(&tmp, 2026, 6);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "---\ncommitments: [\n---\n").unwrap(); // broken YAML
        let result = read_monthly_file_safe(&tmp, 2026, 6);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Manual recovery"));
        let _ = fs::remove_dir_all(&tmp);
    }

    // --- validate_required_dimensions tests ---

    use crate::models::{Config, Dimension};
    use std::collections::HashMap;

    fn make_config(required_keys: &[&str]) -> Config {
        Config {
            dimensions: vec![
                Dimension {
                    name: "Biz".into(), key: "biz".into(), source: "static".into(),
                    values: Some(vec!["A".into()]), required: required_keys.contains(&"biz"),
                },
                Dimension {
                    name: "Cat".into(), key: "cat".into(), source: "static".into(),
                    values: Some(vec!["X".into()]), required: required_keys.contains(&"cat"),
                },
                Dimension {
                    name: "Goal".into(), key: "goal".into(), source: "monthly".into(),
                    values: None, required: required_keys.contains(&"goal"),
                },
            ],
        }
    }

    #[test]
    fn test_validate_required_all_present() {
        let config = make_config(&["biz"]);
        let mut dims = HashMap::new();
        dims.insert("biz".to_string(), "A".to_string());
        assert!(validate_required_dimensions(&config, &dims).is_ok());
    }

    #[test]
    fn test_validate_required_missing_one() {
        let config = make_config(&["biz", "cat"]);
        let mut dims = HashMap::new();
        dims.insert("biz".to_string(), "A".to_string());
        // cat is missing
        let err = validate_required_dimensions(&config, &dims).unwrap_err();
        assert!(err.contains("Cat"), "expected error to mention 'Cat', got: {}", err);
        assert!(err.contains("Missing required dimension"));
    }

    #[test]
    fn test_validate_required_none_required() {
        let config = make_config(&[]);
        let dims = HashMap::new(); // empty is fine — nothing required
        assert!(validate_required_dimensions(&config, &dims).is_ok());
    }

    #[test]
    fn test_validate_required_empty_dimensions() {
        let config = make_config(&["biz"]);
        let dims = HashMap::new();
        let err = validate_required_dimensions(&config, &dims).unwrap_err();
        assert!(err.contains("Biz"));
    }
}
