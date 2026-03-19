use crate::{db::Database, db_vlt};

#[derive(Clone, Debug, clap::Parser)]
pub struct SyncCommand {}

impl SyncCommand {
    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
        db_vlt::sync(db, offline).await?;
        Ok(())
    }
}
