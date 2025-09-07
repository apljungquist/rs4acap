use anyhow::Context;
use device_inventory::{db::Database, db_vlt};
use log::warn;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, clap::ValueEnum)]
pub(crate) enum Destination {
    /// Write information to the filesystem.
    Filesystem,
    /// Print information as a shell script that can be sourced.
    Environment,
}

#[derive(Clone, Debug, clap::Parser)]
pub struct ExportCommand {
    /// The alias of the device to export
    #[arg(long)]
    pub(crate) alias: Option<String>,
    // How to export the device
    #[arg(long, default_value = "environment")]
    pub(crate) destination: Destination,
}

impl ExportCommand {
    pub async fn exec(self, db: Database, offline: bool) -> anyhow::Result<()> {
        let mut devices = if offline {
            db.read_devices()?
        } else {
            // TODO: Consider not importing automatically.
            db_vlt::import(&db, offline).await?
        };

        if let Some(pattern) = &self.alias {
            let pattern = glob::Pattern::new(pattern)?;
            devices.retain(|alias, _| pattern.matches(alias));
        }

        let mut sorted_devices: Vec<_> = devices.into_iter().collect();
        sorted_devices.sort_by(|(left, _), (right, _)| left.cmp(right));
        let mut sorted_devices = sorted_devices.into_iter();

        let (_, device) = sorted_devices.next().context("No matching devices found")?;
        if sorted_devices.next().is_some() {
            warn!("Multiple devices found, using the first one")
        }

        match self.destination {
            Destination::Filesystem => {
                rs4a_dut::Device::from(device).to_fs()?;
            }
            Destination::Environment => {
                // TODO: Consider `unset`ing variables that are not set.
                let envs = device_inventory::env::envs(&device);
                for (key, value) in envs {
                    println!("export {key}={value}");
                }
            }
        }

        Ok(())
    }
}
