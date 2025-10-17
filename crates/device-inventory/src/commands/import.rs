use crate::{db::Database, db_vlt};

#[derive(Clone, Debug, clap::Parser)]
pub struct ImportCommand {}

impl ImportCommand {
    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
        db_vlt::import(db, offline).await?;
        Ok(())
    }
}
