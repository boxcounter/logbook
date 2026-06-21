use crate::files;
use crate::models::{Config, ConfigErrorDetail, MonthlyFile};
use chrono::Datelike;
use notify::{Config as NotifyConfig, Event, EventKind, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

fn is_valid_key(key: &str) -> bool {
    key.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

pub fn validate_config(config: &Config) -> Vec<ConfigErrorDetail> {
    let mut errors = Vec::new();
    let mut monthly_count = 0;

    for (i, dim) in config.dimensions.iter().enumerate() {
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
            "monthly" => {
                monthly_count += 1;
                if monthly_count > 1 {
                    errors.push(ConfigErrorDetail {
                        kind: "MultipleMonthly".to_string(),
                        message: format!(
                            "Dimension '{}': only one dimension may have source: monthly",
                            dim.name
                        ),
                    });
                }
            }
            other => {
                errors.push(ConfigErrorDetail {
                    kind: "InvalidSource".to_string(),
                    message: format!(
                        "Dimension '{}': invalid source '{}' (expected 'static' or 'monthly')",
                        dim.name, other
                    ),
                });
            }
        }
    }
    errors
}

pub fn validate_monthly(monthly: &MonthlyFile) -> Vec<ConfigErrorDetail> {
    let mut errors = Vec::new();
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

pub fn watch_files(app_handle: AppHandle, root_path: PathBuf) {
    crate::error_log::log_info("file_watcher", &format!("Watching {}", root_path.display()));
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    if let Err(e) = tx.send(event) {
                        crate::error_log::log_error(
                            "file_watcher",
                            &format!("send error: {:?}", e),
                        );
                    }
                }
                Err(e) => {
                    crate::error_log::log_error("file_watcher", &format!("notify error: {}", e));
                }
            })
            .expect("Failed to create file watcher");

        watcher
            .configure(NotifyConfig::default())
            .expect("Failed to configure watcher");

        // Watch root directory recursively to catch template.yaml
        // and all _monthly.md files, including across month boundaries.
        if let Err(e) = watcher.watch(&root_path, RecursiveMode::Recursive) {
            crate::error_log::log_error("file_watcher", &format!("Failed to watch: {}", e));
        }

        let debounce_ms = Duration::from_millis(300);
        let mut last_event: HashMap<std::path::PathBuf, Instant> = HashMap::new();

        for event in rx {
            let is_modify = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_));
            if !is_modify {
                continue;
            }

            for path in &event.paths {
                // Debounce: skip if same path fired within 300ms
                let now = Instant::now();
                if let Some(last) = last_event.get(path) {
                    if now.duration_since(*last) < debounce_ms {
                        continue;
                    }
                }
                last_event.insert(path.clone(), now);

                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "template.yaml" {
                    match files::read_template(&root_path) {
                        Ok(config) => {
                            let errors = validate_config(&config);
                            if let Err(e) = app_handle.emit("config-changed", &errors) {
                                crate::error_log::log_error(
                                    "file_watcher",
                                    &format!("emit config-changed failed: {}", e),
                                );
                            }
                        }
                        Err(e) => {
                            if let Err(e2) = app_handle.emit(
                                "config-changed",
                                &vec![ConfigErrorDetail {
                                    kind: "ParseError".to_string(),
                                    message: e,
                                }],
                            ) {
                                crate::error_log::log_error(
                                    "file_watcher",
                                    &format!("emit config-changed failed: {}", e2),
                                );
                            }
                        }
                    }
                } else if file_name == "_monthly.md" {
                    // Re-read current month each time (handles month boundary)
                    let now = chrono::Local::now();
                    match files::read_monthly_file(&root_path, now.year(), now.month()) {
                        Ok(monthly) => {
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
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Commitment, Config, Dimension, MonthlyFile};

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
    fn test_validate_config_valid() {
        let config = Config {
            dimensions: vec![
                Dimension {
                    name: "Biz".into(),
                    key: "biz".into(),
                    source: "static".into(),
                    values: Some(vec!["X".into()]),
                    required: false,
                },
                Dimension {
                    name: "Goal".into(),
                    key: "goal".into(),
                    source: "monthly".into(),
                    values: None,
                    required: false,
                },
            ],
        };
        assert!(validate_config(&config).is_empty());
    }

    #[test]
    fn test_validate_config_missing_values() {
        let config = Config {
            dimensions: vec![Dimension {
                name: "Cat".into(),
                key: "cat".into(),
                source: "static".into(),
                values: None,
                required: false,
            }],
        };
        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "MissingValues");
    }

    #[test]
    fn test_validate_config_empty_values() {
        let config = Config {
            dimensions: vec![Dimension {
                name: "Cat".into(),
                key: "cat".into(),
                source: "static".into(),
                values: Some(vec![]),
                required: false,
            }],
        };
        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "ValuesEmpty");
    }

    #[test]
    fn test_validate_config_invalid_key() {
        let config = Config {
            dimensions: vec![Dimension {
                name: "Bad".into(),
                key: "bad key!".into(),
                source: "static".into(),
                values: Some(vec!["x".into()]),
                required: false,
            }],
        };
        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "KeyInvalidChars");
    }

    #[test]
    fn test_validate_config_multiple_monthly() {
        let config = Config {
            dimensions: vec![
                Dimension {
                    name: "G1".into(),
                    key: "g1".into(),
                    source: "monthly".into(),
                    values: None,
                    required: false,
                },
                Dimension {
                    name: "G2".into(),
                    key: "g2".into(),
                    source: "monthly".into(),
                    values: None,
                    required: false,
                },
            ],
        };
        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "MultipleMonthly");
    }

    #[test]
    fn test_validate_config_invalid_source() {
        let config = Config {
            dimensions: vec![Dimension {
                name: "Bad".into(),
                key: "bad".into(),
                source: "dynamic".into(),
                values: None,
                required: false,
            }],
        };
        let errors = validate_config(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, "InvalidSource");
    }

    #[test]
    fn test_validate_monthly_valid() {
        let monthly = MonthlyFile {
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
}
