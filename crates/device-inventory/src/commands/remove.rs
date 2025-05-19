use device_inventory::db::Database;

#[derive(Clone, Debug, clap::Parser)]
pub struct RemoveCommand {
    /// The alias of the device to remove
    #[arg(long)]
    alias: String,
}

impl RemoveCommand {
    pub async fn exec(self, db: Database) -> anyhow::Result<()> {
        let mut devices = db.read_devices()?;
        devices.remove(&self.alias);
        db.write_devices(&devices)?;
        Ok(())
    }
}
