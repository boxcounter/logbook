use crate::config::{validate_config, validate_monthly};
use crate::files::{self, save_root_path, read_root_path};
use crate::models::*;
use chrono::Datelike;
use regex::Regex;
use tauri::{AppHandle, Manager};

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

    // Scan for all duration patterns
    let re = Regex::new(r"(\d+(?:\.\d+)?)\s*(h|m|H|M)?").unwrap();
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

#[tauri::command]
pub fn init(app: AppHandle) -> InitResult {
    let app_data_dir = app.path().app_local_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let root_path = match read_root_path(&app_data_dir) {
        Some(p) => p,
        None => return InitResult::NeedsSetup,
    };

    let root = std::path::Path::new(&root_path);

    let config = match files::read_config(root) {
        Ok(c) => c,
        Err(e) => return InitResult::ConfigError(vec![ConfigErrorDetail {
            kind: "ConfigReadError".to_string(), message: e,
        }]),
    };

    let mut all_errors = validate_config(&config);

    let now = chrono::Local::now();
    let monthly = files::read_monthly_file(root, now.year(), now.month()).unwrap_or_else(|_| MonthlyFile { commitments: vec![] });
    all_errors.extend(validate_monthly(&monthly));

    if !all_errors.is_empty() {
        return InitResult::ConfigError(all_errors);
    }

    let today_date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());
    let today = files::read_day_file(root, &today_date).unwrap_or_else(|_| DayFile { note: None, entries: vec![] });

    InitResult::Ready { config, today, commitments: monthly.commitments }
}

#[tauri::command]
pub fn set_root_path(app: AppHandle, path: String) -> Result<InitResult, String> {
    let app_data_dir = app.path().app_local_data_dir().map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let root_path = std::path::Path::new(&path);
    if !root_path.exists() { return Err(format!("Path does not exist: {}", path)); }
    if !root_path.is_dir() { return Err(format!("Path is not a directory: {}", path)); }

    save_root_path(&app_data_dir, root_path)?;

    let config = files::read_config(root_path).map_err(|e| format!("Failed to read config: {}", e))?;
    let mut all_errors = validate_config(&config);

    let now = chrono::Local::now();
    let monthly = files::read_monthly_file(root_path, now.year(), now.month()).unwrap_or_else(|_| MonthlyFile { commitments: vec![] });
    all_errors.extend(validate_monthly(&monthly));

    if !all_errors.is_empty() {
        return Ok(InitResult::ConfigError(all_errors));
    }

    let today_date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());
    let today = files::read_day_file(root_path, &today_date).unwrap_or_else(|_| DayFile { note: None, entries: vec![] });

    Ok(InitResult::Ready { config, today, commitments: monthly.commitments })
}

#[tauri::command]
pub fn get_entries(root_path: String, date: String) -> Result<DayFile, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    files::read_day_file(root, &date)
}

#[tauri::command]
pub fn append_entry(root_path: String, date: String, entry: NewEntry) -> Result<Entry, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let duration = parse_duration(&entry.duration)?;
    let entry = Entry {
        id: uuid::Uuid::new_v4().to_string(),
        item: entry.item,
        duration,
        dimensions: entry.dimensions,
    };
    files::append_to_day_file(root, &date, &entry)
}

#[tauri::command]
pub fn update_entry(root_path: String, date: String, entry_id: String, update: UpdateEntry) -> Result<DayFile, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    // Parse duration if it's being updated
    if let Some(ref dur_str) = update.duration {
        parse_duration(dur_str)?; // validate only; files.rs parses again
    }
    files::update_entry_in_file(root, &date, &entry_id, &update)
}

#[tauri::command]
pub fn delete_entry(root_path: String, date: String, entry_id: String) -> Result<DayFile, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    files::delete_entry_from_file(root, &date, &entry_id)
}

#[tauri::command]
pub fn set_day_note(root_path: String, date: String, note: String) -> Result<DayFile, String> {
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    files::set_day_note_in_file(root, &date, &note)
}

#[tauri::command]
pub fn get_commitments(root_path: String, year: i32, month: u32) -> Result<Vec<Commitment>, String> {
    let root = std::path::Path::new(&root_path);
    let monthly = files::read_monthly_file(root, year, month)?;
    Ok(monthly.commitments)
}

fn validate_date_format(date: &str) -> Result<(), String> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}. Expected YYYY-MM-DD", date, e))?;
    Ok(())
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
}
