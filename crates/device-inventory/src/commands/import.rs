use std::io::{self, IsTerminal, Read};

use anyhow::Context;

use crate::{db::Database, db_vlt};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, clap::ValueEnum)]
pub(crate) enum Source {
    /// Parse devices from the provided JSON
    Json,
    /// Fetch reserved devices from a shared pool
    Pool,
}

#[derive(Clone, Debug, clap::Parser)]
pub struct ImportCommand {
    /// How to import devices
    #[arg(long, default_value = "pool")]
    pub(crate) source: Source,
}

fn input(prompt: &str) -> anyhow::Result<String> {
    let mut buf = String::new();
    if io::stdin().is_terminal() {
        println!("{prompt}");
        io::stdin()
            .read_line(&mut buf)
            .context("Failed to read from stdin")?;
    } else {
        io::stdin()
            .read_to_string(&mut buf)
            .context("Failed to read from stdin")?;
    }

    Ok(buf)
}

impl ImportCommand {
    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
        match self.source {
            Source::Json => {
                let loans = input("Enter the loans JSON:")?;
                db_vlt::store(db, &loans)?;
            }
            Source::Pool => {
                db_vlt::import(db, offline).await?;
            }
        };
        Ok(())
    }
}
