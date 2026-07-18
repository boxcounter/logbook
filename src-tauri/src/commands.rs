use crate::config::validate_dimensions;
use crate::error_log;
use crate::integrity;
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


/// Parse a duration string to minutes.
/// NOTE: Kept in sync with TS `parseDurationFromText` in src/utils/format.ts.
/// Any change to the regex, unit handling, or rounding must be mirrored on the TS side.
/// Handles: "90", "1.5h", "30m", "1h 30m", "准备会议（15m），面聊（45m）"
pub fn parse_duration(input: &str) -> Result<u32, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Duration is empty".to_string());
    }

    if let Ok(n) = input.parse::<f64>() {
        if n > 0.0 {
            return Ok(n.round() as u32);
        }
        return Err("Duration is zero".to_string());
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
            "Could not parse duration from '{}'. Expected format like 1.5h, 30m, or 2h 15m",
            input
        ));
    }

    let total = total.round() as u32;
    if total == 0 {
        return Err("Parsed duration is zero".to_string());
    }
    Ok(total)
}

/// Check that the data root has a version.txt matching `expected_version`.
/// Pure function — does not modify files.
pub fn check_data_version(
    root: &std::path::Path,
    expected_version: u32,
) -> Result<(), InitResult> {
    let version = match files::read_version_file(root) {
        Ok(Some(v)) => v,
        Ok(None) => {
            return Err(InitResult::DataVersionNotFound {
                root_path: root.to_string_lossy().into_owned(),
            });
        }
        Err(e) => {
            error_log::log_error("check_data_version", &format!("read_version_file failed: {}", e));
            return Err(InitResult::DataVersionNotFound {
                root_path: root.to_string_lossy().into_owned(),
            });
        }
    };

    if version != expected_version {
        return Err(InitResult::DataVersionMismatch {
            root_path: root.to_string_lossy().into_owned(),
            expected: expected_version,
            found: version,
        });
    }

    Ok(())
}

/// Validate that all required dimensions have values in the entry.
/// Returns Ok(()) or Err with a human-readable message naming the first missing required dimension.
pub fn validate_required_dimensions(
    dimensions: &[Dimension],
    entry_dimensions: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    for dim in dimensions {
        if dim.deleted {
            continue;
        }
        if dim.required && !entry_dimensions.contains_key(&dim.key) {
            return Err(format!("Missing required dimension: {}", dim.name));
        }
    }
    Ok(())
}

/// Validate cross-dimension constraints: if commitments are declared,
/// (1) any role value must be listed in commitments, and
/// (2) if both role and goal are present, the goal must be declared under that role.
fn validate_cross_dimension_constraints(
    dimensions: &BTreeMap<String, String>,
    role_key: &str,
    goal_key: &str,
    role_to_goals: &std::collections::HashMap<String, Vec<String>>,
) -> Result<(), String> {
    let role = dimensions.get(role_key);
    let goal = dimensions.get(goal_key);

    if let Some(r) = role {
        if !role_to_goals.is_empty() && !role_to_goals.contains_key(r.as_str()) {
            return Err(format!(
                "Role '{}' is not declared in commitments",
                r
            ));
        }

        if let Some(g) = goal {
            if let Some(goals) = role_to_goals.get(r.as_str()) {
                if !goals.contains(g) {
                    return Err(format!(
                        "Goal '{}' is not declared under role '{}'",
                        g, r
                    ));
                }
            }
        }
    } else if let Some(g) = goal {
        // Goal present without a role: if commitments exist, the goal must be
        // declared under at least one role — otherwise it's an unknown goal.
        if !role_to_goals.is_empty() && !role_to_goals.values().any(|goals| goals.contains(g)) {
            return Err(format!(
                "Goal '{}' is not declared in any role in commitments",
                g
            ));
        }
    }
    Ok(())
}

/// Validate that all dimension keys in the entry are declared in the
/// dimension config. Deleted dimensions are still considered known keys.
fn check_unknown_dimension_keys(
    dimension_config: &[Dimension],
    dimensions: &BTreeMap<String, String>,
) -> Result<(), String> {
    let known_keys: std::collections::HashSet<&str> = dimension_config
        .iter()
        .map(|d| d.key.as_str())
        .collect();
    for key in dimensions.keys() {
        if !known_keys.contains(key.as_str()) {
            return Err(format!("Unknown dimension key '{}'", key));
        }
    }
    Ok(())
}

/// Unified pre-write validation for entry input (append + update paths).
/// Returns parsed duration (u32 minutes) on success.
pub fn validate_entry_input(
    item: &str,
    duration_str: &str,
    dimensions: &BTreeMap<String, String>,
    dimension_config: &[Dimension],
    role_key: &str,
    goal_key: &str,
    role_to_goals: &std::collections::HashMap<String, Vec<String>>,
) -> Result<u32, String> {
    if item.trim().is_empty() {
        return Err("Entry item cannot be empty".to_string());
    }

    let duration = parse_duration(duration_str)?;

    check_unknown_dimension_keys(dimension_config, dimensions)?;

    validate_required_dimensions(dimension_config, dimensions)?;

    validate_cross_dimension_constraints(dimensions, role_key, goal_key, role_to_goals)?;

    Ok(duration)
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

    let mut all_errors: Vec<ConfigErrorDetail> = validate_dimensions(&template.dimensions)
        .into_iter()
        .map(|e| ConfigErrorDetail {
            kind: e.kind,
            message: format!("dimensions.template.yaml: {}", e.message),
        })
        .collect();

    let now = chrono::Local::now();
    // NOTE: `now` is captured once per init call. The frontend maintains an
    // independent `new Date()` that is refreshed every 60s (App.vue rollover
    // check). A brief discrepancy (<60s) around midnight between the two
    // clocks is harmless — the rollover window is bounded and the worst case
    // is a single entry landing in the wrong day file.

    // Read commitments from commitments.yaml
    let commitments = match files::read_commitments_file(root, now.year(), now.month()) {
        Ok(c) => {
            if !c.is_empty() {
                if let Err(e) = crate::config::validate_commitments(&c) {
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

    // Run repair sweep on startup to recover from crashes during set_commitments
    // pipeline (where commitments.yaml was written but day files have stale
    // role/goal values). Scan every month that has data on disk — not just the
    // current month — so past-month crash residue is always cleaned up.
    if !commitments.is_empty() {
        repair_entry_dimensions_all(root);
    }

    let today_date = format!("{}-{:02}-{:02}", now.year(), now.month(), now.day());
    let today = match read_day_file_safe(root, &today_date) {
        Ok(df) => df,
        Err(e) => {
            all_errors.push(ConfigErrorDetail {
                kind: "DayFileCorrupt".to_string(),
                message: e,
            });
            DayFile { note: None, entries: vec![] }
        }
    };

    let using_default_dimensions = files::read_dimensions_file(root, now.year(), now.month())
        .unwrap_or_else(|e| {
            error_log::log_error("load_root_state:dimensions", &format!("read failed: {e}"));
            all_errors.push(ConfigErrorDetail {
                kind: "DimensionsFileCorrupt".to_string(),
                message: format!("Failed to read dimensions for {}-{:02}: {}", now.year(), now.month(), e),
            });
            Default::default()
        })
        .is_empty();
    let dimensions = files::resolve_month_dimensions(root, now.year(), now.month())
        .unwrap_or_else(|e| {
            error_log::log_error("load_root_state:dimensions", &format!("resolve failed: {e}"));
            all_errors.push(ConfigErrorDetail {
                kind: "DimensionsResolveFailed".to_string(),
                message: format!("Failed to resolve dimensions for {}-{:02}: {}", now.year(), now.month(), e),
            });
            Default::default()
        });

    if !all_errors.is_empty() {
        return InitResult::ConfigError {
            category: RecoveryCategory::InPlace,
            root_path: root.to_string_lossy().into_owned(),
            errors: all_errors,
            scan_warnings,
        };
    }

    let integrity_issues = integrity::check_scoped_integrity(root);
    for issue in &integrity_issues {
        integrity::set_compromised(issue.clone());
    }

    InitResult::Ready {
        root_path: root.to_string_lossy().into_owned(),
        dimensions,
        usingDefaultDimensions: using_default_dimensions,
        today,
        commitments,
        scan_warnings,
        integrity_issues,
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

    match check_data_version(&root_path, CURRENT_DATA_VERSION) {
        Err(e) => {
            match &e {
                InitResult::DataVersionNotFound { .. } => {
                    error_log::log_command_exit("init", true, "DataVersionNotFound");
                }
                InitResult::DataVersionMismatch { expected, found, .. } => {
                    error_log::log_command_exit(
                        "init",
                        false,
                        &format!("DataVersionMismatch: expected {}, found {}", expected, found),
                    );
                }
                _ => unreachable!(),
            }
            return e;
        }
        Ok(()) => {}
    }

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
        InitResult::DataVersionNotFound { .. } | InitResult::DataVersionMismatch { .. } => {
            unreachable!("version check should have returned early")
        }
    }
    result
}

/// Returns true if the directory contains any data-relevant content: a YYYY
/// year directory, or any .yaml/.md file (template, commitments, dimensions,
/// day files) within it. Unrelated files (.DS_Store, *.txt) do not count.
/// Unreadable entries count as "has content" (fail closed: when we cannot
/// confirm emptiness, we must not stamp).
fn dir_has_data_content(root: &std::path::Path) -> bool {
    const MAX_DEPTH: u32 = 3;
    fn recurse(dir: &std::path::Path, depth: u32) -> bool {
        if depth > MAX_DEPTH {
            return false;
        }
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                error_log::log_error(
                    "dir_has_data_content",
                    &format!("Failed to read directory {}: {}", dir.display(), e),
                );
                return true; // cannot confirm emptiness — do not stamp
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    error_log::log_error(
                        "dir_has_data_content",
                        &format!("Failed to read dir entry in {}: {:?}", dir.display(), e),
                    );
                    return true; // cannot confirm emptiness — do not stamp
                }
            };
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                // YYYY year directory → data content
                if name.len() == 4 && name.parse::<u32>().is_ok() {
                    return true;
                }
                if recurse(&path, depth + 1) {
                    return true;
                }
            } else if name.ends_with(".yaml") || name.ends_with(".md") {
                return true;
            }
        }
        false
    }
    recurse(root, 0)
}

/// set_root_path's version guard. Stamps version.txt with the current data
/// version only when the directory is brand new (no version.txt and no data
/// content — preserves the f9eda9a chicken-and-egg fix for first-time setup).
/// Any other directory goes through the same version check as init, so an old
/// (e.g. v1) data tree surfaces DataVersionNotFound / DataVersionMismatch
/// instead of having its version.txt silently overwritten.
pub fn stamp_or_check_version(root: &std::path::Path) -> Result<(), InitResult> {
    if !files::version_path(root).exists() && !dir_has_data_content(root) {
        files::write_version_file(root, CURRENT_DATA_VERSION).map_err(|e| {
            error_log::log_error(
                "stamp_or_check_version",
                &format!("Failed to stamp version.txt in {}: {}", root.display(), e),
            );
            InitResult::ConfigError {
                category: RecoveryCategory::InPlace,
                root_path: root.to_string_lossy().into_owned(),
                errors: vec![ConfigErrorDetail {
                    kind: "VersionStampFailed".to_string(),
                    message: e,
                }],
                scan_warnings: vec![],
            }
        })?;
        return Ok(());
    }
    check_data_version(root, CURRENT_DATA_VERSION)
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

    // Cross-process writer exclusion: swap the data-root writer lock BEFORE
    // persisting anything. If another live process (CLI or other-bundle GUI)
    // holds it, refuse without mutating state — root_path.txt, the watcher
    // and the previously held lock all stay as they were.
    match app.state::<crate::WriterLock>().swap_to(root_path) {
        Ok(_) => {}
        Err(crate::single_instance::InstanceLockError::AlreadyRunning(pid)) => {
            error_log::log_command_exit(
                "set_root_path",
                false,
                &format!("writer lock for {} held by PID {}", path, pid),
            );
            return Err(format!(
                "Another Logbook process is already using this data folder (PID {}). Close it first.",
                pid
            ));
        }
        Err(crate::single_instance::InstanceLockError::Io(e)) => {
            // Fail-open, same posture as startup: log and proceed unlocked.
            error_log::log_error(
                "set_root_path",
                &format!(
                    "Failed to acquire writer lock for {}: {}",
                    root_path.display(),
                    e
                ),
            );
        }
    }

    save_root_path(&app_data_dir, root_path)?;

    // Version guard BEFORE any state load: stamp only brand-new empty dirs,
    // otherwise surface DataVersionNotFound / DataVersionMismatch like init.
    if let Err(version_result) = stamp_or_check_version(root_path) {
        error_log::log_command_exit("set_root_path", true, "version guard rejected");
        return Ok(version_result);
    }

    crate::integrity::reset();
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
        InitResult::DataVersionNotFound { .. } | InitResult::DataVersionMismatch { .. } => {
            // load_root_state never yields version variants; if that ever
            // changes, pass the variant through (the frontend handles both)
            // instead of panicking in the GUI process.
            error_log::log_command_exit("set_root_path", false, "unexpected version variant");
        }
    }
    Ok(result)
}

/// Batch-read all day files for a month. Returns full DayFile (entries + note) keyed by YYYY-MM-DD date.
#[tauri::command]
pub fn get_month_entries(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<std::collections::BTreeMap<String, crate::models::DayFile>, String> {
    error_log::log_command_enter("get_month_entries", &format!("{}-{:02}", year, month));
    let root = std::path::Path::new(&root_path);
    let month_dir = root.join(year.to_string()).join(format!("{:02}", month));

    if !month_dir.exists() {
        error_log::log_command_exit("get_month_entries", true, "no month dir");
        return Ok(std::collections::BTreeMap::new());
    }

    let mut result: std::collections::BTreeMap<String, crate::models::DayFile> =
        std::collections::BTreeMap::new();

    let entries = std::fs::read_dir(&month_dir)
        .map_err(|e| format!("Failed to read month dir: {}", e))?;
    let mut total = 0u32;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                error_log::log_error("get_month_entries", &format!("read_dir entry error: {:?}", e));
                continue;
            }
        };
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !file_name.ends_with(".yaml") {
            continue;
        }
        let date = file_name.trim_end_matches(".yaml");
        if validate_date_format(date).is_err() {
            continue;
        }
        match crate::files::read_day_file(root, date) {
            Ok(day_file) => {
                total += day_file.entries.len() as u32;
                result.insert(date.to_string(), day_file);
            }
            Err(e) => {
                error_log::log_error(
                    "get_month_entries",
                    &format!("Failed to read {}: {:?}", date, e),
                );
                result.insert(date.to_string(), crate::models::DayFile { note: None, entries: vec![] });
            }
        }
    }

    error_log::log_command_exit("get_month_entries", true, &format!("{} days, {} entries", result.len(), total));
    Ok(result)
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
    integrity::check()?;
    validate_date_format(&date)?;

    // Fast-fail: validate item and duration before touching the filesystem.
    // validate_entry_input below will re-check these (cheap), but early
    // rejection prevents unnecessary setup I/O on obviously bad input.
    if entry.item.trim().is_empty() {
        return Err("Entry item cannot be empty".to_string());
    }
    parse_duration(&entry.duration)?;

    let (year, month) = files::year_month_from_date(&date)?;
    files::create_dimensions_if_missing(root, year, month)?;
    let dims = files::resolve_month_dimensions(root, year, month)?;
    // Fail closed: a commitments.yaml that exists but cannot be parsed must
    // reject the write. Swallowing the error into an empty vec would silently
    // disable cross-dimension validation (role_to_goals empty → checks skip).
    // Missing file is fine — read_commitments_file returns Ok(vec![]) then.
    let commitments = crate::files::read_commitments_file(root, year, month).map_err(|e| {
        error_log::log_error(
            "append_entry",
            &format!("commitments.yaml for {}-{:02} unreadable, refusing write: {}", year, month, e),
        );
        format!(
            "commitments.yaml for {}-{:02} exists but cannot be parsed; refusing to write: {}",
            year, month, e
        )
    })?;
    let goal_key = goal_dim_key(root, year, month)?;
    let role_key = role_dim_key(root, year, month)?;
    let (_, role_to_goals) = build_commitment_maps(&commitments);
    let duration = validate_entry_input(
        &entry.item,
        &entry.duration,
        &entry.dimensions,
        &dims,
        &role_key,
        &goal_key,
        &role_to_goals,
    )?;

    // Pre-write integrity check on the target day file
    if let Err(issue) = integrity::check_day_file_integrity(root, &date) {
        integrity::set_compromised(issue.clone());
        return Err(format!(
            "Write denied: target file integrity check failed: {} — {}",
            issue.path, issue.message
        ));
    }

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
    integrity::check()?;
    validate_date_format(&date)?;
    let (year, month) = files::year_month_from_date(&date)?;
    files::create_dimensions_if_missing(root, year, month)?;
    if let Some(ref item) = update.item {
        if item.trim().is_empty() {
            return Err("Entry item cannot be empty".to_string());
        }
    }
    if let Some(ref dur_str) = update.duration {
        parse_duration(dur_str)?;
    }
    if let Some(ref dims) = update.dimensions {
        let effective = files::resolve_month_dimensions(root, year, month)?;
        check_unknown_dimension_keys(&effective, dims)?;
        validate_required_dimensions(&effective, dims)?;
        {
            // Fail closed like append_entry: a corrupt commitments.yaml must
            // reject the write, not silently skip cross-dimension validation.
            let commitments = crate::files::read_commitments_file(root, year, month).map_err(|e| {
                error_log::log_error(
                    "update_entry",
                    &format!("commitments.yaml for {}-{:02} unreadable, refusing write: {}", year, month, e),
                );
                format!(
                    "commitments.yaml for {}-{:02} exists but cannot be parsed; refusing to write: {}",
                    year, month, e
                )
            })?;
            let goal_key = goal_dim_key(root, year, month)?;
            let role_key = role_dim_key(root, year, month)?;
            let (_, role_to_goals) = build_commitment_maps(&commitments);
            validate_cross_dimension_constraints(dims, &role_key, &goal_key, &role_to_goals)?;
        }
    }

    // Pre-write integrity check on the target day file
    if let Err(issue) = integrity::check_day_file_integrity(root, &date) {
        integrity::set_compromised(issue.clone());
        return Err(format!(
            "Write denied: target file integrity check failed: {} — {}",
            issue.path, issue.message
        ));
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
    integrity::check()?;
    validate_date_format(&date)?;
    let (_year, _month) = files::year_month_from_date(&date)?;
    // Deleting an entry does not customize the month's dimensions, so it must not
    // trigger instantiation (would freeze the month to the current template).

    // Pre-write integrity check on the target day file BEFORE reading it
    if let Err(issue) = integrity::check_day_file_integrity(root, &date) {
        integrity::set_compromised(issue.clone());
        return Err(format!(
            "Write denied: target file integrity check failed: {} — {}",
            issue.path, issue.message
        ));
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
    integrity::check()?;
    validate_date_format(&date)?;
    // A day note does not customize the month's dimensions, so it must not
    // trigger instantiation (would freeze the month to the current template).

    // Pre-write integrity check on the target day file BEFORE reading it
    if let Err(issue) = integrity::check_day_file_integrity(root, &date) {
        integrity::set_compromised(issue.clone());
        return Err(format!(
            "Write denied: target file integrity check failed: {} — {}",
            issue.path, issue.message
        ));
    }

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
    // A month uses default dimensions iff its dimensions.yaml is absent or empty.
    let using_default_dimensions = match files::read_dimensions_file(root, year, month) {
        Ok(d) => d.is_empty(),
        Err(e) => {
            error_log::log_error(
                "get_month_dimensions",
                &format!("Failed to read dimensions for {}-{:02}: {:?}", year, month, e),
            );
            true
        }
    };
    let dimensions = files::resolve_month_dimensions(root, year, month)?;
    error_log::log_command_exit(
        "get_month_dimensions",
        true,
        &format!("{} dims, usingDefaultDimensions={}", dimensions.len(), using_default_dimensions),
    );
    Ok(MonthDimensions { dimensions, usingDefaultDimensions: using_default_dimensions })
}

/// The dimension key used to tag a commitment goal for this month. Finds the
/// dimension with source=="commitments:role:goals".
fn goal_dim_key(root: &std::path::Path, year: i32, month: u32) -> Result<String, String> {
    let from_monthly = files::read_dimensions_file(root, year, month)
        .map(|d| !d.is_empty())
        .unwrap_or(false);
    let file = if from_monthly {
        format!("{}/{}/dimensions.yaml", year, format!("{:02}", month))
    } else {
        "dimensions.template.yaml".to_string()
    };
    files::resolve_month_dimensions(root, year, month)?
        .into_iter()
        .find(|d| d.source == "commitments:role:goals")
        .map(|d| d.key)
        .ok_or_else(|| {
            let body = concat!(
                "  - name: Goal\n",
                "    key: goal\n",
                "    source: commitments:role:goals",
            );
            format!("{file} is missing a Goal dimension.\nAdd this to the `dimensions:` list:\n{body}")
        })
}

fn role_dim_key(root: &std::path::Path, year: i32, month: u32) -> Result<String, String> {
    let from_monthly = files::read_dimensions_file(root, year, month)
        .map(|d| !d.is_empty())
        .unwrap_or(false);
    let file = if from_monthly {
        format!("{}/{}/dimensions.yaml", year, format!("{:02}", month))
    } else {
        "dimensions.template.yaml".to_string()
    };
    files::resolve_month_dimensions(root, year, month)?
        .into_iter()
        .find(|d| d.source == "commitments:role")
        .map(|d| d.key)
        .ok_or_else(|| {
            let body = concat!(
                "  - name: Role\n",
                "    key: role\n",
                "    source: commitments:role",
            );
            format!("{file} is missing a Role dimension.\nAdd this to the `dimensions:` list:\n{body}")
        })
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



#[tauri::command]
pub fn get_commitment_progress(
    root_path: String,
    year: i32,
    month: u32,
) -> Result<Vec<CommitmentProgress>, String> {
    use crate::models::{CommitmentProgress, GoalProgress};
    use std::collections::HashMap;

    let root = std::path::Path::new(&root_path);

    // 1. Read commitments.yaml
    let commitments = crate::files::read_commitments_file(root, year, month).map_err(|e| {
        error_log::log_error(
            "get_commitment_progress",
            &format!("Failed to read commitments.yaml for {}-{:02}: {:?}", year, month, e),
        );
        format!("Failed to read commitments.yaml for {}-{:02}: {}", year, month, e)
    })?;

    // 2. Build maps
    let (goal_to_role, role_to_goals) = build_commitment_maps(&commitments);

    // 3. Initialize result structures
    let mut role_goal_spent: HashMap<String, u32> = HashMap::new();
    let mut role_general_spent: HashMap<String, u32> = HashMap::new();
    let mut goal_spent: HashMap<String, u32> = HashMap::new();

    for c in &commitments {
        role_goal_spent.entry(c.role.clone()).or_insert(0);
        role_general_spent.entry(c.role.clone()).or_insert(0);
        for g in &c.goals {
            goal_spent.entry(g.clone()).or_insert(0);
        }
    }

    // 4. Scan day files
    let goal_key = match goal_dim_key(root, year, month) {
        Ok(k) => k,
        Err(e) => {
            error_log::log_error("get_commitment_progress", &format!("goal key missing: {e}"));
            return Err(format!("goal dimension key not configured: {e}"));
        }
    };
    let role_key = match role_dim_key(root, year, month) {
        Ok(k) => k,
        Err(e) => {
            error_log::log_error("get_commitment_progress", &format!("role key missing: {e}"));
            return Err(format!("role dimension key not configured: {e}"));
        }
    };
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
                if !file_name.ends_with(".yaml") {
                    continue;
                }
                match crate::files::read_day_file(root, file_name.trim_end_matches(".yaml")) {
                    Ok(day_file) => {
                        for e in &day_file.entries {
                            let role = e.dimensions.get(&role_key);
                            let goal = e.dimensions.get(&goal_key);

                            match (role, goal) {
                                (Some(r), Some(g)) => {
                                    if let Some(goals) = role_to_goals.get(r.as_str()) {
                                        if goals.contains(g) {
                                            *role_goal_spent.entry(r.clone()).or_insert(0) += e.duration;
                                            *goal_spent.entry(g.clone()).or_insert(0) += e.duration;
                                        } else {
                                            *role_general_spent.entry(r.clone()).or_insert(0) += e.duration;
                                        }
                                    }
                                }
                                (Some(r), None) => {
                                    *role_general_spent.entry(r.clone()).or_insert(0) += e.duration;
                                }
                                (None, Some(g)) => {
                                    if let Some(r) = goal_to_role.get(g.as_str()) {
                                        *role_goal_spent.entry(r.clone()).or_insert(0) += e.duration;
                                        *goal_spent.entry(g.clone()).or_insert(0) += e.duration;
                                    }
                                }
                                (None, None) => {}
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

    Ok(roles)
}

/// Repair sweep: clear role and goal dimension values in day files that don't
/// match any current commitments. Handles crash recovery (where commitments.yaml
/// was updated but day files still have stale names) and manual file edits.
///
/// Returns write errors if any day file updates failed.
pub fn repair_entry_dimensions(root: &std::path::Path, year: i32, month: u32) -> Vec<String> {
    let mut write_errors: Vec<String> = Vec::new();

    let commitments = match crate::files::read_commitments_file(root, year, month) {
        Ok(c) => c,
        Err(e) => {
            error_log::log_error("repair_entry_dimensions",
                &format!("read_commitments_file failed, skipping repair: {}", e));
            return write_errors;
        }
    };
    if commitments.is_empty() {
        return write_errors;
    }

    let role_key = match role_dim_key(root, year, month) {
        Ok(k) => k,
        Err(e) => {
            error_log::log_error("repair_entry_dimensions",
                &format!("role_dim_key failed, skipping repair: {}", e));
            return write_errors;
        }
    };
    let goal_key = goal_dim_key(root, year, month).unwrap_or_else(|e| {
        error_log::log_error("repair_entry_dimensions",
            &format!("goal_dim_key failed, skipping goal repair: {}", e));
        String::new()
    });

    let valid_roles: std::collections::BTreeSet<&String> = commitments.iter().map(|c| &c.role).collect();
    let valid_goals: std::collections::BTreeSet<&str> = commitments
        .iter()
        .flat_map(|c| c.goals.iter().map(|g| g.as_str()))
        .collect();

    let month_dir = root.join(year.to_string()).join(format!("{:02}", month));
    if let Ok(entries) = std::fs::read_dir(&month_dir) {
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    error_log::log_error("repair_entry_dimensions",
                        &format!("read_dir entry error in {}: {}", month_dir.display(), e));
                    continue;
                },
            };
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !file_name.ends_with(".yaml") {
                continue;
            }
            if let Ok(mut day_file) = crate::files::read_day_file(root, file_name.trim_end_matches(".yaml")) {
                let mut cleaned = 0u32;
                for e in &mut day_file.entries {
                    if let Some(role_val) = e.dimensions.get(&role_key) {
                        if !valid_roles.contains(role_val) {
                            e.dimensions.remove(&role_key);
                            cleaned += 1;
                        }
                    }
                    if !goal_key.is_empty() {
                        if let Some(goal_val) = e.dimensions.get(&goal_key) {
                            if !valid_goals.contains(goal_val.as_str()) {
                                e.dimensions.remove(&goal_key);
                                cleaned += 1;
                            }
                        }
                    }
                }
                if cleaned > 0 {
                    error_log::log_info("repair_entry_dimensions",
                        &format!("cleared {} unknown value(s) in {}", cleaned, file_name));
                    if let Err(e) = crate::files::write_day_file(root, file_name.trim_end_matches(".yaml"), &day_file) {
                        write_errors.push(format!("repair {}: {}", file_name, e));
                    }
                }
            }
        }
    } else {
        error_log::log_error("repair_entry_dimensions",
            &format!("failed to read month directory: {}", month_dir.display()));
    }

    write_errors
}

/// Run repair_entry_dimensions for every year/month directory found under the
/// data root. Best-effort — failures are logged but never block init.
pub fn repair_entry_dimensions_all(root: &std::path::Path) {
    let year_entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(e) => {
            error_log::log_error(
                "repair_entry_dimensions_all",
                &format!("Failed to read root dir: {}", e),
            );
            return;
        }
    };

    for year_entry in year_entries {
        let year_entry = match year_entry {
            Ok(e) => e,
            Err(e) => {
                error_log::log_error(
                    "repair_entry_dimensions_all",
                    &format!("Failed to read year entry: {:?}", e),
                );
                continue;
            }
        };
        let is_dir = match year_entry.file_type() {
            Ok(t) => t.is_dir(),
            Err(e) => {
                error_log::log_error(
                    "repair_entry_dimensions_all",
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
            Ok(y) if y >= 2000 && y <= 9999 => y,
            _ => continue,
        };

        let month_entries = match std::fs::read_dir(year_entry.path()) {
            Ok(e) => e,
            Err(e) => {
                error_log::log_error(
                    "repair_entry_dimensions_all",
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
                        "repair_entry_dimensions_all",
                        &format!("Failed to read month entry in year {}: {:?}", year, e),
                    );
                    continue;
                }
            };
            if !month_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let month_name = month_entry.file_name();
            let month_str = month_name.to_string_lossy();
            let month: u32 = match month_str.parse() {
                Ok(m) if (1..=12).contains(&m) => m,
                _ => continue,
            };

            for err in repair_entry_dimensions(root, year, month) {
                error_log::log_error(
                    "repair_entry_dimensions_all",
                    &format!("{}/{}: {}", year_str, month_str, err),
                );
            }
        }
    }
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
    integrity::check()?;
    let role_key = role_dim_key(root, year, month)?;

    // Pre-write integrity check: validate the existing commitments file is readable
    if let Err(e) = files::read_commitments_file(root, year, month) {
        integrity::set_compromised(IntegrityIssue {
            path: format!("{}/{}/commitments.yaml", year, format!("{:02}", month)),
            message: e,
            kind: "CommitmentsFileError".into(),
        });
        return Err(format!(
            "Write denied: target commitments file integrity check failed"
        ));
    }

    // 1. Validate
    crate::config::validate_commitments(&commitments)?;

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

    // 5. Check deleted goals for existing entries (single scan for all goals).
    let goal_counts = batch_count_entries_for_goals(root, year, month, &changes.deleted)?;
    for goal_name in &changes.deleted {
        let count = goal_counts.get(goal_name).copied().unwrap_or(0);
        if count > 0 {
            return Err(format!(
                "Cannot delete goal '{}': used by {} entries this month",
                goal_name, count
            ));
        }
    }

    // 5b. Clean up orphaned goal dimension values in day files.
    //     Even though the guard above ensures count == 0 for every deleted
    //     goal, dimension-key fallback edge-cases could leave ghost values.
    cleanup_deleted_goals_in_entries(root, year, month, &changes.deleted)?;

    // 6. Write commitments.yaml FIRST — before mutating day files.
    //    If a crash occurs during steps 7-7d, step 7d (repair sweep) will fix
    //    stale role values on the next run — it scans for any role not in the
    //    current commitments set and clears it.
    files::write_commitments_file(root, year, month, &commitments)?;

    // 7. Apply goal renames to day files (single scan for all renames).
    batch_rename_goals_in_entries(root, year, month, &changes.renames)?;

    // 7b. Detect and apply role renames.
    let month_dir = root.join(year.to_string()).join(format!("{:02}", month));
    let role_changes = detect_role_changes(&old_commitments, &commitments);
    let mut write_errors: Vec<String> = Vec::new();
    for (old_name, new_name) in &role_changes {
        if let Ok(entries) = std::fs::read_dir(&month_dir) {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        error_log::log_error("set_commitments:role_rename",
                            &format!("read_dir entry error in {}: {}", month_dir.display(), e));
                        continue;
                    },
                };
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !file_name.ends_with(".yaml") {
                    continue;
                }
                if let Ok(mut day_file) = crate::files::read_day_file(root, file_name.trim_end_matches(".yaml")) {
                    let mut changed = false;
                    for e in &mut day_file.entries {
                        if e.dimensions.get(&role_key).map(|r| r == old_name).unwrap_or(false) {
                            e.dimensions.insert(role_key.clone(), new_name.to_string());
                            changed = true;
                        }
                    }
                    if changed {
                        if let Err(e) = crate::files::write_day_file(root, file_name.trim_end_matches(".yaml"), &day_file) {
                            write_errors.push(format!("role rename {}: {}", file_name, e));
                        }
                    }
                }
            }
        } else {
            error_log::log_error("set_commitments:role_rename",
                &format!("failed to read month directory: {}", month_dir.display()));
        }
    }

    // 7c. Clear role dimension for deleted roles.
    let old_role_names: std::collections::BTreeSet<&String> = old_commitments.iter().map(|c| &c.role).collect();
    let new_role_names: std::collections::BTreeSet<&String> = commitments.iter().map(|c| &c.role).collect();
    let deleted_roles: Vec<&String> = old_role_names.difference(&new_role_names).cloned().collect();

    for role_name in &deleted_roles {
        if let Ok(entries) = std::fs::read_dir(&month_dir) {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        error_log::log_error("set_commitments:role_cleanup",
                            &format!("read_dir entry error in {}: {}", month_dir.display(), e));
                        continue;
                    },
                };
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !file_name.ends_with(".yaml") {
                    continue;
                }
                if let Ok(mut day_file) = crate::files::read_day_file(root, file_name.trim_end_matches(".yaml")) {
                    let mut changed = false;
                    for e in &mut day_file.entries {
                        if e.dimensions.get(&role_key).map(|r| r == *role_name).unwrap_or(false) {
                            e.dimensions.remove(&role_key);
                            changed = true;
                        }
                    }
                    if changed {
                        if let Err(e) = crate::files::write_day_file(root, file_name.trim_end_matches(".yaml"), &day_file) {
                            write_errors.push(format!("role cleanup {}: {}", file_name, e));
                        }
                    }
                }
            }
        } else {
            error_log::log_error("set_commitments:role_cleanup",
                &format!("failed to read month directory: {}", month_dir.display()));
        }
    }

    // 7d. Repair sweep: clear role and goal dimension values that don't match
    //     any current commitments role/goal. This handles crash recovery (where
    //     commitments.yaml was updated but day files still have old names)
    //     and defence against manual file edits.
    for err_msg in repair_entry_dimensions(root, year, month) {
        write_errors.push(err_msg);
    }

    if !write_errors.is_empty() {
        error_log::log_error(
            "set_commitments",
            &format!("{} day file write(s) failed: {:?}", write_errors.len(), write_errors),
        );
        return Err(format!(
            "{} day file(s) failed to update (commitments were saved). Details: {}",
            write_errors.len(),
            write_errors.join("; "),
        ));
    }

    let ok = true;
    error_log::log_command_exit("set_commitments", ok, "");
    Ok(commitments)
}

/// Batch version: scan all day files once and return per-goal entry counts.
/// Replaces N independent `count_entries_with_goal` scans with one pass.
fn batch_count_entries_for_goals(
    root: &std::path::Path,
    year: i32,
    month: u32,
    goals: &[String],
) -> Result<std::collections::HashMap<String, usize>, String> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    if goals.is_empty() {
        return Ok(counts);
    }
    let month_dir = root
        .join(year.to_string())
        .join(format!("{:02}", month));
    if !month_dir.exists() {
        return Ok(counts);
    }
    let goal_key = goal_dim_key(root, year, month)?;
    let entries = std::fs::read_dir(&month_dir)
        .map_err(|e| format!("Failed to read month dir: {}", e))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                error_log::log_error(
                    "batch_count",
                    &format!("Failed to read dir entry in {}-{:02}: {:?}", year, month, e),
                );
                continue;
            }
        };
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !file_name.ends_with(".yaml") {
            continue;
        }
        let date = file_name.trim_end_matches(".yaml");
        if let Ok(day_file) = files::read_day_file(root, date) {
            for e in &day_file.entries {
                if let Some(g) = e.dimensions.get(&goal_key) {
                    if goals.iter().any(|goal| goal == g) {
                        *counts.entry(g.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    Ok(counts)
}

/// Batch version: scan all day files once, apply all goal renames in memory,
/// then write back only changed files. Replaces N independent
/// `rename_goal_in_entries` calls with one pass.
fn batch_rename_goals_in_entries(
    root: &std::path::Path,
    year: i32,
    month: u32,
    renames: &[(String, String)],
) -> Result<(), String> {
    if renames.is_empty() {
        return Ok(());
    }
    let month_dir = root
        .join(year.to_string())
        .join(format!("{:02}", month));
    if !month_dir.exists() {
        return Ok(());
    }
    let entries = std::fs::read_dir(&month_dir)
        .map_err(|e| format!("Failed to read month dir: {}", e))?;
    let goal_key = goal_dim_key(root, year, month)?;

    // Phase 1: read + transform every affected day file in memory.
    let mut pending: Vec<(String, crate::models::DayFile)> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                error_log::log_error("batch_rename_goals",
                    &format!("read_dir entry error in {}: {}", month_dir.display(), e));
                continue;
            },
        };
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !file_name.ends_with(".yaml") {
            continue;
        }
        let date = file_name.trim_end_matches(".yaml");
        if validate_date_format(date).is_err() {
            continue;
        }
        let mut day_file = files::read_day_file(root, date)?;
        let mut changed = false;
        for e in &mut day_file.entries {
            if let Some(goal) = e.dimensions.get(&goal_key) {
                if let Some((_, new_name)) = renames.iter().find(|(old, _)| old == goal) {
                    e.dimensions.insert(goal_key.clone(), new_name.clone());
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

/// Remove orphaned goal dimension values from day files when goals are
/// deleted from commitments. This is a safety net: even though the deletion
/// guard (batch_count_entries_for_goals returns 0 for every deleted goal)
/// should prevent this, dimension-key fallback edge-cases could leave entries
/// with ghost goal values.
fn cleanup_deleted_goals_in_entries(
    root: &std::path::Path,
    year: i32,
    month: u32,
    deleted_goals: &[String],
) -> Result<(), String> {
    if deleted_goals.is_empty() {
        return Ok(());
    }
    let goal_key = goal_dim_key(root, year, month)?;
    let month_dir = root
        .join(year.to_string())
        .join(format!("{:02}", month));
    if !month_dir.exists() {
        return Ok(());
    }
    let entries = std::fs::read_dir(&month_dir)
        .map_err(|e| format!("Failed to read month dir: {}", e))?;

    // Phase 1: read + transform every affected day file in memory.
    let mut pending: Vec<(String, crate::models::DayFile)> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                error_log::log_error("cleanup_deleted_goals",
                    &format!("read_dir entry error in {}: {}", month_dir.display(), e));
                continue;
            },
        };
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !file_name.ends_with(".yaml") {
            continue;
        }
        let date = file_name.trim_end_matches(".yaml");
        let mut day_file = match files::read_day_file(root, date) {
            Ok(df) => df,
            Err(e) => {
                error_log::log_error(
                    "cleanup_deleted_goals",
                    &format!("Failed to read day file {}: {}", date, e),
                );
                continue;
            }
        };
        let mut changed = false;
        for e in &mut day_file.entries {
            if let Some(val) = e.dimensions.get(&goal_key) {
                if deleted_goals.iter().any(|g| g == val) {
                    e.dimensions.remove(&goal_key);
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

/// Detects goal renames and deletions between old and new commitments.
///
/// Renames are detected in two passes:
///
/// 1. **Exact**: same role, same goal count, exactly one old goal missing and
///    one new goal added.
/// 2. **Substring**: when a new goal name contains one or more unmatched old
///    goal names as substrings, treat all contained old goals as renames to
///    the new goal. This handles both simple appending and merging multiple
///    goals into one combined name, even when the goal count changes.
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

    let deleted_candidates: Vec<String> = old_goals.difference(&new_goals).cloned().collect();
    let added_candidates: Vec<String> = new_goals.difference(&old_goals).cloned().collect();

    let mut renames: Vec<(String, String)> = Vec::new();
    let mut matched_old_goals: HashSet<String> = HashSet::new();
    let mut matched_new_goals: HashSet<String> = HashSet::new();

    // Step 1: Exact rename detection — same role, same goal count,
    // exactly one old goal missing and one new goal added.
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
                    matched_new_goals.insert(new_not_old[0].clone());
                }
            }
        }
    }

    // Step 2: Substring-based rename detection for remaining unmatched goals.
    // When a new goal name contains an old goal name as a substring, treat it
    // as a rename — but only if the containment is unambiguous (exactly one
    // unmatched old goal is contained by the new goal). This handles the common
    // case of appending text to a goal title, even when the goal count changed
    // due to a simultaneous deletion.
    for new_goal in &added_candidates {
        if matched_new_goals.contains(new_goal) {
            continue;
        }
        let contained: Vec<&String> = deleted_candidates
            .iter()
            .filter(|o| !matched_old_goals.contains(o.as_str()))
            .filter(|o| new_goal.contains(o.as_str()) && o.as_str() != new_goal.as_str())
            .collect();
        if !contained.is_empty() {
            for old_goal in &contained {
                renames.push(((*old_goal).clone(), new_goal.clone()));
                matched_old_goals.insert((*old_goal).clone());
            }
            matched_new_goals.insert(new_goal.clone());
        }
    }

    let deleted: Vec<String> = deleted_candidates
        .into_iter()
        .filter(|g| !matched_old_goals.contains(g))
        .collect();

    GoalChanges { renames, deleted }
}

/// 检测 role 改名：新旧 commitments 之间，role 名变了但 goals 集合相同。
/// 返回 (old_name, new_name) 列表。
fn detect_role_changes(old: &[crate::models::Commitment], new: &[crate::models::Commitment]) -> Vec<(String, String)> {
    // Heuristic: an old role was renamed when (1) same goals, (2) different name,
    // (3) old name vanished from new, (4) new name was absent from old.
    // To avoid false renames when multiple old roles could map to the same new
    // role (merging, or ambiguous empty-goal matching), we require a 1:1
    // correspondence — each new role may be matched by at most one old role.
    let candidate = |o: &crate::models::Commitment| -> Option<&crate::models::Commitment> {
        let old_goals: std::collections::BTreeSet<&String> = o.goals.iter().collect();
        new.iter().find(|n| {
            let new_goals: std::collections::BTreeSet<&String> = n.goals.iter().collect();
            old_goals == new_goals
                && o.role != n.role
                && !new.iter().any(|c| c.role == o.role)
                && !old.iter().any(|c| c.role == n.role)
        })
    };

    // First pass: count how many old roles could map to each new role
    let mut new_role_candidates: std::collections::HashMap<&String, usize> =
        std::collections::HashMap::new();
    for o in old {
        if let Some(n) = candidate(o) {
            *new_role_candidates.entry(&n.role).or_insert(0) += 1;
        }
    }

    // Second pass: only accept unambiguous 1:1 renames
    let mut changes = Vec::new();
    for o in old {
        if let Some(n) = candidate(o) {
            if new_role_candidates.get(&n.role).copied().unwrap_or(0) == 1 {
                changes.push((o.role.clone(), n.role.clone()));
            }
        }
    }
    changes
}

#[tauri::command]
pub fn get_available_months(root_path: String) -> Result<Vec<AvailableMonth>, String> {
    error_log::log_command_enter("get_available_months", &root_path);
    use crate::models::AvailableMonth;
    let root = std::path::Path::new(&root_path);
    if !root.exists() {
        error_log::log_command_exit("get_available_months", true, "root not found, returning empty");
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
            Ok(y) if y >= 2000 && y <= 9999 => y,
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

            // Check if this month directory contains at least one .yaml file
            let has_yaml = match std::fs::read_dir(month_entry.path()) {
                Ok(entries) => {
                    let mut found = false;
                    for e in entries {
                        match e {
                            Ok(entry) => {
                                let name_str = entry.file_name().to_string_lossy().into_owned();
                                if name_str.ends_with(".yaml") {
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

            if has_yaml {
                months.push(AvailableMonth { year, month });
            }
        }
    }

    // Sort descending (newest first)
    months.sort_by(|a, b| b.year.cmp(&a.year).then(b.month.cmp(&a.month)));

    error_log::log_command_exit("get_available_months", true, &format!("{} months", months.len()));
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
/// - day file `root/YYYY/MM/YYYY-MM-DD.yaml` exists → select that file
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
pub fn reveal_file(app: AppHandle, root_path: String, relative_path: String) -> Result<(), String> {
    error_log::log_command_enter("reveal_file", &format!("root={} rel={}", root_path, relative_path));
    let root = std::path::Path::new(&root_path);
    let target = root.join(&relative_path);
    let canonical_root = root.canonicalize().map_err(|e| {
        format!("Cannot resolve root path {}: {}", root_path, e)
    })?;
    let canonical_target = target.canonicalize().unwrap_or_else(|_| target.clone());
    if !canonical_target.starts_with(&canonical_root) {
        let err = format!("Path traversal attempt: {}", relative_path);
        error_log::log_error("reveal_file", &err);
        error_log::log_command_exit("reveal_file", false, "path traversal");
        return Err(err);
    }
    let (target, select) = if target.exists() {
        (target, true)
    } else {
        (root.to_path_buf(), false)
    };
    let result = if select {
        app.opener()
            .reveal_item_in_dir(&target)
            .map_err(|e| format!("Failed to reveal {}: {}", target.display(), e))
    } else {
        app.opener()
            .open_path(target.to_string_lossy().into_owned(), None::<String>)
            .map_err(|e| format!("Failed to open {}: {}", target.display(), e))
    };
    error_log::log_command_exit("reveal_file", result.is_ok(), "");
    result
}

#[tauri::command]
pub fn create_starter_files(path: String) -> Result<(), String> {
    error_log::log_command_enter("create_starter_files", &path);
    let root = std::path::Path::new(&path);
    if !root.exists() {
        std::fs::create_dir_all(root).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let template_path = root.join("dimensions.template.yaml");
    if !template_path.exists() {
        // Atomic write (tmp + rename) per project write-safety convention.
        if let Err(e) = files::atomic_write(
            &template_path,
            concat!(
                "dimensions:\n",
                "  - name: Goal\n    key: goal\n    source: commitments:role:goals\n",
                "  - name: Role\n    key: role\n    source: commitments:role\n",
            ),
        ) {
            let msg = format!("Failed to write dimensions.template.yaml: {}", e);
            error_log::log_error("create_starter_files", &msg);
            error_log::log_command_exit("create_starter_files", false, &msg);
            return Err(msg);
        }
    }
    // Stamp version.txt so the init that follows "Start fresh" passes
    // check_data_version instead of dead-ending on DataVersionNotFound.
    // Never overwrite an existing version file (could be an older data tree).
    if !files::version_path(root).exists() {
        if let Err(e) = files::write_version_file(root, CURRENT_DATA_VERSION) {
            let msg = format!("Failed to write version.txt: {}", e);
            error_log::log_error("create_starter_files", &msg);
            error_log::log_command_exit("create_starter_files", false, &msg);
            return Err(msg);
        }
    }
    error_log::log_command_exit("create_starter_files", true, "");
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
    files::atomic_write(&path, &yaml_body)?;

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

#[tauri::command]
pub fn recheck_integrity(root_path: String) -> crate::models::IntegrityStatus {
    let root = std::path::Path::new(&root_path);
    integrity::reset();
    let issues = integrity::check_scoped_integrity(root);
    if !issues.is_empty() {
        for issue in &issues {
            integrity::set_compromised(issue.clone());
        }
    }
    integrity::status()
}

#[tauri::command]
pub fn check_watcher_health(app: tauri::AppHandle) -> Result<bool, String> {
    error_log::log_command_enter("check_watcher_health", "");
    let state = app.state::<crate::config::WatcherState>();
    let alive = state.is_watcher_alive();
    error_log::log_command_exit("check_watcher_health", true, &format!("alive={}", alive));
    Ok(alive)
}

#[tauri::command]
pub fn restart_watcher(app: tauri::AppHandle, root_path: String) -> Result<(), String> {
    error_log::log_command_enter("restart_watcher", &root_path);
    let path = std::path::PathBuf::from(&root_path);
    crate::config::respawn_watcher(&app, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_accepts_plain_number() {
        assert_eq!(parse_duration("90").unwrap(), 90);
    }

    #[test]
    fn test_parse_duration_accepts_float_without_unit() {
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
                    source: "commitments:role:goals".into(),
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
            tmp.join("dimensions.template.yaml"),
            "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
        )
        .unwrap();
        fs::write(
            monthly_dir.join("commitments.yaml"),
            "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it\n",
        )
        .unwrap();

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "Dev");
        assert_eq!(result[0].allocation_minutes, 2400); // 40 * 60
        assert_eq!(result[0].goal_spent_minutes, 0);
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

        // Create commitments.yaml
        let monthly_dir = tmp.join("2026").join("06");
        fs::create_dir_all(&monthly_dir).unwrap();
        fs::write(
            tmp.join("dimensions.template.yaml"),
            "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
        )
        .unwrap();
        fs::write(
            monthly_dir.join("commitments.yaml"),
            "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it\n    - Review\n- role: PM\n  allocation: 10\n  goals:\n    - Planning\n",
        )
        .unwrap();

        // Create day file with entries matching goals
        fs::write(
            monthly_dir.join("2026-06-01.yaml"),
            "entries:\n  - id: e1\n    item: Code\n    duration: 60\n    dimensions:\n      goal: Ship it\n  - id: e2\n    item: PR\n    duration: 30\n    dimensions:\n      goal: Review\n",
        )
        .unwrap();

        fs::write(
            monthly_dir.join("2026-06-02.yaml"),
            "entries:\n  - id: e3\n    item: Plan\n    duration: 45\n    dimensions:\n      goal: Planning\n",
        )
        .unwrap();

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        // Dev: Ship it(60) + Review(30) = 90 spent
        let dev = result.iter().find(|c| c.role == "Dev").unwrap();
        assert_eq!(dev.goal_spent_minutes, 90);
        assert_eq!(dev.allocation_minutes, 2400);

        // PM: Planning(45) = 45 spent
        let pm = result.iter().find(|c| c.role == "PM").unwrap();
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
            tmp.join("dimensions.template.yaml"),
            "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
        )
        .unwrap();
        fs::write(
            monthly_dir.join("commitments.yaml"),
            "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it\n",
        )
        .unwrap();

        // Entry with a goal NOT in any commitment
        fs::write(
            monthly_dir.join("2026-06-01.yaml"),
            "entries:\n  - id: e1\n    item: Unknown task\n    duration: 60\n    dimensions:\n      goal: Not a goal\n",
        )
        .unwrap();

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        assert_eq!(result[0].goal_spent_minutes, 0);
        assert_eq!(result[0].goals[0].spent_minutes, 0);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_commitment_progress_no_monthly_file() {
        let tmp = std::env::temp_dir().join("logbook_test_cp_nofile");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("dimensions.template.yaml"),
            "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n  - name: Role\n    key: role\n    source: commitments:role\n",
        )
        .unwrap();

        let result = get_commitment_progress(tmp.to_string_lossy().into_owned(), 2026, 6).unwrap();

        assert!(result.is_empty());

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
    source: commitments:role:goals
  - name: Role
    key: role
    source: commitments:role
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
        std::fs::write(month_dir.join("dimensions.yaml"), "[]\n").unwrap();

        // Day 1: entry with role=Dev, goal=Ship X -> Ok, goal segment
        let day1 = "entries:\n  - id: e1\n    item: Code feature\n    duration: 120\n    dimensions:\n      role: Dev\n      goal: Ship X\n  - id: e2\n    item: Standup\n    duration: 30\n    dimensions:\n      role: Dev\n  - id: e3\n    item: Email\n    duration: 15\n    dimensions: {}\n";
        std::fs::write(month_dir.join("2026-07-01.yaml"), day1).unwrap();

        // Day 2: entry via goal fallback (no role dim) + mismatch case
        let day2 = "entries:\n  - id: e4\n    item: Roadmap planning\n    duration: 60\n    dimensions:\n      goal: Roadmap\n  - id: e5\n    item: Mismatch case\n    duration: 45\n    dimensions:\n      role: Dev\n      goal: Roadmap\n";
        std::fs::write(month_dir.join("2026-07-02.yaml"), day2).unwrap();

        let result = get_commitment_progress(
            tmp.to_string_lossy().to_string(),
            2026,
            7,
        ).unwrap();

        // Dev role
        let dev = result.iter().find(|r| r.role == "Dev").unwrap();
        // e1 (120m goal=Ship X) + e2 (30m general) + e5 (45m goal=Roadmap but Dev role -> mismatch -> general)
        assert_eq!(dev.goal_spent_minutes, 120);  // only e1
        assert_eq!(dev.general_spent_minutes, 75);  // e2 + e5
        assert_eq!(dev.allocation_minutes, 1200);

        // PM role
        let pm = result.iter().find(|r| r.role == "PM").unwrap();
        // e4 (60m goal=Roadmap, fallback to PM)
        assert_eq!(pm.goal_spent_minutes, 60);
        assert_eq!(pm.general_spent_minutes, 0);

        // e3 (no role, no goal) and e5 mismatches are ignored (not counted)

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
        let result = crate::config::validate_commitments(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("At least one role"));
    }

    #[test]
    fn test_validate_commitments_empty_role() {
        let c = make_commitments(vec![("", 40, vec!["Goal A"])]);
        let result = crate::config::validate_commitments(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Role name cannot be empty"));
    }

    #[test]
    fn test_validate_commitments_whitespace_role() {
        let c = make_commitments(vec![("   ", 40, vec!["Goal A"])]);
        let result = crate::config::validate_commitments(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Role name cannot be empty"));
    }

    #[test]
    fn test_validate_commitments_zero_allocation() {
        let c = make_commitments(vec![("Dev", 0, vec!["Goal A"])]);
        let result = crate::config::validate_commitments(&c);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Allocation for 'Dev'"));
        assert!(err.contains("must be greater than 0"));
    }

    #[test]
    fn test_validate_commitments_empty_goal() {
        let c = make_commitments(vec![("Dev", 40, vec![""])]);
        let result = crate::config::validate_commitments(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Goal name cannot be empty"));
    }

    #[test]
    fn test_validate_commitments_duplicate_goal_same_role() {
        let c = make_commitments(vec![("Dev", 40, vec!["Ship it", "Ship it"])]);
        let result = crate::config::validate_commitments(&c);
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
        let result = crate::config::validate_commitments(&c);
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
        let result = crate::config::validate_commitments(&c);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Role"));
        assert!(err.contains("already exists"));
        assert!(err.contains("Dev"));
    }

    #[test]
    fn test_validate_commitments_duplicate_goal_ignores_whitespace() {
        let c = make_commitments(vec![("Dev", 40, vec!["Ship it", " Ship it "])]);
        let result = crate::config::validate_commitments(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_validate_commitments_duplicate_role_ignores_whitespace() {
        let c = make_commitments(vec![
            ("Dev", 40, vec!["A"]),
            (" Dev ", 20, vec!["B"]),
        ]);
        let result = crate::config::validate_commitments(&c);
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
        assert!(crate::config::validate_commitments(&c).is_ok());
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
    fn test_detect_goal_rename_when_count_differs_substring_containment() {
        // Rename + delete in same role: goal count changes, but
        // the renamed goal's new name contains the old name as a substring.
        let old = make_commitments(vec![("Dev", 40, vec!["Goal A", "浸泡用户社区"])]);
        let new = make_commitments(vec![("Dev", 40, vec!["浸泡用户社区 + appended text"])]);
        let changes = detect_goal_changes(&old, &new);
        assert_eq!(
            changes.renames.len(),
            1,
            "substring containment must detect rename even when goal count changes"
        );
        assert_eq!(
            changes.renames[0],
            ("浸泡用户社区".to_string(), "浸泡用户社区 + appended text".to_string())
        );
        assert_eq!(changes.deleted, vec!["Goal A"], "Goal A was deleted, not renamed");
    }

    #[test]
    fn test_detect_goal_rename_substring_containment_multiple_old_to_one_new() {
        // Multiple old goals are substrings of the new goal → rename ALL to the new goal.
        // This handles merging two goals into one combined name.
        let old = make_commitments(vec![("Dev", 40, vec!["吃", "很好吃"])]);
        let new = make_commitments(vec![("Dev", 40, vec!["很好吃吃"])]);
        let changes = detect_goal_changes(&old, &new);
        assert_eq!(changes.renames.len(), 2, "both old goals should be renamed");
        let renamed_old: Vec<&String> = changes.renames.iter().map(|(o, _)| o).collect();
        let renamed_new: Vec<&String> = changes.renames.iter().map(|(_, n)| n).collect();
        assert!(renamed_old.contains(&&"吃".to_string()));
        assert!(renamed_old.contains(&&"很好吃".to_string()));
        for n in &renamed_new {
            assert_eq!(*n, "很好吃吃", "all renames should point to the new goal");
        }
        assert!(changes.deleted.is_empty(), "no goals should be deleted");
    }

    #[test]
    fn test_detect_goal_rename_merge_two_goals() {
        // Real-world merge scenario: user deletes one goal and appends its
        // text to another goal's title. Both old names are substrings of
        // the new combined name.
        let old = make_commitments(vec![("Dev", 40, vec!["风险监控", "浸泡用户社区"])]);
        let new = make_commitments(vec![("Dev", 40, vec!["浸泡用户社区 风险监控"])]);
        let changes = detect_goal_changes(&old, &new);
        assert_eq!(changes.renames.len(), 2, "both goals should be detected as renamed (merged)");
        let renamed_old: Vec<&String> = changes.renames.iter().map(|(o, _)| o).collect();
        assert!(renamed_old.contains(&&"风险监控".to_string()));
        assert!(renamed_old.contains(&&"浸泡用户社区".to_string()));
        for (_, n) in &changes.renames {
            assert_eq!(*n, "浸泡用户社区 风险监控");
        }
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

    // --- detect_role_changes tests ---

    #[test]
    fn test_detect_role_changes_empty_goals_no_false_rename() {
        let old = vec![
            crate::models::Commitment { role: "Eng".into(), allocation: 20, goals: vec![] },
            crate::models::Commitment { role: "Design".into(), allocation: 20, goals: vec![] },
        ];
        let new = vec![
            crate::models::Commitment { role: "Engineering".into(), allocation: 40, goals: vec![] },
        ];
        let changes = detect_role_changes(&old, &new);
        assert!(changes.is_empty(), "empty-goal roles should not produce false renames, got {:?}", changes);
    }

    #[test]
    fn test_detect_role_changes_goal_swap_not_rename() {
        let old = vec![
            crate::models::Commitment { role: "Frontend".into(), allocation: 20, goals: vec!["UI".into()] },
            crate::models::Commitment { role: "Backend".into(), allocation: 20, goals: vec!["API".into()] },
        ];
        let new = vec![
            crate::models::Commitment { role: "Frontend".into(), allocation: 20, goals: vec!["API".into()] },
            crate::models::Commitment { role: "Backend".into(), allocation: 20, goals: vec!["UI".into()] },
        ];
        let changes = detect_role_changes(&old, &new);
        assert!(changes.is_empty(), "goal swap should not produce role renames, got {:?}", changes);
    }

    #[test]
    fn test_detect_role_changes_true_rename() {
        let old = vec![
            crate::models::Commitment { role: "OldName".into(), allocation: 30, goals: vec!["Task1".into(), "Task2".into()] },
        ];
        let new = vec![
            crate::models::Commitment { role: "NewName".into(), allocation: 30, goals: vec!["Task1".into(), "Task2".into()] },
        ];
        let changes = detect_role_changes(&old, &new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0], ("OldName".to_string(), "NewName".to_string()));
    }

    // --- check_data_version tests ---

    #[test]
    fn test_check_data_version_ok_when_absent() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_check_v_ok");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("version.txt"), "1").unwrap();
        let result = check_data_version(&tmp, 1);
        assert!(result.is_ok(), "expected ok, got {:?}", result);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_check_data_version_not_found() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_check_v_nf");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let result = check_data_version(&tmp, 1);
        assert!(result.is_err());
        match result.unwrap_err() {
            InitResult::DataVersionNotFound { root_path } => {
                assert_eq!(root_path, tmp.to_string_lossy());
            }
            other => panic!("expected DataVersionNotFound, got {:?}", other),
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_check_data_version_mismatch() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_check_v_mm");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("version.txt"), "5").unwrap();
        let result = check_data_version(&tmp, 1);
        assert!(result.is_err());
        match result.unwrap_err() {
            InitResult::DataVersionMismatch { root_path, expected, found } => {
                assert_eq!(root_path, tmp.to_string_lossy());
                assert_eq!(expected, 1);
                assert_eq!(found, 5);
            }
            other => panic!("expected DataVersionMismatch, got {:?}", other),
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_check_data_version_invalid_content() {
        use std::fs;
        let tmp = std::env::temp_dir().join("logbook_check_v_inv");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("version.txt"), "not-a-number").unwrap();
        let result = check_data_version(&tmp, 1);
        // Invalid content is treated like version not found
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            InitResult::DataVersionNotFound { .. }
        ));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_validate_cross_dimension_ok_matching() {
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Eng".to_string());
        dims.insert("goal".to_string(), "ShipIt".to_string());
        let mut role_to_goals = std::collections::HashMap::new();
        role_to_goals.insert("Eng".to_string(), vec!["ShipIt".to_string()]);
        assert!(validate_cross_dimension_constraints(
            &dims, "role", "goal", &role_to_goals
        ).is_ok());
    }

    #[test]
    fn test_validate_cross_dimension_reject_mismatch() {
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Eng".to_string());
        dims.insert("goal".to_string(), "OtherGoal".to_string());
        let mut role_to_goals = std::collections::HashMap::new();
        role_to_goals.insert("Eng".to_string(), vec!["ShipIt".to_string()]);
        let result = validate_cross_dimension_constraints(
            &dims, "role", "goal", &role_to_goals
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not declared under role"));
    }

    #[test]
    fn test_validate_cross_dimension_ok_no_goal() {
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Eng".to_string());
        // no goal key present
        let role_to_goals = std::collections::HashMap::new();
        assert!(validate_cross_dimension_constraints(
            &dims, "role", "goal", &role_to_goals
        ).is_ok());
    }

    #[test]
    fn test_validate_cross_dimension_ok_empty_commitments() {
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "UnknownRole".to_string());
        dims.insert("goal".to_string(), "SomeGoal".to_string());
        let role_to_goals = std::collections::HashMap::new(); // empty
        assert!(validate_cross_dimension_constraints(
            &dims, "role", "goal", &role_to_goals
        ).is_ok());
    }

    #[test]
    fn test_validate_cross_dimension_reject_unknown_role() {
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "UnknownRole".to_string());
        let mut role_to_goals = std::collections::HashMap::new();
        role_to_goals.insert("Eng".to_string(), vec!["ShipIt".to_string()]);
        let result = validate_cross_dimension_constraints(
            &dims, "role", "goal", &role_to_goals
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not declared in commitments"));
    }

    #[test]
    fn test_validate_cross_dimension_ok_role_in_commitments_no_goal() {
        let mut dims = BTreeMap::new();
        dims.insert("role".to_string(), "Eng".to_string());
        let mut role_to_goals = std::collections::HashMap::new();
        role_to_goals.insert("Eng".to_string(), vec!["ShipIt".to_string()]);
        assert!(validate_cross_dimension_constraints(
            &dims, "role", "goal", &role_to_goals
        ).is_ok());
    }

    // --- validate_entry_input tests ---

    fn make_dim_config() -> Vec<Dimension> {
        vec![
            Dimension {
                name: "Biz".into(),
                key: "biz".into(),
                source: "static".into(),
                values: Some(vec!["A".into()]),
                required: false,
                deleted: false,
            },
            Dimension {
                name: "Goal".into(),
                key: "goal".into(),
                source: "commitments:role:goals".into(),
                values: None,
                required: false,
                deleted: false,
            },
            Dimension {
                name: "Role".into(),
                key: "role".into(),
                source: "commitments:role".into(),
                values: None,
                required: false,
                deleted: false,
            },
            // Deleted dimension — key still valid for unknown key check
            Dimension {
                name: "Old".into(),
                key: "old".into(),
                source: "static".into(),
                values: Some(vec!["X".into()]),
                required: false,
                deleted: true,
            },
        ]
    }

    #[test]
    fn test_validate_entry_input_rejects_empty_item() {
        let config = make_dim_config();
        let dims = BTreeMap::new();
        let role_to_goals = std::collections::HashMap::new();
        let err = validate_entry_input("", "1h", &dims, &config, "role", "goal", &role_to_goals).unwrap_err();
        assert!(err.contains("Entry item cannot be empty"));
    }

    #[test]
    fn test_validate_entry_input_rejects_whitespace_item() {
        let config = make_dim_config();
        let dims = BTreeMap::new();
        let role_to_goals = std::collections::HashMap::new();
        let err = validate_entry_input("   ", "1h", &dims, &config, "role", "goal", &role_to_goals).unwrap_err();
        assert!(err.contains("Entry item cannot be empty"));
    }

    #[test]
    fn test_validate_entry_input_rejects_unknown_dim_key() {
        let config = make_dim_config();
        let mut dims = BTreeMap::new();
        dims.insert("nonexistent".to_string(), "x".to_string());
        let role_to_goals = std::collections::HashMap::new();
        let err = validate_entry_input("Item", "1h", &dims, &config, "role", "goal", &role_to_goals).unwrap_err();
        assert!(err.contains("Unknown dimension key"));
        assert!(err.contains("nonexistent"));
    }

    #[test]
    fn test_validate_entry_input_allows_deleted_dim_key() {
        let config = make_dim_config();
        let mut dims = BTreeMap::new();
        dims.insert("old".to_string(), "legacy_value".to_string());
        let role_to_goals = std::collections::HashMap::new();
        // Should NOT reject—deleted dimension key is still a known key
        assert!(validate_entry_input("Item", "1h", &dims, &config, "role", "goal", &role_to_goals).is_ok());
    }

    #[test]
    fn test_validate_entry_input_ok() {
        let config = make_dim_config();
        let mut dims = BTreeMap::new();
        dims.insert("biz".to_string(), "A".to_string());
        let role_to_goals = std::collections::HashMap::new();
        let result = validate_entry_input("Test item", "1h 30m", &dims, &config, "role", "goal", &role_to_goals).unwrap();
        assert_eq!(result, 90);
    }

    #[test]
    fn test_validate_entry_input_duration_fail() {
        let config = make_dim_config();
        let dims = BTreeMap::new();
        let role_to_goals = std::collections::HashMap::new();
        let err = validate_entry_input("Item", "no duration", &dims, &config, "role", "goal", &role_to_goals).unwrap_err();
        assert!(err.contains("Could not parse duration"));
    }
}
