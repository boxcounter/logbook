/// Contract test runner.
///
/// Reads YAML contract files from `tests/contracts/`, sets up temp fixture dirs,
/// dispatches to real Rust functions, and asserts results against expectations.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// YAML deserialization structs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ContractFile {
    command: String,
    #[allow(dead_code)]
    description: String,
    cases: Vec<ContractCase>,
}

#[derive(Debug, Deserialize)]
struct ContractCase {
    name: String,
    #[serde(default)]
    before: Vec<FixtureStep>,
    input: serde_json::Value,
    expect: ExpectBlock,
}

#[derive(Debug, Deserialize)]
struct FixtureStep {
    action: String,
    path: String,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectBlock {
    ok: Option<serde_json::Value>,
    err_contains: Option<String>,
}

// ---------------------------------------------------------------------------
// Fixture setup
// ---------------------------------------------------------------------------

fn setup_fixture_root(suffix: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("logbook_contract_{}", suffix));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    tmp
}

fn copy_base_fixtures(root: &Path) {
    let base = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures"));
    if base.exists() {
        copy_dir(base, root).expect("failed to copy base fixtures");
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("create_dir: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("read_dir: {}", e))? {
        let entry = entry.map_err(|e| format!("entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("copy {}: {}", src_path.display(), e))?;
        }
    }
    Ok(())
}

fn apply_before_steps(root: &Path, steps: &[FixtureStep]) {
    for step in steps {
        match step.action.as_str() {
            "write_file" => {
                let file_path = root.join(&step.path);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(&file_path, step.content.as_deref().unwrap_or(""))
                    .unwrap_or_else(|_| panic!("failed to write {}", step.path));
            }
            "create_dir" => {
                fs::create_dir_all(root.join(&step.path))
                    .unwrap_or_else(|_| panic!("failed to create dir {}", step.path));
            }
            _ => panic!("Unknown before action: {}", step.action),
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

fn assert_matches(actual: &serde_json::Value, expected: &serde_json::Value) {
    // Special case: expected is an empty array
    if expected.is_array() && expected.as_array().map(|a| a.is_empty()).unwrap_or(false) {
        assert!(
            actual.is_array() && actual.as_array().map(|a| a.is_empty()).unwrap_or(false),
            "Expected empty array, got {:?}",
            actual
        );
        return;
    }

    let obj = expected
        .as_object()
        .expect("expect.ok must be a JSON object");
    for (key, expected_val) in obj {
        // Handle "$exists"
        if expected_val.is_string() && expected_val.as_str() == Some("$exists") {
            let current = resolve_path(actual, key);
            assert!(
                !current.is_null(),
                "Key '{}': expected non-null value, got null",
                key
            );
            continue;
        }

        // Handle `len` or `field.len`
        if key == "len" || key.ends_with(".len") {
            let array_path = if key == "len" {
                ""
            } else {
                key.trim_end_matches(".len")
            };
            let current = if array_path.is_empty() {
                actual
            } else {
                resolve_path(actual, array_path)
            };
            let actual_len = current
                .as_array()
                .unwrap_or_else(|| panic!("Key '{}': expected array, got {:?}", key, current))
                .len();
            let expected_len = expected_val
                .as_u64()
                .unwrap_or_else(|| panic!("Key '{}': expected numeric len value", key))
                as usize;
            assert_eq!(
                actual_len, expected_len,
                "Key '{}': expected length {}, got {}",
                key, expected_len, actual_len
            );
            continue;
        }

        // Dot-separated path resolution
        let current = resolve_path(actual, key);

        if expected_val.is_null() {
            assert!(
                current.is_null(),
                "Key '{}': expected null, got {:?}",
                key,
                current
            );
        } else {
            assert_eq!(
                current, expected_val,
                "Key '{}': expected {:?}, got {:?}",
                key, expected_val, current
            );
        }
    }
}

static JSON_NULL: serde_json::Value = serde_json::Value::Null;

fn resolve_path<'a>(root: &'a serde_json::Value, path: &str) -> &'a serde_json::Value {
    let mut current = root;
    for part in path.split('.') {
        if let Ok(idx) = part.parse::<usize>() {
            match current.get(idx) {
                Some(v) => current = v,
                None => return &JSON_NULL,
            }
        } else {
            match current.get(part) {
                Some(v) => current = v,
                None => return &JSON_NULL,
            }
        }
    }
    current
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

fn dispatch_command(
    command: &str,
    root: &Path,
    input: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let root_str = root.to_string_lossy().to_string();

    match command {
        "get_entries" => {
            let date = input["date"].as_str().unwrap().to_string();
            let df = tauri_app_lib::files::read_day_file(root, &date)?;
            Ok(serde_json::to_value(df).unwrap())
        }

        "append_entry" => {
            let date = input["date"].as_str().unwrap().to_string();
            let entry_input = &input["entry"];
            let new_entry = tauri_app_lib::models::CreateEntryInput {
                item: entry_input["item"].as_str().unwrap().to_string(),
                duration: entry_input["duration"].as_str().unwrap().to_string(),
                dimensions: entry_input["dimensions"]
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                            .collect::<HashMap<_, _>>()
                    })
                    .unwrap_or_default(),
            };
            let entry = tauri_app_lib::files::append_new_entry(root, &date, &new_entry)?;
            Ok(serde_json::to_value(entry).unwrap())
        }

        "update_entry" => {
            let date = input["date"].as_str().unwrap().to_string();
            let entry_id = input["entry_id"].as_str().unwrap().to_string();
            let update_input = &input["update"];
            let update = tauri_app_lib::models::UpdateEntryInput {
                item: update_input.get("item").and_then(|v| v.as_str()).map(String::from),
                duration: update_input.get("duration").and_then(|v| v.as_str()).map(String::from),
                dimensions: update_input.get("dimensions").map(|d| {
                    d.as_object()
                        .map(|obj| {
                            obj.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                                .collect::<std::collections::HashMap<_, _>>()
                        })
                        .unwrap_or_default()
                }),
            };
            let df = tauri_app_lib::files::update_entry_in_file(root, &date, &entry_id, &update)?;
            Ok(serde_json::to_value(df).unwrap())
        }

        "delete_entry" => {
            let date = input["date"].as_str().unwrap().to_string();
            let entry_id = input["entry_id"].as_str().unwrap().to_string();
            let df = tauri_app_lib::files::delete_entry_from_file(root, &date, &entry_id)?;
            Ok(serde_json::to_value(df).unwrap())
        }

        "set_day_note" => {
            let date = input["date"].as_str().unwrap().to_string();
            let note = input["note"].as_str().unwrap().to_string();
            let df = tauri_app_lib::files::set_day_note_in_file(root, &date, &note)?;
            Ok(serde_json::to_value(df).unwrap())
        }

        "get_commitments" => {
            let year = input["year"].as_i64().unwrap() as i32;
            let month = input["month"].as_u64().unwrap() as u32;
            let mf = tauri_app_lib::files::read_monthly_file(root, year, month)?;
            Ok(serde_json::to_value(mf.commitments).unwrap())
        }

        "get_commitment_progress" => {
            let year = input["year"].as_i64().unwrap() as i32;
            let month = input["month"].as_u64().unwrap() as u32;
            let result = tauri_app_lib::commands::get_commitment_progress(
                root_str, year, month,
            )?;
            Ok(serde_json::to_value(result).unwrap())
        }

        "get_available_months" => {
            let result = tauri_app_lib::commands::get_available_months(root_str)?;
            Ok(serde_json::to_value(result).unwrap())
        }

        "create_starter_files" => {
            tauri_app_lib::commands::create_starter_files(root_str)?;
            Ok(serde_json::json!({"created": true}))
        }

        _ => Err(format!("Unknown command: {}", command)),
    }
}

// ---------------------------------------------------------------------------
// Contract runner
// ---------------------------------------------------------------------------

fn run_contract(yaml_path: &str) {
    let full_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/")).join(yaml_path);
    let yaml_content = fs::read_to_string(&full_path)
        .unwrap_or_else(|_| panic!("Failed to read contract file: {}", full_path.display()));

    let contract: ContractFile = yaml_serde::from_str(&yaml_content)
        .unwrap_or_else(|_| panic!("Failed to parse {}", yaml_path));

    for case in &contract.cases {
        println!("  Case: {}", case.name);

        let suffix = format!(
            "{}_{}",
            contract.command,
            case.name.replace(' ', "_").to_lowercase()
        );
        let root = setup_fixture_root(&suffix);
        copy_base_fixtures(&root);
        apply_before_steps(&root, &case.before);

        // Substitute {ROOT} in input
        let mut input = case.input.clone();
        if let Some(obj) = input.as_object_mut() {
            for (_key, val) in obj.iter_mut() {
                if val.is_string() && val.as_str() == Some("{ROOT}") {
                    *val = serde_json::Value::String(root.to_string_lossy().to_string());
                }
            }
        }

        let result = dispatch_command(&contract.command, &root, &input);

        match (&case.expect.ok, &case.expect.err_contains) {
            (Some(expected_ok), None) => {
                let actual = result.unwrap_or_else(|e| {
                    panic!(
                        "Case '{}': expected Ok, got Err: {:?}",
                        case.name, e
                    )
                });
                assert_matches(&actual, expected_ok);
            }
            (None, Some(expected_err)) => {
                let err = result.expect_err(&format!(
                    "Case '{}': expected Err, got Ok",
                    case.name
                ));
                assert!(
                    err.contains(expected_err),
                    "Case '{}': expected error containing '{}', got: {}",
                    case.name,
                    expected_err,
                    err
                );
            }
            _ => panic!(
                "Case '{}': expect must have exactly one of ok or err_contains",
                case.name
            ),
        }

        // Cleanup
        let _ = fs::remove_dir_all(&root);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn contract_get_entries() {
    run_contract("contracts/get_entries.yaml");
}

#[test]
fn contract_append_entry() {
    run_contract("contracts/append_entry.yaml");
}

#[test]
fn contract_update_entry() {
    run_contract("contracts/update_entry.yaml");
}

#[test]
fn contract_delete_entry() {
    run_contract("contracts/delete_entry.yaml");
}

#[test]
fn contract_set_day_note() {
    run_contract("contracts/set_day_note.yaml");
}

#[test]
fn contract_get_commitments() {
    run_contract("contracts/get_commitments.yaml");
}

#[test]
fn contract_get_commitment_progress() {
    run_contract("contracts/get_commitment_progress.yaml");
}

#[test]
fn contract_get_available_months() {
    run_contract("contracts/get_available_months.yaml");
}

#[test]
fn contract_create_starter_files() {
    run_contract("contracts/create_starter_files.yaml");
}
