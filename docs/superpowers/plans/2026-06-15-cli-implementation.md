# CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a standalone CLI binary `logbook-cli` to read commitments/entries and write commitments to `_monthly.md`.

**Architecture:** New `[[bin]]` target in the existing Cargo package, sharing `tauri_app_lib` library code. CLI binary is a thin wrapper: clap for argument parsing, calls into existing `files`/`commands` modules for all IO. One new function `write_monthly_file` added to `files.rs`.

**Tech Stack:** clap 4 (derive), yaml_serde, serde_json. No new deps beyond clap.

---

## File Structure

```
src-tauri/
├── Cargo.toml                          # +[[bin]], +clap dep
├── src/
│   ├── bin/
│   │   └── logbook-cli.rs             # CREATE: entry point
│   ├── cli/
│   │   ├── mod.rs                      # CREATE: route subcommands
│   │   ├── root_path.rs               # CREATE: resolve root_path
│   │   ├── commitments.rs             # CREATE: list, progress, set
│   │   ├── entries.rs                 # CREATE: list
│   │   └── output.rs                  # CREATE: human / JSON output
│   └── files.rs                       # MODIFY: +write_monthly_file
└── tests/
    └── cli_integration.rs             # CREATE: integration tests
```

---

### Task 1: Add `[[bin]]` target and clap dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add `[[bin]]` section and clap dependency to Cargo.toml**

```toml
[[bin]]
name = "logbook-cli"
path = "src/bin/logbook-cli.rs"

[dependencies]
# ... existing deps ...
clap = { version = "4", features = ["derive"] }
```

Actual edit: after existing `[dependencies]` block, add the `[[bin]]` section. Add `clap` to the `[dependencies]` section.

- [ ] **Step 2: Verify build resolves new binary**

Run: `cd src-tauri && cargo check --bin logbook-cli`
Expected: error about missing `src/bin/logbook-cli.rs`, but Cargo.toml parses correctly.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: add [[bin]] logbook-cli target and clap dependency"
```

---

### Task 2: Create `output.rs` — unified human / JSON output

**Files:**
- Create: `src-tauri/src/cli/output.rs`

- [ ] **Step 1: Write `output.rs`**

```rust
use serde::Serialize;

/// Print `data` as either JSON (pretty) or a human-readable string.
/// If `json` is true, print JSON. Otherwise, print `human` string.
pub fn print_output<T: Serialize>(json: bool, data: &T, human: &str) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(data).expect("Failed to serialize output")
        );
    } else {
        println!("{}", human);
    }
}

/// Print error to stderr.
pub fn print_error(msg: &str) {
    eprintln!("Error: {}", msg);
}
```

- [ ] **Step 2: Verify compiles**

Run: `cd src-tauri && cargo check --bin logbook-cli 2>&1 | head -5`
Expected: errors about missing `mod cli`, `mod.rs`, etc. — but `output.rs` itself should not have compilation errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/cli/output.rs
git commit -m "feat(cli): add output module — human / JSON"
```

---

### Task 3: Create `root_path.rs` — resolve root_path

**Files:**
- Create: `src-tauri/src/cli/root_path.rs`

- [ ] **Step 1: Write `root_path.rs`**

```rust
use std::path::PathBuf;

/// Resolve root_path from --root-path flag or GUI's persisted root_path.txt.
///
/// Priority:
/// 1. `flag` (--root-path / -r)
/// 2. `root_path.txt` in macOS app local data dir
/// 3. None → caller prints error and exits
///
/// macOS app data dir: ~/Library/Application Support/com.logbook/
///
/// The bundle ID is determined by the Tauri config; typical default is
/// `com.tauri.dev` in dev. We check a few common names.
pub fn resolve_root_path(flag: Option<String>) -> Option<PathBuf> {
    if let Some(ref p) = flag {
        let path = PathBuf::from(p);
        if path.exists() && path.is_dir() {
            return Some(path);
        }
        eprintln!(
            "Warning: --root-path '{}' does not exist or is not a directory",
            p
        );
        return None;
    }

    // Try common macOS app data dirs for root_path.txt
    let home = std::env::var("HOME").ok()?;
    let candidates = [
        "Library/Application Support/com.logbook/root_path.txt",
        "Library/Application Support/com.tauri.dev/root_path.txt",
    ];

    for candidate in &candidates {
        let p = PathBuf::from(&home).join(candidate);
        if p.exists() {
            if let Ok(content) = std::fs::read_to_string(&p) {
                let trimmed = content.trim();
                let path = PathBuf::from(trimmed);
                if path.exists() && path.is_dir() {
                    return Some(path);
                }
            }
        }
    }

    None
}
```

- [ ] **Step 2: Verify compiles**

Run: `cd src-tauri && cargo check --bin logbook-cli 2>&1 | head -5`
Expected: still missing mod.rs etc., but `root_path.rs` itself should be fine.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/cli/root_path.rs
git commit -m "feat(cli): add root_path resolution module"
```

---

### Task 4: Add `write_monthly_file` to `files.rs`

**Files:**
- Modify: `src-tauri/src/files.rs` — append new function after line 183 (after `read_monthly_file`)

- [ ] **Step 1: Write `write_monthly_file`**

Edit `src-tauri/src/files.rs` — add after the closing `}` of `read_monthly_file` (line 183):

```rust
/// Write a full monthly file (atomic: temp then rename).
pub fn write_monthly_file(
    root: &Path,
    year: i32,
    month: u32,
    monthly: &MonthlyFile,
) -> Result<(), String> {
    let path = monthly_path(root, year, month);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    let yaml_body =
        yaml_serde::to_string(monthly).map_err(|e| format!("Failed to serialize: {}", e))?;
    let content = format!("---\n{}---\n", yaml_body);
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, &content).map_err(|e| format!("Failed to write temp file: {}", e))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to rename temp file: {}", e))?;
    Ok(())
}
```

- [ ] **Step 2: Verify compiles**

Run: `cd src-tauri && cargo check --lib`
Expected: Compiles successfully (no errors from files.rs).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/files.rs
git commit -m "feat(files): add write_monthly_file for atomic _monthly.md writes"
```

---

### Task 5: Create `commitments.rs` — list, progress, set commands

**Files:**
- Create: `src-tauri/src/cli/commitments.rs`

- [ ] **Step 1: Write `commitments.rs`**

```rust
use crate::cli::output;
use tauri_app_lib::config;
use tauri_app_lib::files;
use tauri_app_lib::models::{Commitment, CommitmentProgress, MonthlyFile};
use std::io::Read;
use std::path::Path;

pub fn list(root: &Path, year: i32, month: u32, json: bool) {
    let monthly = files::read_monthly_file(root, year, month).unwrap_or_else(|e| {
        output::print_error(&format!("Failed to read monthly file: {}", e));
        std::process::exit(1);
    });

    output::print_output(
        json,
        &monthly.commitments,
        &format_commitments_human(&monthly.commitments),
    );
}

pub fn progress(root: &Path, year: i32, month: u32, json: bool) {
    let prog = tauri_app_lib::commands::get_commitment_progress(
        root.to_string_lossy().into_owned(),
        year,
        month,
    )
    .unwrap_or_else(|e| {
        output::print_error(&format!("Failed to get commitment progress: {}", e));
        std::process::exit(1);
    });

    output::print_output(
        json,
        &prog,
        &format_progress_human(&prog),
    );
}

pub fn set(root: &Path, year: i32, month: u32) {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| {
            output::print_error(&format!("Failed to read stdin: {}", e));
            std::process::exit(1);
        });

    // Try JSON first, then YAML
    let monthly: MonthlyFile =
        if let Ok(mf) = serde_json::from_str::<MonthlyFile>(&input) {
            mf
        } else if let Ok(mf) = yaml_serde::from_str::<MonthlyFile>(&input) {
            mf
        } else {
            output::print_error(
                "Failed to parse input as JSON or YAML MonthlyFile.\n\
                 Expected JSON: {\"commitments\":[{\"role\":\"...\",\"allocation\":N,\"goals\":[...]}]}\n\
                 Or YAML:\ncommitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Goal name",
            );
            std::process::exit(1);
        };

    // Validate
    let errors = config::validate_monthly(&monthly);
    if !errors.is_empty() {
        for e in &errors {
            output::print_error(&format!("[{}] {}", e.kind, e.message));
        }
        output::print_error("Validation failed — file not written.");
        std::process::exit(1);
    }

    // Write
    files::write_monthly_file(root, year, month, &monthly).unwrap_or_else(|e| {
        output::print_error(&format!("Failed to write monthly file: {}", e));
        std::process::exit(1);
    });

    output::print_output(false, &serde_json::json!({"ok": true}), "Commitments written successfully.");
}

fn format_commitments_human(commitments: &[Commitment]) -> String {
    if commitments.is_empty() {
        return "(no commitments)".to_string();
    }
    let mut out = String::new();
    for c in commitments {
        out.push_str(&format!("Role: {} ({}h/month)\n", c.role, c.allocation));
        for g in &c.goals {
            out.push_str(&format!("  - {}\n", g));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

fn format_progress_human(progress: &[CommitmentProgress]) -> String {
    if progress.is_empty() {
        return "(no commitments)".to_string();
    }
    let mut out = String::new();
    for c in progress {
        let pct = if c.allocation_minutes > 0 {
            (c.spent_minutes as f64 / c.allocation_minutes as f64) * 100.0
        } else {
            0.0
        };
        out.push_str(&format!(
            "Role: {} ({:.0}% — {:.1}h / {}h)\n",
            c.role,
            pct,
            c.spent_minutes as f64 / 60.0,
            c.allocation_minutes / 60
        ));
        for g in &c.goals {
            out.push_str(&format!(
                "  - {}: {:.1}h\n",
                g.name,
                g.spent_minutes as f64 / 60.0
            ));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}
```

- [ ] **Step 2: Verify compiles**

Run: `cd src-tauri && cargo check --lib`
Expected: compiles successfully (commitments.rs depends on `cli::output` and `cli` mod, which don't exist yet — but the file itself doesn't need to pass check until the mod is wired up in Task 7).

Actually, we can't compile this standalone — it depends on `crate::cli::output` and the cli module. Just verify the file is syntactically correct by running `cargo check --bin logbook-cli` after Task 7.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/cli/commitments.rs
git commit -m "feat(cli): add commitments subcommands — list, progress, set"
```

---

### Task 6: Create `entries.rs` — list command

**Files:**
- Create: `src-tauri/src/cli/entries.rs`

- [ ] **Step 1: Write `entries.rs`**

```rust
use crate::cli::output;
use tauri_app_lib::files;
use std::path::Path;

pub fn list(root: &Path, date: &str, json: bool) {
    let day_file = files::read_day_file(root, date).unwrap_or_else(|e| {
        output::print_error(&format!("Failed to read day file: {}", e));
        std::process::exit(1);
    });

    output::print_output(
        json,
        &day_file,
        &format_entries_human(&day_file, date),
    );
}

fn format_entries_human(day_file: &tauri_app_lib::models::DayFile, date: &str) -> String {
    if day_file.entries.is_empty() && day_file.note.is_none() {
        return format!("{}: (no entries)", date);
    }
    let mut out = format!("=== {} ===\n", date);
    if let Some(ref note) = day_file.note {
        out.push_str(&format!("Note: {}\n\n", note));
    }
    if day_file.entries.is_empty() {
        out.push_str("(no entries)\n");
        return out;
    }
    for e in &day_file.entries {
        let dims: Vec<String> = e
            .dimensions
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        out.push_str(&format!(
            "  {} | {}m | {}\n",
            e.item,
            e.duration,
            dims.join(", ")
        ));
    }
    let total: u32 = day_file.entries.iter().map(|e| e.duration).sum();
    out.push_str(&format!("  ---\n  Total: {}m ({:.1}h)\n", total, total as f64 / 60.0));
    out
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/cli/entries.rs
git commit -m "feat(cli): add entries list subcommand"
```

---

### Task 7: Create `mod.rs` and binary entry point — wire everything together

**Files:**
- Create: `src-tauri/src/cli/mod.rs`
- Create: `src-tauri/src/bin/logbook-cli.rs`

- [ ] **Step 1: Write `cli/mod.rs`**

```rust
pub mod commitments;
mod entries;
pub mod output;
pub mod root_path;

use clap::{Parser, Subcommand};
use root_path::resolve_root_path;

#[derive(Parser)]
#[command(name = "logbook-cli", about = "Logbook CLI — read/write time tracking data")]
pub struct Cli {
    /// Data root directory (default: read from GUI config)
    #[arg(short = 'r', long)]
    pub root_path: Option<String>,

    /// Output as JSON instead of human-readable text
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List, view progress, or set monthly commitments
    Commitments {
        #[command(subcommand)]
        action: CommitmentAction,
    },
    /// List entries for a date
    Entries {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
    },
}

#[derive(Subcommand)]
pub enum CommitmentAction {
    /// List commitments for a month
    List {
        #[arg(long)]
        year: i32,
        #[arg(long)]
        month: u32,
    },
    /// Show commitment progress (allocation vs spent) for a month
    Progress {
        #[arg(long)]
        year: i32,
        #[arg(long)]
        month: u32,
    },
    /// Set commitments for a month (read JSON/YAML from stdin)
    Set {
        #[arg(long)]
        year: i32,
        #[arg(long)]
        month: u32,
    },
}

pub fn run() {
    let cli = Cli::parse();
    let root = resolve_root_path(cli.root_path).unwrap_or_else(|| {
        eprintln!(
            "Error: Could not determine data root path.\n\
             Use --root-path to specify, or start the Logbook GUI once to initialize."
        );
        std::process::exit(1);
    });

    match cli.command {
        Commands::Commitments { action } => match action {
            CommitmentAction::List { year, month } => {
                commitments::list(&root, year, month, cli.json);
            }
            CommitmentAction::Progress { year, month } => {
                commitments::progress(&root, year, month, cli.json);
            }
            CommitmentAction::Set { year, month } => {
                commitments::set(&root, year, month);
            }
        },
        Commands::Entries { date } => {
            entries::list(&root, &date, cli.json);
        }
    }
}
```

- [ ] **Step 2: Write `bin/logbook-cli.rs`**

```rust
fn main() {
    tauri_app_lib::cli::run();
}
```

IMPORTANT: Also add `pub mod cli;` to `src-tauri/src/lib.rs` so the binary can access it via `tauri_app_lib::cli`.

Edit `src-tauri/src/lib.rs` — add `pub mod cli;` before the existing `pub mod commands;` line:

```rust
pub mod cli;
pub mod commands;
```

- [ ] **Step 3: Fix a circular reference issue**

The `commitments.rs` module uses `use crate::cli::output;` — but this is a library module (`tauri_app_lib::cli::commitments`), so inside the lib crate, it should reference `use crate::cli::output`. Verify that `lib.rs` declares `pub mod cli;` which makes `crate::cli` valid inside the library.

- [ ] **Step 4: Verify build**

Run: `cd src-tauri && cargo check --bin logbook-cli`
Expected: compiles successfully, no errors.

- [ ] **Step 5: Quick smoke test — build and run --help**

Run: `cd src-tauri && cargo build --bin logbook-cli && ./target/debug/logbook-cli --help`
Expected: help text with commands listed.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/cli/mod.rs src-tauri/src/bin/logbook-cli.rs src-tauri/src/lib.rs
git commit -m "feat(cli): wire up entry point, mod.rs, and lib.rs"
```

---

### Task 8: Integration tests

**Files:**
- Create: `src-tauri/tests/cli_integration.rs`

- [ ] **Step 1: Write integration test**

```rust
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

/// Create a minimal fixture with config.yaml.
fn setup_fixture(tmp: &Path) {
    fs::create_dir_all(tmp).unwrap();
    let config = "dimensions:\n  - name: Goal\n    key: goal\n    source: monthly\n";
    fs::write(tmp.join("config.yaml"), config).unwrap();
}

/// Create a _monthly.md with given commitments YAML body.
fn setup_monthly(tmp: &Path, year: i32, month: u32, body: &str) {
    let dir = tmp.join(year.to_string()).join(format!("{:02}", month));
    fs::create_dir_all(&dir).unwrap();
    let content = format!("---\n{}---\n", body);
    fs::write(dir.join("_monthly.md"), &content).unwrap();
}

/// Create a day file with given entries YAML body.
fn setup_day_file(tmp: &Path, date: &str, body: &str) {
    let parts: Vec<&str> = date.split('-').collect();
    let dir = tmp.join(parts[0]).join(parts[1]);
    fs::create_dir_all(&dir).unwrap();
    let content = format!("---\n{}---\n", body);
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

// ---- Tests ----

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
    setup_monthly(&tmp, 2026, 6, "commitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it");

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
    setup_monthly(&tmp, 2026, 6, "commitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it");

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
    setup_monthly(&tmp, 2026, 6, "commitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Ship it");

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

    let input = "commitments:\n  - role: Dev\n    allocation: 20\n    goals:\n      - Refactor";

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "commitments", "set", "--year", "2026", "--month", "7",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Verify the file was written
    let monthly_path = tmp.join("2026").join("07").join("_monthly.md");
    assert!(monthly_path.exists());
    let content = fs::read_to_string(&monthly_path).unwrap();
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

    let input = "commitments:\n  - role: Dev\n    allocation: 40\n    goals:\n      - Feature A\n      - Bug fixes\n  - role: PM\n    allocation: 10\n    goals:\n      - Planning";

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
fn test_commitments_set_validation_error_no_write() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_val_err");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    // Missing role
    let input = "commitments:\n  - role: ''\n    allocation: 10\n    goals: []";

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "commitments", "set", "--year", "2026", "--month", "9",
    ], input);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("MissingRole") || stderr.contains("role"));

    // Verify file was NOT written
    let monthly_path = tmp.join("2026").join("09").join("_monthly.md");
    assert!(!monthly_path.exists());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_commitments_set_json_input() {
    let tmp = std::env::temp_dir().join("logbook_cli_test_json");
    let _ = fs::remove_dir_all(&tmp);
    setup_fixture(&tmp);

    let input = r#"{"commitments":[{"role":"Dev","allocation":30,"goals":["API work"]}]}"#;

    let output = run_with_stdin(&[
        "--root-path", tmp.to_str().unwrap(),
        "commitments", "set", "--year", "2026", "--month", "10",
    ], input);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let monthly_path = tmp.join("2026").join("10").join("_monthly.md");
    assert!(monthly_path.exists());
    let content = fs::read_to_string(&monthly_path).unwrap();
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
    setup_monthly(&tmp, 2026, 6, "commitments:\n  - role: Dev\n    allocation: 10\n    goals:\n      - Test");

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
```

- [ ] **Step 2: Build binary then run tests**

Run: `cd src-tauri && cargo build --bin logbook-cli && cargo test --test cli_integration`
Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/cli_integration.rs
git commit -m "test(cli): add CLI integration tests"
```

---

### Task 9: Install and smoke test with real data

- [ ] **Step 1: Install the binary**

Run: `cd src-tauri && cargo install --path . --bin logbook-cli`
Expected: binary installed to `~/.cargo/bin/logbook-cli`.

- [ ] **Step 2: Verify against real data**

Run: `logbook-cli commitments list --year 2026 --month 6 --json`
Expected: outputs the real commitments JSON from the user's Logbook data.

- [ ] **Step 3: Commit any final fixes**

If any issues found, fix and commit.
