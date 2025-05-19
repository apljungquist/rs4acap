use std::io::{self, IsTerminal};

use anyhow::Context;
use device_inventory::{db::Database, db_vlt};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, clap::ValueEnum)]
enum Source {
    /// Parse devices from the provided JSON
    Json,
    /// Use the provided cookie to fetch devices from an API
    Cookie,
}

#[derive(Clone, Debug, clap::Parser)]
pub struct LoginCommand;

fn input(prompt: &str) -> anyhow::Result<String> {
    if io::stdin().is_terminal() {
        println!("{prompt}");
    }
    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .context("Failed to read from stdin")?;
    Ok(buf)
}

impl LoginCommand {
    pub async fn exec(self, db: Database, offline: bool) -> anyhow::Result<()> {
        // TODO: My terminal won't let me enter values longer than 1023,
        //  but the token seems superfluous anyway.
        let cookie = input("Enter the cookie:")?;
        db.write_cookie(&cookie)?;
        if !offline {
            db_vlt::import(&db, offline).await?;
        }
        Ok(())
    }
}
