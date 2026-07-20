use acap_build::Cli;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let eap_file_path = Cli::parse().exec()?;
    println!("{eap_file_path}");

    Ok(())
}
