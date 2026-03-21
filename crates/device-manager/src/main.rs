#![forbid(unsafe_code)]

mod commands;

use clap::{Parser, Subcommand};
use rs4a_bin_utils::completions_command::CompletionsCommand;
use url::Host;

use crate::commands::{init::InitCommand, reinit::ReinitCommand, restore::RestoreCommand};

#[derive(Clone, Debug, Parser)]
pub struct Netloc {
    /// Hostname or IP address of the device.
    #[arg(long, value_parser = url::Host::parse, env = "AXIS_DEVICE_IP")]
    pub host: Host,
    /// Override the default port for HTTP.
    #[clap(long, env = "AXIS_DEVICE_HTTP_PORT")]
    pub http_port: Option<u16>,
    /// Override the default port for HTTPS.
    #[clap(long, env = "AXIS_DEVICE_HTTPS_PORT")]
    pub https_port: Option<u16>,
    /// The username to use for authentication.
    #[clap(short, long, env = "AXIS_DEVICE_USER", default_value = "root")]
    pub user: String,
    /// The password to use for authentication.
    #[clap(short, long, env = "AXIS_DEVICE_PASS", default_value = "pass")]
    pub pass: String,
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    async fn exec(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Restore(cmd) => cmd.exec().await?,
            Commands::Init(cmd) => cmd.exec().await?,
            Commands::Reinit(cmd) => cmd.exec().await?,
            Commands::Completions(cmd) => cmd.exec::<Self>()?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Restore the device to a clean state (factory default).
    Restore(RestoreCommand),
    /// Initialize a device in setup mode.
    Init(InitCommand),
    /// Restore and initialize the device to a known, useful state.
    Reinit(ReinitCommand),
    /// Generate shell completions.
    ///
    /// Example: `device-manager completions zsh | source /dev/stdin`.
    Completions(CompletionsCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut guard = rs4a_bin_utils::logger::init();
    Cli::parse().exec().await?;
    guard.disarm();
    Ok(())
}
