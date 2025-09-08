use device_inventory::db::{Database, Device};

#[derive(Clone, Debug, clap::Parser)]
pub struct ListCommand {
    /// The alias of the device to list
    #[arg(long)]
    alias: Option<String>,
}

impl ListCommand {
    pub async fn exec(self, db: Database) -> anyhow::Result<()> {
        let mut devices = db.read_devices()?;

        if let Some(pattern) = &self.alias {
            let pattern = glob::Pattern::new(pattern)?;
            devices.retain(|alias, _| pattern.matches(alias));
        }

        let mut sorted_devices: Vec<_> = devices.into_iter().collect();
        sorted_devices.sort_by(|(left, _), (right, _)| left.cmp(right));

        let mut aliases = vec!["ALIAS".to_string()];
        let mut models = vec!["MODEL".to_string()];
        let mut hosts = vec!["HOST".to_string()];
        for (alias, Device { host, model, .. }) in sorted_devices.into_iter() {
            aliases.push(alias);
            models.push(model.unwrap_or_default());
            hosts.push(host.to_string());
        }

        let aliases_width = 1 + aliases.iter().map(|s| s.len()).max().unwrap();
        let models_width = 1 + models.iter().map(|s| s.len()).max().unwrap();

        // TODO: Consider showing which device is active and figuring out the testing.
        for ((alias, model), host) in aliases
            .into_iter()
            .zip(models.into_iter())
            .zip(hosts.into_iter())
        {
            println!("{alias:aliases_width$} {model:models_width$} {host}");
        }

        Ok(())
    }
}
