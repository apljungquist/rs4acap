use anyhow::Context;
use device_inventory::{
    db::{Database, Device},
    db_vlt,
};
use log::{info, warn};

#[derive(Clone, Debug, clap::Parser)]
pub struct AbandonCommand {
    /// The alias of the device to abandon.
    ///
    /// If not set, any activated device will be abandoned instead.
    #[arg(long)]
    alias: Option<String>,
}

impl AbandonCommand {
    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
        let devices = db.read_devices()?;

        let (removed, retained): (Vec<_>, Vec<_>) = match self.alias {
            Some(alias) => {
                let pattern = glob::Pattern::new(&alias)?;
                devices
                    .into_iter()
                    .partition(|(alias, _)| pattern.matches(alias))
            }
            None => {
                let fingerprint = Device::from(
                    rs4a_dut::Device::from_anywhere()?
                        .context("An alias must be provided if there is no active device")?,
                )
                .fingerprint();
                devices
                    .into_iter()
                    .partition(|(_, device)| device.fingerprint() == fingerprint)
            }
        };

        if removed.is_empty() {
            warn!("No devices were removed");
            return Ok(());
        }

        for (alias, device) in removed {
            info!("Removing device {}", alias);
            if let Some(loan_id) = device.loan_id {
                info!("Canceling loan {loan_id}");
                let client = db_vlt::client(db, offline)
                    .await?
                    .context("No VLT session")?;
                rs4a_vlt::requests::cancel_loan(loan_id)
                    .send(&client)
                    .await?;
            }
        }

        db.write_devices(&retained.into_iter().collect())?;
        Ok(())
    }
}
