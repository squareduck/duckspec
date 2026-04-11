//! duckspec — command-line interface for the duckspec framework.
//!
//! Produces the `ds` binary.

use clap::{Parser, Subcommand};

mod cmd;

/// duckspec CLI — spec-driven development framework.
#[derive(Parser, Debug)]
#[command(name = "ds", version, about = "duckspec CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create duckspec/ directory structure, optionally install agent templates.
    Init {
        /// Harness to install templates for (claude, opencode).
        harness: Option<String>,
    },
    /// Print active changes, capability/codex counts.
    Status,
    /// Validate whole project: backlinks, test coverage, cross-artifact integrity.
    Audit,
    /// Validate artifacts against schemas.
    Check {
        /// File or directory to validate (default: duckspec/).
        path: Option<String>,
        /// Rewrite to canonical order before validating.
        #[arg(long)]
        format: bool,
    },
    /// Resolve @spec backlinks, update test markers.
    Sync {
        /// Show changes without writing.
        #[arg(long)]
        dry: bool,
    },
    /// Apply change to caps, move to archive.
    Archive {
        /// Name of the change to archive.
        name: String,
    },
    /// Preview what a change would introduce if archived now.
    Diff,
    /// Print artifact tree with summaries.
    Index {
        /// Show only capabilities.
        #[arg(long)]
        caps: bool,
        /// Show only codex entries.
        #[arg(long)]
        codex: bool,
        /// Show only project constitution.
        #[arg(long)]
        project: bool,
    },
    /// Print embedded agent command template.
    Template {
        /// Template name (e.g. ds-explore, ds-spec).
        name: String,
    },
    /// Print embedded schema description.
    Schema {
        /// Schema name.
        name: String,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init { harness } => cmd::init::run(harness),
        Command::Status => cmd::status::run(),
        Command::Audit => cmd::audit::run(),
        Command::Check { path, format } => cmd::check::run(path, format),
        Command::Sync { dry } => cmd::sync::run(dry),
        Command::Archive { name } => cmd::archive::run(name),
        Command::Diff => cmd::diff::run(),
        Command::Index {
            caps,
            codex,
            project,
        } => cmd::index::run(caps, codex, project),
        Command::Template { name } => cmd::template::run(name),
        Command::Schema { name } => cmd::schema::run(name),
    }
}
