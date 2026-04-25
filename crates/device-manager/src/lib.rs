pub mod commands;
mod restart_detector;
mod ssh_keygen;

use clap::{Parser, Subcommand};
use rs4a_bin_utils::completions_command::CompletionsCommand;
use url::Host;

pub use crate::commands::{
    init::InitCommand, reinit::ReinitCommand, restore::RestoreCommand, upgrade::UpgradeCommand,
};

#[derive(Clone, Debug, clap::Args)]
pub struct Netloc {
    /// Hostname or IP address of the device.
    #[arg(long, value_parser = url::Host::parse, env = "AXIS_DEVICE_IP")]
    pub host: Host,
    /// Override the default port for HTTP.
    #[arg(long, env = "AXIS_DEVICE_HTTP_PORT")]
    pub http_port: Option<u16>,
    /// Override the default port for HTTPS.
    #[arg(long, env = "AXIS_DEVICE_HTTPS_PORT")]
    pub https_port: Option<u16>,
    /// The username to use for authentication.
    #[arg(short, long, env = "AXIS_DEVICE_USER", default_value = "root")]
    pub user: String,
    /// The password to use for authentication.
    #[arg(short, long, env = "AXIS_DEVICE_PASS", default_value = "pass")]
    pub pass: String,
    /// Accept self-signed HTTPS certificates.
    #[arg(long, env = "AXIS_DEVICE_HTTPS_SELF_SIGNED", value_parser = clap::builder::BoolishValueParser::new())]
    pub https_self_signed: bool,
}

impl Netloc {
    pub async fn connect(&self) -> anyhow::Result<rs4a_vapix::Client> {
        rs4a_vapix::ClientBuilder::new(self.host.clone())
            .plain_port(self.http_port)
            .secure_port(self.https_port)
            .username_password(&self.user, &self.pass)
            .with_inner(|b| b.danger_accept_invalid_certs(self.https_self_signed))
            .build()
            .await
    }

    pub async fn connect_anonymous(&self) -> anyhow::Result<rs4a_vapix::Client> {
        rs4a_vapix::ClientBuilder::new(self.host.clone())
            .plain_port(self.http_port)
            .secure_port(self.https_port)
            .with_inner(|b| b.danger_accept_invalid_certs(self.https_self_signed))
            .build()
            .await
    }
}

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub async fn exec(self) -> anyhow::Result<String> {
        match self.command {
            Commands::Restore(cmd) => cmd.exec().await,
            Commands::Init(cmd) => cmd.exec().await,
            Commands::Reinit(cmd) => cmd.exec().await,
            Commands::Upgrade(cmd) => cmd.exec().await,
            Commands::Completions(cmd) => {
                cmd.exec::<Self>()?;
                Ok(String::new())
            }
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Restore the device to a clean state (factory default)
    Restore(RestoreCommand),
    /// Initialize a device in setup mode
    Init(InitCommand),
    /// Restore and initialize the device to a known, useful state
    Reinit(ReinitCommand),
    /// Upgrade the device firmware
    Upgrade(UpgradeCommand),
    /// Generate shell completions
    ///
    /// Example: `device-manager completions zsh | source /dev/stdin`
    Completions(CompletionsCommand),
}
