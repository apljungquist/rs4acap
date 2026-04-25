mod commands;

use clap::{Parser, Subcommand};
use rs4a_bin_utils::completions_command::CompletionsCommand;

use crate::commands::install::InstallCommand;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn exec(self) -> anyhow::Result<String> {
        match self.command {
            Commands::Install(cmd) => cmd.exec().await,
            Commands::Completions(cmd) => {
                cmd.exec::<Self>()?;
                Ok(String::new())
            }
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Install firmware matching a semver requirement
    ///
    /// Fetches the device's model, resolves matching firmware, and applies it.
    /// Downgrades automatically use factory-default soft and re-initialize the device.
    Install(InstallCommand),
    /// Generate shell completions
    ///
    /// Example: `cli4a completions zsh | source /dev/stdin`
    Completions(CompletionsCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut guard = rs4a_bin_utils::logger::init();
    let out = Cli::parse().exec().await?;
    if !out.is_empty() {
        print!("{out}");
    }
    guard.disarm();
    Ok(())
}
