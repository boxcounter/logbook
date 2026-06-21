use crate::config::{validate_config, validate_monthly};
use crate::error_log;
use crate::operation_log;
use crate::files::{self, read_root_path, save_root_path};
use crate::models::*;
use chrono::Datelike;
use regex::Regex;
use std::sync::LazyLock;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

static DURATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+(?:\.\d+)?)\s*(h|m|H|M)?").unwrap());

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
                Ok(DayFile {
                    note: None,
                    entries: vec![],
                })
            }
        }
    }
}

/// Read a monthly file, distinguishing "file not found" from "file found but corrupt".
fn read_monthly_file_safe(
    root: &std::path::Path,
    year: i32,
    month: u32,
) -> Result<MonthlyFile, String> {
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
                Ok(MonthlyFile {
                    commitments: vec![],
                })
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
        if n > 0 {
            return Ok(n);
        }
        return Err("Duration must be positive".to_string());
    }

    // Try float (e.g. "1.5")
    if let Ok(n) = input.parse::<f64>() {
        if n > 0.0 {
            return Ok(n.round() as u32);
        }
        return Err("Duration must be positive".to_string());
    }

    // Scan for all duration patterns (compiled once via LazyLock)
    let re = &*DURATION_RE;
    let mut total: f64 = 0.0;
    let mut matched = false;

    for cap in re.captures_iter(input) {
        let value: f64 = cap[1].parse().unwrap_or(0.0);
        let unit = cap
            .get(2)
            .map(|m| m.as_str().to_lowercase())
            .unwrap_or_else(|| "m".to_string());
        match unit.as_str() {
            "h" => {
                total += value * 60.0;
                matched = true;
            }
            "m" | "" => {
                total += value;
                matched = true;
            }
            _ => {}
        }
    }

    if !matched {
        return Err(format!(
            "Could not parse duration from '{}'. Examples: 1.5h, 30m, 45",
            input
        ));
    }

    let total = total.round() as u32;
    if total == 0 {
        return Err("Parsed duration is zero".to_string());
    }
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
    let app_data_dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let root_path = match read_root_path(&app_data_dir) {
        Some(p) => p,
        None => {
            error_log::log_command_exit("init", true, "NeedsSetup");
            return InitResult::NeedsSetup;
        }
    };

    let root = std::path::Path::new(&root_path);

    let scan_warnings = crate::scan::scan_data_dir(root);
    if !scan_warnings.is_empty() {
        error_log::log_info("init", &format!("{} scan warnings", scan_warnings.len()));
    }

    let config = match files::read_template(root) {
        Ok(c) => c,
        Err(e) => {
            error_log::log_error("init: read_template", &e);
            error_log::log_command_exit("init", false, "ConfigReadError");
            return InitResult::ConfigError {
                errors: vec![ConfigErrorDetail {
                    kind: "ConfigReadError".to_string(),
                    message: e,
                }],
                scan_warnings,
            };
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
            MonthlyFile {
                commitments: vec![],
            }
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
            DayFile {
                note: None,
                entries: vec![],
            }
        }
    };

    if !all_errors.is_empty() {
        error_log::log_command_exit(
            "init",
            false,
            &format!("{} config errors", all_errors.len()),
        );
        for w in &scan_warnings {
            error_log::log_error("init: scan", &format!("{}: {}", w.path, w.message));
        }
        return InitResult::ConfigError {
            errors: all_errors,
            scan_warnings,
        };
    }

    error_log::log_command_exit(
        "init",
        true,
        &format!("Ready, {} entries today", today.entries.len()),
    );
    InitResult::Ready {
        root_path: root_path.to_string_lossy().into_owned(),
        config,
        today,
        commitments: monthly.commitments,
        scan_warnings,
    }
}

#[tauri::command]
pub fn set_root_path(app: AppHandle, path: String) -> Result<InitResult, String> {
    error_log::log_command_enter("set_root_path", &format!("path={}", path));
    let app_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let root_path = std::path::Path::new(&path);
    if !root_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if !root_path.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    save_root_path(&app_data_dir, root_path)?;

    let scan_warnings = crate::scan::scan_data_dir(root_path);
    if !scan_warnings.is_empty() {
        error_log::log_info("set_root_path", &format!("{} scan warnings", scan_warnings.len()));
    }

    let config = files::read_template(root_path).map_err(|e| {
        error_log::log_error("set_root_path: read_template", &e);
        format!("Failed to read template: {}", e)
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
            MonthlyFile {
                commitments: vec![],
            }
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
            DayFile {
                note: None,
                entries: vec![],
            }
        }
    };

    if !all_errors.is_empty() {
        error_log::log_command_exit(
            "set_root_path",
            true,
            &format!("{} config errors", all_errors.len()),
        );
        for w in &scan_warnings {
            error_log::log_error("set_root_path: scan", &format!("{}: {}", w.path, w.message));
        }
        return Ok(InitResult::ConfigError {
            errors: all_errors,
            scan_warnings,
        });
    }

    error_log::log_command_exit("set_root_path", true, "Ready");
    Ok(InitResult::Ready {
        root_path: path.clone(),
        config,
        today,
        commitments: monthly.commitments,
        scan_warnings,
    })
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
pub fn append_entry(root_path: String, date: String, entry: CreateEntryInput) -> Result<Entry, String> {
    error_log::log_command_enter(
        "append_entry",
        &format!("date={} item={} dur={}", date, entry.item, entry.duration),
    );
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let duration = parse_duration(&entry.duration)?;
    let config = files::read_template(root)?;
    validate_required_dimensions(&config, &entry.dimensions)?;

    let entry_id = uuid::Uuid::new_v4().to_string();

    // Log before mutation
    let params = serde_json::json!({
        "item": entry.item,
        "duration": entry.duration,
        "dimensions": entry.dimensions,
    });
    operation_log::append(
        &root_path,
        operation_log::Operation::Append {
            date: date.clone(),
            entry_id: entry_id.clone(),
            params,
        },
    )?;

    let entry = Entry {
        id: entry_id,
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
pub fn update_entry(
    root_path: String,
    date: String,
    entry_id: String,
    update: UpdateEntryInput,
) -> Result<DayFile, String> {
    error_log::log_command_enter("update_entry", &format!("date={} id={}", date, entry_id));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    if let Some(ref dur_str) = update.duration {
        parse_duration(dur_str)?;
    }
    if let Some(ref dims) = update.dimensions {
        let config = files::read_template(root)?;
        validate_required_dimensions(&config, dims)?;
    }

    // Read before snapshot
    let day_file = files::read_day_file(root, &date)?;
    let before = day_file
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .cloned()
        .ok_or_else(|| format!("Entry {} not found", entry_id))?;

    // Log before mutation
    let params = serde_json::json!({
        "item": update.item,
        "duration": update.duration,
        "dimensions": update.dimensions,
    });
    operation_log::append(
        &root_path,
        operation_log::Operation::Update {
            date: date.clone(),
            entry_id: entry_id.clone(),
            before,
            params,
        },
    )?;

    let result = files::update_entry_in_file(root, &date, &entry_id, &update);
    let ok = result.is_ok();
    error_log::log_command_exit(
        "update_entry",
        ok,
        &format!(
            "{} entries",
            result.as_ref().map(|d| d.entries.len()).unwrap_or(0)
        ),
    );
    result
}

#[tauri::command]
pub fn delete_entry(root_path: String, date: String, entry_id: String) -> Result<DayFile, String> {
    error_log::log_command_enter("delete_entry", &format!("date={} id={}", date, entry_id));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;

    // Read before snapshot
    let day_file = files::read_day_file(root, &date)?;
    let before = day_file
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .cloned()
        .ok_or_else(|| format!("Entry {} not found", entry_id))?;

    // Log before mutation
    operation_log::append(
        &root_path,
        operation_log::Operation::Delete {
            date: date.clone(),
            entry_id: entry_id.clone(),
            before,
        },
    )?;

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

    // Read before snapshot
    let day_file = files::read_day_file(root, &date)?;
    let before = day_file.note.clone();

    // Log before mutation
    operation_log::append(
        &root_path,
        operation_log::Operation::SetDayNote {
            date: date.clone(),
            before,
            params: note.clone(),
        },
    )?;

    let result = files::set_day_note_in_file(root, &date, &note);
    let ok = result.is_ok();
    error_log::log_command_exit("set_day_note", ok, "");
    result
}

#[tauri::command]
pub fn get_commitments(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<Vec<Commitment>, String> {
    error_log::log_command_enter("get_commitments", &format!("{}-{:02}", year, month));
    let root = std::path::Path::new(&root_path);
    let result = files::read_monthly_file(root, year, month).map(|m| m.commitments);
    let ok = result.is_ok();
    let count = result.as_ref().map(|c| c.len()).unwrap_or(0);
    error_log::log_command_exit("get_commitments", ok, &format!("{} commitments", count));
    result
}

#[tauri::command]
pub fn get_commitment_progress(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<Vec<CommitmentProgress>, String> {
    use crate::models::{CommitmentProgress, GoalProgress};
    use std::collections::HashMap;

    let root = std::path::Path::new(&root_path);

    // 1. Read _monthly.md
    let monthly =
        crate::files::read_monthly_file(root, year, month).unwrap_or_else(|_| MonthlyFile {
            commitments: vec![],
        });

    let commitments = monthly.commitments;

    // 2. Build goal -> (role, goal_name) map
    let mut goal_to_role: HashMap<String, (String, String)> = HashMap::new();
    for c in &commitments {
        for g in &c.goals {
            goal_to_role.insert(g.clone(), (c.role.clone(), g.clone()));
        }
    }

    // 3. Initialize result structures
    let mut role_spent: HashMap<String, u32> = HashMap::new();
    let mut goal_spent: HashMap<String, u32> = HashMap::new();
    for c in &commitments {
        role_spent.entry(c.role.clone()).or_insert(0);
        for g in &c.goals {
            goal_spent.entry(g.clone()).or_insert(0);
        }
    }

    // 4. Scan day files in the month directory
    let month_dir = root.join(year.to_string()).join(format!("{:02}", month));

    if month_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&month_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip _monthly.md and non-.md files
                if file_name == "_monthly.md" || !file_name.ends_with(".md") {
                    continue;
                }

                // Read the day file
                if let Ok(day_file) =
                    crate::files::read_day_file(root, file_name.trim_end_matches(".md"))
                {
                    for e in &day_file.entries {
                        if let Some(goal) = e.dimensions.get("goal") {
                            if let Some((role, goal_name)) = goal_to_role.get(goal) {
                                *role_spent.entry(role.clone()).or_insert(0) += e.duration;
                                *goal_spent.entry(goal_name.clone()).or_insert(0) += e.duration;
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Build result vector
    let mut results: Vec<CommitmentProgress> = Vec::new();
    for c in &commitments {
        let goals: Vec<GoalProgress> = c
            .goals
            .iter()
            .map(|g| GoalProgress {
                name: g.clone(),
                spent_minutes: *goal_spent.get(g).unwrap_or(&0),
            })
            .collect();
        results.push(CommitmentProgress {
            role: c.role.clone(),
            allocation_minutes: c.allocation * 60,
            spent_minutes: *role_spent.get(&c.role).unwrap_or(&0),
            goals,
        });
    }

    Ok(results)
}

#[tauri::command]
pub fn set_commitments(
    root_path: String,
    year: i32,
    month: u32,
    commitments: Vec<Commitment>,
) -> Result<Vec<Commitment>, String> {
    error_log::log_command_enter(
        "set_commitments",
        &format!("{}-{:02} {} roles", year, month, commitments.len()),
    );
    let root = std::path::Path::new(&root_path);

    // 1. Validate
    validate_commitments(&commitments)?;

    // 2. Read old state for diff
    let old = read_monthly_file_safe(root, year, month)?;

    // 3. Detect changes
    let changes = detect_goal_changes(&old.commitments, &commitments);

    // 4. Check deleted goals for existing entries
    for goal_name in &changes.deleted {
        let count = count_entries_with_goal(root, year, month, goal_name)?;
        if count > 0 {
            return Err(format!(
                "Cannot delete goal '{}': used by {} entries this month",
                goal_name, count
            ));
        }
    }

    // 5. Apply renames to all day files
    for (old_name, new_name) in &changes.renames {
        rename_goal_in_entries(root, year, month, old_name, new_name)?;
    }

    // 6. Write _monthly.md
    let monthly = MonthlyFile { commitments };
    files::write_monthly_file(root, year, month, &monthly)?;

    let ok = true;
    error_log::log_command_exit("set_commitments", ok, "");
    Ok(monthly.commitments)
}

/// Count entries in a month that reference a specific goal.
fn count_entries_with_goal(
    root: &std::path::Path,
    year: i32,
    month: u32,
    goal_name: &str,
) -> Result<usize, String> {
    let month_dir = root
        .join(year.to_string())
        .join(format!("{:02}", month));

    if !month_dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    let entries = match std::fs::read_dir(&month_dir) {
        Ok(e) => e,
        Err(e) => return Err(format!("Failed to read month dir: {}", e)),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "_monthly.md" || !file_name.ends_with(".md") {
            continue;
        }
        let date = file_name.trim_end_matches(".md");
        if let Ok(day_file) = files::read_day_file(root, date) {
            count += day_file
                .entries
                .iter()
                .filter(|e| e.dimensions.get("goal").map(|g| g == goal_name).unwrap_or(false))
                .count();
        }
    }
    Ok(count)
}

/// Rename a goal in all day files of a given month.
fn rename_goal_in_entries(
    root: &std::path::Path,
    year: i32,
    month: u32,
    old_name: &str,
    new_name: &str,
) -> Result<(), String> {
    let month_dir = root
        .join(year.to_string())
        .join(format!("{:02}", month));

    if !month_dir.exists() {
        return Ok(());
    }

    let entries = match std::fs::read_dir(&month_dir) {
        Ok(e) => e,
        Err(e) => return Err(format!("Failed to read month dir: {}", e)),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "_monthly.md" || !file_name.ends_with(".md") {
            continue;
        }
        let date = file_name.trim_end_matches(".md");
        let mut day_file = files::read_day_file(root, date)?;
        let mut changed = false;
        for e in &mut day_file.entries {
            if let Some(goal) = e.dimensions.get("goal") {
                if goal == old_name {
                    e.dimensions.insert("goal".to_string(), new_name.to_string());
                    changed = true;
                }
            }
        }
        if changed {
            files::write_day_file(root, date, &day_file)?;
        }
    }
    Ok(())
}

fn validate_date_format(date: &str) -> Result<chrono::NaiveDate, String> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}. Expected YYYY-MM-DD", date, e))
}

/// Validate commitments before saving (no IO).
fn validate_commitments(commitments: &[Commitment]) -> Result<(), String> {
    if commitments.is_empty() {
        return Err("At least one role is required".to_string());
    }
    let mut role_set = std::collections::HashSet::new();
    let mut goal_set = std::collections::HashSet::new();
    for c in commitments {
        let role = c.role.trim();
        if role.is_empty() {
            return Err("Role name cannot be empty".to_string());
        }
        if !role_set.insert(role.to_string()) {
            return Err(format!("Role '{}' already exists", role));
        }
        if c.allocation == 0 {
            return Err(format!(
                "Allocation for '{}' must be greater than 0",
                role
            ));
        }
        for g in &c.goals {
            let goal = g.trim();
            if goal.is_empty() {
                return Err("Goal name cannot be empty".to_string());
            }
            if !goal_set.insert(goal.to_string()) {
                return Err(format!("Goal '{}' already exists", goal));
            }
        }
    }
    Ok(())
}

/// Detects goal renames and deletions between old and new commitments.
///
/// A rename is detected when a role has the same number of goals and exactly
/// one goal differs between old and new.
#[derive(Debug, PartialEq)]
struct GoalChanges {
    renames: Vec<(String, String)>,
    deleted: Vec<String>,
}

fn detect_goal_changes(old: &[Commitment], new: &[Commitment]) -> GoalChanges {
    use std::collections::HashSet;

    let old_goals: HashSet<String> = old
        .iter()
        .flat_map(|c| c.goals.iter().cloned())
        .collect();
    let new_goals: HashSet<String> = new
        .iter()
        .flat_map(|c| c.goals.iter().cloned())
        .collect();

    let deleted: Vec<String> = old_goals.difference(&new_goals).cloned().collect();

    let mut renames: Vec<(String, String)> = Vec::new();
    let mut matched_old_goals: HashSet<String> = HashSet::new();

    for old_c in old {
        if let Some(new_c) = new.iter().find(|c| c.role == old_c.role) {
            if old_c.goals.len() == new_c.goals.len() {
                let old_set: HashSet<_> = old_c.goals.iter().cloned().collect();
                let new_set: HashSet<_> = new_c.goals.iter().cloned().collect();

                let old_not_new: Vec<_> = old_set.difference(&new_set).cloned().collect();
                let new_not_old: Vec<_> = new_set.difference(&old_set).cloned().collect();

                if old_not_new.len() == 1 && new_not_old.len() == 1 {
                    renames.push((old_not_new[0].clone(), new_not_old[0].clone()));
                    matched_old_goals.insert(old_not_new[0].clone());
                }
            }
        }
    }

    let deleted: Vec<String> = deleted
        .into_iter()
        .filter(|g| !matched_old_goals.contains(g))
        .collect();

    GoalChanges { renames, deleted }
}

#[tauri::command]
pub fn get_available_months(root_path: String) -> Result<Vec<AvailableMonth>, String> {
    use crate::models::AvailableMonth;
    let root = std::path::Path::new(&root_path);
    if !root.exists() {
        return Ok(vec![]);
    }

    let mut months: Vec<AvailableMonth> = Vec::new();

    let year_entries = std::fs::read_dir(root)
        .map_err(|e| format!("Failed to read root dir: {}", e))?;

    for year_entry in year_entries.flatten() {
        if !year_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let year_name = year_entry.file_name();
        let year_str = year_name.to_string_lossy();
        let year: i32 = match year_str.parse() {
            Ok(y) if y >= 2000 && y <= 2100 => y,
            _ => continue,
        };

        let month_entries = match std::fs::read_dir(year_entry.path()) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for month_entry in month_entries.flatten() {
            if !month_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let month_name = month_entry.file_name();
            let month_str = month_name.to_string_lossy();
            let month: u32 = match month_str.parse() {
                Ok(m) if m >= 1 && m <= 12 => m,
                _ => continue,
            };

            // Check if this month directory contains at least one .md file
            // (skip _monthly.md — it is metadata, not day-entry data)
            let has_md = match std::fs::read_dir(month_entry.path()) {
                Ok(entries) => entries.flatten().any(|e| {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    name_str.ends_with(".md") && name_str != "_monthly.md"
                }),
                Err(_) => false,
            };

            if has_md {
                months.push(AvailableMonth { year, month });
            }
        }
    }

    // Sort descending (newest first)
    months.sort_by(|a, b| b.year.cmp(&a.year).then(b.month.cmp(&a.month)));

    Ok(months)
}

/// What the file manager should reveal/open for a given day.
struct RevealTarget {
    path: std::path::PathBuf,
    /// true  → reveal `path` and select it (it is the day file)
    /// false → open `path` as a directory (no file to select)
    select: bool,
}

/// Decide what to reveal for `date`:
/// - day file `root/YYYY/MM/YYYY-MM-DD.md` exists → select that file
/// - else the month dir `root/YYYY/MM/` exists    → open that dir
/// - else                                         → open the data root
fn resolve_reveal_target(root: &std::path::Path, date: &str) -> Result<RevealTarget, String> {
    let file = files::day_path(root, date)?;
    if file.exists() {
        return Ok(RevealTarget { path: file, select: true });
    }
    if let Some(month_dir) = file.parent() {
        if month_dir.exists() {
            return Ok(RevealTarget {
                path: month_dir.to_path_buf(),
                select: false,
            });
        }
    }
    Ok(RevealTarget {
        path: root.to_path_buf(),
        select: false,
    })
}

#[tauri::command]
pub fn reveal_day_file(app: AppHandle, root_path: String, date: String) -> Result<(), String> {
    error_log::log_command_enter("reveal_day_file", &format!("date={}", date));
    validate_date_format(&date)?;
    let root = std::path::Path::new(&root_path);
    let target = resolve_reveal_target(root, &date)?;

    let result = if target.select {
        app.opener()
            .reveal_item_in_dir(&target.path)
            .map_err(|e| format!("Failed to reveal {}: {}", target.path.display(), e))
    } else {
        app.opener()
            .open_path(target.path.to_string_lossy().into_owned(), None::<String>)
            .map_err(|e| format!("Failed to open {}: {}", target.path.display(), e))
    };

    error_log::log_command_exit("reveal_day_file", result.is_ok(), "");
    result
}

#[tauri::command]
pub fn create_starter_files(path: String) -> Result<(), String> {
    let root = std::path::Path::new(&path);
    if !root.exists() {
        std::fs::create_dir_all(root).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let template_path = root.join("template.yaml");
    if !template_path.exists() {
        std::fs::write(
            &template_path,
            "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
        )
        .map_err(|e| format!("Failed to write template.yaml: {}", e))?;
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
    fn test_parse_duration_plain_number() {
        assert_eq!(parse_duration("90").unwrap(), 90);
    }

    #[test]
    fn test_parse_duration_float() {
        assert_eq!(parse_duration("1.5").unwrap(), 2);
    }

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration("1.5h").unwrap(), 90);
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("30m").unwrap(), 30);
    }

    #[test]
    fn test_parse_duration_compound() {
        assert_eq!(parse_duration("1h 30m").unwrap(), 90);
    }

    #[test]
    fn test_parse_duration_embedded_chinese() {
        assert_eq!(parse_duration("准备会议（15m），面聊（45m）").unwrap(), 60);
    }

    #[test]
    fn test_parse_duration_zero() {
        assert!(parse_duration("0").is_err());
    }

    #[test]
    fn test_parse_duration_empty() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("no duration").is_err());
    }

    #[test]
    fn test_validate_date_format_valid() {
        assert!(validate_date_format("2026-06-12").is_ok());
    }

    #[test]
    fn test_validate_date_format_invalid() {
        assert!(validate_date_format("bad").is_err());
    }

    #[test]
    fn test_validate_date_format_month_99() {
        assert!(validate_date_format("2026-99-12").is_err());
    }

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
                    name: "Biz".into(),
                    key: "biz".into(),
                    source: "static".into(),
                    values: Some(vec!["A".into()]),
                    required: required_keys.contains(&"biz"),
                },
                Dimension {
                    name: "Cat".into(),
                    key: "cat".into(),
                    source: "static".into(),
                    values: Some(vec!["X".into()]),
                    required: required_keys.contains(&"cat"),
                },
                Dimension {
                    name: "Goal".into(),
                    key: "goal".into(),
                    source: "monthly".into(),
                    values: None,
                    required: required_keys.contains(&"goal"),
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
        assert!(
            err.contains("Cat"),
            "expected error to mention 'Cat', got: {}",
            err
        );
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

    #[test]
    fn test_get_commitment_progress_empty_month() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_test_cp_empty");
        let _ = fs::remove_dir_all(&tmp);

        // Create directory structure with _monthly.md but no day files
        let monthly_dir = tmp.join("2026").join("06");
        fs::create_dir_all(&monthly_dir).unwrap();
        fs::write(
            monthly_dir.join("_monthly.md"),
            "---\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it\n---\n",
        )
        .unwrap();

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "Dev");
        assert_eq!(result[0].allocation_minutes, 2400); // 40 * 60
        assert_eq!(result[0].spent_minutes, 0);
        assert_eq!(result[0].goals.len(), 1);
        assert_eq!(result[0].goals[0].name, "Ship it");
        assert_eq!(result[0].goals[0].spent_minutes, 0);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_commitment_progress_aggregates_spent() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_test_cp_agg");
        let _ = fs::remove_dir_all(&tmp);

        // Create _monthly.md
        let monthly_dir = tmp.join("2026").join("06");
        fs::create_dir_all(&monthly_dir).unwrap();
        fs::write(
            monthly_dir.join("_monthly.md"),
            "---\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it\n      - Review\n  - role: PM\n    allocation: 10\n    goals:\n      - Planning\n---\n",
        )
        .unwrap();

        // Create day file with entries matching goals
        fs::write(
            monthly_dir.join("2026-06-01.md"),
            "---\nentries:\n  - id: e1\n    item: Code\n    duration: 60\n    dimensions:\n      goal: Ship it\n  - id: e2\n    item: PR\n    duration: 30\n    dimensions:\n      goal: Review\n---\n",
        )
        .unwrap();

        fs::write(
            monthly_dir.join("2026-06-02.md"),
            "---\nentries:\n  - id: e3\n    item: Plan\n    duration: 45\n    dimensions:\n      goal: Planning\n---\n",
        )
        .unwrap();

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        // Dev: Ship it(60) + Review(30) = 90 spent
        let dev = result.iter().find(|c| c.role == "Dev").unwrap();
        assert_eq!(dev.spent_minutes, 90);
        assert_eq!(dev.allocation_minutes, 2400);

        // PM: Planning(45) = 45 spent
        let pm = result.iter().find(|c| c.role == "PM").unwrap();
        assert_eq!(pm.spent_minutes, 45);
        assert_eq!(pm.allocation_minutes, 600); // 10 * 60

        // Goal-level check
        let ship_it = dev.goals.iter().find(|g| g.name == "Ship it").unwrap();
        assert_eq!(ship_it.spent_minutes, 60);
        let review = dev.goals.iter().find(|g| g.name == "Review").unwrap();
        assert_eq!(review.spent_minutes, 30);
        let planning = pm.goals.iter().find(|g| g.name == "Planning").unwrap();
        assert_eq!(planning.spent_minutes, 45);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_commitment_progress_ignores_unmatched_goals() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_test_cp_unmatch");
        let _ = fs::remove_dir_all(&tmp);

        let monthly_dir = tmp.join("2026").join("06");
        fs::create_dir_all(&monthly_dir).unwrap();
        fs::write(
            monthly_dir.join("_monthly.md"),
            "---\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it\n---\n",
        )
        .unwrap();

        // Entry with a goal NOT in any commitment
        fs::write(
            monthly_dir.join("2026-06-01.md"),
            "---\nentries:\n  - id: e1\n    item: Unknown task\n    duration: 60\n    dimensions:\n      goal: Not a goal\n---\n",
        )
        .unwrap();

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        assert_eq!(result[0].spent_minutes, 0);
        assert_eq!(result[0].goals[0].spent_minutes, 0);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_commitment_progress_no_monthly_file() {
        let tmp = std::env::temp_dir().join("logbook_test_cp_nofile");
        let _ = std::fs::remove_dir_all(&tmp);

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        assert!(result.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // --- set_commitments validation tests ---

    use crate::models::Commitment;

    fn make_commitments(roles: Vec<(&str, u32, Vec<&str>)>) -> Vec<Commitment> {
        roles
            .into_iter()
            .map(|(role, alloc, goals)| Commitment {
                role: role.to_string(),
                allocation: alloc,
                goals: goals.into_iter().map(|g| g.to_string()).collect(),
            })
            .collect()
    }

    #[test]
    fn test_validate_commitments_empty_list() {
        let result = validate_commitments(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("At least one role"));
    }

    #[test]
    fn test_validate_commitments_empty_role() {
        let c = make_commitments(vec![("", 40, vec!["Goal A"])]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Role name cannot be empty"));
    }

    #[test]
    fn test_validate_commitments_whitespace_role() {
        let c = make_commitments(vec![("   ", 40, vec!["Goal A"])]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Role name cannot be empty"));
    }

    #[test]
    fn test_validate_commitments_zero_allocation() {
        let c = make_commitments(vec![("Dev", 0, vec!["Goal A"])]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Allocation for 'Dev'"));
        assert!(err.contains("must be greater than 0"));
    }

    #[test]
    fn test_validate_commitments_empty_goal() {
        let c = make_commitments(vec![("Dev", 40, vec![""])]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Goal name cannot be empty"));
    }

    #[test]
    fn test_validate_commitments_duplicate_goal_same_role() {
        let c = make_commitments(vec![("Dev", 40, vec!["Ship it", "Ship it"])]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("already exists"));
        assert!(err.contains("Ship it"));
    }

    #[test]
    fn test_validate_commitments_duplicate_goal_across_roles() {
        let c = make_commitments(vec![
            ("Dev", 40, vec!["Shared goal"]),
            ("TL", 20, vec!["Shared goal"]),
        ]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("already exists"));
        assert!(err.contains("Shared goal"));
    }

    #[test]
    fn test_validate_commitments_duplicate_role() {
        let c = make_commitments(vec![
            ("Dev", 40, vec!["A"]),
            ("Dev", 20, vec!["B"]),
        ]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Role"));
        assert!(err.contains("already exists"));
        assert!(err.contains("Dev"));
    }

    #[test]
    fn test_validate_commitments_duplicate_goal_ignores_whitespace() {
        let c = make_commitments(vec![("Dev", 40, vec!["Ship it", " Ship it "])]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_validate_commitments_duplicate_role_ignores_whitespace() {
        let c = make_commitments(vec![
            ("Dev", 40, vec!["A"]),
            (" Dev ", 20, vec!["B"]),
        ]);
        let result = validate_commitments(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    // Guard: reordering goals within a role (same set, different order) must
    // NOT be misread as a rename by detect_goal_changes.
    #[test]
    fn test_detect_goal_changes_reorder_is_not_rename() {
        let old = make_commitments(vec![("Dev", 40, vec!["A", "B", "C"])]);
        let new = make_commitments(vec![("Dev", 40, vec!["C", "A", "B"])]);
        let changes = detect_goal_changes(&old, &new);
        assert!(changes.renames.is_empty(), "reorder must not produce renames");
        assert!(changes.deleted.is_empty(), "reorder must not produce deletions");
    }

    #[test]
    fn test_validate_commitments_valid() {
        let c = make_commitments(vec![
            ("Dev", 80, vec!["Ship it", "Review"]),
            ("TL", 40, vec!["1:1", "Architecture"]),
        ]);
        assert!(validate_commitments(&c).is_ok());
    }

    // --- detect_goal_changes tests ---

    #[test]
    fn test_detect_goal_rename_single_role() {
        let old = make_commitments(vec![("Dev", 40, vec!["Old name"])]);
        let new = make_commitments(vec![("Dev", 40, vec!["New name"])]);
        let changes = detect_goal_changes(&old, &new);
        assert_eq!(changes.renames.len(), 1);
        assert_eq!(changes.renames[0], ("Old name".to_string(), "New name".to_string()));
        assert!(changes.deleted.is_empty());
    }

    #[test]
    fn test_detect_goal_deleted() {
        let old = make_commitments(vec![("Dev", 40, vec!["Ship it", "Review"])]);
        let new = make_commitments(vec![("Dev", 40, vec!["Ship it"])]);
        let changes = detect_goal_changes(&old, &new);
        assert!(changes.renames.is_empty());
        assert_eq!(changes.deleted, vec!["Review"]);
    }

    #[test]
    fn test_detect_goal_added_no_rename() {
        let old = make_commitments(vec![("Dev", 40, vec!["Ship it"])]);
        let new = make_commitments(vec![("Dev", 40, vec!["Ship it", "Review"])]);
        let changes = detect_goal_changes(&old, &new);
        assert!(changes.renames.is_empty());
        assert!(changes.deleted.is_empty());
    }

    #[test]
    fn test_detect_goal_rename_when_count_matches() {
        let old = make_commitments(vec![("Dev", 40, vec!["A", "B", "C"])]);
        let new = make_commitments(vec![("Dev", 40, vec!["A", "B", "D"])]);
        let changes = detect_goal_changes(&old, &new);
        assert_eq!(changes.renames.len(), 1);
        assert_eq!(changes.renames[0], ("C".to_string(), "D".to_string()));
        assert!(changes.deleted.is_empty());
    }

    #[test]
    fn test_detect_goal_delete_add_not_rename() {
        // Count differs: delete + add, NOT rename
        let old = make_commitments(vec![("Dev", 40, vec!["A", "B"])]);
        let new = make_commitments(vec![("Dev", 40, vec!["A", "B", "C"])]);
        let changes = detect_goal_changes(&old, &new);
        assert!(changes.renames.is_empty());
        // C is new, nothing deleted
        assert!(changes.deleted.is_empty());
    }

    #[test]
    fn test_detect_goal_changes_role_removed() {
        let old = make_commitments(vec![
            ("Dev", 40, vec!["A"]),
            ("PM", 10, vec!["B"]),
        ]);
        let new = make_commitments(vec![
            ("Dev", 40, vec!["A"]),
        ]);
        let changes = detect_goal_changes(&old, &new);
        assert!(changes.renames.is_empty());
        // Goal "B" from removed role "PM" is a deletion
        assert_eq!(changes.deleted, vec!["B"]);
    }

    #[test]
    fn test_detect_goal_changes_role_added() {
        let old = make_commitments(vec![("Dev", 40, vec!["A"])]);
        let new = make_commitments(vec![
            ("Dev", 40, vec!["A"]),
            ("PM", 10, vec!["B"]),
        ]);
        let changes = detect_goal_changes(&old, &new);
        assert!(changes.renames.is_empty());
        assert!(changes.deleted.is_empty());
    }

    #[test]
    fn test_detect_goal_changes_no_diff() {
        let c = make_commitments(vec![("Dev", 40, vec!["A", "B"])]);
        let changes = detect_goal_changes(&c, &c);
        assert!(changes.renames.is_empty());
        assert!(changes.deleted.is_empty());
    }

    #[test]
    fn test_resolve_reveal_target_file_exists() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_test_reveal_file");
        let _ = fs::remove_dir_all(&tmp);
        let date = "2026-06-21";
        let file = files::day_path(&tmp, date).unwrap();
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "---\nentries: []\n---\n").unwrap();

        let t = resolve_reveal_target(&tmp, date).unwrap();
        assert_eq!(t.path, file);
        assert!(t.select, "existing file must be selected");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_resolve_reveal_target_falls_back_to_month_dir() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_test_reveal_month");
        let _ = fs::remove_dir_all(&tmp);
        let date = "2026-06-21";
        let month_dir = tmp.join("2026").join("06");
        fs::create_dir_all(&month_dir).unwrap(); // dir exists, day file does not

        let t = resolve_reveal_target(&tmp, date).unwrap();
        assert_eq!(t.path, month_dir);
        assert!(!t.select, "directory target must not be selected");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_resolve_reveal_target_falls_back_to_root() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_test_reveal_root");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap(); // only root exists
        let date = "2026-06-21";

        let t = resolve_reveal_target(&tmp, date).unwrap();
        assert_eq!(t.path, tmp);
        assert!(!t.select);

        let _ = fs::remove_dir_all(&tmp);
    }
}
