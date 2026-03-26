use std::fmt;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FirmwareVersion(Version);

impl FirmwareVersion {
    pub fn to_dir_name(&self) -> String {
        if self.0.build.is_empty() {
            // New format: "{major}.{minor}.{patch}"
            format!("{}_{}_{}", self.0.major, self.0.minor, self.0.patch)
        } else {
            // Old format: "{year}.{release}.{major}.{minor}"
            // Major x0: normal
            // Major x5: PFW
            format!(
                "{}_{}_{}_{}",
                self.0.major, self.0.minor, self.0.patch, self.0.build
            )
        }
    }

    pub fn from_dir_name(s: &str) -> Option<Self> {
        let mut parts = s.splitn(4, '_');
        let major: u64 = parts.next()?.parse().ok()?;
        let minor: u64 = parts.next()?.parse().ok()?;
        let patch: u64 = parts.next()?.parse().ok()?;
        let mut v = Version::new(major, minor, patch);
        if let Some(build) = parts.next() {
            let _: u64 = build.parse().ok()?;
            v.build = semver::BuildMetadata::new(build).ok()?;
        }
        let v = Self(v);
        debug_assert_eq!(v.to_dir_name(), s);
        Some(v)
    }

    ///
    pub fn matches_req(&self, req: &VersionReq) -> bool {
        let mut v = self.0.clone();
        v.pre = semver::Prerelease::EMPTY;
        req.matches(&v)
    }
}

impl fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.build.is_empty() {
            write!(f, "{}.{}.{}", self.0.major, self.0.minor, self.0.patch)
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordering() {
        let v0 = FirmwareVersion::from_dir_name("5_85_4").unwrap();
        let v2 = FirmwareVersion::from_dir_name("5_85_4_2").unwrap();
        assert!(v0 < v2);
    }

    #[test]
    fn test_rejects_non_numeric() {
        assert!(FirmwareVersion::from_dir_name("latest").is_none());
        assert!(FirmwareVersion::from_dir_name("abc_def_ghi").is_none());
    }

    #[test]
    fn test_round_trip_dir_name() {
        assert_eq!(
            FirmwareVersion::from_dir_name("5_85_4")
                .unwrap()
                .to_dir_name(),
            "5_85_4"
        );
        assert_eq!(
            FirmwareVersion::from_dir_name("5_85_4_2")
                .unwrap()
                .to_dir_name(),
            "5_85_4_2"
        );
    }

    #[test]
    fn test_round_trip() {
        assert_eq!(
            FirmwareVersion::from_dir_name("5.85.4")
                .unwrap()
                .to_dir_name(),
            "585.4"
        );
        assert_eq!(
            FirmwareVersion::from_dir_name("5.85.4.2")
                .unwrap()
                .to_dir_name(),
            "5.85.4.2"
        );
    }
}
