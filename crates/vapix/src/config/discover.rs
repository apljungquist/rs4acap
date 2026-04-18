//! The [Device Configuration Discovery] API.
//!
//! [Device Configuration Discovery]: https://developer.axis.com/vapix/device-configuration/device-configuration-apis/#discovery

use std::collections::HashMap;

use anyhow::Context;
use reqwest::Method;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::http::{Error, HttpClient, Request};

const PATH: &str = "config/discover";

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
pub struct DiscoverData {
    pub framework_version: Version,
    pub apis: HashMap<String, HashMap<String, ApiVersionInfo>>,
    pub device: DeviceInfo,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceInfo {
    pub rest_openapi: String,
    pub rest_ui: String,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiVersionInfo {
    pub state: String,
    pub version: Version,
    pub doc: String,
    pub doc_html: String,
    pub model: String,
    pub rest_api: String,
    pub rest_openapi: String,
    pub rest_ui: String,
}

impl ApiVersionInfo {
    pub fn parse_state(&self) -> Result<ApiState, anyhow::Error> {
        self.state.parse()
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiState {
    Alpha,
    Beta,
    Released,
}

impl std::fmt::Display for ApiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alpha => write!(f, "alpha"),
            Self::Beta => write!(f, "beta"),
            Self::Released => write!(f, "released"),
        }
    }
}

impl std::str::FromStr for ApiState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "alpha" => Ok(Self::Alpha),
            "beta" => Ok(Self::Beta),
            "released" => Ok(Self::Released),
            _ => Err(anyhow::anyhow!("unrecognized API state '{s}'")),
        }
    }
}

#[derive(Debug)]
pub struct DiscoverRequest;

impl Default for DiscoverRequest {
    fn default() -> Self {
        Self
    }
}

fn parse_lossless<T>(text: &str) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let data: T = serde_json::from_str(text).context("parsing response")?;
    if cfg!(debug_assertions) {
        let expected: Value =
            serde_json::from_str(text).expect("already deserialized successfully");
        let actual: Value = serde_json::from_str(&serde_json::to_string(&data)?)?;
        debug_assert_eq!(actual, expected);
    }
    Ok(data)
}

impl DiscoverRequest {
    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<DiscoverData, Error<std::convert::Infallible>> {
        let request = Request::new(Method::GET, PATH.to_string());
        let response = client.execute(request).await.map_err(Error::Transport)?;
        let text = response
            .body
            .context("reading discover response")
            .map_err(Error::Decode)?;
        parse_lossless(&text).map_err(Error::Decode)
    }
}
