use std::{collections::BTreeSet, fmt::Write, fs};

use anyhow::{bail, Context};
use log::info;
use semver::Version;

use crate::{
    authenticated_client,
    db::Database,
    track::{self, Selector},
    version, SOFTWARE_BASE_URL,
};

#[derive(Clone, Debug, clap::Args)]
#[command(group(clap::ArgGroup::new("get_selector").required(true).args(["version", "track"])))]
pub struct GetCommand {
    /// Glob patterns to match product names (each must match exactly one)
    #[clap(required = true)]
    pub products: Vec<glob::Pattern>,
    #[command(flatten)]
    pub selector: Selector,
}

async fn download(
    client: &reqwest::Client,
    db: &Database,
    product: &str,
    version: &str,
    fileurl: &str,
) -> anyhow::Result<()> {
    let url = format!("{SOFTWARE_BASE_URL}{}", fileurl.trim_start_matches('/'));
    info!("Downloading {url}");

    let response = client
        .get(&url)
        .send()
        .await?
        .error_for_status()
        .context("Failed to download firmware")?;
    let bytes = response.bytes().await?;

    let path = db.firmware_path(product, version);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create firmware directory")?;
    }
    fs::write(&path, &bytes).context("Failed to write firmware file")
}

impl GetCommand {
    pub(crate) async fn exec(self, db: &Database, offline: bool) -> anyhow::Result<String> {
        let Self { products, selector } = self;

        let index = db.read_index()?;

        // Resolve every pattern before fetching anything, so that a pattern that cannot be
        // satisfied is reported before any bytes are spent on the ones before it.
        let mut resolved: Vec<(String, String, String, Version)> = Vec::new();
        for pattern in &products {
            let matching: Vec<_> = index.iter().filter(|(p, _)| pattern.matches(p)).collect();
            let (product, versions) = match matching.as_slice() {
                [] => bail!("No indexed products matched {pattern:?}. Run update first."),
                [pair] => *pair,
                pairs => {
                    let names: Vec<_> = pairs.iter().map(|(p, _)| p.as_str()).collect();
                    bail!(
                        "Product glob {pattern} matched {} products: {names:?}. Use a more specific pattern.",
                        names.len()
                    )
                }
            };

            let candidates = version::parse_versions(versions);
            let semvers: Vec<_> = candidates.iter().map(|(_, _, v)| v.clone()).collect();

            let Some(req) = selector.resolve(&semvers) else {
                let available = track::available_tracks(&semvers);
                let available = if available.is_empty() {
                    "none".to_string()
                } else {
                    available.join(", ")
                };
                bail!(
                    "{product} has no firmware on {}. Tracks with firmware for {product}: {available}",
                    selector.describe()
                );
            };

            let (version_str, fileurl, semver) = candidates
                .into_iter()
                .filter(|(_, _, v)| req.matches(v))
                .max_by(|(_, _, a), (_, _, b)| a.cmp(b))
                .with_context(|| {
                    format!("No version of {product} matched {}", selector.describe())
                })?;

            info!("Best match: {product} {semver} ({version_str})");
            resolved.push((
                product.clone(),
                version_str.to_string(),
                fileurl.to_string(),
                semver,
            ));
        }

        // A pattern may be given twice, or two patterns may resolve to the same firmware; fetch it
        // once and report it once per pattern.
        let missing: BTreeSet<(&str, &str, &str)> = resolved
            .iter()
            .filter(|(p, v, _, _)| !db.firmware_path(p, v).exists())
            .map(|(p, v, f, _)| (p.as_str(), v.as_str(), f.as_str()))
            .collect();

        if !missing.is_empty() {
            if offline {
                let paths: Vec<_> = missing
                    .iter()
                    .map(|(p, v, _)| db.firmware_path(p, v).display().to_string())
                    .collect();
                bail!(
                    "Firmware not cached and offline mode is enabled: {}",
                    paths.join(", ")
                );
            }

            let cookie = db
                .read_cookie()?
                .context("No login session, please run the login command")?;
            let client = authenticated_client(cookie)?;

            for (product, version_str, fileurl) in missing {
                download(&client, db, product, version_str, fileurl).await?;
            }
        }

        let mut out = String::new();
        for (product, version_str, _, _) in &resolved {
            writeln!(out, "{}", db.firmware_path(product, version_str).display())?;
        }
        Ok(out)
    }
}
