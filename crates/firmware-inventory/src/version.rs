use semver::Version;

/// Parse a version the way it appears in archive directory names, e.g. `12_11_68`.
pub(crate) fn version_from_underscore(s: &str) -> Option<Version> {
    coerce_firmware_version(&s.replace('_', ".")).ok()
}

/// Firmware versions are not semver: some have a fourth component, which is dropped.
pub(crate) fn coerce_firmware_version(s: &str) -> anyhow::Result<Version> {
    let mut parts = s.splitn(4, '.');
    let major = parts.next().unwrap_or_default().parse()?;
    let minor = parts.next().unwrap_or_default().parse()?;
    let patch = parts.next().unwrap_or_default().parse()?;
    Ok(Version::new(major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_from_underscore_drops_a_fourth_component() {
        assert_eq!(
            version_from_underscore("12_11_68"),
            Some(Version::new(12, 11, 68))
        );
        assert_eq!(
            version_from_underscore("9_80_3_9"),
            Some(Version::new(9, 80, 3))
        );
    }

    #[test]
    fn version_from_underscore_rejects_incomplete_versions() {
        assert_eq!(version_from_underscore("12_11"), None);
        assert_eq!(version_from_underscore("latest"), None);
    }
}
