use device_inventory::db::Database;
use log::{info, warn};

#[derive(Clone, Debug, clap::Parser)]
pub struct RemoveCommand {
    /// The alias of the device to remove
    #[arg(long)]
    alias: String,
}

impl RemoveCommand {
    pub async fn exec(self, db: Database) -> anyhow::Result<()> {
        let devices = db.read_devices()?;

        let pattern = glob::Pattern::new(&self.alias)?;
        let (removed, retained): (Vec<_>, Vec<_>) = devices
            .into_iter()
            .partition(|(alias, _)| pattern.matches(alias));

        if removed.is_empty() {
            warn!("No devices matched the pattern");
        }
        for (alias, _) in removed {
            info!("Removing device {}", alias);
        }

        db.write_devices(&retained.into_iter().collect())?;
        Ok(())
    }
}
