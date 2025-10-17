use crate::db::Database;

#[derive(Clone, Debug, clap::Parser)]
pub struct DumpCommand {}

impl DumpCommand {
    pub async fn exec(self, db: &Database) -> anyhow::Result<()> {
        let db = serde_json::to_string_pretty(&db.read_devices()?)?;
        println!("{db}");
        Ok(())
    }
}
