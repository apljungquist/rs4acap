use crate::Netloc;

#[derive(Clone, Debug, clap::Args)]
pub struct ReinitCommand {
    #[command(flatten)]
    pub netloc: Netloc,
}

impl ReinitCommand {
    pub async fn exec(self) -> anyhow::Result<String> {
        super::init::require_root_user(&self.netloc)?;
        super::restore::restore(&self.netloc).await?;
        super::init::initialize(&self.netloc).await?;
        Ok(String::new())
    }
}
