#![allow(non_snake_case)]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// --- Template ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub dimensions: Vec<Dimension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    pub name: String,
    pub key: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(default)] // false when absent
    pub required: bool,
    #[serde(default)] // false when absent; backward-compatible with existing files
    pub deleted: bool,
}

fn default_source() -> String {
    "static".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commitment {
    pub role: String,
    pub allocation: u32, // hours per month
    #[serde(default)]
    pub goals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentProgress {
    pub role: String,
    pub allocation_minutes: u32,
    pub goal_spent_minutes: u32,
    pub general_spent_minutes: u32,
    pub goals: Vec<GoalProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalProgress {
    pub name: String,
    pub spent_minutes: u32,
}


// --- Entries ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default)]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String, // UUID v4, generated at creation
    pub item: String,
    pub duration: u32, // minutes
    #[serde(default)]
    pub dimensions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntryInput {
    pub item: String,
    pub duration: String, // pre-parsed total by frontend, e.g. "60"
    #[serde(default)]
    pub dimensions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEntryInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<BTreeMap<String, String>>,
}

/// Current data format version. The main app never bumps this — only a
/// format-changing PR does. A separate migration tool bumps version.txt on disk.
pub const CURRENT_DATA_VERSION: u32 = 2;

// --- Init result ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryCategory {
    InPlace,
    ConfigMissing,
    RootMissing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum InitResult {
    NeedsSetup,
    DataVersionNotFound {
        root_path: String,
    },
    DataVersionMismatch {
        root_path: String,
        expected: u32,
        found: u32,
    },
    ConfigError {
        category: RecoveryCategory,
        root_path: String,
        errors: Vec<ConfigErrorDetail>,
        scan_warnings: Vec<ScanWarning>,
    },
    Ready {
        root_path: String,
        dimensions: Vec<Dimension>,
        usingDefaultDimensions: bool,
        today: DayFile,
        commitments: Vec<Commitment>,
        scan_warnings: Vec<ScanWarning>,
        #[serde(default)]
        integrity_issues: Vec<IntegrityIssue>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigErrorDetail {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanWarning {
    pub kind: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityIssue {
    pub path: String,
    pub message: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityStatus {
    pub compromised: bool,
    pub issues: Vec<IntegrityIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableMonth {
    pub year: i32,
    pub month: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthDimensions {
    pub dimensions: Vec<Dimension>,
    pub usingDefaultDimensions: bool,
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // ---------------------------------------------------------------------------
    // ScanWarning
    // ---------------------------------------------------------------------------

    #[test]
    fn scan_warning_fields() {
        let w = ScanWarning {
            kind: "SkippedFile".to_string(),
            path: "2026/06/broken.md".to_string(),
            message: "unrecognized frontmatter".to_string(),
        };

        assert_eq!(w.kind, "SkippedFile");
        assert_eq!(w.path, "2026/06/broken.md");
        assert_eq!(w.message, "unrecognized frontmatter");
    }

    #[test]
    fn scan_warning_serialize_deserialize() {
        let w = ScanWarning {
            kind: "OrphanedTemp".to_string(),
            path: "tmp/leftover.tmp".to_string(),
            message: "temp file without parent".to_string(),
        };

        let json = serde_json::to_string(&w).expect("serialize ScanWarning");
        let back: ScanWarning = serde_json::from_str(&json).expect("deserialize ScanWarning");

        assert_eq!(back.kind, "OrphanedTemp");
        assert_eq!(back.path, "tmp/leftover.tmp");
        assert_eq!(back.message, "temp file without parent");
    }

    // ---------------------------------------------------------------------------
    // InitResult::Ready with scan_warnings
    // ---------------------------------------------------------------------------

    #[test]
    fn init_result_ready_with_scan_warnings() {
        let today = DayFile {
            note: None,
            entries: vec![],
        };

        let warning = ScanWarning {
            kind: "SkippedFile".to_string(),
            path: "2026/06/bad.md".to_string(),
            message: "parse error".to_string(),
        };

        let result = InitResult::Ready {
            root_path: "/tmp/logbook-test".to_string(),
            dimensions: vec![],
            usingDefaultDimensions: false,
            today,
            commitments: vec![],
            scan_warnings: vec![warning],
            integrity_issues: vec![],
        };

        match &result {
            InitResult::Ready {
                scan_warnings, ..
            } => {
                assert_eq!(scan_warnings.len(), 1);
                assert_eq!(scan_warnings[0].kind, "SkippedFile");
            }
            _ => panic!("expected Ready variant"),
        }
    }

    #[test]
    fn init_result_ready_empty_scan_warnings() {
        let today = DayFile {
            note: None,
            entries: vec![],
        };

        let result = InitResult::Ready {
            root_path: "/tmp/logbook-test".to_string(),
            dimensions: vec![],
            usingDefaultDimensions: false,
            today,
            commitments: vec![],
            scan_warnings: vec![],
            integrity_issues: vec![],
        };

        match &result {
            InitResult::Ready {
                scan_warnings, ..
            } => {
                assert!(scan_warnings.is_empty());
            }
            _ => panic!("expected Ready variant"),
        }
    }

    // ---------------------------------------------------------------------------
    // InitResult::ConfigError with scan_warnings
    // ---------------------------------------------------------------------------

    #[test]
    fn init_result_config_error_with_scan_warnings() {
        let errors = vec![ConfigErrorDetail {
            kind: "MissingFile".to_string(),
            message: "dimensions.template.yaml not found".to_string(),
        }];
        let warnings = vec![ScanWarning {
            kind: "CorruptedFile".to_string(),
            path: "2026/06/dimensions.yaml".to_string(),
            message: "invalid YAML frontmatter".to_string(),
        }];

        let result = InitResult::ConfigError {
            category: RecoveryCategory::ConfigMissing,
            root_path: "/tmp/x".to_string(),
            errors,
            scan_warnings: warnings,
        };

        match &result {
            InitResult::ConfigError {
                errors,
                scan_warnings,
                ..
            } => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].kind, "MissingFile");
                assert_eq!(scan_warnings.len(), 1);
                assert_eq!(scan_warnings[0].kind, "CorruptedFile");
            }
            _ => panic!("expected ConfigError variant"),
        }
    }

    #[test]
    fn recovery_category_serializes_snake_case() {
        let json = serde_json::to_string(&RecoveryCategory::RootMissing).expect("serialize");
        assert_eq!(json, "\"root_missing\"");
        let back: RecoveryCategory =
            serde_json::from_str("\"config_missing\"").expect("deserialize");
        assert_eq!(back, RecoveryCategory::ConfigMissing);
    }

    #[test]
    fn config_error_carries_category_and_root_path() {
        let result = InitResult::ConfigError {
            category: RecoveryCategory::ConfigMissing,
            root_path: "/tmp/logbook".to_string(),
            errors: vec![],
            scan_warnings: vec![],
        };
        match result {
            InitResult::ConfigError { category, root_path, .. } => {
                assert_eq!(category, RecoveryCategory::ConfigMissing);
                assert_eq!(root_path, "/tmp/logbook");
            }
            _ => panic!("expected ConfigError"),
        }
    }

    #[test]
    fn init_result_config_error_empty_warnings() {
        let errors = vec![ConfigErrorDetail {
            kind: "InvalidConfig".to_string(),
            message: "dimension missing key".to_string(),
        }];

        let result = InitResult::ConfigError {
            category: RecoveryCategory::InPlace,
            root_path: "/tmp/x".to_string(),
            errors,
            scan_warnings: vec![],
        };

        match &result {
            InitResult::ConfigError {
                errors,
                scan_warnings,
                ..
            } => {
                assert_eq!(errors.len(), 1);
                assert!(scan_warnings.is_empty());
            }
            _ => panic!("expected ConfigError variant"),
        }
    }

    // ---------------------------------------------------------------------------
    // JSON roundtrip for the new InitResult shapes
    // ---------------------------------------------------------------------------

    #[test]
    fn init_result_ready_json_roundtrip() {
        let warning = ScanWarning {
            kind: "SkippedFile".to_string(),
            path: "2026/06/x.md".to_string(),
            message: "bad".to_string(),
        };

        let today = DayFile {
            note: None,
            entries: vec![],
        };

        let result = InitResult::Ready {
            root_path: "/tmp/lb".to_string(),
            dimensions: vec![],
            usingDefaultDimensions: false,
            today,
            commitments: vec![],
            scan_warnings: vec![warning],
            integrity_issues: vec![],
        };

        let json = serde_json::to_string_pretty(&result).expect("serialize Ready");
        let back: InitResult = serde_json::from_str(&json).expect("deserialize Ready");

        match &back {
            InitResult::Ready {
                root_path,
                scan_warnings,
                ..
            } => {
                assert_eq!(root_path, "/tmp/lb");
                assert_eq!(scan_warnings.len(), 1);
                assert_eq!(scan_warnings[0].kind, "SkippedFile");
            }
            _ => panic!("expected Ready after roundtrip"),
        }
    }

    #[test]
    fn init_result_config_error_json_roundtrip() {
        let errors = vec![ConfigErrorDetail {
            kind: "BadYaml".to_string(),
            message: "invalid syntax".to_string(),
        }];
        let warnings = vec![
            ScanWarning {
                kind: "SkippedFile".to_string(),
                path: "a.md".to_string(),
                message: "skip".to_string(),
            },
            ScanWarning {
                kind: "OrphanedTemp".to_string(),
                path: "b.tmp".to_string(),
                message: "orphan".to_string(),
            },
        ];

        let result = InitResult::ConfigError {
            category: RecoveryCategory::InPlace,
            root_path: "/tmp/x".to_string(),
            errors,
            scan_warnings: warnings,
        };

        let json = serde_json::to_string_pretty(&result).expect("serialize ConfigError");
        let back: InitResult = serde_json::from_str(&json).expect("deserialize ConfigError");

        match &back {
            InitResult::ConfigError {
                errors,
                scan_warnings,
                ..
            } => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].kind, "BadYaml");
                assert_eq!(scan_warnings.len(), 2);
                assert_eq!(scan_warnings[1].kind, "OrphanedTemp");
            }
            _ => panic!("expected ConfigError after roundtrip"),
        }
    }

    #[test]
    fn init_result_needs_setup_json_roundtrip() {
        // NeedsSetup should still serialize/deserialize as before (no warnings field)
        let result = InitResult::NeedsSetup;

        let json = serde_json::to_string(&result).expect("serialize NeedsSetup");
        let back: InitResult = serde_json::from_str(&json).expect("deserialize NeedsSetup");

        match &back {
            InitResult::NeedsSetup => {} // ok
            _ => panic!("expected NeedsSetup after roundtrip"),
        }
    }

    // ---------------------------------------------------------------------------
    // ScanWarning JSON shape
    // ---------------------------------------------------------------------------

    #[test]
    fn scan_warning_json_shape() {
        let w = ScanWarning {
            kind: "SkippedFile".to_string(),
            path: "2026/06/f.md".to_string(),
            message: "msg".to_string(),
        };

        let json = serde_json::to_string_pretty(&w).expect("serialize");

        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("parse as generic JSON");

        assert_eq!(parsed["kind"], "SkippedFile");
        assert_eq!(parsed["path"], "2026/06/f.md");
        assert_eq!(parsed["message"], "msg");
        // ScanWarning should have exactly three keys
        assert_eq!(parsed.as_object().unwrap().len(), 3);
    }
}
