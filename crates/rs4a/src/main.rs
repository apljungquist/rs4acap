#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

use crate::commands::discover_devices::DiscoverDevicesCommand;

mod commands;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub fn exec(self) -> anyhow::Result<()> {
        match self.command {
            Commands::DiscoverDevices(cmd) => cmd.exec()?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Discover devices on the local network
    DiscoverDevices(DiscoverDevicesCommand),
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    Cli::parse().exec()
}
