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

        let mut statuses = vec!["STATUS".to_string()];
        let mut aliases = vec!["ALIAS".to_string()];
        let mut models = vec!["MODEL".to_string()];
        let mut hosts = vec!["HOST".to_string()];

        let active = rs4a_dut::Device::from_anywhere()?; //.map(|d|(d.host, d.http_port.unwrap_or(80)));
        if let Some(active) = active.as_ref() {
            let device = sorted_devices
                .iter()
                .find(|(_, d)| d.host == active.host && d.http_port == active.http_port);

            statuses.push("ACTIVE".to_string());
            aliases.push(device.map(|(a, _)| a.clone()).unwrap_or_default());
            models.push(
                device
                    .map(|(_, d)| d.model.clone().unwrap_or_default())
                    .unwrap_or_default(),
            );
            hosts.push(active.host.to_string());
        }

        for (
            alias,
            Device {
                host,
                model,
                http_port,
                ..
            },
        ) in sorted_devices.into_iter()
        {
            if let Some(d) = active.as_ref() {
                if d.host == host && d.http_port.unwrap_or(80) == http_port.unwrap_or(80) {
                    continue;
                }
            }

            statuses.push("".to_string());
            aliases.push(alias);
            models.push(model.unwrap_or_default());
            hosts.push(host.to_string());
        }

        let statuses_width = 1 + statuses.iter().map(|s| s.len()).max().unwrap();
        let aliases_width = 1 + aliases.iter().map(|s| s.len()).max().unwrap();
        let models_width = 1 + models.iter().map(|s| s.len()).max().unwrap();

        // TODO: Consider showing which device is active and figuring out the testing.
        for (((status, alias), model), host) in statuses
            .into_iter()
            .zip(aliases.into_iter())
            .zip(models.into_iter())
            .zip(hosts.into_iter())
        {
            println!(
                "{status:statuses_width$} {alias:aliases_width$} {model:models_width$} {host}"
            );
        }

        Ok(())
    }
}
