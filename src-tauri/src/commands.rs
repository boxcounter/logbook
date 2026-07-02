use crate::config::{validate_dimensions, validate_monthly};
use crate::error_log;
use crate::operation_log;
use crate::files::{self, read_root_path, save_root_path};
use crate::models::*;
use chrono::Datelike;
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::LazyLock;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

static DURATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+(?:\.\d+)?)\s*(h|m|H|M)").unwrap());

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
                    dimensions: vec![],
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
        let value: f64 = cap[1]
            .parse()
            .unwrap_or_else(|e| {
                error_log::log_error(
                    "parse_duration",
                    &format!("Failed to parse captured number '{}': {:?}", &cap[1], e),
                );
                0.0
            });
        let unit = cap
            .get(2)
            .map(|m| m.as_str().to_lowercase())
            .unwrap_or_else(|| "m".to_string());
        match unit.as_str() {
            "h" => {
                total += value * 60.0;
                matched = true;
            }
            "m" => {
                total += value;
                matched = true;
            }
            _ => {
                error_log::log_error(
                    "parse_duration",
                    &format!("Unknown duration unit '{}' in input '{}'", unit, input),
                );
            }
        }
    }

    if !matched {
        return Err(format!(
            "Could not parse duration from '{}'. Examples: 1.5h, 30m, 2h 15m",
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
    dimensions: &[Dimension],
    entry_dimensions: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    for dim in dimensions {
        if dim.required && !entry_dimensions.contains_key(&dim.key) {
            return Err(format!("Missing required dimension: {}", dim.name));
        }
    }
    Ok(())
}

/// Classify the data root and load initial state.
/// No AppHandle → unit/integration testable. init/set_root_path delegate here.
pub fn load_root_state(root: &std::path::Path) -> InitResult {
    if !root.exists() {
        return InitResult::ConfigError {
            category: RecoveryCategory::RootMissing,
            root_path: root.to_string_lossy().into_owned(),
            errors: vec![ConfigErrorDetail {
                kind: "RootMissing".to_string(),
                message: format!("Data folder not found: {}", root.display()),
            }],
            scan_warnings: vec![],
        };
    }

    let scan_warnings = crate::scan::scan_data_dir(root);

    // Migrate old template.yaml → dimensions.template.yaml
    let old_template = files::template_path(root);
    let new_template = files::dimensions_template_path(root);
    if old_template.exists() && !new_template.exists() {
        if let Err(e) = std::fs::rename(&old_template, &new_template) {
            crate::error_log::log_error(
                "migration",
                &format!(
                    "Failed to rename template.yaml → dimensions.template.yaml: {}",
                    e
                ),
            );
        } else {
            crate::error_log::log_info(
                "migration",
                "Renamed template.yaml → dimensions.template.yaml",
            );
        }
    }

    if !files::dimensions_template_path(root).exists() {
        return InitResult::ConfigError {
            category: RecoveryCategory::ConfigMissing,
            root_path: root.to_string_lossy().into_owned(),
            errors: vec![ConfigErrorDetail {
                kind: "ConfigMissing".to_string(),
                message: format!("dimensions.template.yaml not found in {}", root.display()),
            }],
            scan_warnings,
        };
    }

    let template = match files::read_dimensions_template(root) {
        Ok(c) => c,
        Err(e) => {
            return InitResult::ConfigError {
                category: RecoveryCategory::InPlace,
                root_path: root.to_string_lossy().into_owned(),
                errors: vec![ConfigErrorDetail {
                    kind: "ConfigReadError".to_string(),
                    message: e,
                }],
                scan_warnings,
            };
        }
    };

    let mut all_errors = validate_dimensions(&template.dimensions);

    let now = chrono::Local::now();

    // Migrate old _monthly.md → dimensions.yaml + commitments.yaml (current month only)
    if files::monthly_path(root, now.year(), now.month()).exists() {
        let (had_dims, had_commits) = files::migrate_monthly_file(root, now.year(), now.month())
            .unwrap_or((false, false));
        if had_dims || had_commits {
            crate::error_log::log_info(
                "migration",
                &format!(
                    "Migrated {}/{:02}/_monthly.md → dimensions.yaml ({}) + commitments.yaml ({})",
                    now.year(),
                    now.month(),
                    had_dims,
                    had_commits
                ),
            );
        }
    }

    let monthly = match read_monthly_file_safe(root, now.year(), now.month()) {
        Ok(mf) => mf,
        Err(e) => {
            all_errors.push(ConfigErrorDetail {
                kind: "MonthlyFileCorrupt".to_string(),
                message: e,
            });
            MonthlyFile { dimensions: vec![], commitments: vec![] }
        }
    };
    all_errors.extend(validate_monthly(&monthly));

    // Read commitments from commitments.yaml (separate from _monthly.md)
    let commitments = match files::read_commitments_file(root, now.year(), now.month()) {
        Ok(c) => {
            if !c.is_empty() {
                if let Err(e) = validate_commitments(&c) {
                    all_errors.push(ConfigErrorDetail {
                        kind: "CommitmentValidation".to_string(),
                        message: e,
                    });
                }
            }
            c
        }
        Err(e) => {
            all_errors.push(ConfigErrorDetail {
                kind: "CommitmentsFileCorrupt".to_string(),
                message: e,
            });
            vec![]
        }
    };

    let today_date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());
    let mut today = match read_day_file_safe(root, &today_date) {
        Ok(df) => df,
        Err(e) => {
            all_errors.push(ConfigErrorDetail {
                kind: "DayFileCorrupt".to_string(),
                message: e,
            });
            DayFile { note: None, entries: vec![] }
        }
    };

    if !all_errors.is_empty() {
        return InitResult::ConfigError {
            category: RecoveryCategory::InPlace,
            root_path: root.to_string_lossy().into_owned(),
            errors: all_errors,
            scan_warnings,
        };
    }

    let from_template = monthly.dimensions.is_empty();
    let dimensions = if from_template {
        template.dimensions
    } else {
        monthly.dimensions.clone()
    };

    // Inject attribution into today's entries
    {
        let commitments = crate::files::read_commitments_file(root, now.year(), now.month()).unwrap_or_default();
        let goal_key = monthly_dim_key(root, now.year(), now.month());
        let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
        annotate_day_file(&mut today, &goal_key, &goal_to_role, &role_to_goals);
    }

    InitResult::Ready {
        root_path: root.to_string_lossy().into_owned(),
        dimensions,
        from_template,
        today,
        commitments,
        scan_warnings,
    }
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

    let result = load_root_state(&root_path);
    if root_path.exists() {
        crate::config::ensure_watcher(&app, root_path.clone());
    }
    match &result {
        InitResult::ConfigError { errors, scan_warnings, category, .. } => {
            for e in errors {
                error_log::log_error("init", &format!("{}: {}", e.kind, e.message));
            }
            for w in scan_warnings {
                error_log::log_error("init: scan", &format!("{}: {}", w.path, w.message));
            }
            error_log::log_command_exit(
                "init",
                false,
                &format!("{:?}: {} errors", category, errors.len()),
            );
        }
        InitResult::Ready { today, .. } => {
            error_log::log_command_exit(
                "init",
                true,
                &format!("Ready, {} entries today", today.entries.len()),
            );
        }
        InitResult::NeedsSetup => {
            error_log::log_command_exit("init", true, "NeedsSetup");
        }
    }
    result
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

    let result = load_root_state(root_path);
    crate::config::ensure_watcher(&app, root_path.to_path_buf());
    match &result {
        InitResult::ConfigError { errors, scan_warnings, category, .. } => {
            for e in errors {
                error_log::log_error("set_root_path", &format!("{}: {}", e.kind, e.message));
            }
            for w in scan_warnings {
                error_log::log_error("set_root_path: scan", &format!("{}: {}", w.path, w.message));
            }
            error_log::log_command_exit(
                "set_root_path",
                true,
                &format!("{:?}: {} errors", category, errors.len()),
            );
        }
        InitResult::Ready { today, .. } => {
            error_log::log_command_exit(
                "set_root_path",
                true,
                &format!("Ready, {} entries today", today.entries.len()),
            );
        }
        InitResult::NeedsSetup => {
            error_log::log_command_exit("set_root_path", true, "NeedsSetup");
        }
    }
    Ok(result)
}

#[tauri::command]
pub fn get_entries(root_path: String, date: String) -> Result<DayFile, String> {
    error_log::log_command_enter("get_entries", &format!("date={}", date));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    let mut result = files::read_day_file(root, &date);

    // Inject attribution
    if let Ok(d) = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
        if let Ok(ref mut day_file) = result {
            let year = d.format("%Y").to_string().parse::<i32>().unwrap_or(0);
            let month = d.format("%m").to_string().parse::<u32>().unwrap_or(0);
            let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
            let goal_key = monthly_dim_key(root, year, month);
            let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
            annotate_day_file(day_file, &goal_key, &goal_to_role, &role_to_goals);
        }
    }

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
    let (year, month) = files::year_month_from_date(&date)?;
    files::create_dimensions_if_missing(root, year, month)?;
    let dims = files::resolve_month_dimensions(root, year, month)?;
    validate_required_dimensions(&dims, &entry.dimensions)?;

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

    let mut entry = Entry {
        id: entry_id,
        item: entry.item,
        duration,
        dimensions: entry.dimensions,
        attribution: crate::models::Attribution::default(),
    };

    // Inject attribution for the new entry
    {
        let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
        let goal_key = monthly_dim_key(root, year, month);
        let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
        entry.attribution = compute_attribution(&entry.dimensions, &goal_key, &goal_to_role, &role_to_goals);
    }

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
    let (year, month) = files::year_month_from_date(&date)?;
    files::create_dimensions_if_missing(root, year, month)?;
    if let Some(ref dur_str) = update.duration {
        parse_duration(dur_str)?;
    }
    if let Some(ref dims) = update.dimensions {
        let effective = files::resolve_month_dimensions(root, year, month)?;
        validate_required_dimensions(&effective, dims)?;
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

    let mut result = files::update_entry_in_file(root, &date, &entry_id, &update);

    // Inject attribution
    if let Ok(ref mut day_file) = result {
        let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
        let goal_key = monthly_dim_key(root, year, month);
        let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
        annotate_day_file(day_file, &goal_key, &goal_to_role, &role_to_goals);
    }

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
    let (year, month) = files::year_month_from_date(&date)?;
    // Deleting an entry does not customize the month's dimensions, so it must not
    // trigger instantiation (would freeze the month to the current template).

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

    let mut result = files::delete_entry_from_file(root, &date, &entry_id);

    // Inject attribution
    if let Ok(ref mut day_file) = result {
        let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_default();
        let goal_key = monthly_dim_key(root, year, month);
        let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);
        annotate_day_file(day_file, &goal_key, &goal_to_role, &role_to_goals);
    }

    let ok = result.is_ok();
    error_log::log_command_exit("delete_entry", ok, "");
    result
}

#[tauri::command]
pub fn set_day_note(root_path: String, date: String, note: String) -> Result<DayFile, String> {
    error_log::log_command_enter("set_day_note", &format!("date={}", date));
    let root = std::path::Path::new(&root_path);
    validate_date_format(&date)?;
    // A day note does not customize the month's dimensions, so it must not
    // trigger instantiation (would freeze the month to the current template).

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
    let result = files::read_commitments_file(root, year, month);
    let ok = result.is_ok();
    let count = result.as_ref().map(|c| c.len()).unwrap_or(0);
    error_log::log_command_exit("get_commitments", ok, &format!("{} commitments", count));
    result
}

#[tauri::command]
pub fn get_month_dimensions(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<MonthDimensions, String> {
    error_log::log_command_enter("get_month_dimensions", &format!("{}-{:02}", year, month));
    let root = std::path::Path::new(&root_path);
    // A month is "instantiated" iff its _monthly.md has a non-empty dimensions block.
    let from_template = match files::read_monthly_file(root, year, month) {
        Ok(m) => m.dimensions.is_empty(),
        Err(e) => {
            error_log::log_error(
                "get_month_dimensions",
                &format!("Failed to read _monthly.md for {}-{:02}: {:?}", year, month, e),
            );
            true
        }
    };
    let dimensions = files::resolve_month_dimensions(root, year, month)?;
    error_log::log_command_exit(
        "get_month_dimensions",
        true,
        &format!("{} dims, from_template={}", dimensions.len(), from_template),
    );
    Ok(MonthDimensions { dimensions, from_template })
}

/// The dimension key used to tag a commitment goal for this month. Conventionally
/// "goal", but validate_dimensions only requires source=="monthly" (the key is
/// free), so resolve it dynamically and fall back to "goal" when none/unreadable.
fn monthly_dim_key(root: &std::path::Path, year: i32, month: u32) -> String {
    files::resolve_month_dimensions(root, year, month)
        .ok()
        .and_then(|dims| dims.into_iter().find(|d| d.source == "monthly").map(|d| d.key))
        .unwrap_or_else(|| "goal".to_string())
}

fn compute_attribution(
    dimensions: &BTreeMap<String, String>,
    goal_key: &str,
    goal_to_role: &std::collections::HashMap<String, String>,
    role_to_goals: &std::collections::HashMap<String, Vec<String>>,
) -> crate::models::Attribution {
    use crate::models::Attribution;
    let role = dimensions.get("role");
    let goal = dimensions.get(goal_key);

    match (role, goal) {
        (None, None) => Attribution::Unattributed,
        (None, Some(g)) => {
            if goal_to_role.contains_key(g.as_str()) {
                Attribution::Ok
            } else {
                Attribution::Unattributed
            }
        }
        (Some(_), None) => Attribution::Ok,
        (Some(r), Some(g)) => {
            if let Some(goals) = role_to_goals.get(r.as_str()) {
                if goals.contains(g) {
                    Attribution::Ok
                } else {
                    Attribution::Mismatch
                }
            } else {
                Attribution::Unattributed
            }
        }
    }
}

/// 从 commitments 构建 goal→role 和 role→goals 映射
fn build_commitment_maps(
    commitments: &[crate::models::Commitment],
) -> (
    std::collections::HashMap<String, String>,
    std::collections::HashMap<String, Vec<String>>,
) {
    let mut goal_to_role: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut role_to_goals: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for c in commitments {
        let goals = c.goals.clone();
        for g in &goals {
            goal_to_role.insert(g.clone(), c.role.clone());
        }
        role_to_goals.insert(c.role.clone(), goals);
    }
    (goal_to_role, role_to_goals)
}

/// 为 DayFile 中所有 entry 计算 attribution
fn annotate_day_file(
    day_file: &mut crate::models::DayFile,
    goal_key: &str,
    goal_to_role: &std::collections::HashMap<String, String>,
    role_to_goals: &std::collections::HashMap<String, Vec<String>>,
) {
    for entry in &mut day_file.entries {
        entry.attribution = compute_attribution(&entry.dimensions, goal_key, goal_to_role, role_to_goals);
    }
}

#[tauri::command]
pub fn get_commitment_progress(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<crate::models::CommitmentProgressResult, String> {
    use crate::models::{CommitmentProgress, GoalProgress};
    use std::collections::HashMap;

    let root = std::path::Path::new(&root_path);

    // 1. Read commitments.yaml
    let commitments = crate::files::read_commitments_file(root, year, month).unwrap_or_else(|e| {
        error_log::log_error(
            "get_commitment_progress",
            &format!("Failed to read commitments.yaml for {}-{:02}: {:?}", year, month, e),
        );
        vec![]
    });

    // 2. Build maps
    let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);

    // 3. Initialize result structures
    let mut role_goal_spent: HashMap<String, u32> = HashMap::new();
    let mut role_general_spent: HashMap<String, u32> = HashMap::new();
    let mut goal_spent: HashMap<String, u32> = HashMap::new();
    let mut unattributed_count: u32 = 0;
    let mut unattributed_total: u32 = 0;
    let mut mismatch_count: u32 = 0;

    for c in &commitments {
        role_goal_spent.entry(c.role.clone()).or_insert(0);
        role_general_spent.entry(c.role.clone()).or_insert(0);
        for g in &c.goals {
            goal_spent.entry(g.clone()).or_insert(0);
        }
    }

    // 4. Scan day files
    let goal_key = monthly_dim_key(root, year, month);
    let month_dir = root.join(year.to_string()).join(format!("{:02}", month));

    if month_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&month_dir) {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        error_log::log_error("get_commitment_progress", &format!("read_dir error: {:?}", e));
                        continue;
                    }
                };
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "_monthly.md" || !file_name.ends_with(".md") {
                    continue;
                }
                match crate::files::read_day_file(root, file_name.trim_end_matches(".md")) {
                    Ok(day_file) => {
                        for e in &day_file.entries {
                            let attr = compute_attribution(&e.dimensions, &goal_key, &goal_to_role, &role_to_goals);

                            match attr {
                                crate::models::Attribution::Ok => {
                                    // Determine which role and whether it's goal or general
                                    if let Some(role) = e.dimensions.get("role") {
                                        if let Some(goal_val) = e.dimensions.get(&goal_key) {
                                            if let Some(goals) = role_to_goals.get(role) {
                                                if goals.contains(goal_val) {
                                                    // Matching goal -> goal segment
                                                    *role_goal_spent.entry(role.clone()).or_insert(0) += e.duration;
                                                    *goal_spent.entry(goal_val.clone()).or_insert(0) += e.duration;
                                                } else {
                                                    // Goal not declared for this role -> general segment
                                                    *role_general_spent.entry(role.clone()).or_insert(0) += e.duration;
                                                }
                                            } else {
                                                *role_general_spent.entry(role.clone()).or_insert(0) += e.duration;
                                            }
                                        } else {
                                            // No goal -> general segment
                                            *role_general_spent.entry(role.clone()).or_insert(0) += e.duration;
                                        }
                                    } else if let Some(goal_val) = e.dimensions.get(&goal_key) {
                                        // No role, but has goal -> fallback to goal's role
                                        if let Some(role) = goal_to_role.get(goal_val) {
                                            *role_goal_spent.entry(role.clone()).or_insert(0) += e.duration;
                                            *goal_spent.entry(goal_val.clone()).or_insert(0) += e.duration;
                                        }
                                    }
                                }
                                crate::models::Attribution::Unattributed => {
                                    unattributed_count += 1;
                                    unattributed_total += e.duration;
                                }
                                crate::models::Attribution::Mismatch => {
                                    mismatch_count += 1;
                                    // Still count toward the role's general segment
                                    if let Some(role) = e.dimensions.get("role") {
                                        *role_general_spent.entry(role.clone()).or_insert(0) += e.duration;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error_log::log_error("get_commitment_progress", &format!("read_day_file error: {}", e));
                    }
                }
            }
        }
    }

    // 5. Build result
    let mut roles: Vec<CommitmentProgress> = Vec::new();
    for c in &commitments {
        let goals: Vec<GoalProgress> = c
            .goals
            .iter()
            .map(|g| GoalProgress {
                name: g.clone(),
                spent_minutes: *goal_spent.get(g).unwrap_or(&0),
            })
            .collect();
        roles.push(CommitmentProgress {
            role: c.role.clone(),
            allocation_minutes: c.allocation * 60,
            goal_spent_minutes: *role_goal_spent.get(&c.role).unwrap_or(&0),
            general_spent_minutes: *role_general_spent.get(&c.role).unwrap_or(&0),
            goals,
        });
    }

    Ok(crate::models::CommitmentProgressResult {
        roles,
        unattributed_count,
        unattributed_total_minutes: unattributed_total,
        mismatch_count,
    })
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

    // 2. Snapshot template dims if this month is fresh (preserves any dims block)
    files::create_dimensions_if_missing(root, year, month)?;

    // 3. Read old commitments for diff
    let old_commitments = files::read_commitments_file(root, year, month).unwrap_or_else(|e| {
        error_log::log_error(
            "set_commitments",
            &format!("Failed to read old commitments: {}", e),
        );
        vec![]
    });

    // 4. Detect changes
    let changes = detect_goal_changes(&old_commitments, &commitments);

    // 5. Check deleted goals for existing entries
    for goal_name in &changes.deleted {
        let count = count_entries_with_goal(root, year, month, goal_name)?;
        if count > 0 {
            return Err(format!(
                "Cannot delete goal '{}': used by {} entries this month",
                goal_name, count
            ));
        }
    }

    // 6. Apply renames to all day files
    for (old_name, new_name) in &changes.renames {
        rename_goal_in_entries(root, year, month, old_name, new_name)?;
    }

    // 6b. Detect and apply role renames
    let month_dir = root.join(year.to_string()).join(format!("{:02}", month));
    let role_changes = detect_role_changes(&old_commitments, &commitments);
    for (old_name, new_name) in &role_changes {
        if let Ok(entries) = std::fs::read_dir(&month_dir) {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "_monthly.md" || !file_name.ends_with(".md") {
                    continue;
                }
                if let Ok(mut day_file) = crate::files::read_day_file(root, file_name.trim_end_matches(".md")) {
                    let mut changed = false;
                    for e in &mut day_file.entries {
                        if e.dimensions.get("role").map(|r| r == old_name).unwrap_or(false) {
                            e.dimensions.insert("role".to_string(), new_name.to_string());
                            changed = true;
                        }
                    }
                    if changed {
                        let _ = crate::files::write_day_file(root, file_name.trim_end_matches(".md"), &day_file);
                    }
                }
            }
        }
    }

    // 6c. Detect and clear role dimension for deleted roles
    let old_role_names: std::collections::BTreeSet<&String> = old_commitments.iter().map(|c| &c.role).collect();
    let new_role_names: std::collections::BTreeSet<&String> = commitments.iter().map(|c| &c.role).collect();
    let deleted_roles: Vec<&String> = old_role_names.difference(&new_role_names).cloned().collect();

    for role_name in &deleted_roles {
        if let Ok(entries) = std::fs::read_dir(&month_dir) {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "_monthly.md" || !file_name.ends_with(".md") {
                    continue;
                }
                if let Ok(mut day_file) = crate::files::read_day_file(root, file_name.trim_end_matches(".md")) {
                    let mut changed = false;
                    for e in &mut day_file.entries {
                        if e.dimensions.get("role").map(|r| r == *role_name).unwrap_or(false) {
                            e.dimensions.remove("role");
                            changed = true;
                        }
                    }
                    if changed {
                        let _ = crate::files::write_day_file(root, file_name.trim_end_matches(".md"), &day_file);
                    }
                }
            }
        }
    }

    // 7. Write commitments.yaml
    files::write_commitments_file(root, year, month, &commitments)?;

    let ok = true;
    error_log::log_command_exit("set_commitments", ok, "");
    Ok(commitments)
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
    let goal_key = monthly_dim_key(root, year, month);
    let entries = match std::fs::read_dir(&month_dir) {
        Ok(e) => e,
        Err(e) => return Err(format!("Failed to read month dir: {}", e)),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                error_log::log_error(
                    "count_entries_with_goal",
                    &format!("Failed to read dir entry in {}-{:02}: {:?}", year, month, e),
                );
                continue;
            }
        };
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "_monthly.md" || !file_name.ends_with(".md") {
            continue;
        }
        let date = file_name.trim_end_matches(".md");
        match files::read_day_file(root, date) {
            Ok(day_file) => {
                count += day_file
                    .entries
                    .iter()
                    .filter(|e| e.dimensions.get(&goal_key).map(|g| g == goal_name).unwrap_or(false))
                    .count();
            }
            Err(e) => {
                error_log::log_error(
                    "count_entries_with_goal",
                    &format!("Failed to read day file {}: {}", date, e),
                );
            }
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

    let goal_key = monthly_dim_key(root, year, month);

    // Phase 1: read + transform every affected day file in memory. A stray
    // non-date .md is skipped (tolerant); a valid-date file that fails to read
    // aborts here, BEFORE any write, so we never leave a partial rename.
    let mut pending: Vec<(String, crate::models::DayFile)> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                error_log::log_error(
                    "rename_goal_in_entries",
                    &format!("Failed to read dir entry in {}-{:02}: {:?}", year, month, e),
                );
                continue;
            }
        };
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "_monthly.md" || !file_name.ends_with(".md") {
            continue;
        }
        let date = file_name.trim_end_matches(".md");
        // Skip files whose name is not a valid date (user scratch files etc.).
        if validate_date_format(date).is_err() {
            continue;
        }
        let mut day_file = files::read_day_file(root, date)?;
        let mut changed = false;
        for e in &mut day_file.entries {
            if let Some(goal) = e.dimensions.get(&goal_key) {
                if goal == old_name {
                    e.dimensions.insert(goal_key.clone(), new_name.to_string());
                    changed = true;
                }
            }
        }
        if changed {
            pending.push((date.to_string(), day_file));
        }
    }

    // Phase 2: all reads succeeded — now commit the writes.
    for (date, day_file) in &pending {
        files::write_day_file(root, date, day_file)?;
    }
    Ok(())
}

pub fn validate_date_format(date: &str) -> Result<chrono::NaiveDate, String> {
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

/// 检测 role 改名：新旧 commitments 之间，role 名变了但 goals 集合相同。
/// 返回 (old_name, new_name) 列表。
fn detect_role_changes(old: &[crate::models::Commitment], new: &[crate::models::Commitment]) -> Vec<(String, String)> {
    let mut changes = Vec::new();
    for o in old {
        let old_goals: std::collections::BTreeSet<&String> = o.goals.iter().collect();
        if let Some(n) = new.iter().find(|n| {
            let new_goals: std::collections::BTreeSet<&String> = n.goals.iter().collect();
            old_goals == new_goals && o.role != n.role
        }) {
            changes.push((o.role.clone(), n.role.clone()));
        }
    }
    changes
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

    for year_entry in year_entries {
        let year_entry = match year_entry {
            Ok(e) => e,
            Err(e) => {
                error_log::log_error(
                    "get_available_months",
                    &format!("Failed to read year entry: {:?}", e),
                );
                continue;
            }
        };
        let is_dir = match year_entry.file_type() {
            Ok(t) => t.is_dir(),
            Err(e) => {
                error_log::log_error(
                    "get_available_months",
                    &format!("Failed to stat year entry {}: {:?}", year_entry.file_name().to_string_lossy(), e),
                );
                false
            }
        };
        if !is_dir {
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
            Err(e) => {
                error_log::log_error(
                    "get_available_months",
                    &format!("Failed to read month dir for year {}: {:?}", year, e),
                );
                continue;
            }
        };

        for month_entry in month_entries {
            let month_entry = match month_entry {
                Ok(e) => e,
                Err(e) => {
                    error_log::log_error(
                        "get_available_months",
                        &format!("Failed to read month entry in year {}: {:?}", year, e),
                    );
                    continue;
                }
            };
            let is_dir = match month_entry.file_type() {
                Ok(t) => t.is_dir(),
                Err(e) => {
                    error_log::log_error(
                        "get_available_months",
                        &format!("Failed to stat month entry {}: {:?}", month_entry.file_name().to_string_lossy(), e),
                    );
                    false
                }
            };
            if !is_dir {
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
                Ok(entries) => {
                    let mut found = false;
                    for e in entries {
                        match e {
                            Ok(entry) => {
                                let name_str = entry.file_name().to_string_lossy().into_owned();
                                if name_str.ends_with(".md") && name_str != "_monthly.md" {
                                    found = true;
                                    break;
                                }
                            }
                            Err(e) => {
                                error_log::log_error(
                                    "get_available_months",
                                    &format!("Failed to read entry in month dir {}-{:02}: {:?}", year, month, e),
                                );
                            }
                        }
                    }
                    found
                }
                Err(e) => {
                    error_log::log_error(
                        "get_available_months",
                        &format!("Failed to read month contents for {}-{:02}: {:?}", year, month, e),
                    );
                    false
                }
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

/// (path, select) for revealing the template: select dimensions.template.yaml if it exists,
/// else open the root dir.
pub fn reveal_template_target(root: &std::path::Path) -> (std::path::PathBuf, bool) {
    let template = files::dimensions_template_path(root);
    if template.exists() {
        (template, true)
    } else {
        (root.to_path_buf(), false)
    }
}

#[tauri::command]
pub fn reveal_template_file(app: AppHandle, root_path: String) -> Result<(), String> {
    error_log::log_command_enter("reveal_template_file", &format!("root={}", root_path));
    let root = std::path::Path::new(&root_path);
    let (target, select) = reveal_template_target(root);
    let result = if select {
        app.opener()
            .reveal_item_in_dir(&target)
            .map_err(|e| format!("Failed to reveal {}: {}", target.display(), e))
    } else {
        app.opener()
            .open_path(target.to_string_lossy().into_owned(), None::<String>)
            .map_err(|e| format!("Failed to open {}: {}", target.display(), e))
    };
    error_log::log_command_exit("reveal_template_file", result.is_ok(), "");
    result
}

#[tauri::command]
pub fn create_starter_files(path: String) -> Result<(), String> {
    let root = std::path::Path::new(&path);
    if !root.exists() {
        std::fs::create_dir_all(root).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let template_path = root.join("dimensions.template.yaml");
    if !template_path.exists() {
        std::fs::write(
            &template_path,
            "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n",
        )
        .map_err(|e| format!("Failed to write dimensions.template.yaml: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_dimensions(
    root_path: String,
    year: i32,
    month: u32,
    dimensions: Vec<Dimension>,
) -> Result<Vec<Dimension>, String> {
    error_log::log_command_enter(
        "save_dimensions",
        &format!("{}-{:02} {} dims", year, month, dimensions.len()),
    );
    let root = std::path::Path::new(&root_path);
    if !root.exists() {
        return Err("Root path does not exist".to_string());
    }

    // Validate before writing
    let errors = validate_dimensions(&dimensions);
    if !errors.is_empty() {
        let messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        let msg = messages.join("; ");
        error_log::log_command_exit("save_dimensions", false, &msg);
        return Err(msg);
    }

    // Write to dimensions.yaml (creates file if month not instantiated)
    files::write_dimensions_file(root, year, month, &dimensions)?;

    error_log::log_command_exit("save_dimensions", true, "");
    Ok(dimensions)
}

#[tauri::command]
pub fn save_dimensions_template(
    root_path: String,
    dimensions: Vec<Dimension>,
) -> Result<(), String> {
    error_log::log_command_enter(
        "save_dimensions_template",
        &format!("{} dims", dimensions.len()),
    );
    let root = std::path::Path::new(&root_path);

    // Validate before writing
    let errors = validate_dimensions(&dimensions);
    if !errors.is_empty() {
        let messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        let msg = messages.join("; ");
        error_log::log_command_exit("save_dimensions_template", false, &msg);
        return Err(msg);
    }

    // Write to dimensions.template.yaml (atomic: tmp then rename)
    let template = Template { dimensions };
    let path = files::dimensions_template_path(root);
    let yaml_body = yaml_serde::to_string(&template)
        .map_err(|e| format!("Failed to serialize template: {}", e))?;
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, yaml_body)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    std::fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;

    error_log::log_command_exit("save_dimensions_template", true, "");
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

    use crate::models::{Dimension, Template};
    use std::collections::BTreeMap;

    fn make_config(required_keys: &[&str]) -> Template {
        Template {
            dimensions: vec![
                Dimension {
                    name: "Biz".into(),
                    key: "biz".into(),
                    source: "static".into(),
                    values: Some(vec!["A".into()]),
                    required: required_keys.contains(&"biz"),
                    deleted: false,
                },
                Dimension {
                    name: "Cat".into(),
                    key: "cat".into(),
                    source: "static".into(),
                    values: Some(vec!["X".into()]),
                    required: required_keys.contains(&"cat"),
                    deleted: false,
                },
                Dimension {
                    name: "Goal".into(),
                    key: "goal".into(),
                    source: "monthly".into(),
                    values: None,
                    required: required_keys.contains(&"goal"),
                    deleted: false,
                },
            ],
        }
    }

    #[test]
    fn test_validate_required_all_present() {
        let config = make_config(&["biz"]);
        let mut dims = BTreeMap::new();
        dims.insert("biz".to_string(), "A".to_string());
        assert!(validate_required_dimensions(&config.dimensions, &dims).is_ok());
    }

    #[test]
    fn test_validate_required_missing_one() {
        let config = make_config(&["biz", "cat"]);
        let mut dims = BTreeMap::new();
        dims.insert("biz".to_string(), "A".to_string());
        // cat is missing
        let err = validate_required_dimensions(&config.dimensions, &dims).unwrap_err();
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
        let dims = BTreeMap::new(); // empty is fine — nothing required
        assert!(validate_required_dimensions(&config.dimensions, &dims).is_ok());
    }

    #[test]
    fn test_validate_required_empty_dimensions() {
        let config = make_config(&["biz"]);
        let dims = BTreeMap::new();
        let err = validate_required_dimensions(&config.dimensions, &dims).unwrap_err();
        assert!(err.contains("Biz"));
    }

    #[test]
    fn test_get_commitment_progress_empty_month() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_test_cp_empty");
        let _ = fs::remove_dir_all(&tmp);

        // Create directory structure with commitments.yaml but no day files
        let monthly_dir = tmp.join("2026").join("06");
        fs::create_dir_all(&monthly_dir).unwrap();
        fs::write(
            monthly_dir.join("commitments.yaml"),
            "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it\n",
        )
        .unwrap();

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        assert_eq!(result.roles.len(), 1);
        assert_eq!(result.roles[0].role, "Dev");
        assert_eq!(result.roles[0].allocation_minutes, 2400); // 40 * 60
        assert_eq!(result.roles[0].goal_spent_minutes, 0);
        assert_eq!(result.roles[0].goals.len(), 1);
        assert_eq!(result.roles[0].goals[0].name, "Ship it");
        assert_eq!(result.roles[0].goals[0].spent_minutes, 0);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_commitment_progress_aggregates_spent() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_test_cp_agg");
        let _ = fs::remove_dir_all(&tmp);

        // Create commitments.yaml
        let monthly_dir = tmp.join("2026").join("06");
        fs::create_dir_all(&monthly_dir).unwrap();
        fs::write(
            monthly_dir.join("commitments.yaml"),
            "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it\n    - Review\n- role: PM\n  allocation: 10\n  goals:\n    - Planning\n",
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
        let dev = result.roles.iter().find(|c| c.role == "Dev").unwrap();
        assert_eq!(dev.goal_spent_minutes, 90);
        assert_eq!(dev.allocation_minutes, 2400);

        // PM: Planning(45) = 45 spent
        let pm = result.roles.iter().find(|c| c.role == "PM").unwrap();
        assert_eq!(pm.goal_spent_minutes, 45);
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
            monthly_dir.join("commitments.yaml"),
            "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it\n",
        )
        .unwrap();

        // Entry with a goal NOT in any commitment
        fs::write(
            monthly_dir.join("2026-06-01.md"),
            "---\nentries:\n  - id: e1\n    item: Unknown task\n    duration: 60\n    dimensions:\n      goal: Not a goal\n---\n",
        )
        .unwrap();

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        assert_eq!(result.roles[0].goal_spent_minutes, 0);
        assert_eq!(result.roles[0].goals[0].spent_minutes, 0);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_commitment_progress_no_monthly_file() {
        let tmp = std::env::temp_dir().join("logbook_test_cp_nofile");
        let _ = std::fs::remove_dir_all(&tmp);

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        assert!(result.roles.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_commitment_progress_with_role_dimension() {
        let tmp = std::env::temp_dir().join("logbook-test-role-progress");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // Setup: dimensions.template.yaml
        let template = r#"dimensions:
  - name: Goal
    key: goal
    source: monthly
"#;
        std::fs::write(tmp.join("dimensions.template.yaml"), template).unwrap();

        // Setup: commitments.yaml
        let commitments_yaml = r#"- role: Dev
  allocation: 20
  goals:
    - Ship X
- role: PM
  allocation: 10
  goals:
    - Roadmap
"#;
        let month_dir = tmp.join("2026").join("07");
        std::fs::create_dir_all(&month_dir).unwrap();
        std::fs::write(month_dir.join("commitments.yaml"), commitments_yaml).unwrap();
        std::fs::write(month_dir.join("dimensions.yaml"), "dimensions: []\n").unwrap();

        // Day 1: entry with role=Dev, goal=Ship X -> Ok, goal segment
        let day1 = r#"---
entries:
  - id: e1
    item: Code feature
    duration: 120
    dimensions:
      role: Dev
      goal: Ship X
  - id: e2
    item: Standup
    duration: 30
    dimensions:
      role: Dev
  - id: e3
    item: Email
    duration: 15
    dimensions: {}
---"#;
        std::fs::write(month_dir.join("2026-07-01.md"), day1).unwrap();

        // Day 2: entry via goal fallback (no role dim) + mismatch case
        let day2 = r#"---
entries:
  - id: e4
    item: Roadmap planning
    duration: 60
    dimensions:
      goal: Roadmap
  - id: e5
    item: Mismatch case
    duration: 45
    dimensions:
      role: Dev
      goal: Roadmap
---"#;
        std::fs::write(month_dir.join("2026-07-02.md"), day2).unwrap();

        let result = get_commitment_progress(
            tmp.to_string_lossy().to_string(),
            2026,
            7,
        ).unwrap();

        // Dev role
        let dev = result.roles.iter().find(|r| r.role == "Dev").unwrap();
        // e1 (120m goal=Ship X) + e2 (30m general) + e5 (45m goal=Roadmap but Dev role -> mismatch -> general)
        assert_eq!(dev.goal_spent_minutes, 120);  // only e1
        assert_eq!(dev.general_spent_minutes, 75);  // e2 + e5
        assert_eq!(dev.allocation_minutes, 1200);

        // PM role
        let pm = result.roles.iter().find(|r| r.role == "PM").unwrap();
        // e4 (60m goal=Roadmap, fallback to PM)
        assert_eq!(pm.goal_spent_minutes, 60);
        assert_eq!(pm.general_spent_minutes, 0);

        // Unattributed
        assert_eq!(result.unattributed_count, 1);  // e3
        assert_eq!(result.unattributed_total_minutes, 15);

        // Mismatch
        assert_eq!(result.mismatch_count, 1);  // e5

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

    #[test]
    fn test_compute_attribution_unattributed_no_dimensions() {
        use std::collections::HashMap;
        let dims = BTreeMap::new();
        let goal_to_role: HashMap<String, String> = HashMap::new();
        let role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
        let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
        assert_eq!(result, Attribution::Unattributed);
    }

    #[test]
    fn test_compute_attribution_ok_via_goal_fallback() {
        use std::collections::HashMap;
        let mut dims = BTreeMap::new();
        dims.insert("goal".to_string(), "Ship X".to_string());
        let mut goal_to_role: HashMap<String, String> = HashMap::new();
        goal_to_role.insert("Ship X".to_string(), "Dev".to_string());
        let role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
        let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
        assert_eq!(result, Attribution::Ok);
    }

    #[test]
    fn test_compute_attribution_unattributed_unknown_goal() {
        use std::collections::HashMap;
        let mut dims = BTreeMap::new();
        dims.insert("goal".to_string(), "Unknown".to_string());
        let goal_to_role: HashMap<String, String> = HashMap::new();
        let role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
        let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
        assert_eq!(result, Attribution::Unattributed);
    }

    #[test]
    fn test_compute_attribution_ok_role_only() {
        use std::collections::HashMap;
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Dev".to_string());
        let goal_to_role: HashMap<String, String> = HashMap::new();
        let mut role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
        role_to_goals.insert("Dev".to_string(), vec!["Ship X".to_string()]);
        let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
        assert_eq!(result, Attribution::Ok);
    }

    #[test]
    fn test_compute_attribution_ok_role_and_matching_goal() {
        use std::collections::HashMap;
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Dev".to_string());
        dims.insert("goal".to_string(), "Ship X".to_string());
        let goal_to_role: HashMap<String, String> = HashMap::new();
        let mut role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
        role_to_goals.insert("Dev".to_string(), vec!["Ship X".to_string()]);
        let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
        assert_eq!(result, Attribution::Ok);
    }

    #[test]
    fn test_compute_attribution_mismatch() {
        use std::collections::HashMap;
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Dev".to_string());
        dims.insert("goal".to_string(), "Design review".to_string());
        let goal_to_role: HashMap<String, String> = HashMap::new();
        let mut role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
        role_to_goals.insert("Dev".to_string(), vec!["Ship X".to_string()]);
        let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
        assert_eq!(result, Attribution::Mismatch);
    }

    #[test]
    fn test_compute_attribution_unattributed_unknown_role() {
        use std::collections::HashMap;
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Ghost".to_string());
        dims.insert("goal".to_string(), "Ship X".to_string());
        let goal_to_role: HashMap<String, String> = HashMap::new();
        let role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
        let result = compute_attribution(&dims, "goal", &goal_to_role, &role_to_goals);
        assert_eq!(result, Attribution::Unattributed);
    }

    #[test]
    fn test_compute_attribution_dynamic_goal_key() {
        use std::collections::HashMap;
        let mut dims = BTreeMap::new();
        dims.insert("objective".to_string(), "Launch".to_string());
        let mut goal_to_role: HashMap<String, String> = HashMap::new();
        goal_to_role.insert("Launch".to_string(), "PM".to_string());
        let role_to_goals: HashMap<String, Vec<String>> = HashMap::new();
        let result = compute_attribution(&dims, "objective", &goal_to_role, &role_to_goals);
        assert_eq!(result, Attribution::Ok);
    }
}
