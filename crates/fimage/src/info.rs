//! The specification of the `info.json` member of a firmware image.
//!
//! There is no published schema; the fields are inferred from inspecting images.

use std::str::FromStr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The content of the `info.json` member of a firmware image.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ImageInfo {
    pub release: String,
    pub build_nbr: String,
    pub part_nbr: String,
    /// Seconds since the Unix epoch.
    pub build_time: i64,
    pub signing_domain: String,
    /// Observed only in AXIS OS 13 images, e.g. as "preview-13".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
    /// Absent from images for AXIS OS before 11.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgradeable_from: Option<Vec<String>>,
    pub products: Vec<Product>,
}

impl ImageInfo {
    pub fn try_build_time(&self) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::from_timestamp(self.build_time, 0).ok_or(anyhow!(
            "Expected a UNIX timestamp, got {}",
            self.build_time
        ))
    }
}

/// A product that a firmware image can be installed on.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Product {
    pub brand: String,
    pub prod_nbr: String,
    #[serde(rename = "HardwareID")]
    pub hardware_id: String,
    pub prod_type: String,
    pub prod_full_name: String,
    pub prod_short_name: String,
    /// Observed only in images for products that come in variants, e.g. as
    /// "7mm" for a thermal camera lens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prod_variant: Option<String>,
}

/// Deserialize JSON, asserting in debug builds that no information is lost.
fn parse_lossless<T>(text: &str) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let data: T = serde_json::from_str(text)?;
    if cfg!(debug_assertions) {
        let expected: Value =
            serde_json::from_str(text).expect("already deserialized successfully");
        let actual: Value = serde_json::to_value(&data)?;
        assert_eq!(actual, expected);
    }
    Ok(data)
}

impl FromStr for ImageInfo {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_lossless(s)
    }
}
