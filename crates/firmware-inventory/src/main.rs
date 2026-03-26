#![forbid(unsafe_code)]

mod commands;
mod db;
mod scrape;
mod version;

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use rs4a_authentication::SessionCookie;
use rs4a_bin_utils::completions_command::CompletionsCommand;

use crate::{
    commands::{
        dump::DumpCommand, get::GetCommand, list::ListCommand, load::LoadCommand,
        login::LoginCommand, update::UpdateCommand,
    },
    db::Database,
};

pub fn authenticated_client(cookie: SessionCookie) -> anyhow::Result<reqwest::Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::COOKIE, cookie.into_header_value());
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .context("Failed to build HTTP client")
}

#[derive(Parser)]
struct Cli {
    /// Location of the application data.
    #[clap(long, env = "FIRMWARE_INVENTORY_LOCATION")]
    inventory: Option<PathBuf>,
    #[clap(long, env = "FIRMWARE_INVENTORY_OFFLINE")]
    offline: bool,
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn exec(self) -> anyhow::Result<()> {
        let Self {
            inventory,
            offline,
            command,
        } = self;
        let db = Database::open_or_create(inventory)?;
        match command {
            Commands::Login(cmd) => cmd.exec(db).await?,
            Commands::Update(cmd) => cmd.exec(&db, offline).await?,
            Commands::List(cmd) => cmd.exec(&db)?,
            Commands::Get(cmd) => cmd.exec(&db, offline).await?,
            Commands::Dump(cmd) => cmd.exec(&db)?,
            Commands::Load(cmd) => cmd.exec(&db)?,
            Commands::Completions(cmd) => cmd.exec::<Self>()?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Login to access firmware downloads
    Login(LoginCommand),
    /// Update the local firmware index for products matching a glob
    Update(UpdateCommand),
    /// List indexed firmware versions, showing which are cached locally
    List(ListCommand),
    /// Get firmware matching product and version requirement
    Get(GetCommand),
    /// Dump the index to stdout as JSON
    Dump(DumpCommand),
    /// Load the index from stdin as JSON
    Load(LoadCommand),
    /// Print a completion file for the given shell.
    ///
    /// Example: `firmware-inventory completions zsh | source /dev/stdin`.
    Completions(CompletionsCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut guard = rs4a_bin_utils::logger::init();
    Cli::parse().exec().await?;
    guard.disarm();
    Ok(())
}
