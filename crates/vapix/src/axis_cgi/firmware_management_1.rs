//! The [Firmware Management API].
//!
//! [Firmware Management API]: https://developer.axis.com/vapix/network-video/firmware-management-api/

use std::{
    convert::Infallible,
    fmt::{Display, Formatter},
};

use anyhow::Context;
use log::trace;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
    http::{Error, HttpClient, Request},
    json_rpc_http::{from_response, from_response_lossless, JsonRpcHttp, JsonRpcHttpLossless},
};

const PATH: &str = "axis-cgi/firmwaremanagement.cgi";

#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FactoryDefaultMode {
    None,
    Soft,
    Hard,
}

impl Display for FactoryDefaultMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Soft => write!(f, "soft"),
            Self::Hard => write!(f, "hard"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FactoryDefaultParams {
    factory_default_mode: FactoryDefaultMode,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FactoryDefaultData {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FactoryDefaultRequest {
    api_version: &'static str,
    method: &'static str,
    params: FactoryDefaultParams,
}

impl FactoryDefaultRequest {
    pub fn new(mode: FactoryDefaultMode) -> Self {
        Self {
            api_version: "1.0",
            method: "factoryDefault",
            params: FactoryDefaultParams {
                factory_default_mode: mode,
            },
        }
    }
}

impl JsonRpcHttp for FactoryDefaultRequest {
    type Data = FactoryDefaultData;
    const PATH: &'static str = PATH;
}

impl JsonRpcHttpLossless for FactoryDefaultRequest {
    type Data = FactoryDefaultData;
}

#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AutoCommit {
    Never,
    Boot,
    Started,
    Default,
}

impl Display for AutoCommit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Never => write!(f, "never"),
            Self::Boot => write!(f, "boot"),
            Self::Started => write!(f, "started"),
            Self::Default => write!(f, "default"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum AutoRollback {
    Never,
    Minutes(u32),
    Default,
}

impl Serialize for AutoRollback {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Never => serializer.serialize_str("never"),
            Self::Minutes(m) => serializer.serialize_str(&m.to_string()),
            Self::Default => serializer.serialize_str("default"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpgradeParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    factory_default_mode: Option<FactoryDefaultMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_commit: Option<AutoCommit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_rollback: Option<AutoRollback>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeData {
    pub firmware_version: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeRequestJson {
    api_version: &'static str,
    method: &'static str,
    params: UpgradeParams,
}

pub struct UpgradeRequest {
    json: UpgradeRequestJson,
    bin: Vec<u8>,
}

impl UpgradeRequest {
    pub fn new(bin: Vec<u8>) -> Self {
        Self {
            json: UpgradeRequestJson {
                api_version: "1.0",
                method: "upgrade",
                params: UpgradeParams {
                    factory_default_mode: None,
                    auto_commit: None,
                    auto_rollback: None,
                },
            },
            bin,
        }
    }

    pub fn factory_default_mode(mut self, mode: FactoryDefaultMode) -> Self {
        self.json.params.factory_default_mode = Some(mode);
        self
    }

    pub fn auto_commit(mut self, commit: AutoCommit) -> Self {
        self.json.params.auto_commit = Some(commit);
        self
    }

    pub fn auto_rollback(mut self, rollback: AutoRollback) -> Self {
        self.json.params.auto_rollback = Some(rollback);
        self
    }

    fn build_multipart_body(json: &[u8], firmware: &[u8], boundary: &str) -> Vec<u8> {
        let mut body = Vec::new();

        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());

        body.extend_from_slice(b"Content-Disposition: form-data; name=\"data\"\r\n");
        body.extend_from_slice(b"Content-Type: application/json\r\n\r\n");
        body.extend_from_slice(json);
        body.extend_from_slice(b"\r\n");

        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());

        body.extend_from_slice(b"Content-Disposition: form-data; name=\"firmwareImage\"; filename=\"firmware.bin\"\r\n");
        body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
        body.extend_from_slice(firmware);
        body.extend_from_slice(b"\r\n");

        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        body
    }

    pub async fn send(self, client: &impl HttpClient) -> Result<UpgradeData, Error<Infallible>> {
        let boundary = "----FormBoundaryS6untlhO8j7poXo";

        let json = serde_json::to_string(&self.json)
            .context("serialize request failed")
            .map_err(Error::Request)?;

        let body = Self::build_multipart_body(json.as_bytes(), &self.bin, boundary);

        let request =
            Request::multipart_form_data(Method::POST, PATH.to_string(), boundary).body_bytes(body);

        let response = client.execute(request).await.map_err(Error::Transport)?;

        let status = response.status;

        let text = response.body;

        if cfg!(debug_assertions) {
            if let Ok(text) = text.as_deref() {
                trace!("Received {status}: {text}");
            }
        }

        match cfg!(debug_assertions) {
            true => from_response_lossless(status, text),
            false => from_response(status, text),
        }
        .map_err(Error::Decode)
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn upgrade_request_json_envelope() {
        let request = UpgradeRequest::new(Vec::new())
            .factory_default_mode(FactoryDefaultMode::None)
            .auto_commit(AutoCommit::Default)
            .auto_rollback(AutoRollback::Minutes(15));
        let json = serde_json::to_string_pretty(&request.json).unwrap();
        expect![[r#"
            {
              "apiVersion": "1.0",
              "method": "upgrade",
              "params": {
                "factoryDefaultMode": "none",
                "autoCommit": "default",
                "autoRollback": "15"
              }
            }"#]]
        .assert_eq(&json);
    }

    #[test]
    fn upgrade_request_minimal() {
        let request = UpgradeRequest::new(Vec::new());
        let json = serde_json::to_string_pretty(&request.json).unwrap();
        expect![[r#"
            {
              "apiVersion": "1.0",
              "method": "upgrade",
              "params": {}
            }"#]]
        .assert_eq(&json);
    }
}
