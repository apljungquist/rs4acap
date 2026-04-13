use crate::Netloc;

#[derive(Clone, Debug, clap::Args)]
pub struct ReinitCommand {
    #[command(flatten)]
    netloc: Netloc,
}

impl ReinitCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        super::restore::restore(&self.netloc).await?;
        super::init::initialize(&self.netloc).await?;
        Ok(())
    }
}
