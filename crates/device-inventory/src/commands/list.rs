use device_inventory::{
    db::{Database, Device},
    db_vlt,
};

#[derive(Clone, Debug, clap::Parser)]
pub struct ListCommand;

impl ListCommand {
    pub async fn exec(self, db: Database, offline: bool) -> anyhow::Result<()> {
        let devices = if offline {
            db.read_devices()?
        } else {
            db_vlt::import(&db, offline).await?
        };

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
