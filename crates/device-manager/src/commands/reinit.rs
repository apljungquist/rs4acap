use super::init::Profile;
use crate::Netloc;

#[derive(Clone, Debug, clap::Args)]
pub struct ReinitCommand {
    #[command(flatten)]
    pub netloc: Netloc,
    #[arg(long, default_value_t)]
    pub profile: Profile,
}

impl ReinitCommand {
    pub async fn exec(self) -> anyhow::Result<String> {
        super::restore::restore(&self.netloc).await?;
        super::init::initialize(&self.netloc, &self.profile).await?;
        Ok(String::new())
    }
}
