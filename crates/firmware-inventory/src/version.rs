use std::collections::BTreeMap;

use semver::Version;

/// Coerce a firmware revision into a [`Version`] by keeping only its first three components.
///
/// Firmware revisions aren't strict semver:
/// - some have more than three components (a build number tail, dropped here), and
/// - some have fewer (rejected, since major/minor/patch can't all be inferred).
// TODO: Improve the support for non-semver versions
pub(crate) fn coerce_firmware_version(s: &str) -> anyhow::Result<Version> {
    let mut parts = s.splitn(4, '.');
    let major = parts.next().unwrap_or_default().parse()?;
    let minor = parts.next().unwrap_or_default().parse()?;
    let patch = parts.next().unwrap_or_default().parse()?;
    Ok(Version::new(major, minor, patch))
}

/// Pair each indexed revision with its semver form and the path to its firmware file.
///
/// NB: drops unparseable revisions.
pub(crate) fn parse_versions(versions: &BTreeMap<String, String>) -> Vec<(&str, &str, Version)> {
    versions
        .iter()
        .filter_map(|(revision, fileurl)| {
            let semver = coerce_firmware_version(revision).ok()?;
            Some((revision.as_str(), fileurl.as_str(), semver))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coerce_firmware_version_drops_a_fourth_component() {
        assert_eq!(
            coerce_firmware_version("12.11.68").unwrap(),
            Version::new(12, 11, 68)
        );
        assert_eq!(
            coerce_firmware_version("9.80.3.9").unwrap(),
            Version::new(9, 80, 3)
        );
    }

    #[test]
    fn coerce_firmware_version_rejects_incomplete_versions() {
        assert!(coerce_firmware_version("12.11").is_err());
        assert!(coerce_firmware_version("latest").is_err());
    }
}
