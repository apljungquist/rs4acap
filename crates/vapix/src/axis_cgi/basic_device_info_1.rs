//! The [Basic device information] API.
//!
//! [Basic device information]: https://developer.axis.com/vapix/network-video/basic-device-information/

use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use anyhow::bail;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::json_rpc_http::{JsonRpcHttp, JsonRpcHttpLossless};

fn serialize_none_as_empty_string<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Display,
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_str(&v.to_string()),
        None => serializer.serialize_str(""),
    }
}

fn deserialize_empty_string_as_none<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if s.is_empty() {
        Ok(None)
    } else {
        T::from_str(&s).map(Some).map_err(serde::de::Error::custom)
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllUnrestrictedPropertiesData {
    pub property_list: UnrestrictedProperties,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UnrestrictedProperties {
    pub brand: String,
    pub build_date: String,
    #[serde(rename = "HardwareID")]
    pub hardware_id: String,
    pub prod_full_name: String,
    pub prod_nbr: String,
    pub prod_short_name: String,
    pub prod_type: String,
    /// Appears to not be set on 8.45.4.5, as well as more recent versions of AXIS OS.
    #[serde(
        serialize_with = "serialize_none_as_empty_string",
        deserialize_with = "deserialize_empty_string_as_none"
    )]
    pub prod_variant: Option<String>,
    pub serial_number: String,
    pub version: String,
    #[serde(rename = "WebURL")]
    pub web_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAllUnrestrictedPropertiesRequest {
    api_version: &'static str,
    method: &'static str,
}

impl Default for GetAllUnrestrictedPropertiesRequest {
    fn default() -> Self {
        Self {
            api_version: "1.0",
            method: "getAllUnrestrictedProperties",
        }
    }
}

impl JsonRpcHttp for GetAllUnrestrictedPropertiesRequest {
    type Data = AllUnrestrictedPropertiesData;
    const PATH: &'static str = "axis-cgi/basicdeviceinfo.cgi";
}

impl JsonRpcHttpLossless for GetAllUnrestrictedPropertiesRequest {
    type Data = AllUnrestrictedPropertiesData;
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    Aarch64,
    Armv7hf,
    Armv7l,
    Mips,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SocSerialNumber(u128);

impl Display for SocSerialNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:08X}-{:08X}-{:08X}-{:08X}",
            (self.0 >> 96) as u32,
            (self.0 >> 64) as u32,
            (self.0 >> 32) as u32,
            self.0 as u32
        )
    }
}

impl FromStr for SocSerialNumber {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 4 {
            bail!("Expected 4 segments, got {}", parts.len());
        }
        let mut bits: u128 = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.len() != 8 {
                bail!(
                    "Expected each segment to be 8 characters long, but segment {} is {}",
                    i + 1,
                    part.len()
                );
            }
            let segment = u32::from_str_radix(part, 16)?;
            bits |= (segment as u128) << (96 - i * 32);
        }
        Ok(Self(bits))
    }
}

impl Serialize for SocSerialNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SocSerialNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SocSerialNumber::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RestrictedProperties {
    pub architecture: Architecture,
    /// Appears to not be set on 8.45.4.5
    #[serde(
        serialize_with = "serialize_none_as_empty_string",
        deserialize_with = "deserialize_empty_string_as_none"
    )]
    pub soc_serial_number: Option<SocSerialNumber>,
    // TODO: Consider enumerating all known variants
    pub soc: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AllProperties {
    #[serde(flatten)]
    pub unrestricted: UnrestrictedProperties,
    #[serde(flatten)]
    pub restricted: RestrictedProperties,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllPropertiesData {
    pub property_list: AllProperties,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAllPropertiesRequest {
    api_version: &'static str,
    method: &'static str,
}

impl Default for GetAllPropertiesRequest {
    fn default() -> Self {
        Self {
            api_version: "1.0",
            method: "getAllProperties",
        }
    }
}

impl JsonRpcHttp for GetAllPropertiesRequest {
    type Data = AllPropertiesData;
    const PATH: &'static str = "axis-cgi/basicdeviceinfo.cgi";
}

impl JsonRpcHttpLossless for GetAllPropertiesRequest {
    type Data = AllPropertiesData;
}

pub fn get_all_properties() -> GetAllPropertiesRequest {
    GetAllPropertiesRequest::default()
}

pub fn get_all_unrestricted_properties() -> GetAllUnrestrictedPropertiesRequest {
    GetAllUnrestrictedPropertiesRequest::default()
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn soc_serial_number_from_to_string_roundtrip() {
        let expected = "00000000-00000000-032CDEEE-01349999";
        let actual = SocSerialNumber::from_str(expected).unwrap().to_string();
        assert_eq!(expected, actual);
    }

    #[test]
    fn invalid_soc_serial_number_strings_fail_to_parse() {
        expect!("Expected each segment to be 8 characters long, but segment 4 is 7").assert_eq(
            &SocSerialNumber::from_str("00000000-00000000-00000000-0000000")
                .unwrap_err()
                .to_string(),
        );
        expect!("Expected 4 segments, got 3").assert_eq(
            &SocSerialNumber::from_str("00000000-00000000-0000000000000000")
                .unwrap_err()
                .to_string(),
        );
        expect!("invalid digit found in string").assert_eq(
            &SocSerialNumber::from_str("00000000-00000000-00000000-0000000G")
                .unwrap_err()
                .to_string(),
        );
        // Note that lower-case hex letters are accepted even though they never appear in responses from the API
    }
}
