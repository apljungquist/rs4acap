use anyhow::{bail, Context};
use log::info;

use crate::{authenticated_client, db::Database, scrape};

const MPQT_BASE_URL: &str = "https://www.axis.com/ftp/pub/axis/software/MPQT/";

#[derive(Clone, Debug, clap::Args)]
pub struct UpdateCommand {
    /// Glob pattern to match product names
    pub product: glob::Pattern,
}

impl UpdateCommand {
    pub(crate) async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<String> {
        let Self { product } = self;

        if offline {
            bail!("Cannot update index when offline");
        }

        let cookie = db
            .read_cookie()?
            .context("No login session, please run the login command")?;
        let client = authenticated_client(cookie)?;

        // Fetch product listing
        info!("Fetching product listing from {MPQT_BASE_URL}");
        let html = client
            .get(MPQT_BASE_URL)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let all_products = scrape::parse_directory_listing(&html);

        let matching: Vec<_> = all_products
            .into_iter()
            .filter(|p| product.matches(p))
            .collect();

        if matching.is_empty() {
            bail!("No products matched the pattern {product:?}");
        }

        info!("Found {} matching product(s)", matching.len());

        let mut index = db.read_index()?;

        for product in &matching {
            let url = format!("{MPQT_BASE_URL}{product}/");
            info!("Fetching versions for {product}");
            let html = client
                .get(&url)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            let versions = scrape::parse_directory_listing(&html);
            info!("Found {} version(s) for {product}", versions.len());
            index.insert(product.clone(), versions);
        }

        db.write_index(&index)?;
        info!("Index updated");

        Ok(String::new())
    }
}
