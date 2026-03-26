use crate::db::Database;

#[derive(Clone, Debug, clap::Args)]
pub struct DumpCommand {}

impl DumpCommand {
    pub fn exec(self, db: &Database) -> anyhow::Result<()> {
        let index = serde_json::to_string_pretty(&db.read_index()?)?;
        println!("{index}");
        Ok(())
    }
}
