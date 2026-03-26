use std::time::Duration;

use anyhow::{bail, Context};
use log::{info, warn};
use tokio::time::sleep;

use crate::{
    authenticated_client,
    db::{Database, Index, ProductEntry, ProductName},
    scrape,
    scrape::DirectoryEntry,
    version::FirmwareVersion,
};

const MPQT_BASE_URL: &str = "https://www.axis.com/ftp/pub/axis/software/MPQT/";
const REQUEST_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, clap::Args)]
pub struct UpdateCommand {
    /// Glob pattern to match product names
    product: glob::Pattern,
}

fn needs_update(scraped: &DirectoryEntry, index: &Index) -> bool {
    let name = ProductName::new(scraped.name.clone());
    match (scraped.last_modified, index.get(&name)) {
        (Some(scraped_ts), Some(existing)) => match existing.last_modified {
            Some(indexed_ts) => scraped_ts > indexed_ts,
            None => true,
        },
        _ => true,
    }
}

async fn fetch_versions(
    client: &reqwest::Client,
    index: &mut Index,
    products: &[DirectoryEntry],
) -> anyhow::Result<()> {
    for scraped in products {
        let name = ProductName::new(scraped.name.clone());

        sleep(REQUEST_INTERVAL).await;

        let url = format!("{MPQT_BASE_URL}{name}/");
        info!("Fetching versions for {name}");

        let html = client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let dirs = scrape::parse_directory_listing(&html);
        let versions: Vec<FirmwareVersion> = dirs
            .into_iter()
            .filter(|e| e.name != "latest")
            .filter_map(|e| {
                let version = FirmwareVersion::from_dir_name(&e.name);
                if version.is_none() {
                    warn!("Failed to parse version {name}/{}", e.name);
                }
                version
            })
            .collect();
        info!("Found {} version(s) for {name}", versions.len());
        index.insert(
            name,
            ProductEntry {
                last_modified: scraped.last_modified,
                versions,
            },
        );
    }
    Ok(())
}

impl UpdateCommand {
    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
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

        let mut index = db.read_index()?;

        let matching: Vec<_> = all_products
            .into_iter()
            .filter(|p| product.matches(&p.name))
            .collect();

        if matching.is_empty() {
            bail!("No products matched the pattern {product:?}");
        }

        let (stale, up_to_date): (Vec<_>, Vec<_>) =
            matching.into_iter().partition(|p| needs_update(p, &index));

        info!(
            "{} product(s) to update, {} already up-to-date",
            stale.len(),
            up_to_date.len()
        );

        let result = tokio::select! {
            r = fetch_versions(&client, &mut index, &stale) => r,
            _ = tokio::signal::ctrl_c() => {
                warn!("Interrupted");
                Err(anyhow::anyhow!("Interrupted by Ctrl+C"))
            }
        };

        db.write_index(&index)?;

        match result {
            Ok(()) => {
                info!("Index updated");
                Ok(())
            }
            Err(e) => {
                warn!("Saved partial index before exiting");
                Err(e)
            }
        }
    }
}
