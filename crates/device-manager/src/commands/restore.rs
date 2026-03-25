use std::time::Duration;

use log::{debug, info, trace, warn};
use rs4a_vapix::{
    firmware_management_1::{FactoryDefaultMode, FactoryDefaultRequest},
    json_rpc_http::JsonRpcHttp,
    system_ready_1,
    system_ready_1::SystemreadyData,
};
use tokio::time::{sleep, timeout};

use crate::Netloc;

#[derive(Clone, Debug, clap::Args)]
pub struct RestoreCommand {
    #[command(flatten)]
    netloc: Netloc,
}

impl RestoreCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        restore(&self.netloc).await
    }
}

#[derive(Clone, Copy, Debug)]
enum DetectorState {
    WaitingToGoDown,
    WaitingToComeBack,
    WaitingToBeReady,
    Ready,
}

struct RestartDetector<'a> {
    client: &'a rs4a_vapix::Client,
    boot_id: Option<String>,
    uptime: Option<Duration>,
}

impl<'a> RestartDetector<'a> {
    async fn try_new(client: &'a rs4a_vapix::Client) -> anyhow::Result<Self> {
        let data = system_ready_1::system_ready().send(client).await?;
        let boot_id = data.bootid.clone();
        let uptime = data.try_uptime()?;
        Ok(Self {
            client,
            boot_id,
            uptime,
        })
    }

    fn boot_id_has_changed(&self, data: &SystemreadyData) -> bool {
        matches!(
            (self.boot_id.as_deref(), data.bootid.as_deref()),
            (Some(old), Some(new)) if old != new
        )
    }

    fn uptime_has_decreased(&self, data: &SystemreadyData) -> bool {
        matches!(
            (self.uptime, data.try_uptime()),
            (Some(old), Ok(Some(new))) if new < old
        )
    }

    fn next_state(&self, prev: DetectorState, data: Option<SystemreadyData>) -> DetectorState {
        use DetectorState::*;

        match (prev, data) {
            (WaitingToGoDown, None) => {
                debug!("Device went down, waiting for it to come back up");
                WaitingToComeBack
            }
            (WaitingToGoDown, Some(data)) => {
                if self.boot_id_has_changed(&data) {
                    debug!("Boot ID changed, device has restarted");
                    match data.systemready {
                        false => WaitingToBeReady,
                        true => Ready,
                    }
                } else if self.uptime_has_decreased(&data) {
                    debug!("Uptime decreased, device has restarted");
                    match data.systemready {
                        false => WaitingToBeReady,
                        true => Ready,
                    }
                } else {
                    trace!("Still up, waiting for it to go down");
                    WaitingToGoDown
                }
            }
            (WaitingToComeBack, None) => {
                trace!("Device still down, waiting for it to come back up");
                WaitingToComeBack
            }
            (WaitingToComeBack, Some(data)) => {
                debug!("Device came back up, waiting for it to become ready");
                match data.systemready {
                    false => WaitingToBeReady,
                    true => Ready,
                }
            }
            (WaitingToBeReady, Some(data)) => {
                trace!("Already waiting to become ready");
                match data.systemready {
                    false => WaitingToBeReady,
                    true => Ready,
                }
            }
            (WaitingToBeReady, None) => {
                warn!("Waiting to become ready, but device is down");
                WaitingToBeReady
            }
            (Ready, _) => {
                unreachable!("Device is already ready");
            }
        }
    }

    async fn wait(self) {
        let mut state = DetectorState::WaitingToGoDown;

        while !matches!(state, DetectorState::Ready) {
            sleep(Duration::from_secs(1)).await;

            let data = system_ready_1::system_ready()
                .send(self.client)
                .await
                .inspect_err(|e| debug!("converting error to option: {e}"))
                .ok();

            state = self.next_state(state, data);
        }
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
    FactoryDefaultRequest::new(FactoryDefaultMode::Soft)
        .send(&client)
        .await?;

    info!("Waiting for restart");
    let () = timeout(Duration::from_secs(120), restart_detector.wait()).await?;

    info!("Factory default complete");
    Ok(())
}
