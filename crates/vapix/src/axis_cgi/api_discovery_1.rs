//! The [API Discovery] service.
//!
//! [API Discovery]: https://developer.axis.com/vapix/network-video/api-discovery-service/

use serde::{Deserialize, Serialize};

use crate::{
    http::{Error, HttpClient},
    json_rpc, json_rpc_http,
};

/// An identifier used with [`ApiListData`] to determine if an API exists and,if so, what version
/// it is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiId(&'static str);

impl ApiId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiListData {
    pub api_list: Vec<Api>,
}

impl ApiListData {
    pub fn find(&self, id: ApiId) -> Option<&Api> {
        self.api_list.iter().find(|a| a.id == id.0)
    }

    pub fn is_supported(&self, id: ApiId, req: &str) -> Result<bool, semver::Error> {
        let req = semver::VersionReq::parse(req)?;
        match self.find(id) {
            Some(api) => api.parse_version().map(|v| req.matches(&v)),
            None => Ok(false),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Api {
    pub id: String,
    pub version: String,
    pub name: String,
    pub doc_link: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetApiListRequest {
    api_version: &'static str,
    method: &'static str,
}

impl Default for GetApiListRequest {
    fn default() -> Self {
        Self {
            api_version: "1.0",
            method: "getApiList",
        }
    }
}

const PATH: &str = "axis-cgi/apidiscovery.cgi";

impl GetApiListRequest {
    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<ApiListData, Error<json_rpc::Error>> {
        json_rpc_http::send_request(client, PATH, &self).await
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedVersionsData {
    pub api_versions: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSupportedVersionsRequest {
    method: &'static str,
}

impl Default for GetSupportedVersionsRequest {
    fn default() -> Self {
        Self {
            method: "getSupportedVersions",
        }
    }
}

impl GetSupportedVersionsRequest {
    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<SupportedVersionsData, Error<json_rpc::Error>> {
        json_rpc_http::send_request(client, PATH, &self).await
    }
}

impl Api {
    pub fn parse_version(&self) -> Result<semver::Version, semver::Error> {
        // API versions are typically major.minor; coerce to semver by appending .0
        let v = &self.version;
        let semver_str = match v.matches('.').count() {
            1 => format!("{v}.0"),
            _ => v.clone(),
        };
        semver::Version::parse(&semver_str)
    }

    pub fn parse_status(&self) -> Result<Option<ApiStatus>, anyhow::Error> {
        self.status
            .as_deref()
            .map(|s| s.parse::<ApiStatus>())
            .transpose()
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiStatus {
    Official,
    Alpha,
    Beta,
    Deprecated,
}

impl std::str::FromStr for ApiStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "official" => Ok(Self::Official),
            "alpha" => Ok(Self::Alpha),
            "beta" => Ok(Self::Beta),
            "deprecated" => Ok(Self::Deprecated),
            _ => Err(anyhow::anyhow!("unrecognized API status '{s}'")),
        }
    }
}
