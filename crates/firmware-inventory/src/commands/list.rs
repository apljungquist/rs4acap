use std::fmt::Write;

use anyhow::bail;
use semver::{Version, VersionReq};

use crate::db::Database;

#[derive(Clone, Debug, clap::Args)]
pub struct ListCommand {
    /// Glob pattern to match product names (default: all)
    pub product: Option<glob::Pattern>,
    /// Semver version requirement to filter versions
    pub version: Option<VersionReq>,
}

fn version_from_underscore(s: &str) -> Option<Version> {
    let dotted = s.replace('_', ".");
    let mut parts = dotted.splitn(4, '.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some(Version::new(major, minor, patch))
}

impl ListCommand {
    pub(crate) fn exec(self, db: &Database) -> anyhow::Result<String> {
        let Self {
            product,
            version: req,
        } = self;

        let index = db.read_index()?;
        let matching: Vec<_> = index
            .keys()
            .filter(|p| product.as_ref().map_or(true, |pat| pat.matches(p)))
            .collect();

        if matching.is_empty() {
            bail!("No indexed products found. Run update first.");
        }

        let mut out = String::new();
        for product in &matching {
            let versions = &index[product.as_str()];
            let mut entries: Vec<_> = versions
                .iter()
                .filter_map(|v| {
                    let semver = version_from_underscore(v)?;
                    if let Some(req) = &req {
                        if !req.matches(&semver) {
                            return None;
                        }
                    }
                    Some((v.as_str(), semver))
                })
                .collect();
            entries.sort_by(|(_, a), (_, b)| b.cmp(a));

            for (version_str, semver) in entries {
                let cached = if db.firmware_path(product, version_str).exists() {
                    " [cached]"
                } else {
                    ""
                };
                writeln!(out, "{product} {semver}{cached}")?;
            }
        }

        Ok(out)
    }
}
