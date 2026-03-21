use std::time::Duration;

use anyhow::Context;
use log::{debug, info};
use rs4a_vapix::{
    firmware_management_1, json_rpc_http::JsonRpcHttp, system_ready_1, ClientBuilder,
};
use tokio::time::sleep;

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

fn parse_uptime(s: &str) -> anyhow::Result<Duration> {
    let secs: u64 = s.parse().context("Failed to parse uptime")?;
    Ok(Duration::from_secs(secs))
}

enum RestartDetector<'a> {
    BootId(BootIdDetector<'a>),
    Uptime(UptimeDetector<'a>),
    StateTransition(StateTransitionDetector<'a>),
}

impl<'a> RestartDetector<'a> {
    async fn try_new(client: &'a rs4a_vapix::Client) -> anyhow::Result<Self> {
        let data = system_ready_1::system_ready().send(client).await?;
        if let Some(bootid) = data.bootid {
            return Ok(Self::BootId(BootIdDetector { client, bootid }));
        }
        if let Some(uptime) = data.uptime.as_deref().map(parse_uptime).transpose()? {
            return Ok(Self::Uptime(UptimeDetector { client, uptime }));
        }
        Ok(Self::StateTransition(StateTransitionDetector {
            client,
            ready: true,
        }))
    }

    async fn wait(self) -> anyhow::Result<()> {
        match self {
            Self::BootId(d) => d.wait().await,
            Self::Uptime(d) => d.wait().await,
            Self::StateTransition(d) => d.wait().await,
        }
    }
}

struct BootIdDetector<'a> {
    client: &'a rs4a_vapix::Client,
    bootid: String,
}

impl BootIdDetector<'_> {
    async fn wait(self) -> anyhow::Result<()> {
        loop {
            match system_ready_1::system_ready().send(self.client).await {
                Ok(data) => {
                    if let Some(bootid) = &data.bootid {
                        if *bootid != self.bootid {
                            debug!(
                                "Presumed restarted because bootid changed from {:?} to {:?}",
                                self.bootid, bootid
                            );
                            return Ok(());
                        }
                        debug!("Device has not restarted yet (bootid unchanged)");
                    }
                }
                Err(e) => {
                    debug!("Presumed offline because {e}");
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    }
}

struct UptimeDetector<'a> {
    client: &'a rs4a_vapix::Client,
    uptime: Duration,
}

impl UptimeDetector<'_> {
    async fn wait(mut self) -> anyhow::Result<()> {
        loop {
            match system_ready_1::system_ready().send(self.client).await {
                Ok(data) => {
                    if let Some(uptime) = data.uptime.as_deref().map(parse_uptime).transpose()? {
                        if uptime < self.uptime {
                            debug!(
                                "Presumed restarted because uptime decreased from {:?} to {:?}",
                                self.uptime, uptime
                            );
                            return Ok(());
                        }
                        debug!(
                            "Presumed online still because uptime increased from {:?} to {:?}",
                            self.uptime, uptime
                        );
                        self.uptime = uptime;
                    }
                }
                Err(e) => {
                    debug!("Presumed offline because {e}");
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    }
}

struct StateTransitionDetector<'a> {
    client: &'a rs4a_vapix::Client,
    ready: bool,
}

impl StateTransitionDetector<'_> {
    async fn wait(mut self) -> anyhow::Result<()> {
        loop {
            let is_ready = match system_ready_1::system_ready().send(self.client).await {
                Ok(data) => data.systemready,
                Err(e) => {
                    debug!("Presumed not ready because {e}");
                    false
                }
            };
            match (self.ready, is_ready) {
                (true, true) => {
                    debug!("Device is still ready");
                }
                (true, false) => {
                    debug!("Device became not ready");
                }
                (false, false) => {
                    debug!("Device is still not ready");
                }
                (false, true) => {
                    debug!("Device became ready again");
                    return Ok(());
                }
            }
            self.ready = is_ready;
            sleep(Duration::from_secs(1)).await;
        }
    }
}

pub async fn restore(netloc: &Netloc) -> anyhow::Result<()> {
    info!("Restoring device...");
    let client = ClientBuilder::new(netloc.host.clone())
        .plain_port(netloc.http_port)
        .secure_port(netloc.https_port)
        .basic_authentication(&netloc.user, &netloc.pass)
        .with_inner(|b| b.danger_accept_invalid_certs(true))
        .build_with_automatic_scheme()
        .await?;

    debug!("Checking if factory default is needed...");
    let data = system_ready_1::system_ready().send(&client).await?;
    if data.needsetup {
        info!("Device is already in default state");
        return Ok(());
    }

    let restart_detector = RestartDetector::try_new(&client).await?;

    info!("Requesting factory default...");
    firmware_management_1::factory_default(firmware_management_1::FactoryDefaultMode::Soft)
        .send(&client)
        .await?;

    info!("Waiting for device to restart...");
    restart_detector.wait().await?;
    info!("Device restored");
    Ok(())
}
