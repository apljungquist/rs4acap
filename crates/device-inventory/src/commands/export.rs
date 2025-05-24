use anyhow::{bail, Context};
use device_inventory::{db::Database, db_vlt};
use log::warn;

#[derive(Clone, Debug, clap::Parser)]
pub struct ExportCommand {
    /// The alias of the device to export.
    #[arg(long)]
    alias: Option<String>,
}

impl ExportCommand {
    pub async fn exec(self, db: Database, offline: bool) -> anyhow::Result<()> {
        let mut devices = if offline {
            db.read_devices()?
        } else {
            // TODO: Consider not importing automatically.
            db_vlt::import(&db, offline).await?
        };
        let device = match self.alias {
            None => {
                let mut sorted_devices: Vec<_> = devices.into_iter().collect();
                sorted_devices.sort_by(|(left, _), (right, _)| left.cmp(right));
                let mut sorted_devices = sorted_devices.into_iter();

                let (_, device) = sorted_devices.next().context("no devices found")?;
                if sorted_devices.next().is_some() {
                    warn!("Multiple devices found, using the first one")
                }
                device
            }
            Some(alias) => {
                let Some(device) = devices.remove(&alias) else {
                    bail!("No matching device found")
                };
                device
            }
        };

        // TODO: Consider resolving to IPv4 if possible.
        println!("export AXIS_DEVICE_IP={}", device.host);
        println!("export AXIS_DEVICE_USER={}", device.username);
        println!(
            "export AXIS_DEVICE_PASS={}",
            device.password.dangerous_reveal()
        );
        if let Some(p) = device.ssh_port {
            println!("export AXIS_DEVICE_SSH_PORT={p}",);
        }
        if let Some(p) = device.http_port {
            println!("export AXIS_DEVICE_HTTP_PORT={p}",);
        }
        if let Some(p) = device.https_port {
            println!("export AXIS_DEVICE_HTTPS_PORT={p}",);
        }
        // TODO: Don't assume that all stored devices use a self signed certificate.
        println!("export AXIS_DEVICE_HTTPS_SELF_SIGNED=1");

        Ok(())
    }
}
