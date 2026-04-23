//! The [Firmware Management API].
//!
//! [Firmware Management API]: https://developer.axis.com/vapix/network-video/firmware-management-api/

use std::fmt::{Display, Formatter};

use anyhow::Context;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
    http::{Error, HttpClient, Request},
    json_rpc, json_rpc_http,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
    BadRequest = 400,
    NoPreviousFirmware = 404,
    UnknownMethod = 405,
    DowngradeNotAllowed = 409,
    FirmwareRevoked = 410,
    UncommittedFirmware = 412,
    InvalidImage = 415,
    IncompatibleApiVersion = 417,
    /// Also returned for otherwise invalid firmware binaries.
    ImageMismatch = 421,
    MissingSignature = 422,
    SystemBusy = 423,
    UnknownCertificate = 424,
    InternalError = 500,
}

impl TryFrom<u16> for ErrorKind {
    type Error = u16;

    fn try_from(code: u16) -> Result<Self, u16> {
        match code {
            400 => Ok(Self::BadRequest),
            404 => Ok(Self::NoPreviousFirmware),
            405 => Ok(Self::UnknownMethod),
            409 => Ok(Self::DowngradeNotAllowed),
            410 => Ok(Self::FirmwareRevoked),
            412 => Ok(Self::UncommittedFirmware),
            415 => Ok(Self::InvalidImage),
            417 => Ok(Self::IncompatibleApiVersion),
            421 => Ok(Self::ImageMismatch),
            422 => Ok(Self::MissingSignature),
            423 => Ok(Self::SystemBusy),
            424 => Ok(Self::UnknownCertificate),
            500 => Ok(Self::InternalError),
            _ => Err(code),
        }
    }
}

const PATH: &str = "axis-cgi/firmwaremanagement.cgi";

#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FactoryDefaultMode {
    Soft,
    Hard,
}

impl Display for FactoryDefaultMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
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
    pub fn new() -> Self {
        Self {
            api_version: "1.0",
            method: "factoryDefault",
            params: FactoryDefaultParams {
                factory_default_mode: FactoryDefaultMode::Soft,
            },
        }
    }

    pub fn hard(mut self) -> Self {
        self.params.factory_default_mode = FactoryDefaultMode::Hard;
        self
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<FactoryDefaultData, Error<json_rpc::Error>> {
        json_rpc_http::send_request(client, PATH, &self).await
    }
}

impl Default for FactoryDefaultRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AutoCommit {
    Never,
    Boot,
    Started,
}

impl Display for AutoCommit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Never => write!(f, "never"),
            Self::Boot => write!(f, "boot"),
            Self::Started => write!(f, "started"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum AutoRollback {
    Never,
    Minutes(u32),
}

impl Serialize for AutoRollback {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Never => serializer.serialize_str("never"),
            Self::Minutes(m) => serializer.serialize_str(&m.to_string()),
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

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<UpgradeData, Error<json_rpc::Error>> {
        let boundary = "----FormBoundaryS6untlhO8j7poXo";

        let json = serde_json::to_string(&self.json)
            .context("serialize request failed")
            .map_err(Error::Request)?;

        let body = Self::build_multipart_body(json.as_bytes(), &self.bin, boundary);

        let request = Request::new(Method::POST, PATH.to_string()).multipart(body, boundary);

        let response = client.execute(request).await.map_err(Error::Transport)?;

        json_rpc_http::from_response(response.status, response.body)
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn upgrade_request_json_envelope() {
        let request = UpgradeRequest::new(Vec::new())
            .factory_default_mode(FactoryDefaultMode::Soft)
            .auto_commit(AutoCommit::Never)
            .auto_rollback(AutoRollback::Minutes(15));
        let json = serde_json::to_string_pretty(&request.json).unwrap();
        expect![[r#"
            {
              "apiVersion": "1.0",
              "method": "upgrade",
              "params": {
                "factoryDefaultMode": "soft",
                "autoCommit": "never",
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
