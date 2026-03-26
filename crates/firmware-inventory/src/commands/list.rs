use anyhow::bail;
use semver::VersionReq;

use crate::db::Database;

#[derive(Clone, Debug, clap::Args)]
pub struct ListCommand {
    /// Glob pattern to match product names (default: all)
    product: Option<glob::Pattern>,
    /// Semver version requirement to filter versions
    version: Option<VersionReq>,
}

impl ListCommand {
    pub fn exec(self, db: &Database) -> anyhow::Result<()> {
        let Self {
            product,
            version: req,
        } = self;

        let index = db.read_index()?;
        let matching: Vec<_> = index
            .products()
            .filter(|(name, _)| {
                product
                    .as_ref()
                    .map_or(true, |pat| pat.matches(name.as_str()))
            })
            .collect();

        if matching.is_empty() {
            bail!("No indexed products found. Run update first.");
        }

        for (product_name, product_entry) in &matching {
            let mut versions: Vec<_> = product_entry
                .versions
                .iter()
                .filter(|v| req.as_ref().map_or(true, |req| v.matches_req(req)))
                .collect();
            versions.sort_unstable_by(|a, b| b.cmp(a));

            for version in versions {
                let cached = if db.firmware_path(product_name, version).exists() {
                    " [cached]"
                } else {
                    ""
                };
                println!("{product_name} {version}{cached}");
            }
        }

        Ok(())
    }
}
