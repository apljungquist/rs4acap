//! The [Systemready API].
//!
//! [Systemready API]: https://developer.axis.com/vapix/network-video/systemready-api/

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    http::HttpClient,
    protocol_helpers::{http::Error, json_rpc, json_rpc_http},
};

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

// TODO: Consider parsing `bootid` as a UUID
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previewmode: Option<String>,
    /// New in 1.5
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrasepolicy: Option<String>,
}

impl SystemreadyData {
    // TODO: Consider parsing `uptime` eagerly
    /// Parse the uptime field as a duration in seconds.
    pub fn try_uptime(&self) -> Result<Option<Duration>, std::num::ParseIntError> {
        self.uptime
            .as_deref()
            .map(|s| s.parse::<u64>().map(Duration::from_secs))
            .transpose()
    }

    pub fn parse_preview_mode(&self) -> Result<Option<Duration>, std::num::ParseIntError> {
        self.previewmode
            .as_deref()
            .map(|s| s.parse::<u64>().map(Duration::from_secs))
            .transpose()
    }
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

const PATH: &str = "axis-cgi/systemready.cgi";

impl SystemReadyRequest {
    pub fn new() -> Self {
        Self {
            api_version: "1",
            method: "systemready",
            params: None,
        }
    }

    pub fn timeout(mut self, timeout: u16) -> Self {
        self.params.get_or_insert(SystemReadyParams { timeout });
        self
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<SystemreadyData, Error<json_rpc::Error>> {
        json_rpc_http::send_request(client, PATH, &self).await
    }
}

impl Default for SystemReadyRequest {
    fn default() -> Self {
        Self::new()
    }
}
