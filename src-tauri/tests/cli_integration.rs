/// CLI integration tests.
/// These tests build and run the logbook-cli binary against temporary fixture directories.
use std::fs;
use std::path::Path;
use std::process::Command;

/// Get path to the compiled binary. Assumes `cargo build --bin logbook-cli` has been run.
fn cli_binary() -> std::path::PathBuf {
    // CARGO_BIN_EXE_logbook-cli is set by Cargo when running `cargo test`
    // For IDE/test runner compatibility, fall back to a manual path.
    std::env::var("CARGO_BIN_EXE_logbook-cli")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                .unwrap_or_else(|_| "src-tauri".to_string());
            Path::new(&manifest_dir)
                .join("target/debug/logbook-cli")
        })
}

/// Create a minimal fixture with dimensions.template.yaml + version.txt.
/// A usable data directory now requires a version.txt matching the CLI's
/// CURRENT_DATA_VERSION; non-migrate commands refuse to run otherwise.
fn setup_fixture(tmp: &Path) {
    fs::create_dir_all(tmp).unwrap();
    let config = "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:role:goals\n  - name: Role\n    key: role\n    source: commitments:role\n";
    fs::write(tmp.join("dimensions.template.yaml"), config).unwrap();
    fs::write(
        tmp.join("version.txt"),
        tauri_app_lib::models::CURRENT_DATA_VERSION.to_string(),
    )
    .unwrap();
}

/// Create a commitments.yaml with given YAML body (commitments array, pure YAML).
fn setup_commitments(tmp: &Path, year: i32, month: u32, body: &str) {
    let dir = tmp.join(year.to_string()).join(format!("{:02}", month));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("commitments.yaml"), body).unwrap();
}

/// Create a day file with given entries YAML body.
fn setup_day_file(tmp: &Path, date: &str, body: &str) {
    let parts: Vec<&str> = date.split('-').collect();
    let dir = tmp.join(parts[0]).join(parts[1]);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(format!("{}.yaml", date)), body.as_bytes()).unwrap();
}

use std::sync::atomic::{AtomicU32, Ordering};

static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

fn test_log_dir() -> std::path::PathBuf {
    let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("logbook_cli_test_{}_{}", std::process::id(), n))
}

fn run(args: &[&str]) -> std::process::Output {
    let log_dir = test_log_dir();
    let _ = std::fs::create_dir_all(&log_dir);
    Command::new(cli_binary())
        .args(args)
        .env("LOGBOOK_LOG_DIR", &log_dir)
        .env("LOGBOOK_LOCK_DIR", &log_dir)
        .output()
        .expect("Failed to execute CLI binary")
}

fn run_with_stdin(args: &[&str], stdin: &str) -> std::process::Output {
    use std::process::Stdio;
    let log_dir = test_log_dir();
    let _ = std::fs::create_dir_all(&log_dir);
    let mut child = Command::new(cli_binary())
        .args(args)
        .env("LOGBOOK_LOG_DIR", &log_dir)
        .env("LOGBOOK_LOCK_DIR", &log_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn CLI binary");

    use std::io::Write;
    if let Some(ref mut stdin_pipe) = child.stdin {
        stdin_pipe.write_all(stdin.as_bytes()).unwrap();
    }

    child.wait_with_output().expect("Failed to wait on CLI binary")
}

/// Run with LOGBOOK_LOG_DIR and LOGBOOK_LOCK_DIR pointed at a temp dir so the
/// test exercises real logging and lock isolation without touching the
/// developer's actual product dirs.
fn run_with_log_dir(args: &[&str], log_dir: &Path) -> std::process::Output {
    Command::new(cli_binary())
        .args(args)
        .env("LOGBOOK_LOG_DIR", log_dir)
        .env("LOGBOOK_LOCK_DIR", log_dir)
        .output()
        .expect("Failed to execute CLI binary")
}

// ---- Tests ----

#[test]
fn test_cli_writes_log_file() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_logging");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    let log_dir = std::env::temp_dir().join("logbook_cli_test_logdir");
    let _ = fs::remove_dir_all(&log_dir);
    fs::create_dir_all(&log_dir).unwrap();

    let output = run_with_log_dir(
        &[
            "--root-path", tmp.to_str().unwrap(),
            "--json",
            "commitments", "list", "--year", "2026", "--month", "6",
        ],
        &log_dir,
    );
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    // The CLI must now leave a persistent diagnostic trail (it left none before).
    let log_path = log_dir.join("logbook.log");
    assert!(log_path.exists(), "CLI did not create logbook.log");
    let content = fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("START"), "missing START marker: {}", content);
    assert!(content.contains("invoked"), "missing invocation trace: {}", content);

    let _ = fs::remove_dir_all(&tmp);
    let _ = fs::remove_dir_all(&log_dir);
}

#[test]
fn test_help() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("logbook-cli"));
    assert!(stdout.contains("commitments"));
    assert!(stdout.contains("entries"));
}

#[test]
fn test_missing_required_args() {
    let output = run(&["commitments"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("help") || stderr.contains("error") || stderr.contains("SUBCOMMAND"));
}

#[test]
fn test_no_root_path_errors() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_no_root");
    let _ = fs::remove_dir_all(&tmp);

    let output = run(&["--root-path", tmp.to_str().unwrap(), "commitments", "list", "--year", "2026", "--month", "6"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error") || stderr.contains("error") || stderr.contains("exist"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_commitments_list_empty() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_list_empty");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "commitments", "list", "--year", "2026", "--month", "6",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[]")); // empty commitments array

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_commitments_list_with_data() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_list");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_commitments(&tmp, 2026, 6, "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it");

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "commitments", "list", "--year", "2026", "--month", "6",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dev"));
    assert!(stdout.contains("40"));
    assert!(stdout.contains("Ship it"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_commitments_list_human() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_list_human");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_commitments(&tmp, 2026, 6, "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it");

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "commitments", "list", "--year", "2026", "--month", "6",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dev"));
    assert!(stdout.contains("40h/month"));
    assert!(stdout.contains("Ship it"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_commitments_progress() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_progress");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_commitments(&tmp, 2026, 6, "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it");

    // Add a day entry with matching goal
    setup_day_file(
        &tmp,
        "2026-06-01",
        "entries:\n  - id: e1\n    item: Code\n    duration: 120\n    dimensions:\n      goal: Ship it",
    );

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "commitments", "progress", "--year", "2026", "--month", "6",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dev"));
    assert!(stdout.contains("2400")); // allocation_minutes = 40 * 60
    assert!(stdout.contains("120"));  // spent_minutes
    assert!(stdout.contains("Ship it"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_commitments_set_valid() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_set");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = "- role: Dev\n  allocation: 20\n  goals:\n    - Refactor";

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "commitments", "set", "--year", "2026", "--month", "7",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Verify the file was written
    let commitments_path = tmp.join("2026").join("07").join("commitments.yaml");
    assert!(commitments_path.exists());
    let content = fs::read_to_string(&commitments_path).unwrap();
    assert!(content.contains("Dev"));
    assert!(content.contains("20"));
    assert!(content.contains("Refactor"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_commitments_set_roundtrip() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_roundtrip");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = "- role: Dev\n  allocation: 40\n  goals:\n    - Feature A\n    - Bug fixes\n- role: PM\n  allocation: 10\n  goals:\n    - Planning";

    // Write via set
    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "commitments", "set", "--year", "2026", "--month", "8",
    ], input);
    assert!(output.status.success(), "set failed: {}", String::from_utf8_lossy(&output.stderr));

    // Read back via list
    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "commitments", "list", "--year", "2026", "--month", "8",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dev"));
    assert!(stdout.contains("40"));
    assert!(stdout.contains("PM"));
    assert!(stdout.contains("10"));
    assert!(stdout.contains("Feature A"));
    assert!(stdout.contains("Bug fixes"));
    assert!(stdout.contains("Planning"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_commitments_set_preserves_dimensions_block() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_setdims");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    // Instantiated month: dimensions.yaml carries a dimensions snapshot.
    let dir = tmp.join("2026").join("08");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("dimensions.yaml"),
        "- name: Biz\n  key: biz\n  source: static\n  values: [Product]\n- name: Goal\n  key: goal\n  source: commitments:role:goals\n- name: Role\n  key: role\n  source: commitments:role\n",
    ).unwrap();
    // Old commitments in commitments.yaml (to test rename detection doesn't wipe dimensions).
    setup_commitments(&tmp, 2026, 8, "- role: Dev\n  allocation: 40\n  goals:\n    - Feature A");

    // set with the documented commitments-only input.
    let input = "- role: PM\n  allocation: 10\n  goals:\n    - Planning";
    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "commitments", "set", "--year", "2026", "--month", "8",
    ], input);
    assert!(output.status.success(), "set failed: {}", String::from_utf8_lossy(&output.stderr));

    // The dimensions.yaml must survive; commitments.yaml has new commitments.
    let dims_content = fs::read_to_string(tmp.join("2026").join("08").join("dimensions.yaml")).unwrap();
    assert!(dims_content.contains("biz"), "dimensions.yaml was wiped: {}", dims_content);

    let comm_content = fs::read_to_string(tmp.join("2026").join("08").join("commitments.yaml")).unwrap();
    assert!(comm_content.contains("PM"));
    assert!(!comm_content.contains("Feature A"), "old commitments should be replaced: {}", comm_content);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_commitments_set_validation_error_no_write() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_val_err");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    // Missing role
    let input = "- role: ''\n  allocation: 10\n  goals: []";

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "commitments", "set", "--year", "2026", "--month", "9",
    ], input);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Role name cannot be empty") || stderr.contains("Role"),
        "unexpected error: {}",
        stderr
    );

    // Verify file was NOT written
    let commitments_path = tmp.join("2026").join("09").join("commitments.yaml");
    assert!(!commitments_path.exists());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_commitments_set_json_input() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_json");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"[{"role":"Dev","allocation":30,"goals":["API work"]}]"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "commitments", "set", "--year", "2026", "--month", "10",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let commitments_path = tmp.join("2026").join("10").join("commitments.yaml");
    assert!(commitments_path.exists());
    let content = fs::read_to_string(&commitments_path).unwrap();
    assert!(content.contains("Dev"));
    assert!(content.contains("30"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_list() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_day_file(
        &tmp,
        "2026-06-15",
        "note: Test note\nentries:\n  - id: e1\n    item: Code review\n    duration: 45\n    dimensions:\n      goal: Review",
    );

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "list", "--date", "2026-06-15",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Code review"));
    assert!(stdout.contains("45"));
    assert!(stdout.contains("Test note"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_list_human() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_human");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_day_file(
        &tmp,
        "2026-06-15",
        "entries:\n  - id: e1\n    item: Code\n    duration: 30\n    dimensions:\n      goal: Dev",
    );

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "list", "--date", "2026-06-15",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("2026-06-15"));
    assert!(stdout.contains("Code"));
    assert!(stdout.contains("30m"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_dimensions_list() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_dims_list");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "dimensions", "list", "--year", "2026", "--month", "6",
    ]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"Goal\""), "stdout: {}", stdout);
    assert!(stdout.contains("\"Role\""), "stdout: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_root_path_flag_priority() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_root_flag");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_commitments(&tmp, 2026, 6, "- role: Dev\n  allocation: 10\n  goals:\n    - Test");

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "commitments", "list", "--year", "2026", "--month", "6",
    ]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dev"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_add_valid_with_dimensions() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"Code review","duration":"30m","dimensions":{"role":"Dev","goal":"Review"}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Code review"), "stdout: {}", stdout);
    assert!(stdout.contains("30m"), "stdout: {}", stdout);

    // Verify day file was written
    let day_path = tmp.join("2026").join("06").join("2026-06-15.yaml");
    assert!(day_path.exists(), "day file not created");
    let content = fs::read_to_string(&day_path).unwrap();
    assert!(content.contains("Code review"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_add_minimal() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_min");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"Coffee break","duration":"15m","dimensions":{"role":"Developer"}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Coffee break"), "stdout: {}", stdout);
    assert!(stdout.contains("15m"), "stdout: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_add_bad_json() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_bad_json");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], "not json");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to parse"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_add_bad_date() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_bad_date");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"x","duration":"10m"}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "not-a-date",
    ], input);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid date format") || stderr.contains("Error"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_add_empty_stdin() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_empty");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], "");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to parse"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_add_json_output() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_json");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"Write docs","duration":"60m","dimensions":{"role":"Dev"}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "add", "--date", "2026-06-15",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"id\""), "stdout: {}", stdout);
    assert!(stdout.contains("\"Write docs\""), "stdout: {}", stdout);
    assert!(stdout.contains("\"duration\""), "stdout: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_add_rejects_empty_item() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_empty_item");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"","duration":"1h","dimensions":{}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], input);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Entry item cannot be empty"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_add_rejects_unknown_dim_key() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_add_unknown_key");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"item":"Work","duration":"1h","dimensions":{"nonexistent":"x"}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "add", "--date", "2026-06-15",
    ], input);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown dimension key"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_dimensions_set_rejects_empty_value_string() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_dims_set_empty_val");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"[{"name":"X","key":"x","source":"static","values":["a",""]}]"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "dimensions", "set", "--year", "2026", "--month", "6",
    ], input);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ValuesEmpty"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_migrate_converts_md_to_yaml() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_migrate_md2yaml");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let day_dir = tmp.join("2026/06");
    fs::create_dir_all(&day_dir).unwrap();
    fs::write(
        day_dir.join("2026-06-20.md"),
        "note: migrate test\nentries:\n  - id: m1\n    item: Migrated\n    duration: 10\n    dimensions: {}\n",
    )
    .unwrap();

    let output = run(&["--root-path", tmp.to_str().unwrap(), "migrate"]);
    assert!(output.status.success());
    assert!(!day_dir.join("2026-06-20.md").exists());
    assert!(day_dir.join("2026-06-20.yaml").exists());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_update_changes_item_and_dimensions() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_update");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_commitments(&tmp, 2026, 6, "- role: Dev\n  allocation: 40\n  goals:\n    - Ship it\n    - Review");
    // The integrity check requires a valid UUID v4 for entry ids.
    let entry_id = "a505d5b7-4475-42f3-bafe-55e7df2cec0c";
    setup_day_file(
        &tmp,
        "2026-06-15",
        &format!(
            "entries:\n  - id: {}\n    item: Old item\n    duration: 30\n    dimensions:\n      role: Dev\n      goal: Review",
            entry_id
        ),
    );

    // Update: change item text + switch goal from Review to Ship it
    let input = r#"{"item":"Updated item","dimensions":{"role":"Dev","goal":"Ship it"}}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "update", "--date", "2026-06-15", "--entry-id", entry_id,
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated item"), "stdout: {}", stdout);
    assert!(stdout.contains("Ship it"), "stdout: {}", stdout);
    assert!(!stdout.contains("Review"), "old goal should be replaced: {}", stdout);

    // Verify the change persisted via entries list
    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "list", "--date", "2026-06-15",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated item"), "update did not persist: {}", stdout);
    assert!(stdout.contains("Ship it"), "dimension update did not persist: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_update_bad_json() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_update_bad_json");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_day_file(
        &tmp,
        "2026-06-15",
        "entries:\n  - id: e1\n    item: Keep\n    duration: 30\n    dimensions: {}",
    );

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "update", "--date", "2026-06-15", "--entry-id", "e1",
    ], "not json");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to parse"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_delete_removes_entry() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_delete");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    // delete_entry does not instantiate the month; create dimensions.yaml so the
    // pre-write integrity check has the goal dimension key available.
    let dim_dir = tmp.join("2026").join("06");
    fs::create_dir_all(&dim_dir).unwrap();
    fs::write(
        dim_dir.join("dimensions.yaml"),
        "- name: Goal\n  key: goal\n  source: commitments:role:goals\n",
    )
    .unwrap();
    // Integrity check requires UUID v4 entry ids (brief's "e1"/"e2" would be rejected).
    let keep_id = "b505d5b7-4475-42f3-bafe-55e7df2cec0c";
    let delete_id = "a505d5b7-4475-42f3-bafe-55e7df2cec0c";
    setup_day_file(
        &tmp,
        "2026-06-15",
        &format!(
            "entries:\n  - id: {del}\n    item: Delete me\n    duration: 30\n    dimensions:\n      goal: Review\n  - id: {keep}\n    item: Keep me\n    duration: 20\n    dimensions:\n      goal: Review",
            del = delete_id,
            keep = keep_id
        ),
    );

    // Delete delete_id (JSON output)
    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "delete", "--date", "2026-06-15", "--entry-id", delete_id,
    ]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"ok\": true"), "stdout: {}", stdout);
    assert!(stdout.contains(delete_id), "stdout should echo entry_id: {}", stdout);
    assert!(stdout.contains("2026-06-15"), "stdout should echo date: {}", stdout);

    // Verify the deleted entry is gone and the other remains via entries list
    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "--json",
        "entries", "list", "--date", "2026-06-15",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Delete me"), "deleted entry still present: {}", stdout);
    assert!(stdout.contains("Keep me"), "remaining entry missing: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_entries_delete_human_output() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_entries_delete_human");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    let entry_id = "c505d5b7-4475-42f3-bafe-55e7df2cec0c";
    setup_day_file(
        &tmp,
        "2026-06-15",
        &format!(
            "entries:\n  - id: {id}\n    item: Temp\n    duration: 10\n    dimensions: {{}}",
            id = entry_id
        ),
    );

    let output = run(&[
        "--root-path", tmp.to_str().unwrap(),
        "entries", "delete", "--date", "2026-06-15", "--entry-id", entry_id,
    ]);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("Deleted: {}", entry_id)), "stdout: {}", stdout);
    assert!(stdout.contains("2026-06-15"), "stdout: {}", stdout);

    let _ = fs::remove_dir_all(&tmp);
}

// ---- Writer-lock tests (cross-process write exclusion) ----

/// Spawn a live placeholder process and record its PID as the writer-lock
/// holder, simulating a running GUI (either bundle) or another CLI holding
/// `{lock_dir}/writer.lock` (LOGBOOK_LOCK_DIR overrides the lock directory).
#[cfg(unix)]
fn hold_writer_lock(lock_dir: &Path) -> std::process::Child {
    let child = Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("spawn sleep");
    fs::create_dir_all(lock_dir).unwrap();
    fs::write(lock_dir.join("writer.lock"), format!("{}\n", child.id())).unwrap();
    child
}

/// A write command must be refused while a live process holds the writer
/// lock — this is what stops a CLI write from racing a GUI of the *other*
/// bundle (dev/prod) on the same data root. Once the holder dies, the stale
/// lock is replaced and the write proceeds.
#[cfg(unix)]
#[test]
fn test_write_command_refused_while_writer_lock_held() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_writer_lock_block");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    // delete_entry does not instantiate the month; create dimensions.yaml so
    // the pre-write integrity check has the goal dimension key available.
    let dim_dir = tmp.join("2026").join("06");
    fs::create_dir_all(&dim_dir).unwrap();
    fs::write(
        dim_dir.join("dimensions.yaml"),
        "- name: Goal\n  key: goal\n  source: commitments:role:goals\n",
    )
    .unwrap();
    let entry_id = "d505d5b7-4475-42f3-bafe-55e7df2cec0c";
    setup_day_file(
        &tmp,
        "2026-06-15",
        &format!(
            "entries:\n  - id: {id}\n    item: Locked work\n    duration: 30\n    dimensions:\n      goal: Review",
            id = entry_id
        ),
    );

    let lock_dir = std::env::temp_dir().join("logbook_cli_test_writer_lock_dir");
    let _ = fs::remove_dir_all(&lock_dir);
    let mut holder = hold_writer_lock(&lock_dir);

    // Write refused while the lock is held by a live process.
    let output = run_with_log_dir(
        &[
            "--root-path", tmp.to_str().unwrap(),
            "entries", "delete", "--date", "2026-06-15", "--entry-id", entry_id,
        ],
        &lock_dir,
    );
    assert!(
        !output.status.success(),
        "write must be refused while writer.lock is held"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Close the GUI before using CLI write commands"),
        "stderr should keep the GUI-close guidance: {}",
        stderr
    );
    let day_content = fs::read_to_string(tmp.join("2026/06/2026-06-15.yaml")).unwrap();
    assert!(
        day_content.contains("Locked work"),
        "entry must survive the refused write"
    );

    // Holder dies → stale lock is replaced → write succeeds.
    holder.kill().unwrap();
    holder.wait().unwrap();
    let output = run_with_log_dir(
        &[
            "--root-path", tmp.to_str().unwrap(),
            "entries", "delete", "--date", "2026-06-15", "--entry-id", entry_id,
        ],
        &lock_dir,
    );
    assert!(
        output.status.success(),
        "stale lock must be replaced, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let day_content = fs::read_to_string(tmp.join("2026/06/2026-06-15.yaml")).unwrap();
    assert!(
        !day_content.contains("Locked work"),
        "entry should be deleted after the lock holder died"
    );

    let _ = fs::remove_dir_all(&tmp);
    let _ = fs::remove_dir_all(&lock_dir);
}

/// Read-only commands never touch the writer lock and must keep working
/// while another process holds it.
#[cfg(unix)]
#[test]
fn test_read_command_runs_while_writer_lock_held() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_writer_lock_read");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);
    setup_day_file(
        &tmp,
        "2026-06-15",
        "entries:\n  - id: e1\n    item: Readable\n    duration: 30\n    dimensions: {}",
    );

    let lock_dir = std::env::temp_dir().join("logbook_cli_test_writer_lock_read_dir");
    let _ = fs::remove_dir_all(&lock_dir);
    let mut holder = hold_writer_lock(&lock_dir);

    let output = run_with_log_dir(
        &[
            "--root-path", tmp.to_str().unwrap(),
            "entries", "list", "--date", "2026-06-15",
        ],
        &lock_dir,
    );
    assert!(
        output.status.success(),
        "read must work while writer.lock is held, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Readable"), "stdout: {}", stdout);

    holder.kill().unwrap();
    holder.wait().unwrap();
    let _ = fs::remove_dir_all(&tmp);
    let _ = fs::remove_dir_all(&lock_dir);
}
