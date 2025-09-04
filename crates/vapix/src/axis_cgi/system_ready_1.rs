//! The [Systemready API].
//!
//! [Systemready API]: https://developer.axis.com/vapix/network-video/systemready-api/

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::json_rpc_http::JsonRpcHttp;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EnglishBoolean {
    Yes,
    No,
}

impl Display for EnglishBoolean {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EnglishBoolean::Yes => write!(f, "yes"),
            EnglishBoolean::No => write!(f, "no"),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemreadyData {
    // TODO: Deserialize as real booleans
    pub needsetup: EnglishBoolean,
    pub systemready: EnglishBoolean,
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
