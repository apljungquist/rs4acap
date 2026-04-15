use std::time::Duration;

use log::{debug, trace, warn};
use rs4a_vapix::{system_ready_1, system_ready_1::SystemreadyData};
use tokio::time::sleep;

#[derive(Clone, Copy, Debug)]
enum DetectorState {
    WaitingToGoDown,
    WaitingToComeBack,
    WaitingToBeReady,
    Ready,
}

pub(crate) struct RestartDetector<'a> {
    client: &'a rs4a_vapix::Client,
    boot_id: Option<String>,
    uptime: Option<Duration>,
}

impl<'a> RestartDetector<'a> {
    pub(crate) async fn try_new(client: &'a rs4a_vapix::Client) -> anyhow::Result<Self> {
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

    pub(crate) async fn wait(self) {
        let mut state = DetectorState::WaitingToGoDown;

        while !matches!(state, DetectorState::Ready) {
            sleep(Duration::from_secs(1)).await;

            let data = tokio::time::timeout(
                Duration::from_secs(5),
                system_ready_1::system_ready().send(self.client),
            )
            .await
            .inspect_err(|_| debug!("system_ready request timed out"))
            .ok()
            .and_then(|r| {
                r.inspect_err(|e| debug!("converting error to option: {e}"))
                    .ok()
            });

            state = self.next_state(state, data);
        }
    }
}
