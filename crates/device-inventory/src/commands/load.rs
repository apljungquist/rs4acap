use std::{
    io,
    io::{IsTerminal, Read},
};

use anyhow::Context;

use crate::db::Database;

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

#[derive(Clone, Debug, clap::Parser)]
pub struct LoadCommand {}

impl LoadCommand {
    pub async fn exec(self, db: &Database) -> anyhow::Result<()> {
        let s = input("Enter the database content:")?;
        let devices = serde_json::from_str(&s)?;
        db.write_devices(&devices)?;
        Ok(())
    }
}
