use anyhow::{bail, Context};
use log::info;

use crate::{
    authenticated_client,
    catalog::{self, CatalogEntry},
    db::{Database, Index},
    CATALOG_PATH, SOFTWARE_BASE_URL,
};

fn index_from_catalog(catalog: Vec<CatalogEntry>) -> Index {
    let mut index = Index::new();
    for entry in catalog {
        let CatalogEntry {
            product,
            revision,
            fileurl,
        } = entry;
        index
            .entry(normalize_product(product))
            .or_default()
            .insert(revision, fileurl);
    }
    index
}

#[derive(Clone, Debug, clap::Args)]
pub struct UpdateCommand {}

impl UpdateCommand {
    pub(crate) async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<String> {
        let Self {} = self;

        if offline {
            bail!("Cannot update index when offline");
        }

        let cookie = db
            .read_cookie()?
            .context("No login session, please run the login command")?;
        let client = authenticated_client(cookie)?;

        let url = format!("{SOFTWARE_BASE_URL}{CATALOG_PATH}");
        info!("Fetching firmware catalog from {url}");
        let xml = client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let entries = catalog::parse_catalog(&xml)?;
        let index = index_from_catalog(entries);

        db.write_index(&index)?;
        info!("Index updated");

        Ok(String::new())
    }
}
