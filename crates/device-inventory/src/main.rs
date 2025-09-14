#![forbid(unsafe_code)]

mod commands;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use commands::{activate::ActivateCommand, import::ImportCommand};
use device_inventory::db::Database;
use rs4a_bin_utils::completions_command::CompletionsCommand;

use crate::commands::{
    add::AddCommand, adopt::AdoptCommand, deactivate::DeactivateCommand, for_each::ForEachCommand,
    list::ListCommand, login::LoginCommand, remove::RemoveCommand,
};

#[derive(Parser)]
struct Cli {
    /// Location of the application data.
    #[clap(long, env = "DEVICE_INVENTORY_LOCATION")]
    inventory: Option<PathBuf>,
    #[clap(long, env = "DEVICE_INVENTORY_OFFLINE")]
    offline: bool,
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn exec(self) -> anyhow::Result<()> {
        let Self {
            inventory: db,
            offline,
            command,
        } = self;
        let db = Database::open_or_create(db)?;
        match command {
            Commands::Login(cmd) => cmd.exec(db, offline).await?,
            Commands::Add(cmd) => cmd.exec(db).await?,
            Commands::Adopt(cmd) => cmd.exec(db, offline).await?,
            Commands::Deactivate(cmd) => cmd.exec().await?,
            Commands::Import(cmd) => cmd.exec(&db, offline).await?,
            Commands::ForEach(cmd) => cmd.exec(db).await?,
            Commands::List(cmd) => cmd.exec(db).await?,
            Commands::Activate(cmd) => cmd.exec(db).await?,
            Commands::Remove(cmd) => cmd.exec(db).await?,
            Commands::Completions(cmd) => cmd.exec::<Self>()?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Login to a pool of shared devices
    Login(LoginCommand),
    /// Add a device
    Add(AddCommand),
    /// Import all matching devices and activate at most one matching device.
    Adopt(AdoptCommand),
    /// Deactivate any active device.
    ///
    /// For devices activated using environment variables, the printed commands must be run.
    Deactivate(DeactivateCommand),
    /// Import devices
    Import(ImportCommand),
    /// Run a command with environment variables set for each device
    ForEach(ForEachCommand),
    /// List available devices
    List(ListCommand),
    /// Activate an existing device.
    Activate(ActivateCommand),
    /// Remove a device
    Remove(RemoveCommand),
    /// Print a completion file for the given shell.
    ///
    /// Example: `device-inventory completions zsh | source /dev/stdin`.
    Completions(CompletionsCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut guard = rs4a_bin_utils::logger::init();
    Cli::parse().exec().await?;
    guard.disarm();
    Ok(())
}
