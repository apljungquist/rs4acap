use anyhow::bail;
use device_inventory::{db::Database, db_vlt, env::envs};

#[derive(Clone, Debug, clap::Parser)]
pub struct ForEachCommand {
    /// Program to run
    program: String,
    /// Arguments to pass to the program
    arguments: Vec<String>,
}

impl ForEachCommand {
    pub async fn exec(self, db: Database, offline: bool) -> anyhow::Result<()> {
        let Self { program, arguments } = self;
        let devices = if offline {
            db.read_devices()?
        } else {
            db_vlt::import(&db, offline).await?
        };

        let mut sorted_devices: Vec<_> = devices.into_iter().collect();
        sorted_devices.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (alias, device) in sorted_devices {
            let mut cmd = std::process::Command::new(&program);
            cmd.args(&arguments).envs(envs(&device));
            let status = cmd.status()?;
            if !status.success() {
                bail!("Command exited with status {status} for device {alias}");
            }
        }
        Ok(())
    }
}
