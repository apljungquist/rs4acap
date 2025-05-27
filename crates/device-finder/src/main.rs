#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use rs4a_bin_utils::completions_command::CompletionsCommand;

use crate::commands::discover_devices::DiscoverDevicesCommand;

mod commands;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub async fn exec(self) -> anyhow::Result<()> {
        match self.command.unwrap_or_default() {
            Commands::DiscoverDevices(cmd) => cmd.exec().await?,
            Commands::Completions(cmd) => cmd.exec::<Self>()?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Discover devices on the local network
    DiscoverDevices(DiscoverDevicesCommand),
    Completions(CompletionsCommand),
}

impl Default for Commands {
    fn default() -> Self {
        Self::DiscoverDevices(DiscoverDevicesCommand::default())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    Cli::parse().exec().await
}
