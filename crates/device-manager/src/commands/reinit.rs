use crate::Netloc;

#[derive(Clone, Debug, clap::Args)]
pub struct ReinitCommand {
    #[command(flatten)]
    netloc: Netloc,
}

impl ReinitCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        super::init::require_root_user(&self.netloc)?;
        super::restore::restore(&self.netloc).await?;
        super::init::initialize(&self.netloc).await?;
        Ok(())
    }
}
