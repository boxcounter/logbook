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
    }
}
