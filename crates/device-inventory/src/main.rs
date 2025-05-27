#![forbid(unsafe_code)]

mod commands;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use commands::{export::ExportCommand, import::ImportCommand};
use device_inventory::db::Database;
use rs4a_bin_utils::completions_command::CompletionsCommand;

use crate::commands::{
    add::AddCommand, for_each::ForEachCommand, list::ListCommand, login::LoginCommand,
    remove::RemoveCommand,
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
            Commands::Import(cmd) => cmd.exec(db, offline).await?,
            Commands::ForEach(cmd) => cmd.exec(db, offline).await?,
            Commands::List(cmd) => cmd.exec(db, offline).await?,
            Commands::Export(cmd) => cmd.exec(db, offline).await?,
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
    /// Import devices
    Import(ImportCommand),
    /// Run a command with environment variables set for each device
    ForEach(ForEachCommand),
    /// List available devices
    List(ListCommand),
    /// Print export statements for a device
    ///
    /// Example: `device-inventory export | source /dev/stdin`
    Export(ExportCommand),
    /// Remove a device
    Remove(RemoveCommand),
    Completions(CompletionsCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    Cli::parse().exec().await
}
