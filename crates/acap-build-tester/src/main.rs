//! Facilities for testing `acap-build` implementations.

mod commands;
mod invocation;
mod output;

use clap::{Parser, Subcommand};

use crate::commands::replay::ReplayCommand;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    fn exec(self) -> anyhow::Result<()> {
        let Self { command } = self;
        match command {
            Commands::Replay(cmd) => cmd.exec()?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Verify that this workspace's `acap-build` builds the given apps like the reference
    /// `acap-build` on the `PATH`
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
