pub mod commitments;
pub mod dimensions;
mod entries;
pub mod install;
pub mod migrate;
pub mod output;
pub mod root_path;

use clap::{Parser, Subcommand};
use dimensions::{DimensionsCommands, handle_dimensions};
use root_path::resolve_root_path;

use crate::single_instance::{InstanceLock, InstanceLockError};

/// Where the CLI writes logbook.log. Honours `LOGBOOK_LOG_DIR` (used by tests to
/// avoid polluting the real product log), else the shared GUI app-data dir.
fn log_dir() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("LOGBOOK_LOG_DIR") {
        return Some(std::path::PathBuf::from(dir));
    }
    root_path::app_data_dir()
}

/// InstanceLock directory. Always the shared GUI app-data dir, regardless of
/// LOGBOOK_LOG_DIR — the lock must protect the same data directory the GUI uses.
/// Overridable via LOGBOOK_LOCK_DIR for test isolation.
fn lock_dir() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("LOGBOOK_LOCK_DIR") {
        return Some(std::path::PathBuf::from(dir));
    }
    root_path::app_data_dir()
}

#[derive(Parser)]
#[command(name = "logbook-cli", version, about = "Logbook CLI — read/write time tracking data")]
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
    /// List or add entries
    Entries {
        #[command(subcommand)]
        action: EntryAction,
    },
    /// List or set dimensions for a month or the template
    #[command(subcommand)]
    Dimensions(DimensionsCommands),
    /// Migrate day files from .md (frontmatter) to .yaml format
    Migrate,
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

#[derive(Subcommand)]
pub enum EntryAction {
    /// List entries for a date
    List {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
    },
    /// Add an entry (read JSON from stdin)
    Add {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
    },
    /// Update an entry (read JSON from stdin)
    Update {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
        /// Entry ID to update
        #[arg(long)]
        entry_id: String,
    },
    /// Delete an entry
    Delete {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
        /// Entry ID to delete
        #[arg(long)]
        entry_id: String,
    },
}

impl Commands {
    /// Whether this command only reads data and never writes.
    ///
    /// Read-only commands skip the instance lock so they can run while the
    /// GUI is open. Write commands acquire the lock to prevent cross-process
    /// read-modify-write races that would silently lose data.
    fn is_read_only(&self) -> bool {
        match self {
            Self::Commitments { action } => {
                matches!(action, CommitmentAction::List { .. } | CommitmentAction::Progress { .. })
            }
            Self::Entries { action } => matches!(action, EntryAction::List { .. }),
            Self::Dimensions(cmd) => matches!(cmd, DimensionsCommands::List { .. }),
            Self::Migrate => false,
        }
    }
}

/// Check the data directory's version.txt against `CURRENT_DATA_VERSION`.
///
/// `migrate` is exempt: it may legitimately read an older version and write a
/// newer one. All other commands refuse on missing/mismatched version so an
/// outdated CLI can't silently corrupt a newer-format data directory.
///
/// Returns `Err(msg)` with a human-readable message; the caller should print it
/// and exit(1). Extracted from `run()` (which calls `process::exit`) so the
/// exemption + message formatting stays unit-testable.
fn ensure_compatible_version(root: &std::path::Path, command: &Commands) -> Result<(), String> {
    if matches!(command, Commands::Migrate) {
        return Ok(());
    }

    use crate::commands::check_data_version;
    use crate::models::{CURRENT_DATA_VERSION, InitResult};

    match check_data_version(root, CURRENT_DATA_VERSION) {
        Ok(()) => Ok(()),
        Err(InitResult::DataVersionNotFound { root_path }) => Err(format!(
            "Data version file not found in {}.\n\
             Run `logbook-cli migrate` to initialize the data version, \
             or start the Logbook GUI once to set up.",
            root_path
        )),
        Err(InitResult::DataVersionMismatch {
            root_path,
            expected,
            found,
        }) => Err(format!(
            "Data format version mismatch in {}.\n\
             Expected version {}, found version {}.\n\
             This CLI (version {}) expects data version {}. \
             Update Logbook and retry, or run `logbook-cli migrate` with a newer build.",
            root_path, expected, found,
            env!("CARGO_PKG_VERSION"),
            expected
        )),
        // check_data_version only ever returns the two variants above.
        Err(other) => unreachable!("check_data_version returned unexpected variant: {other:?}"),
    }
}

pub fn run() {
    // Diagnostics: a panic backtrace lands on disk, and the shared command
    // functions' log_command_enter/exit calls become live (they no-op until
    // error_log is initialized). Without this the CLI mutated data with zero
    // persistent diagnostic trail.
    crate::error_log::install_panic_hook();
    if let Some(dir) = log_dir() {
        crate::error_log::init(&dir);
    }

    let cli = Cli::parse();
    crate::error_log::log_info("cli", &format!("invoked: {:?}", std::env::args().collect::<Vec<_>>()));

    // Prevent concurrent writes: if the GUI is running, refuse CLI write
    // commands to avoid cross-process read-modify-write races that would
    // silently lose data. Read-only commands skip the lock so they can run
    // alongside the GUI.
    let _lock = if cli.command.is_read_only() {
        None
    } else if let Some(lock_dir) = lock_dir() {
        match InstanceLock::try_acquire(&lock_dir) {
            Ok(guard) => Some(guard),
            Err(e) => {
                match e {
                    InstanceLockError::AlreadyRunning(pid) => {
                        eprintln!(
                            "Error: Logbook GUI is already running (PID {}).\n\
                             Close the GUI before using CLI write commands.",
                            pid
                        );
                    }
                    InstanceLockError::Io(io_err) => {
                        eprintln!(
                            "Error: Failed to acquire instance lock: {}. Check permissions on {}.",
                            io_err, lock_dir.display()
                        );
                    }
                }
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let root = resolve_root_path(cli.root_path).unwrap_or_else(|| {
        crate::error_log::log_error("cli", "could not determine data root path");
        eprintln!(
            "Error: Could not determine data root path.\n\
             Use --root-path to specify, or start the Logbook GUI once to initialize."
        );
        std::process::exit(1);
    });

    eprintln!("Using data root: {}", root.display());

    // Refuse to operate on an incompatible-version data directory. Without
    // this, an outdated CLI would silently read/write the wrong format and
    // corrupt data (version check otherwise lives only in the GUI's `init`).
    // `migrate` is exempt inside ensure_compatible_version.
    if let Err(msg) = ensure_compatible_version(&root, &cli.command) {
        crate::error_log::log_error("cli", &msg);
        eprintln!("Error: {}", msg);
        std::process::exit(1);
    }

    match cli.command {
        Commands::Commitments { action } => match action {
            CommitmentAction::List { year, month } => {
                commitments::list(&root, year, month, cli.json);
            }
            CommitmentAction::Progress { year, month } => {
                commitments::progress(&root, year, month, cli.json);
            }
            CommitmentAction::Set { year, month } => {
                commitments::set(&root, year, month, cli.json);
            }
        },
        Commands::Entries { action } => match action {
            EntryAction::List { date } => {
                entries::list(&root, &date, cli.json);
            }
            EntryAction::Add { date } => {
                entries::add(&root, &date, cli.json);
            }
            EntryAction::Update { date, entry_id } => {
                entries::update(&root, &date, &entry_id, cli.json);
            }
            EntryAction::Delete { date, entry_id } => {
                entries::delete(&root, &date, &entry_id, cli.json);
            }
        },
        Commands::Dimensions(cmd) => {
            if let Err(e) = handle_dimensions(cmd, &root) {
                output::print_error(&e);
                std::process::exit(1);
            }
        }
        Commands::Migrate => {
            if let Err(e) = migrate::run(&root) {
                output::print_error(&e);
                std::process::exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_commands() {
        // 只读命令 → true
        assert!(Commands::Commitments {
            action: CommitmentAction::List { year: 2026, month: 7 },
        }
        .is_read_only());
        assert!(Commands::Commitments {
            action: CommitmentAction::Progress { year: 2026, month: 7 },
        }
        .is_read_only());
        assert!(Commands::Entries {
            action: EntryAction::List { date: "2026-07-11".to_string() },
        }
        .is_read_only());
        assert!(Commands::Dimensions(DimensionsCommands::List {
            year: Some(2026),
            month: Some(7),
            template: false,
            json: false,
        })
        .is_read_only());
        // dimensions list --template 也只读
        assert!(Commands::Dimensions(DimensionsCommands::List {
            year: None,
            month: None,
            template: true,
            json: false,
        })
        .is_read_only());
    }

    #[test]
    fn test_write_commands() {
        // 写命令 → false
        assert!(!Commands::Commitments {
            action: CommitmentAction::Set { year: 2026, month: 7 },
        }
        .is_read_only());
        assert!(!Commands::Entries {
            action: EntryAction::Add { date: "2026-07-11".to_string() },
        }
        .is_read_only());
        assert!(!Commands::Dimensions(DimensionsCommands::Set {
            year: Some(2026),
            month: Some(7),
            template: false,
            json: false,
        })
        .is_read_only());
        assert!(!Commands::Migrate.is_read_only());
        assert!(!Commands::Entries {
            action: EntryAction::Update {
                date: "2026-07-11".to_string(),
                entry_id: "test-id".to_string(),
            },
        }
        .is_read_only());
        assert!(!Commands::Entries {
            action: EntryAction::Delete {
                date: "2026-07-11".to_string(),
                entry_id: "test-id".to_string(),
            },
        }
        .is_read_only());
    }

    // --- ensure_compatible_version ---

    fn temp_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "logbook_cli_version_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ))
    }

    fn list_entries_cmd() -> Commands {
        Commands::Entries {
            action: EntryAction::List {
                date: "2026-07-12".to_string(),
            },
        }
    }

    #[test]
    fn ensure_compatible_version_migrate_is_exempt_even_when_version_missing() {
        // migrate must run regardless of version.txt state — it may be the very
        // tool that creates/bumps the version file.
        let root = temp_root();
        std::fs::create_dir_all(&root).unwrap();
        // No version.txt at all.
        let result = ensure_compatible_version(&root, &Commands::Migrate);
        assert!(result.is_ok(), "migrate should be exempt, got {:?}", result);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ensure_compatible_version_migrate_is_exempt_on_version_mismatch() {
        let root = temp_root();
        std::fs::create_dir_all(&root).unwrap();
        crate::files::write_version_file(&root, 999).unwrap();
        let result = ensure_compatible_version(&root, &Commands::Migrate);
        assert!(result.is_ok(), "migrate should be exempt, got {:?}", result);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ensure_compatible_version_ok_when_version_matches() {
        let root = temp_root();
        std::fs::create_dir_all(&root).unwrap();
        crate::files::write_version_file(&root, crate::models::CURRENT_DATA_VERSION).unwrap();
        let result = ensure_compatible_version(&root, &list_entries_cmd());
        assert!(result.is_ok(), "got {:?}", result);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ensure_compatible_version_err_on_mismatch() {
        let root = temp_root();
        std::fs::create_dir_all(&root).unwrap();
        crate::files::write_version_file(&root, 999).unwrap();
        let result = ensure_compatible_version(&root, &list_entries_cmd());
        let msg = result.expect_err("should refuse on mismatch");
        assert!(
            msg.contains("mismatch"),
            "message should mention mismatch: {msg}"
        );
        assert!(
            msg.contains(&crate::models::CURRENT_DATA_VERSION.to_string()),
            "message should mention expected version: {msg}"
        );
        assert!(msg.contains("999"), "message should mention found version: {msg}");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ensure_compatible_version_err_when_version_file_missing() {
        let root = temp_root();
        std::fs::create_dir_all(&root).unwrap();
        // No version.txt.
        let result = ensure_compatible_version(&root, &list_entries_cmd());
        let msg = result.expect_err("should refuse when version.txt is missing");
        assert!(
            msg.to_lowercase().contains("not found"),
            "message should mention not found: {msg}"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
