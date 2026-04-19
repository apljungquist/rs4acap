use std::time::Duration;

use log::{debug, info};
use rs4a_vapix::{firmware_management_1::FactoryDefaultRequest, system_ready_1};
use tokio::time::timeout;

use crate::{restart_detector::RestartDetector, Netloc};

#[derive(Clone, Debug, clap::Args)]
pub struct RestoreCommand {
    #[command(flatten)]
    pub netloc: Netloc,
}

impl RestoreCommand {
    pub async fn exec(self) -> anyhow::Result<String> {
        restore(&self.netloc).await?;
        Ok(String::new())
    }
}

pub async fn restore(netloc: &Netloc) -> anyhow::Result<()> {
    info!("Restoring to factory defaults");
    let client = netloc.connect().await?;

    debug!("Querying device state");
    let data = system_ready_1::system_ready().send(&client).await?;
    if data.needsetup {
        info!("Already in setup mode, nothing to do");
        return Ok(());
    }

    let restart_detector = RestartDetector::try_new(&client).await?;

    info!("Sending factory default request");
    FactoryDefaultRequest::new().send(&client).await?;

    info!("Waiting for restart");
    let () = timeout(Duration::from_secs(120), restart_detector.wait()).await?;

    info!("Factory default complete");
    Ok(())
}
