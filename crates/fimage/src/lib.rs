pub mod archive;
pub mod commands;
pub mod info;

use clap::{Parser, Subcommand};

pub use crate::commands::{extract::ExtractCommand, inspect::InspectCommand};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn exec(self) -> anyhow::Result<String> {
        let Self { command } = self;
        match command {
            Commands::Extract(cmd) => cmd.exec(),
            Commands::Inspect(cmd) => cmd.exec(),
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Extract a firmware image into a directory
    Extract(ExtractCommand),
    /// Print identifying metadata read from a firmware image on disk
    Inspect(InspectCommand),
}
