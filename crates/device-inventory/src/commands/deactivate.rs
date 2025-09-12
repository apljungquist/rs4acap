use log::{debug, warn};

#[derive(Clone, Debug, clap::Parser)]
pub struct DeactivateCommand {
    #[arg(long)]
    dry_run: bool,
}

impl DeactivateCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        if rs4a_dut::Device::from_fs()?.is_some() {
            if self.dry_run {
                warn!("Would clear the active device on filesystem");
            } else {
                debug!("Clearing the active device on filesystem");
                rs4a_dut::Device::clear_fs()?;
            }
        }
        if rs4a_dut::Device::from_env()?.is_some() {
            for key in rs4a_dut::Device::clear_env() {
                println!("unset {key}");
            }
        }

        Ok(())
    }
}
