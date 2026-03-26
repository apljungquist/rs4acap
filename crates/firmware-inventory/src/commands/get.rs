use std::fs;

use anyhow::{bail, Context};
use log::info;
use semver::VersionReq;

use crate::{authenticated_client, db::Database};

const MPQT_BASE_URL: &str = "https://www.axis.com/ftp/pub/axis/software/MPQT/";

#[derive(Clone, Debug, clap::Args)]
pub struct GetCommand {
    /// Glob pattern to match a product name (must match exactly one)
    product: glob::Pattern,
    /// Semver version requirement (e.g. "12", "^12.8", "<13")
    version: VersionReq,
}

impl GetCommand {
    pub async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<()> {
        let Self {
            product,
            version: req,
        } = self;

        let index = db.read_index()?;
        let matching: Vec<_> = index
            .products()
            .filter(|(name, _)| product.matches(name.as_str()))
            .map(|(name, _)| name)
            .collect();

        match matching.len() {
            0 => bail!(
                "No indexed products matched {product:?}. Run update-index first.",
            ),
            1 => {}
            n => bail!(
                "Product glob {product} matched {n} products: {matching:?}. Use a more specific pattern."
            ),
        }

        let product = matching[0];
        let entry = index.get(product).unwrap();

        let version = entry
            .versions
            .iter()
            .filter(|v| v.matches_req(&req))
            .max()
            .context("No versions matched the requirement")?;

        info!("Best match: {product} {version}");

        let path = db.firmware_path(product, version);

        if path.exists() {
            println!("{}", path.display());
            return Ok(());
        }

        if offline {
            bail!(
                "Firmware not cached and offline mode is enabled: {}",
                path.display()
            );
        }

        let cookie = db
            .read_cookie()?
            .context("No login session, please run the login command")?;
        let client = authenticated_client(cookie)?;

        let dir_name = version.to_dir_name();
        let url = format!("{MPQT_BASE_URL}{product}/{dir_name}/{product}_{dir_name}.bin");
        info!("Downloading {url}");

        let response = client
            .get(&url)
            .send()
            .await?
            .error_for_status()
            .context("Failed to download firmware")?;
        let bytes = response.bytes().await?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create firmware directory")?;
        }
        fs::write(&path, &bytes).context("Failed to write firmware file")?;

        println!("{}", path.display());
        Ok(())
    }
}
