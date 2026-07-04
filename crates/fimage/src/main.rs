use clap::Parser;

fn main() -> anyhow::Result<()> {
    let mut guard = rs4a_bin_utils::logger::init();
    let out = rs4a_fimage::Cli::parse().exec()?;
    if !out.is_empty() {
        print!("{out}");
    }
    guard.disarm();
    Ok(())
}
