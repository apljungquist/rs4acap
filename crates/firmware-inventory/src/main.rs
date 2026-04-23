use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut guard = rs4a_bin_utils::logger::init();
    let out = rs4a_firmware_inventory::Cli::parse().exec().await?;
    if !out.is_empty() {
        print!("{out}");
    }
    guard.disarm();
    Ok(())
}
