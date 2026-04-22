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
    /// Print active changes, capability/codex counts, or details for a path.
    Status {
        /// Path to inspect (change, spec, step, or steps dir).
        path: Option<String>,
    },
    /// Validate whole project: backlinks, test coverage, cross-artifact integrity.
    Audit {
        /// Change to audit (name or path). Omit for full project audit.
        change: Option<String>,
    },
    /// Validate artifacts against schemas.
    Check {
        /// File or directory to validate (default: duckspec/).
        path: Option<String>,
    },
    /// Format artifacts to canonical markdown (in place).
    Format {
        /// File or directory to format (default: duckspec/).
        path: Option<String>,
        /// Print formatted output instead of writing files.
        #[arg(long)]
        dry: bool,
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
        /// Preview the archive without writing.
        #[arg(long)]
        dry: bool,
    },
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
    /// Create a new duckspec artifact.
    Create {
        #[command(subcommand)]
        command: cmd::create::CreateCommand,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init { harness } => cmd::init::run(harness),
        Command::Status { path } => cmd::status::run(path),
        Command::Audit { change } => cmd::audit::run(change),
        Command::Check { path } => cmd::check::run(path),
        Command::Format { path, dry } => cmd::format::run(path, dry),
        Command::Sync { dry } => cmd::sync::run(dry),
        Command::Archive { name, dry } => cmd::archive::run(name, dry),
        Command::Index {
            caps,
            codex,
            project,
        } => cmd::index::run(caps, codex, project),
        Command::Template { name } => cmd::template::run(name),
        Command::Schema { name } => cmd::schema::run(name),
        Command::Create { command } => cmd::create::run(command),
    }
}
