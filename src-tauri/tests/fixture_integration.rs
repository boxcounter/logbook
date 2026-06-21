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

#[test]
fn test_read_and_validate_config() {
    let root = fixture_root();
    let config = tauri_app_lib::files::read_template(&root).expect("read_template should succeed");
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
fn test_read_and_validate_monthly() {
    let root = fixture_root();
    let monthly = tauri_app_lib::files::read_monthly_file(&root, 2026, 6)
        .expect("read_monthly_file should succeed");
    let errors = tauri_app_lib::config::validate_monthly(&monthly);
    assert!(
        errors.is_empty(),
        "Monthly validation failed:\n{}",
        errors
            .iter()
            .map(|e| format!("  [{}] {}", e.kind, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_config_dimensions_count() {
    let root = fixture_root();
    let config = tauri_app_lib::files::read_template(&root).unwrap();
    assert_eq!(config.dimensions.len(), 4);
    let keys: Vec<&str> = config.dimensions.iter().map(|d| d.key.as_str()).collect();
    assert!(keys.contains(&"importance-urgency"));
    assert!(keys.contains(&"business-line"));
    assert!(keys.contains(&"category"));
    assert!(keys.contains(&"goal"));
}

#[test]
fn test_monthly_commitments_count() {
    let root = fixture_root();
    let monthly = tauri_app_lib::files::read_monthly_file(&root, 2026, 6).unwrap();
    assert_eq!(monthly.commitments.len(), 2);
    assert_eq!(monthly.commitments[0].role, "Developer");
    assert_eq!(monthly.commitments[1].role, "Director");
}
