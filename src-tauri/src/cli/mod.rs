pub mod commitments;
pub mod dimensions;
mod entries;
pub mod install;
pub mod output;
pub mod root_path;

use clap::{Parser, Subcommand};
use dimensions::{DimensionsCommands, handle_dimensions};
use root_path::resolve_root_path;

use crate::single_instance::InstanceLock;

/// Where the CLI writes logbook.log. Honours `LOGBOOK_LOG_DIR` (used by tests to
/// avoid polluting the real product log), else the shared GUI app-data dir.
fn log_dir() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("LOGBOOK_LOG_DIR") {
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
    /// List entries for a date
    Entries {
        /// Date in YYYY-MM-DD format
        #[arg(long)]
        date: String,
    },
    /// Get or set dimensions for a month or the template
    #[command(subcommand)]
    Dimensions(DimensionsCommands),
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

    // Prevent concurrent writes: if the GUI is running, refuse CLI writes to
    // avoid cross-process read-modify-write races that would silently lose data.
    let _lock = if let Some(lock_dir) = log_dir() {
        match InstanceLock::try_acquire(&lock_dir) {
            Ok(guard) => Some(guard),
            Err(pid) => {
                eprintln!(
                    "Error: Logbook GUI is already running (PID {}).\n\
                     Close the GUI before using CLI commands, or use LOGBOOK_LOG_DIR\n\
                     to point to a separate data directory for CLI-only use.",
                    pid
                );
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
        Commands::Entries { date } => {
            entries::list(&root, &date, cli.json);
        }
        Commands::Dimensions(cmd) => {
            if let Err(e) = handle_dimensions(cmd, &root) {
                output::print_error(&e);
                std::process::exit(1);
            }
        }
    }
}
