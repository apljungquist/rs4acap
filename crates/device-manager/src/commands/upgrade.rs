use std::{path::PathBuf, time::Duration};

use anyhow::Context;
use log::info;
use rs4a_vapix::firmware_management_1::{
    AutoCommit, AutoRollback, FactoryDefaultMode, UpgradeRequest,
};
use tokio::time::timeout;

use crate::{restart_detector::RestartDetector, Netloc};

fn parse_auto_rollback(s: &str) -> anyhow::Result<AutoRollback> {
    match s {
        "never" => Ok(AutoRollback::Never),
        "default" => Ok(AutoRollback::Default),
        other => {
            let minutes: u32 = other.parse().with_context(|| {
                format!("expected 'never', 'default', or a number of minutes, got '{other}'")
            })?;
            Ok(AutoRollback::Minutes(minutes))
        }
    }
}

#[derive(Clone, Debug, clap::Args)]
pub struct UpgradeCommand {
    #[command(flatten)]
    netloc: Netloc,
    /// Path to the firmware image
    firmware: PathBuf,
    /// Factory default mode to apply during upgrade
    #[arg(long, short, default_value_t = FactoryDefaultMode::None)]
    factory_default_mode: FactoryDefaultMode,
    /// Auto-commit behavior after upgrade
    #[arg(long, short='c', default_value_t = AutoCommit::Default)]
    auto_commit: AutoCommit,
    /// Auto-rollback behavior: "never", "default", or minutes
    #[arg(long, short = 'r', default_value = "default")]
    auto_rollback: String,
}

impl UpgradeCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        let auto_rollback = parse_auto_rollback(&self.auto_rollback)?;

        info!("Reading firmware from {:?}", self.firmware);
        let firmware = tokio::fs::read(&self.firmware)
            .await
            .with_context(|| format!("Could not read firmware file {:?}", self.firmware))?;

        info!("Connecting to device");
        let client = self.netloc.connect().await?;

        let restart_detector = RestartDetector::try_new(&client).await?;

        info!("Sending upgrade request");
        let data = UpgradeRequest::new(firmware)
            .factory_default_mode(self.factory_default_mode)
            .auto_commit(self.auto_commit)
            .auto_rollback(auto_rollback)
            .send(&client, None)
            .await?;

        info!(
            "Upgrade accepted, firmware version: {}",
            data.firmware_version
        );

        info!("Waiting for restart");
        let () = timeout(Duration::from_secs(300), restart_detector.wait()).await?;

        info!("Upgrade complete");
        Ok(())
    }
}
