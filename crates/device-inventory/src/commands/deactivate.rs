#[derive(Clone, Debug, clap::Parser)]
pub struct DeactivateCommand {}

impl DeactivateCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        if rs4a_dut::Device::from_env()?.is_some() {
            for key in rs4a_dut::Device::clear_env() {
                println!("unset {key}");
            }
        }

        Ok(())
    }
}
