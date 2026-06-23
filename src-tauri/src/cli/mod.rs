pub mod commitments;
mod entries;
pub mod output;
pub mod root_path;

use clap::{Parser, Subcommand};
use root_path::resolve_root_path;

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
    }
}
