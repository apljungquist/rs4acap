//! Facilities for testing `acap-build` implementations.

mod commands;
mod input;
mod invocation;
mod output;
mod source;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::commands::{fuzz::FuzzCommand, replay::ReplayCommand};

#[derive(Parser)]
struct Cli {
    /// Path to the candidate `acap-build` executable to test against the reference.
    #[clap(long, env = "ACAP_BUILD_CANDIDATE")]
    candidate: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    fn exec(self) -> anyhow::Result<()> {
        let Self { candidate, command } = self;
        match command {
            Commands::Fuzz(cmd) => cmd.exec(&candidate)?,
            Commands::Replay(cmd) => cmd.exec(&candidate)?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Verify that, on generated examples, whenever the candidate `acap-build` succeeds it
    /// produces artifacts bit-identical to the reference `acap-build` on the `PATH`.
    Fuzz(FuzzCommand),
    /// Verify that the candidate `acap-build` builds the given apps like the reference
    /// `acap-build` on the `PATH` on recorded examples.
    Replay(ReplayCommand),
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    Cli::parse().exec()
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_is_valid() {
        Cli::command().debug_assert();
    }
}
