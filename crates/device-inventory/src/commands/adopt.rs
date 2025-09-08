use std::collections::HashSet;

use device_inventory::db::Database;
use log::warn;

use crate::commands::{
    export::{Destination, ExportCommand},
    import::{ImportCommand, Source},
};

fn infer_alias(before: HashSet<String>, after: impl Iterator<Item = String>) -> Option<String> {
    let mut new = after.filter(|k| !before.contains(k)).collect::<Vec<_>>();
    new.sort();
    let mut new = new.into_iter();
    let first = new.next()?;
    if new.next().is_some() {
        warn!("More than one device imported, activating the first one");
    }
    Some(first)
}

#[derive(Clone, Debug, clap::Parser)]
pub struct AdoptCommand {
    /// The alias of the device to export
    #[arg(long)]
    alias: Option<String>,
    /// How to import devices
    #[arg(long, default_value = "pool")]
    source: Source,
    // How to export the device
    #[arg(long, default_value = "filesystem")]
    destination: Destination,
}

impl AdoptCommand {
    pub async fn exec(self, db: Database, offline: bool) -> anyhow::Result<()> {
        let Self {
            alias,
            source,
            destination,
        } = self;
        let before = db.read_devices()?.into_keys();
        ImportCommand { source }.exec(&db, offline).await?;
        let after = db.read_devices()?.into_keys();
        ExportCommand {
            alias: alias.or_else(|| infer_alias(before.into_iter().collect(), after.into_iter())),
            destination,
        }
        .exec(db)
        .await?;
        Ok(())
    }
}
