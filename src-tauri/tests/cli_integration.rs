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

/// Create a minimal fixture with dimensions.template.yaml.
fn setup_fixture(tmp: &Path) {
    fs::create_dir_all(tmp).unwrap();
    let config = "dimensions:\n  - name: Goal\n    key: goal\n    source: commitments:goals\n";
    fs::write(tmp.join("dimensions.template.yaml"), config).unwrap();
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
    let content = format!("---\n{}\n---\n", body);
    fs::write(dir.join(format!("{}.md", date)), &content).unwrap();
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(cli_binary())
        .args(args)
        .output()
        .expect("Failed to execute CLI binary")
}

fn run_with_stdin(args: &[&str], stdin: &str) -> std::process::Output {
    use std::process::Stdio;
    let mut child = Command::new(cli_binary())
        .args(args)
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

/// Run with LOGBOOK_LOG_DIR pointed at a temp dir so the test exercises real
/// logging without writing to the developer's actual ~/Library product log.
fn run_with_log_dir(args: &[&str], log_dir: &Path) -> std::process::Output {
    Command::new(cli_binary())
        .args(args)
        .env("LOGBOOK_LOG_DIR", log_dir)
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
        "- name: Biz\n  key: biz\n  source: static\n  values: [Product]\n",
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
        "entries", "--date", "2026-06-15",
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
        "entries", "--date", "2026-06-15",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("2026-06-15"));
    assert!(stdout.contains("Code"));
    assert!(stdout.contains("30m"));

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
