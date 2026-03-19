use anyhow::Context;
use log::warn;

use crate::db::Database;

#[derive(Clone, Debug, clap::Parser)]
pub struct ActivateCommand {
    /// The alias of the device to activate.
    #[arg(short, long)]
    pub(crate) alias: Option<String>,
}

impl ActivateCommand {
    pub async fn exec(self, db: Database) -> anyhow::Result<()> {
        let mut devices = db.read_devices()?;

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

        // TODO: Consider `unset`ing variables that are not set.
        let envs = crate::env::envs(&device);
        for (key, value) in envs {
            if let Some(value) = value {
                println!("export {key}={value}");
            } else {
                println!("unset {key}");
            }
        }

        Ok(())
    }
}
