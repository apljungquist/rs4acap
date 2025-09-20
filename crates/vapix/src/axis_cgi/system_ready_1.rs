//! The [Systemready API].
//!
//! [Systemready API]: https://developer.axis.com/vapix/network-video/systemready-api/

use serde::{Deserialize, Serialize};

use crate::json_rpc_http::JsonRpcHttp;

fn deserialize_english_boolean<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s: &str = serde::de::Deserialize::deserialize(deserializer)?;
    match s {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(serde::de::Error::custom("invalid boolean value")),
    }
}

fn serialize_english_boolean<S>(b: &bool, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match b {
        true => s.serialize_str("yes"),
        false => s.serialize_str("no"),
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemreadyData {
    #[serde(
        deserialize_with = "deserialize_english_boolean",
        serialize_with = "serialize_english_boolean"
    )]
    pub needsetup: bool,
    #[serde(
        deserialize_with = "deserialize_english_boolean",
        serialize_with = "serialize_english_boolean"
    )]
    pub systemready: bool,
    // TODO: Extract uptime and boot id
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemReadyParams {
    timeout: u16,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemReadyRequest {
    api_version: &'static str,
    method: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<SystemReadyParams>,
}

impl Default for SystemReadyRequest {
    fn default() -> Self {
        Self {
            method: "systemready",
            api_version: "1",
            params: None,
        }
    }
}

impl SystemReadyRequest {
    pub fn timeout(mut self, timeout: u16) -> Self {
        self.params.get_or_insert(SystemReadyParams { timeout });
        self
    }
}

impl JsonRpcHttp for SystemReadyRequest {
    type Data = SystemreadyData;
    const PATH: &'static str = "axis-cgi/systemready.cgi";
}

pub fn system_ready() -> SystemReadyRequest {
    SystemReadyRequest::default()
}
