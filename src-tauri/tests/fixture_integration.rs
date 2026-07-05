/// Integration tests using fixture data.
/// Set LOGBOOK_TEST_FIXTURE env var to override the default ~/Downloads/logbook-test path.
use std::path::Path;

fn fixture_root() -> std::path::PathBuf {
    std::env::var("LOGBOOK_TEST_FIXTURE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            Path::new(&home).join("Downloads/logbook-test")
        })
}

/// These tests assert on a developer's local scratch dir. Skip cleanly when it is
/// absent (CI, or the dir was cleared) instead of failing the whole suite on
/// environment state unrelated to the code under test.
macro_rules! skip_if_no_fixture {
    () => {
        if !fixture_root().join("dimensions.template.yaml").exists() {
            eprintln!(
                "skipping fixture test: no dimensions.template.yaml under {:?} (set LOGBOOK_TEST_FIXTURE to override)",
                fixture_root()
            );
            return;
        }
    };
}

#[test]
fn test_read_and_validate_config() {
    skip_if_no_fixture!();
    let root = fixture_root();
    let config = tauri_app_lib::files::read_dimensions_template(&root).expect("read_dimensions_template should succeed");
    let errors = tauri_app_lib::config::validate_dimensions(&config.dimensions);
    assert!(
        errors.is_empty(),
        "Config validation failed:\n{}",
        errors
            .iter()
            .map(|e| format!("  [{}] {}", e.kind, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_read_and_validate_commitments() {
    skip_if_no_fixture!();
    let root = fixture_root();
    let commitments = tauri_app_lib::files::read_commitments_file(&root, 2026, 6)
        .expect("read_commitments_file should succeed");
    let errors = match tauri_app_lib::config::validate_commitments(&commitments) {
        Ok(()) => vec![],
        Err(e) => vec![tauri_app_lib::models::ConfigErrorDetail {
            kind: "Validation".to_string(),
            message: e,
        }],
    };
    assert!(
        errors.is_empty(),
        "Commitment validation failed:\n{}",
        errors
            .iter()
            .map(|e| format!("  [{}] {}", e.kind, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_config_dimensions_count() {
    skip_if_no_fixture!();
    let root = fixture_root();
    let config = tauri_app_lib::files::read_dimensions_template(&root).unwrap();
    assert_eq!(config.dimensions.len(), 8);
    let keys: Vec<&str> = config.dimensions.iter().map(|d| d.key.as_str()).collect();
    assert!(keys.contains(&"importance-urgency"));
    assert!(keys.contains(&"business-line"));
    assert!(keys.contains(&"category"));
    assert!(keys.contains(&"goal"));
    assert!(keys.contains(&"energy"));
    assert!(keys.contains(&"location"));
}

#[test]
fn test_monthly_commitments_count() {
    skip_if_no_fixture!();
    let root = fixture_root();
    let commitments = tauri_app_lib::files::read_commitments_file(&root, 2026, 6).unwrap();
    assert_eq!(commitments.len(), 2);
    assert_eq!(commitments[0].role, "Developer");
    assert_eq!(commitments[1].role, "Director");
}
