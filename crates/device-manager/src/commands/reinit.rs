use anyhow::bail;

use crate::Netloc;

#[derive(Clone, Debug, clap::Args)]
pub struct ReinitCommand {
    #[command(flatten)]
    netloc: Netloc,
}

impl ReinitCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        if self.netloc.user != "root" {
            bail!(
                "The --user must be 'root' (got '{}'); the initial user is always 'root' \
                 because older firmware requires it",
                self.netloc.user
            );
        }
        super::restore::restore(&self.netloc).await?;
        super::init::initialize(&self.netloc).await?;
        Ok(())
    }
}
