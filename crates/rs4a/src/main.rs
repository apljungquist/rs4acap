#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

use crate::commands::{discover_devices::DiscoverDevicesCommand, export_loans::ExportLoansCommand};

mod commands;
mod psst;
mod vlt;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn exec(self) -> anyhow::Result<()> {
        match self.command {
            Commands::DiscoverDevices(cmd) => cmd.exec().await?,
            Commands::ExportLoans(cmd) => cmd.exec().await?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Discover devices on the local network
    DiscoverDevices(DiscoverDevicesCommand),
    /// Print export statements for the current loan
    ///
    /// Example: `pbpaste | rs4a export-loans | source /dev/stdin`.
    ExportLoans(ExportLoansCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    Cli::parse().exec().await
}
