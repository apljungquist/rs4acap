use std::fs;

use anyhow::{bail, Context};
use log::info;
use semver::{Version, VersionReq};

use crate::{authenticated_client, db::Database};

const MPQT_BASE_URL: &str = "https://www.axis.com/ftp/pub/axis/software/MPQT/";

#[derive(Clone, Debug, clap::Args)]
pub struct GetCommand {
    /// Glob pattern to match a product name (must match exactly one)
    pub product: glob::Pattern,
    /// Semver version requirement (e.g. "12", "^12.8", "<13")
    pub version: VersionReq,
}

fn version_from_underscore(s: &str) -> Option<Version> {
    let dotted = s.replace('_', ".");
    coerce_firmware_version(&dotted).ok()
}

fn coerce_firmware_version(s: &str) -> anyhow::Result<Version> {
    let mut parts = s.splitn(4, '.');
    let major = parts.next().unwrap_or_default().parse()?;
    let minor = parts.next().unwrap_or_default().parse()?;
    let patch = parts.next().unwrap_or_default().parse()?;
    Ok(Version::new(major, minor, patch))
}

impl GetCommand {
    pub(crate) async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<String> {
        let Self {
            product,
            version: req,
        } = self;

        let index = db.read_index()?;
        let matching: Vec<_> = index.keys().filter(|p| product.matches(p)).collect();

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
        let versions = &index[product];

        let best = versions
            .iter()
            .filter_map(|v| {
                let semver = version_from_underscore(v)?;
                if req.matches(&semver) {
                    Some((v.clone(), semver))
                } else {
                    None
                }
            })
            .max_by(|(_, a), (_, b)| a.cmp(b));

        let (version_str, semver) = best.context("No versions matched the requirement")?;

        info!("Best match: {product} {semver} ({})", version_str);

        let path = db.firmware_path(product, &version_str);

        if path.exists() {
            return Ok(format!("{}\n", path.display()));
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

        let url = format!("{MPQT_BASE_URL}{product}/{version_str}/{product}_{version_str}.bin");
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

        Ok(format!("{}\n", path.display()))
    }
}
