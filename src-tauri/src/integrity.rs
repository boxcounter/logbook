use std::path::Path;

use crate::models::{IntegrityIssue, IntegrityStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

static INTEGRITY_OK: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(true));
static INTEGRITY_ISSUES: LazyLock<Mutex<Vec<IntegrityIssue>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub fn check() -> Result<(), String> {
    if INTEGRITY_OK.load(Ordering::Acquire) {
        Ok(())
    } else {
        let issues = INTEGRITY_ISSUES
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let msg = if issues.is_empty() {
            "Write denied: data integrity compromised".to_string()
        } else {
            format!(
                "Write denied: data integrity compromised ({} issue{})",
                issues.len(),
                if issues.len() == 1 { "" } else { "s" }
            )
        };
        Err(msg)
    }
}

pub fn set_compromised(issue: IntegrityIssue) {
    INTEGRITY_ISSUES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(issue);
    INTEGRITY_OK.store(false, Ordering::Release);
}

pub fn reset() {
    INTEGRITY_ISSUES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    INTEGRITY_OK.store(true, Ordering::Release);
}

pub fn status() -> IntegrityStatus {
    let ok = INTEGRITY_OK.load(Ordering::Acquire);
    let issues = INTEGRITY_ISSUES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    IntegrityStatus {
        compromised: !ok,
        issues,
    }
}

pub fn check_day_file_integrity(root: &Path, date: &str) -> Result<(), IntegrityIssue> {
    use crate::files;

    let rel_path = {
        let dp = files::day_path(root, date).map_err(|e| IntegrityIssue {
            path: date.to_string(),
            message: format!("Failed to resolve day path: {}", e),
            kind: "PathError".into(),
        })?;
        dp.strip_prefix(root)
            .unwrap_or(&dp)
            .to_string_lossy()
            .to_string()
    };

    let day_file = match files::read_day_file(root, date) {
        Ok(df) => df,
        Err(e) => {
            return Err(IntegrityIssue {
                path: rel_path,
                message: format!("YAML parse failed: {}", e),
                kind: "YamlParseError".into(),
            });
        }
    };

    let (year, month) = crate::files::year_month_from_date(date).map_err(|e| IntegrityIssue {
        path: rel_path.clone(),
        message: e,
        kind: "DateParseError".into(),
    })?;

    let dims = match crate::files::read_dimensions_file(root, year, month) {
        Ok(d) => d,
        Err(e) => {
            return Err(IntegrityIssue {
                path: rel_path,
                message: format!("Cannot read monthly dimensions.yaml: {}", e),
                kind: "DimensionsFileError".into(),
            });
        }
    };

    for entry in &day_file.entries {
        if entry.duration == 0 {
            return Err(IntegrityIssue {
                path: rel_path,
                message: format!("Entry {} has duration = 0", entry.id),
                kind: "InvalidDuration".into(),
            });
        }

        if !is_valid_uuid_v4(&entry.id) {
            return Err(IntegrityIssue {
                path: rel_path,
                message: format!("Entry {} has invalid UUID", entry.id),
                kind: "InvalidUuid".into(),
            });
        }

        for key in entry.dimensions.keys() {
            if !dims.iter().any(|d| &d.key == key) {
                return Err(IntegrityIssue {
                    path: rel_path,
                    message: format!(
                        "Entry {} has unknown dimension key '{}' (not in monthly dimensions.yaml)",
                        entry.id, key
                    ),
                    kind: "UnknownDimensionKey".into(),
                });
            }
        }

        for dim in &dims {
            if dim.required && !dim.deleted {
                match entry.dimensions.get(&dim.key) {
                    None => {
                        return Err(IntegrityIssue {
                            path: rel_path,
                            message: format!(
                                "Entry {} missing required dimension '{}'",
                                entry.id, dim.name
                            ),
                            kind: "MissingRequiredDimension".into(),
                        });
                    }
                    Some(v) if v.trim().is_empty() => {
                        return Err(IntegrityIssue {
                            path: rel_path,
                            message: format!(
                                "Entry {} has empty value for required dimension '{}'",
                                entry.id, dim.name
                            ),
                            kind: "EmptyRequiredDimension".into(),
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    let op_log_path = root
        .join(".logbook")
        .join("operations")
        .join(format!("{:04}", year))
        .join(format!("{:02}", month))
        .join(format!("{}.jsonl", date));
    if op_log_path.exists() {
        match std::fs::read_to_string(&op_log_path) {
            Ok(content) => {
                for (line_num, line) in content.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if serde_json::from_str::<serde_json::Value>(line).is_err() {
                        return Err(IntegrityIssue {
                            path: format!(
                                ".logbook/operations/{:04}/{:02}/{}.jsonl",
                                year, month, date
                            ),
                            message: format!("Line {} is invalid JSON", line_num + 1),
                            kind: "JsonlParseError".into(),
                        });
                    }
                }
            }
            Err(_) => {
                return Err(IntegrityIssue {
                    path: format!(
                        ".logbook/operations/{:04}/{:02}/{}.jsonl",
                        year, month, date
                    ),
                    message: "File is not valid UTF-8".into(),
                    kind: "Utf8Error".into(),
                });
            }
        }
    }

    Ok(())
}

fn is_valid_uuid_v4(s: &str) -> bool {
    uuid::Uuid::parse_str(s).map_or(false, |u| u.get_version_num() == 4)
}

pub fn check_scoped_integrity(root: &Path) -> Vec<IntegrityIssue> {
    use chrono::{Datelike, Local};

    let now = Local::now();
    let today = now.date_naive();
    let mut issues = Vec::new();

    for offset in 0..=3 {
        let target = if offset == 0 {
            today
        } else {
            let mut y = today.year();
            let mut m = today.month() as i32 - offset as i32;
            while m <= 0 {
                m += 12;
                y -= 1;
            }
            chrono::NaiveDate::from_ymd_opt(y, m as u32, 1).unwrap_or(today)
        };
        let year = target.year();
        let month = target.month();

        let month_dir = root.join(year.to_string()).join(format!("{:02}", month));
        if !month_dir.exists() {
            continue;
        }

        let entries = match std::fs::read_dir(&month_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if !file_name.ends_with(".yaml") {
                continue;
            }
            let date = file_name.trim_end_matches(".yaml");
            if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
                continue;
            }

            match check_day_file_integrity(root, date) {
                Ok(()) => {}
                Err(issue) => {
                    issues.push(issue);
                    return issues;
                }
            }
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_uncompromised() {
        reset();
        assert!(check().is_ok());
        let s = status();
        assert!(!s.compromised);
        assert!(s.issues.is_empty());
    }

    #[test]
    fn set_compromised_blocks_writes() {
        reset();
        set_compromised(IntegrityIssue {
            path: "2026/07/05.yaml".into(),
            message: "corrupt YAML".into(),
            kind: "YamlParseError".into(),
        });
        assert!(check().is_err());
        let s = status();
        assert!(s.compromised);
        assert_eq!(s.issues.len(), 1);
        assert_eq!(s.issues[0].kind, "YamlParseError");
    }

    #[test]
    fn reset_restores_writes() {
        set_compromised(IntegrityIssue {
            path: "x.yaml".into(),
            message: "bad".into(),
            kind: "Test".into(),
        });
        reset();
        assert!(check().is_ok());
        let s = status();
        assert!(!s.compromised);
        assert!(s.issues.is_empty());
    }

    #[test]
    fn multiple_issues_accumulate() {
        reset();
        set_compromised(IntegrityIssue {
            path: "a.yaml".into(),
            message: "e1".into(),
            kind: "K1".into(),
        });
        set_compromised(IntegrityIssue {
            path: "b.yaml".into(),
            message: "e2".into(),
            kind: "K2".into(),
        });
        assert!(check().is_err());
        let s = status();
        assert_eq!(s.issues.len(), 2);
    }
}
