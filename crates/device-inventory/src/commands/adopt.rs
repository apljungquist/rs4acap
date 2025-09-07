use device_inventory::db::Database;

use crate::commands::{
    export::{Destination, ExportCommand},
    import::{ImportCommand, Source},
};

#[derive(Clone, Debug, clap::Parser)]
pub struct AdoptCommand {
    /// The alias of the device to export
    #[arg(long)]
    alias: Option<String>,
    /// How to import devices
    #[arg(long, default_value = "pool")]
    source: Source,
    // How to export the device
    #[arg(long, default_value = "environment")]
    destination: Destination,
}

impl AdoptCommand {
    pub async fn exec(self, db: Database, offline: bool) -> anyhow::Result<()> {
        let Self {
            alias,
            source,
            destination,
        } = self;
        ImportCommand { source }.exec(&db, offline).await?;
        ExportCommand { alias, destination }
            .exec(db, offline)
            .await?;
        Ok(())
    }
}
