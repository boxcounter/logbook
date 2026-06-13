use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Config ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
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
}

fn default_source() -> String {
    "static".to_string()
}

// --- Monthly file ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyFile {
    #[serde(default)]
    pub commitments: Vec<Commitment>,
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
    pub spent_minutes: u32,
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
    pub dimensions: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEntry {
    pub item: String,
    pub duration: String, // pre-parsed total by frontend, e.g. "60"
    #[serde(default)]
    pub dimensions: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<HashMap<String, String>>,
}

// --- Init result ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum InitResult {
    NeedsSetup,
    ConfigError(Vec<ConfigErrorDetail>),
    Ready {
        root_path: String,
        config: Config,
        today: DayFile,
        commitments: Vec<Commitment>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigErrorDetail {
    pub kind: String,
    pub message: String,
}
