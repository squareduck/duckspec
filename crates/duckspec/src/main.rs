//! duckspec — command-line interface for the duckspec framework.
//!
//! Produces the `ds` binary.

use clap::Parser;

/// duckspec CLI — spec-driven development framework.
#[derive(Parser, Debug)]
#[command(name = "ds", version, about = "duckspec CLI", long_about = None)]
struct Cli {}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let _cli = Cli::parse();

    tracing::info!("duckspec starting");

    Ok(())
}
