#![forbid(unsafe_code)]

use clap::Parser;

use crate::commands::discover_devices::DiscoverDevicesCommand;

mod commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    DiscoverDevicesCommand::parse().exec().await
}
