use std::{path::PathBuf, time::Duration};

use anyhow::Context;
use log::info;
use rs4a_vapix::apis::firmware_management_1::{
    AutoCommit, AutoRollback, FactoryDefaultMode, UpgradeRequest,
};
use tokio::time::timeout;

use crate::{restart_detector::RestartDetector, Netloc};

fn parse_auto_rollback(s: &str) -> anyhow::Result<AutoRollback> {
    match s {
        "never" => Ok(AutoRollback::Never),
        other => {
            let minutes: u32 = other.parse().with_context(|| {
                format!("expected 'never' or a number of minutes, got '{other}'")
            })?;
            Ok(AutoRollback::Minutes(minutes))
        }
    }
}

#[derive(Clone, Debug, clap::Args)]
pub struct UpgradeCommand {
    #[command(flatten)]
    pub netloc: Netloc,
    /// Path to the firmware image
    pub firmware: PathBuf,
    /// Factory default mode to apply during upgrade
    #[arg(long, short = 'm')]
    pub factory_default_mode: Option<FactoryDefaultMode>,
    /// Auto-commit behavior after upgrade
    #[arg(long, short = 'c')]
    pub auto_commit: Option<AutoCommit>,
    /// Auto-rollback behavior: "never", or minutes
    #[arg(long, short = 'r')]
    pub auto_rollback: Option<String>,
}

impl UpgradeCommand {
    pub async fn exec(self) -> anyhow::Result<String> {
        let auto_rollback = self
            .auto_rollback
            .as_deref()
            .map(parse_auto_rollback)
            .transpose()?;

        info!("Reading firmware from {:?}", self.firmware);
        let firmware = tokio::fs::read(&self.firmware)
            .await
            .with_context(|| format!("Could not read firmware file {:?}", self.firmware))?;

        info!("Connecting to device");
        let client = self.netloc.connect().await?;

        let restart_detector = RestartDetector::try_new(&client).await?;

        info!("Sending upgrade request");
        let mut request = UpgradeRequest::new(firmware);
        if let Some(mode) = self.factory_default_mode {
            request = request.factory_default_mode(mode);
        }
        if let Some(auto_commit) = self.auto_commit {
            request = request.auto_commit(auto_commit);
        }
        if let Some(auto_rollback) = auto_rollback {
            request = request.auto_rollback(auto_rollback);
        }
        let data = request.send(&client).await?;

        info!(
            "Upgrade accepted, firmware version: {}",
            data.firmware_version
        );

        info!("Waiting for restart");
        let () = timeout(Duration::from_secs(300), restart_detector.wait()).await?;

        info!("Upgrade complete");
        Ok(String::new())
    }
}
