use crate::db::Database;

#[derive(Clone, Debug, clap::Parser)]
pub struct ExportCommand {}

impl ExportCommand {
    pub async fn exec(self, db: &Database) -> anyhow::Result<()> {
        let db = serde_json::to_string(&db.read_devices()?)?;
        println!("{db}");
        Ok(())
    }
}
