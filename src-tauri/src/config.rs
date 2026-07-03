use crate::files;
use crate::models::{ConfigErrorDetail, Dimension, MonthlyFile};
use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Debounce decision for the file watcher, with bounded memory.
///
/// Returns true if the event for `path` should be processed (no recent prior
/// event within `window`), false to skip. On a process decision it records
/// `now` for `path` and prunes every entry older than `window` — those can
/// never cause a future skip, so dropping them keeps `last_event` bounded to
/// paths touched within the window instead of growing unbounded forever.
fn debounce_and_record(
    last_event: &mut HashMap<std::path::PathBuf, Instant>,
    path: &std::path::Path,
    now: Instant,
    window: Duration,
) -> bool {
    if let Some(last) = last_event.get(path) {
        if now.duration_since(*last) < window {
            return false;
        }
    }
    last_event.retain(|_, &mut t| now.duration_since(t) < window);
    last_event.insert(path.to_path_buf(), now);
    true
}

fn is_valid_key(key: &str) -> bool {
    key.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Extract (year, month) from a path laid out as
/// `{root}/{year}/{month:02}/<filename>`. Returns None if the parent
/// directories aren't numeric year/month — the watcher must reflect the month
/// of the file that actually changed, not the current wall-clock month.
fn month_from_monthly_path(path: &std::path::Path) -> Option<(i32, u32)> {
    let mut comps = path.components().rev();
    comps.next()?; // _monthly.md
    let month: u32 = comps.next()?.as_os_str().to_str()?.parse().ok()?;
    let year: i32 = comps.next()?.as_os_str().to_str()?.parse().ok()?;
    if (1..=12).contains(&month) {
        Some((year, month))
    } else {
        None
    }
}

pub fn validate_dimensions(dimensions: &[Dimension]) -> Vec<ConfigErrorDetail> {
    let mut errors = Vec::new();
    let mut goal_source_count = 0;
    let mut role_source_count = 0;

    for (i, dim) in dimensions.iter().enumerate() {
        if dim.name.is_empty() {
            errors.push(ConfigErrorDetail {
                kind: "MissingName".to_string(),
                message: format!("Dimension at index {}: name is required", i),
            });
        }
        if dim.key.is_empty() {
            errors.push(ConfigErrorDetail {
                kind: "MissingKey".to_string(),
                message: format!("Dimension at index {}: key is required", i),
            });
        } else if !is_valid_key(&dim.key) {
            errors.push(ConfigErrorDetail {
                kind: "KeyInvalidChars".to_string(),
                message: format!(
                    "Dimension '{}': key '{}' contains invalid characters (use a-z, 0-9, -, _)",
                    dim.name, dim.key
                ),
            });
        }
        match dim.source.as_str() {
            "static" => match &dim.values {
                None => errors.push(ConfigErrorDetail {
                    kind: "MissingValues".to_string(),
                    message: format!(
                        "Dimension '{}' (key: {}): source is 'static' but values is not set",
                        dim.name, dim.key
                    ),
                }),
                Some(vals) if vals.is_empty() => errors.push(ConfigErrorDetail {
                    kind: "ValuesEmpty".to_string(),
                    message: format!(
                        "Dimension '{}' (key: {}): values list is empty",
                        dim.name, dim.key
                    ),
                }),
                _ => {}
            },
            "commitments:goals" => {
                goal_source_count += 1;
                if goal_source_count > 1 {
                    errors.push(ConfigErrorDetail {
                        kind: "MultipleGoalSource".to_string(),
                        message: format!(
                            "Dimension '{}': only one dimension may have source: commitments:goals",
                            dim.name
                        ),
                    });
                }
            }
            "commitments:role" => {
                role_source_count += 1;
                if role_source_count > 1 {
                    errors.push(ConfigErrorDetail {
                        kind: "MultipleRoleSource".to_string(),
                        message: format!(
                            "Dimension '{}': only one dimension may have source: commitments:role",
                            dim.name
                        ),
                    });
                }
            }
            other => {
                errors.push(ConfigErrorDetail {
                    kind: "InvalidSource".to_string(),
                    message: format!(
                        "Dimension '{}': invalid source '{}' (expected 'static', 'commitments:goals', or 'commitments:role')",
                        dim.name, other
                    ),
                });
            }
        }
    }
    errors
}

pub fn validate_monthly(monthly: &MonthlyFile) -> Vec<ConfigErrorDetail> {
    let mut errors = validate_dimensions(&monthly.dimensions);
    let mut seen_goals = std::collections::HashSet::new();

    for (i, c) in monthly.commitments.iter().enumerate() {
        if c.role.is_empty() {
            errors.push(ConfigErrorDetail {
                kind: "MissingRole".to_string(),
                message: format!("Commitment at index {}: role is required", i),
            });
        }
        if c.allocation == 0 {
            errors.push(ConfigErrorDetail {
                kind: "ZeroAllocation".to_string(),
                message: format!(
                    "Commitment '{}': allocation is 0 (should be hours per month)",
                    c.role
                ),
            });
        }
        for goal in &c.goals {
            if !seen_goals.insert(goal.clone()) {
                errors.push(ConfigErrorDetail {
                    kind: "DuplicateGoal".to_string(),
                    message: format!(
                        "Goal '{}' appears in multiple commitments (each goal must be unique)",
                        goal
                    ),
                });
            }
        }
    }
    errors
}

/// Managed state holding the live file watcher. Dropping the inner watcher stops
/// its event stream (the receiver thread exits when the channel closes).
pub struct WatcherState {
    inner: Mutex<Option<WatcherHandle>>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self { inner: Mutex::new(None) }
    }

    /// Returns true if the watcher is currently running and its receiver
    /// thread has not exited (checked via the alive flag).
    pub fn is_watcher_alive(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|h| h.alive.load(Ordering::Acquire))
            .unwrap_or(false)
    }
}

impl Default for WatcherState {
    fn default() -> Self {
        Self::new()
    }
}

struct WatcherHandle {
    path: PathBuf,
    _watcher: RecommendedWatcher, // kept alive; drop = stop watching
    alive: Arc<AtomicBool>,       // set to false when the receiver thread exits
}

/// Pure decision: do we need to (re)start the watcher for `requested`?
pub fn needs_restart(current: Option<&std::path::Path>, requested: &std::path::Path) -> bool {
    current != Some(requested)
}

/// Start (or restart) the recursive file watcher for `root_path`.
/// Idempotent for the same path; replaces the watcher when the path changes.
pub fn ensure_watcher(app: &AppHandle, root_path: PathBuf) {
    let state = app.state::<WatcherState>();
    let mut guard = state.inner.lock().expect("WatcherState lock poisoned");
    if !needs_restart(guard.as_ref().map(|h| h.path.as_path()), &root_path) {
        return;
    }
    match spawn_watcher(app.clone(), root_path.clone()) {
        Ok((watcher, alive)) => {
            // Assigning Some replaces (and drops) any previous handle → old watcher stops.
            *guard = Some(WatcherHandle { path: root_path, _watcher: watcher, alive });
        }
        Err(e) => {
            crate::error_log::log_error("ensure_watcher", &e);
            *guard = None;
        }
    }
}

/// Build the watcher and spawn its receiver thread. Returns the watcher to be
/// held in WatcherState; the receiver thread exits when the watcher is dropped.
fn spawn_watcher(
    app_handle: AppHandle,
    root_path: PathBuf,
) -> Result<(RecommendedWatcher, Arc<AtomicBool>), String> {
    crate::error_log::log_info("file_watcher", &format!("Watching {}", root_path.display()));
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
        Ok(event) => {
            if let Err(e) = tx.send(event) {
                crate::error_log::log_error("file_watcher", &format!("send error: {:?}", e));
            }
        }
        Err(e) => {
            crate::error_log::log_error("file_watcher", &format!("notify error: {}", e));
        }
    })
    .map_err(|e| format!("Failed to create file watcher: {}", e))?;

    watcher
        .configure(NotifyConfig::default())
        .map_err(|e| format!("Failed to configure watcher: {}", e))?;

    watcher
        .watch(&root_path, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch {}: {}", root_path.display(), e))?;

    let alive = Arc::new(AtomicBool::new(true));
    let alive_clone = Arc::clone(&alive);

    let watch_root = root_path.clone();
    std::thread::spawn(move || {
        // Guard: unconditionally mark the watcher as dead when this thread
        // exits — whether normally or via panic unwind.
        struct AliveGuard(Arc<AtomicBool>);
        impl Drop for AliveGuard {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _guard = AliveGuard(Arc::clone(&alive_clone));

        let debounce_ms = Duration::from_millis(300);
        let mut last_event: HashMap<std::path::PathBuf, Instant> = HashMap::new();

        for event in rx {
            let is_modify = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_));
            if !is_modify {
                continue;
            }

            for path in &event.paths {
                let now = Instant::now();
                if !debounce_and_record(&mut last_event, path, now, debounce_ms) {
                    continue;
                }

                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "dimensions.template.yaml" {
                    match files::read_dimensions_template(&watch_root) {
                        Ok(config) => {
                            let _ = validate_dimensions(&config.dimensions);
                            // Template changes do not affect the current view — no emit.
                            // Validation errors are surfaced on the next init/dimensions read.
                        }
                        Err(_) => {
                            // Parse error; next read will surface it.
                        }
                    }
                } else if file_name == "dimensions.yaml" {
                    // Reflect the month of the file that actually changed, not the
                    // current wall-clock month — editing a past month's dimensions
                    // must not broadcast the current month's data.
                    let (year, month) = match month_from_monthly_path(path) {
                        Some(ym) => ym,
                        None => {
                            crate::error_log::log_error(
                                "file_watcher",
                                &format!("could not parse month from dimensions.yaml path: {}", path.display()),
                            );
                            continue;
                        }
                    };
                    match files::read_dimensions_file(&watch_root, year, month) {
                        Ok(dims) => {
                            let errors = validate_dimensions(&dims);
                            if let Err(e) = app_handle.emit("dimensions-changed", &errors) {
                                crate::error_log::log_error(
                                    "file_watcher",
                                    &format!("emit dimensions-changed failed: {}", e),
                                );
                            }
                        }
                        Err(e) => {
                            if let Err(e2) = app_handle.emit(
                                "dimensions-changed",
                                &vec![ConfigErrorDetail {
                                    kind: "ParseError".to_string(),
                                    message: e,
                                }],
                            ) {
                                crate::error_log::log_error(
                                    "file_watcher",
                                    &format!("emit dimensions-changed failed: {}", e2),
                                );
                            }
                        }
                    }
                } else if file_name == "commitments.yaml" {
                    // Reflect the month of the file that actually changed, not the
                    // current wall-clock month — editing a past month's commitments
                    // must not broadcast the current month's data.
                    let (year, month) = match month_from_monthly_path(path) {
                        Some(ym) => ym,
                        None => {
                            crate::error_log::log_error(
                                "file_watcher",
                                &format!("could not parse month from commitments.yaml path: {}", path.display()),
                            );
                            continue;
                        }
                    };
                    match files::read_commitments_file(&watch_root, year, month) {
                        Ok(commitments) => {
                            // Validate alongside any existing dimensions
                            let dims = files::read_dimensions_file(&watch_root, year, month)
                                .unwrap_or_default();
                            let monthly = MonthlyFile { dimensions: dims, commitments };
                            let errors = validate_monthly(&monthly);
                            if let Err(e) = app_handle.emit("commitments-changed", &errors) {
                                crate::error_log::log_error(
                                    "file_watcher",
                                    &format!("emit commitments-changed failed: {}", e),
                                );
                            }
                        }
                        Err(e) => {
                            if let Err(e2) = app_handle.emit(
                                "commitments-changed",
                                &vec![ConfigErrorDetail {
                                    kind: "ParseError".to_string(),
                                    message: e,
                                }],
                            ) {
                                crate::error_log::log_error(
                                    "file_watcher",
                                    &format!("emit commitments-changed failed: {}", e2),
                                );
                            }
                        }
                    }
                }
            }
        }
        crate::error_log::log_info("file_watcher", "receiver thread exited");
    });

    Ok((watcher, alive))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Commitment, Dimension, MonthlyFile, Template};

    #[test]
    fn needs_restart_logic() {
        use std::path::Path;
        assert!(super::needs_restart(None, Path::new("/a")), "no watcher → start");
        assert!(
            !super::needs_restart(Some(Path::new("/a")), Path::new("/a")),
            "same path → no-op"
        );
        assert!(
            super::needs_restart(Some(Path::new("/a")), Path::new("/b")),
            "different path → restart"
        );
    }

    #[test]
    fn test_dimension_required_defaults_to_false() {
        let yaml = "name: Test\nkey: test\nsource: static\nvalues: [a]";
        let dim: Dimension = yaml_serde::from_str(yaml).unwrap();
        assert!(!dim.required);
    }

    #[test]
    fn test_dimension_required_true() {
        let yaml = "name: Test\nkey: test\nsource: static\nvalues: [a]\nrequired: true";
        let dim: Dimension = yaml_serde::from_str(yaml).unwrap();
        assert!(dim.required);
    }

    #[test]
    fn test_validate_dimensions_valid() {
        let config = Template {
            dimensions: vec![
                Dimension {
                    name: "Biz".into(),
                    key: "biz".into(),
                    source: "static".into(),
                    values: Some(vec!["X".into()]),
                    required: false,
                    deleted: false,
                },
                Dimension {
                    name: "Goal".into(),
                    key: "goal".into(),
                    source: "commitments:goals".into(),
                    values: None,
                    required: false,
                    deleted: false,
                },
            ],
        };
        assert!(validate_dimensions(&config.dimensions).is_empty());
    }

    #[test]
    fn test_validate_dimensions_missing_values() {
        let config = Template {
            dimensions: vec![Dimension {
                name: "Cat".into(),
                key: "cat".into(),
                source: "static".into(),
                values: None,
                required: false,
                deleted: false,
            }],
        };
        let errors = validate_dimensions(&config.dimensions);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "MissingValues");
    }

    #[test]
    fn test_validate_dimensions_empty_values() {
        let config = Template {
            dimensions: vec![Dimension {
                name: "Cat".into(),
                key: "cat".into(),
                source: "static".into(),
                values: Some(vec![]),
                required: false,
                deleted: false,
            }],
        };
        let errors = validate_dimensions(&config.dimensions);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "ValuesEmpty");
    }

    #[test]
    fn test_validate_dimensions_invalid_key() {
        let config = Template {
            dimensions: vec![Dimension {
                name: "Bad".into(),
                key: "bad key!".into(),
                source: "static".into(),
                values: Some(vec!["x".into()]),
                required: false,
                deleted: false,
            }],
        };
        let errors = validate_dimensions(&config.dimensions);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "KeyInvalidChars");
    }

    #[test]
    fn test_validate_dimensions_multiple_monthly() {
        let config = Template {
            dimensions: vec![
                Dimension {
                    name: "G1".into(),
                    key: "g1".into(),
                    source: "commitments:goals".into(),
                    values: None,
                    required: false,
                    deleted: false,
                },
                Dimension {
                    name: "G2".into(),
                    key: "g2".into(),
                    source: "commitments:goals".into(),
                    values: None,
                    required: false,
                    deleted: false,
                },
            ],
        };
        let errors = validate_dimensions(&config.dimensions);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "MultipleGoalSource");
    }

    #[test]
    fn test_validate_dimensions_invalid_source() {
        let config = Template {
            dimensions: vec![Dimension {
                name: "Bad".into(),
                key: "bad".into(),
                source: "dynamic".into(),
                values: None,
                required: false,
                deleted: false,
            }],
        };
        let errors = validate_dimensions(&config.dimensions);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "InvalidSource");
    }

    #[test]
    fn test_validate_monthly_valid() {
        let monthly = MonthlyFile {
            dimensions: vec![],
            commitments: vec![Commitment {
                role: "Dev".into(),
                allocation: 40,
                goals: vec!["Ship X".into()],
            }],
        };
        assert!(validate_monthly(&monthly).is_empty());
    }

    #[test]
    fn test_validate_monthly_empty_role() {
        let monthly = MonthlyFile {
            dimensions: vec![],
            commitments: vec![Commitment {
                role: "".into(),
                allocation: 10,
                goals: vec![],
            }],
        };
        let errors = validate_monthly(&monthly);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "MissingRole");
    }

    #[test]
    fn test_validate_monthly_zero_allocation() {
        let monthly = MonthlyFile {
            dimensions: vec![],
            commitments: vec![Commitment {
                role: "Dev".into(),
                allocation: 0,
                goals: vec![],
            }],
        };
        let errors = validate_monthly(&monthly);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "ZeroAllocation");
    }

    #[test]
    fn test_validate_monthly_duplicate_goal() {
        let monthly = MonthlyFile {
            dimensions: vec![],
            commitments: vec![
                Commitment {
                    role: "Dev".into(),
                    allocation: 20,
                    goals: vec!["Shared".into()],
                },
                Commitment {
                    role: "PM".into(),
                    allocation: 10,
                    goals: vec!["Shared".into()],
                },
            ],
        };
        let errors = validate_monthly(&monthly);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "DuplicateGoal");
    }

    #[test]
    fn test_month_from_monthly_path_extracts_changed_month() {
        let p = std::path::Path::new("/data/2026/05/dimensions.yaml");
        assert_eq!(month_from_monthly_path(p), Some((2026, 5)));
    }

    #[test]
    fn test_month_from_monthly_path_rejects_non_numeric_and_bad_month() {
        assert_eq!(
            month_from_monthly_path(std::path::Path::new("/data/abc/05/dimensions.yaml")),
            None
        );
        assert_eq!(
            month_from_monthly_path(std::path::Path::new("/data/2026/13/commitments.yaml")),
            None
        );
        assert_eq!(
            month_from_monthly_path(std::path::Path::new("/dimensions.yaml")),
            None
        );
    }

    #[test]
    fn test_debounce_skips_within_window_and_bounds_map() {
        let mut m: HashMap<std::path::PathBuf, Instant> = HashMap::new();
        let window = Duration::from_millis(300);
        let t0 = Instant::now();
        let p = std::path::Path::new("/data/2026/06/dimensions.yaml");

        // First event for the path → process.
        assert!(debounce_and_record(&mut m, p, t0, window));
        // A second event 100ms later → skipped (within window).
        assert!(!debounce_and_record(&mut m, p, t0 + Duration::from_millis(100), window));
        // After the window → processed again.
        assert!(debounce_and_record(&mut m, p, t0 + Duration::from_millis(400), window));
    }

    #[test]
    fn test_debounce_prunes_stale_entries() {
        let mut m: HashMap<std::path::PathBuf, Instant> = HashMap::new();
        let window = Duration::from_millis(300);
        let t0 = Instant::now();

        // Touch 5 distinct one-shot paths at t0.
        for i in 0..5 {
            let p = std::path::PathBuf::from(format!("/data/2026/06/{}.md", i));
            assert!(debounce_and_record(&mut m, &p, t0, window));
        }
        assert_eq!(m.len(), 5);

        // A later event well past the window prunes all the stale ones, leaving
        // only the just-recorded path — the map cannot grow unbounded.
        let later = std::path::Path::new("/data/2026/06/new.md");
        assert!(debounce_and_record(&mut m, later, t0 + Duration::from_millis(500), window));
        assert_eq!(m.len(), 1, "stale entries must be pruned");
    }
}
